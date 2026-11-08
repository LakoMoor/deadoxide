//! Window-free scene: turns real cells into sprite instances (world-pixel rects + atlas UVs) and a collision grid.
use crate::sim::{Collision, BLOCK, EDGE_N, EDGE_W};
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;
use zomboid_content::{
    atlas::{build_atlas, Atlas},
    lot::*,
    tiledefs::TileDefs,
};

/// Instance layout matches the shader: rect (x,y,w,h in world px), uv (u0,v0,u1,v1), atlas layer.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub layer: f32,
}

pub struct Scene {
    pub atlas: Atlas,
    pub instances: Vec<Instance>,
    /// instance index where each level starts (len = levels + 1)
    pub level_ranges: Vec<u32>,
    pub unresolved: u64,
    pub collision: Collision,
    /// per instance: (gx, gy, z, room_id), parallel to `instances` (used by `retain_room`)
    pub meta: Vec<(i32, i32, i32, i32)>,
    /// room id of every square that has one (room_id >= 0)
    pub rooms: std::collections::HashMap<(i32, i32, i32), i32>,
    pub clear: [f32; 3],
}

pub const TILE_W: i32 = 128;
pub const TILE_H: i32 = 64;
pub const LEVEL_PX: i32 = 192;

/// Isometric world-pixel position of the top vertex of tile (gx, gy) on level z.
pub fn iso(gx: f32, gy: f32, z: i32) -> [f32; 2] {
    [
        (gx - gy) * (TILE_W as f32 / 2.0),
        (gx + gy) * (TILE_H as f32 / 2.0) - (z * LEVEL_PX) as f32,
    ]
}

impl Scene {
    /// Instance for a solid marker (the reserved `__player` atlas block) standing at tile (x, y).
    pub fn marker(&self, name: &str, x: f32, y: f32, z: i32) -> Instance {
        let s = self.atlas.sprites[name];
        let (aw, ah) = (self.atlas.width as f32, self.atlas.height as f32);
        let p = iso(x, y, z);
        let (w, h) = (24.0, 96.0);
        Instance {
            rect: [p[0] - w / 2.0, p[1] - h, w, h], // iso() of a fractional position is already the tile centre = feet
            uv: [
                (s.ax as f32 + 1.0) / aw,
                (s.ay as f32 + 1.0) / ah,
                (s.ax as f32 + 7.0) / aw,
                (s.ay as f32 + 7.0) / ah,
            ],
            layer: s.layer as f32,
        }
    }
}

struct Cell {
    names: Vec<String>,
    chunks: Vec<Chunk>,
    min_level: i32,
    max_level: i32,
    rooms: Vec<Room>,
}

fn read_cell(root: &Path, cx: i32, cy: i32) -> Result<Option<Cell>> {
    let dir = root.join("media/maps/Muldraugh, KY");
    let hp = dir.join(format!("{cx}_{cy}.lotheader"));
    let pp = dir.join(format!("world_{cx}_{cy}.lotpack"));
    if !hp.exists() || !pp.exists() {
        return Ok(None);
    }
    let hb = std::fs::read(hp)?;
    let h = parse_lotheader(&hb).map_err(|e| anyhow!("{e}"))?;
    let d = parse_lotheader_dims(&hb, &h).map_err(|e| anyhow!("{e}"))?;
    let pb = std::fs::read(pp)?;
    let pk = parse_lotpack(&pb).map_err(|e| anyhow!("{e}"))?;
    let mut chunks = Vec::with_capacity(1024);
    for off in &pk.offsets {
        chunks.push(
            parse_chunk(&pb, *off as usize, d.min_level, d.max_level)
                .map_err(|e| anyhow!("{e}"))?,
        );
    }
    let (rooms, _) = parse_rooms(&hb, &h).map_err(|e| anyhow!("{e}"))?;
    Ok(Some(Cell {
        names: h.tile_names,
        chunks,
        min_level: d.min_level,
        max_level: d.max_level,
        rooms,
    }))
}

/// Load cells `x0..=x1` x `y0..=y1` (cell coordinates) into one scene in global painter's order.
pub fn load_region(root: &Path, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<Scene> {
    let mut cells = std::collections::HashMap::new();
    let mut wanted: HashSet<String> = HashSet::new();
    let (mut zmin, mut zmax) = (i32::MAX, i32::MIN);
    for cy in y0..=y1 {
        for cx in x0..=x1 {
            if let Some(c) = read_cell(root, cx, cy)? {
                wanted.extend(c.names.iter().cloned());
                zmin = zmin.min(c.min_level);
                zmax = zmax.max(c.max_level);
                cells.insert((cx, cy), c);
            }
        }
    }
    if cells.is_empty() {
        return Err(anyhow!("no cells in region {x0},{y0}..{x1},{y1}"));
    }
    let atlas = build_atlas(root, &wanted, 8192, 8192).map_err(|e| anyhow!(e))?;
    let defs = TileDefs::load(root)?;
    let mut collision = Collision::default();
    let (aw, ah) = (atlas.width as f32, atlas.height as f32);
    let mut instances = Vec::new();
    let mut level_ranges = vec![];
    let mut unresolved = 0u64;
    let mut meta = Vec::new();
    // rooms come from the cell header (rect lists per floor), numbered uniquely across the region
    let mut rooms = std::collections::HashMap::new();
    let mut next_room = 0;
    for ((cx, cy), cell) in &cells {
        for r in &cell.rooms {
            for &(rx, ry, rw, rh) in &r.rects {
                for yy in ry..ry + rh {
                    for xx in rx..rx + rw {
                        rooms.insert((cx * 256 + xx, cy * 256 + yy, r.level), next_room);
                    }
                }
            }
            next_room += 1;
        }
    }
    for z in zmin..=zmax {
        level_ranges.push(instances.len() as u32);
        for gy in y0 * 256..(y1 + 1) * 256 {
            for gx in x0 * 256..(x1 + 1) * 256 {
                let Some(cell) = cells.get(&(gx.div_euclid(256), gy.div_euclid(256))) else {
                    continue;
                };
                if z < cell.min_level || z > cell.max_level {
                    continue;
                }
                let (sx, sy) = (gx.rem_euclid(256), gy.rem_euclid(256));
                let Some(sq) = &cell.chunks[((sx / 8) * 32 + sy / 8) as usize].levels
                    [(z - cell.min_level) as usize][((sx % 8) * 8 + sy % 8) as usize]
                else {
                    continue;
                };
                let p = iso(gx as f32, gy as f32, z);
                let (px, py) = (p[0] as i32, p[1] as i32);
                for &t in &sq.tiles {
                    let nm = &cell.names[t as usize];
                    let mut fl = 0u8;
                    if defs.has(nm, "solid") || defs.has(nm, "solidtrans") {
                        fl |= BLOCK;
                    }
                    if defs.has(nm, "WallN") || defs.has(nm, "collideN") {
                        fl |= EDGE_N;
                    }
                    if defs.has(nm, "WallW") || defs.has(nm, "collideW") {
                        fl |= EDGE_W;
                    }
                    if fl != 0 {
                        collision.add(gx, gy, z, fl);
                    }
                    let Some(s) = atlas.sprites.get(nm) else {
                        unresolved += 1;
                        continue;
                    };
                    let x = (px - s.orig_w / 2 + s.off_x) as f32;
                    let y = (py + TILE_H - s.orig_h + s.off_y) as f32;
                    meta.push((gx, gy, z, rooms.get(&(gx, gy, z)).copied().unwrap_or(-1)));
                    instances.push(Instance {
                        rect: [x, y, s.w as f32, s.h as f32],
                        uv: [
                            s.ax as f32 / aw,
                            s.ay as f32 / ah,
                            (s.ax as i32 + s.w) as f32 / aw,
                            (s.ay as i32 + s.h) as f32 / ah,
                        ],
                        layer: s.layer as f32,
                    });
                }
            }
        }
    }
    level_ranges.push(instances.len() as u32);
    Ok(Scene {
        atlas,
        instances,
        level_ranges,
        unresolved,
        collision,
        meta,
        rooms,
        clear: [20.0 / 255.0, 24.0 / 255.0, 20.0 / 255.0],
    })
}

impl Scene {
    /// Debug overlay: outline dots (5x5 px, atlas block `name`) along the edges of zones, sampled every `step` tiles,
    /// limited to zones that touch the tile box `bx = (x0, y0, x1, y1)`. Rectangles use x..x+w, y..y+h; polygons use their points.
    pub fn zone_dots(
        &self,
        name: &str,
        zones: &[zomboid_content::zones::Zone],
        kind: &str,
        bx: (i32, i32, i32, i32),
        z: i32,
        step: f32,
    ) -> Vec<Instance> {
        let s = self.atlas.sprites[name];
        let (aw, ah) = (self.atlas.width as f32, self.atlas.height as f32);
        let uv = [
            (s.ax as f32 + 1.0) / aw,
            (s.ay as f32 + 1.0) / ah,
            (s.ax as f32 + 4.0) / aw,
            (s.ay as f32 + 4.0) / ah,
        ];
        let mut out = vec![];
        let mut seg = |a: (f32, f32), b: (f32, f32)| {
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let n = (len / step).ceil().max(1.0) as i32;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let (x, y) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                if x < bx.0 as f32 || x > bx.2 as f32 || y < bx.1 as f32 || y > bx.3 as f32 {
                    continue;
                }
                let p = iso(x, y, z);
                out.push(Instance {
                    rect: [p[0] - 2.5, p[1] - 2.5, 5.0, 5.0],
                    uv,
                    layer: s.layer as f32,
                });
            }
        };
        for q in zones.iter().filter(|q| q.kind == kind && q.z == z) {
            let pts: Vec<(f32, f32)> = if q.points.is_empty() {
                let (x, y, w, h) = (q.x as f32, q.y as f32, q.w as f32, q.h as f32);
                vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
            } else {
                q.points.iter().map(|p| (p.0 as f32, p.1 as f32)).collect()
            };
            for i in 0..pts.len() {
                seg(pts[i], pts[(i + 1) % pts.len()]);
            }
        }
        out
    }
}

impl Scene {
    /// Visibility mask measured against the real game (night, player indoors): squares outside the player's room are black.
    /// Keeps instances whose square is in the player's room, plus room-less squares touching it (walls/doors), at every level.
    /// Does nothing if the player's square has no room. Light spill from sources (door cone) is NOT modelled.
    pub fn retain_room(&mut self, px: f32, py: f32, z: i32) -> Option<i32> {
        let rid = *self.rooms.get(&(px.floor() as i32, py.floor() as i32, z))?;
        // "Cutaway" rule measured on two real frames: a room-less square is kept only when the player's room lies on its +x or +y side,
        // i.e. walls on the north/west edges. Walls on the south/east edges (facing the camera) are not drawn by the real game.
        let near = |x: i32, y: i32, zz: i32| {
            [(0, 0), (1, 0), (0, 1)]
                .iter()
                .any(|&(dx, dy)| self.rooms.get(&(x + dx, y + dy, zz)) == Some(&rid))
        };
        let keep: Vec<bool> = self
            .meta
            .iter()
            .map(|&(x, y, zz, r)| r == rid || (r < 0 && near(x, y, zz)))
            .collect();
        let (mut inst, mut meta) = (vec![], vec![]);
        for (i, k) in keep.iter().enumerate() {
            if *k {
                inst.push(self.instances[i]);
                meta.push(self.meta[i]);
            }
        }
        // rebuild level starts from the surviving instances' z (instances are already ordered by level)
        let (zmin, n) = (
            self.meta.iter().map(|m| m.2).min().unwrap_or(0),
            self.level_ranges.len() - 1,
        );
        let starts: Vec<u32> = (0..=n)
            .map(|lv| {
                meta.iter()
                    .take_while(|m| ((m.2 - zmin) as usize) < lv)
                    .count() as u32
            })
            .collect();
        self.instances = inst;
        self.meta = meta;
        self.level_ranges = starts;
        self.clear = [0.0, 0.0, 0.0];
        Some(rid)
    }
}
