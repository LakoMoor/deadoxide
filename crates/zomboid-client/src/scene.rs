//! Window-free scene: turns real cells into sprite instances (world-pixel rects + atlas UVs) and a collision grid.
use crate::sim::{
    Collision, BLOCK, EDGE_N, EDGE_W, HOPPABLE, SOLID, SOLID_TRANS, WATER, WINDOW, WINDOW_N,
    WINDOW_W,
};
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

/// zombie.iso.IsoGridSquare.CalculateCollide: collect the measured static square flags.
/// Doors, windows, stairs and dynamic objects still need their own collision rules.
fn tile_collision_flags(defs: &TileDefs, name: &str) -> u16 {
    let mut flags = 0;
    if defs.has(name, "solid") || defs.has(name, "solidtrans") {
        flags |= BLOCK;
    }
    if defs.has(name, "solid") {
        flags |= SOLID;
    }
    if defs.has(name, "solidtrans") {
        flags |= SOLID_TRANS;
    }
    if defs.has(name, "water") {
        flags |= WATER;
    }
    if ["windowN", "windowW", "WindowN", "WindowW"]
        .iter()
        .any(|prop| defs.has(name, prop))
    {
        flags |= WINDOW;
    }
    if defs.has(name, "windowN") || defs.has(name, "WindowN") {
        flags |= WINDOW_N;
    }
    if defs.has(name, "windowW") || defs.has(name, "WindowW") {
        flags |= WINDOW_W;
    }
    if defs.has(name, "HoppableN") || defs.has(name, "HoppableW") {
        flags |= HOPPABLE;
    }
    if defs.has(name, "collideN") {
        flags |= EDGE_N;
    }
    if defs.has(name, "collideW") {
        flags |= EDGE_W;
    }
    flags
}

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
                    let fl = tile_collision_flags(&defs, nm);
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
    /// zombie.iso.IsoCell.SetCutawayRoomsForPlayer (read from the class file, see docs/COMPATIBILITY.md): the real game shows the player's room
    /// plus the rooms at (x+-1.5, y+-1.5) that the player can see and that lie in front of the facing direction. `probes` adds the four
    /// probe squares WITHOUT the can-see and facing tests (the port has neither yet), so it can over-reveal.
    pub fn retain_rooms(
        &mut self,
        px: f32,
        py: f32,
        z: i32,
        probes: bool,
        los: Option<f32>,
    ) -> Option<i32> {
        let rid = *self.rooms.get(&(px.floor() as i32, py.floor() as i32, z))?;
        let mut set = vec![rid];
        if probes {
            for (dx, dy) in [(-1.5f32, -1.5f32), (1.5, -1.5), (-1.5, 1.5), (1.5, 1.5)] {
                if let Some(&r) =
                    self.rooms
                        .get(&((px + dx).floor() as i32, (py + dy).floor() as i32, z))
                {
                    if r >= 0 && !set.contains(&r) {
                        set.push(r);
                    }
                }
            }
        }
        // "Cutaway" rule measured on two real frames: a room-less square is kept only when the player's room lies on its +x or +y side,
        // i.e. walls on the north/west edges. Walls on the south/east edges (facing the camera) are not drawn by the real game.
        let near = |x: i32, y: i32, zz: i32| {
            [(0, 0), (1, 0), (0, 1)]
                .iter()
                .any(|&(dx, dy)| matches!(self.rooms.get(&(x + dx, y + dy, zz)), Some(r) if set.contains(r)))
        };
        // EXPERIMENT: squares within `radius` that the player has line of sight to (walls block; see `Collision::los`).
        let mut seen = std::collections::HashSet::new();
        if let Some(rad) = los {
            let r = rad.ceil() as i32;
            for x in (px.floor() as i32 - r)..=(px.floor() as i32 + r) {
                for y in (py.floor() as i32 - r)..=(py.floor() as i32 + r) {
                    let (cx, cy) = (x as f32 + 0.5 - px, y as f32 + 0.5 - py);
                    if cx * cx + cy * cy <= rad * rad && self.collision.los((px, py), (x, y), z) {
                        seen.insert((x, y));
                    }
                }
            }
        }
        let keep: Vec<bool> = self
            .meta
            .iter()
            .map(|&(x, y, zz, r)| {
                set.contains(&r) || (r < 0 && near(x, y, zz)) || (zz == z && seen.contains(&(x, y)))
            })
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

#[cfg(test)]
mod collision_oracle_tests {
    use super::*;
    use zomboid_content::tiledefs::Props;

    #[test]
    #[ignore = "real-content acceptance: loads a complete cell and texture atlas"]
    fn real_water_square_is_wired_into_loaded_scene() -> anyhow::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let cell = read_cell(&root, 0, 18)?.ok_or_else(|| anyhow!("cell 0_18 missing"))?;
        let water_id =
            cell.names
                .iter()
                .position(|name| name == "blends_natural_02_0")
                .ok_or_else(|| anyhow!("water tile missing from cell header"))? as i32;
        let mut found = None;
        'outer: for sx in 1..255 {
            for sy in 1..255 {
                let chunk = &cell.chunks[((sx / 8) * 32 + sy / 8) as usize];
                let Some(square) =
                    &chunk.levels[(0 - cell.min_level) as usize][((sx % 8) * 8 + sy % 8) as usize]
                else {
                    continue;
                };
                if square.tiles.contains(&water_id) {
                    found = Some((sx, 18 * 256 + sy));
                    break 'outer;
                }
            }
        }
        let (x, y) = found.ok_or_else(|| anyhow!("water tile not used on z=0"))?;
        let scene = load_region(&root, 0, 18, 0, 18)?;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            assert!(
                !scene.collision.can_step((x, y), (x + dx, y + dy), 0),
                "water source at {x},{y}, direction {dx},{dy}"
            );
        }
        println!("real water square: {x},{y},0");
        Ok(())
    }

    #[test]
    fn real_water_tile_uses_original_measured_collision_flags() -> anyhow::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let defs = TileDefs::load(&root)?;
        // The installed newtiledefinitions.tiles.txt declares these three
        // properties together. WaterProbe records blocked entry and exit.
        let tile = "blends_natural_02_0";
        for prop in ["water", "solidtrans", "solidfloor"] {
            assert!(defs.has(tile, prop), "{tile} missing {prop}");
        }
        let mut c = Collision::default();
        c.add(1, 0, 0, tile_collision_flags(&defs, tile));
        assert!(!c.can_step((0, 0), (1, 0), 0));
        assert!(!c.can_step((1, 0), (0, 0), 0));
        Ok(())
    }

    #[test]
    fn water_solidtrans_window_destination_blocks_entry() {
        // WaterWindowProbe: eastward cases 48 and 56 differ only by water.
        let line = include_str!("../../../tests/golden/collision_water_window.txt")
            .lines()
            .next()
            .expect("east oracle");
        let (direction, bits) = line.split_once(':').expect("oracle line");
        assert_eq!(direction, "1,0");
        let mut defs = TileDefs::default();
        for name in ["water", "solidtrans", "windowN"] {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        for mask in [48usize, 56] {
            let mut c = Collision::default();
            for (bit, name) in ["water", "solidtrans", "windowN"].iter().enumerate() {
                if mask & (1 << (bit + 3)) != 0 {
                    c.add(1, 0, 0, tile_collision_flags(&defs, name));
                }
            }
            assert_eq!(
                !c.can_step((0, 0), (1, 0), 0),
                bits.as_bytes()[mask] == b'1'
            );
        }
    }

    #[test]
    fn water_and_window_on_both_squares_match_original() {
        let names = ["water", "solidtrans", "windowN"];
        let mut defs = TileDefs::default();
        for name in names {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        let mut count = 0;
        for line in include_str!("../../../tests/golden/collision_water_window.txt").lines() {
            let (direction, bits) = line.split_once(':').expect("oracle line");
            let (dx, dy) = direction.split_once(',').expect("direction");
            let to = (
                dx.parse::<i32>().expect("dx"),
                dy.parse::<i32>().expect("dy"),
            );
            assert_eq!(bits.len(), 64);
            for (mask, expected) in bits.bytes().enumerate() {
                let mut c = Collision::default();
                for (square, shift) in [((0, 0), 0), (to, 3)] {
                    for (bit, name) in names.iter().enumerate() {
                        if mask & (1 << (shift + bit)) != 0 {
                            c.add(square.0, square.1, 0, tile_collision_flags(&defs, name));
                        }
                    }
                }
                assert_eq!(
                    !c.can_step((0, 0), to, 0),
                    expected == b'1',
                    "{direction} mask {mask}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 256);
    }

    #[test]
    fn adjacent_window_and_hoppable_flags_match_original() {
        let names = [
            "solidtrans",
            "windowN",
            "windowW",
            "WindowN",
            "WindowW",
            "HoppableN",
            "HoppableW",
        ];
        let mut defs = TileDefs::default();
        for name in names {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        let mut count = 0;
        for line in include_str!("../../../tests/golden/collision_adjacent_window.txt").lines() {
            let (input, expected) = line.split_once(':').expect("oracle line");
            let values: Vec<i32> = input
                .split(',')
                .map(|s| s.parse().expect("oracle integer"))
                .collect();
            assert_eq!(values.len(), 3);
            let (x, y, mask) = (values[0], values[1], values[2]);
            let mut c = Collision::default();
            c.add(0, 0, 0, tile_collision_flags(&defs, "solidtrans"));
            for (bit, name) in names.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    c.add(x, y, 0, tile_collision_flags(&defs, name));
                }
            }
            assert_eq!(!c.can_step((-1, 0), (0, 0), 0), expected == "1", "{input}");
            count += 1;
        }
        assert_eq!(count, 1152);
    }

    #[test]
    fn adjacent_window_flags_match_original_all_directions() {
        let names = [
            "solidtrans",
            "windowN",
            "windowW",
            "WindowN",
            "WindowW",
            "HoppableN",
            "HoppableW",
        ];
        let mut defs = TileDefs::default();
        for name in names {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        let mut count = 0;
        for line in
            include_str!("../../../tests/golden/collision_adjacent_window_directions.txt").lines()
        {
            let (input, expected) = line.split_once(':').expect("oracle line");
            let values: Vec<i32> = input
                .split(',')
                .map(|s| s.parse().expect("oracle integer"))
                .collect();
            assert_eq!(values.len(), 5);
            let (sx, sy, x, y, mask) = (values[0], values[1], values[2], values[3], values[4]);
            let mut c = Collision::default();
            c.add(0, 0, 0, tile_collision_flags(&defs, "solidtrans"));
            for (bit, name) in names.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    c.add(x, y, 0, tile_collision_flags(&defs, name));
                }
            }
            assert_eq!(!c.can_step((sx, sy), (0, 0), 0), expected == "1", "{input}");
            count += 1;
        }
        assert_eq!(count, 9216);
    }

    #[test]
    fn window_and_solidtrans_match_original_square_cases() {
        let names = [
            "solidtrans",
            "windowN",
            "windowW",
            "WindowN",
            "WindowW",
            "collideN",
            "collideW",
            "transparentN",
            "transparentW",
        ];
        let mut defs = TileDefs::default();
        for name in names {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        let mut checked = 0;
        for line in include_str!("../../../tests/golden/collision_window_solid.txt").lines() {
            let (direction, bits) = line.split_once(':').expect("oracle line");
            let (dx, dy) = direction.split_once(',').expect("oracle direction");
            let to = (
                dx.parse::<i32>().expect("dx"),
                dy.parse::<i32>().expect("dy"),
            );
            assert_eq!(bits.len(), 512);
            for (mask, expected) in bits.bytes().enumerate() {
                let mut collision = Collision::default();
                for (bit, name) in names.iter().enumerate() {
                    if mask & (1 << bit) != 0 {
                        collision.add(to.0, to.1, 0, tile_collision_flags(&defs, name));
                    }
                }
                assert_eq!(
                    !collision.can_step((0, 0), to, 0),
                    expected == b'1',
                    "direction={direction} mask={mask}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 4096);
    }

    #[test]
    fn mixed_solid_and_window_flags_match_original_square_cases() {
        let names = [
            "solid",
            "solidtrans",
            "windowN",
            "windowW",
            "WindowN",
            "WindowW",
            "collideN",
            "collideW",
            "transparentN",
            "transparentW",
        ];
        let mut defs = TileDefs::default();
        for name in names {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        let mut checked = 0;
        for line in include_str!("../../../tests/golden/collision_mixed_solid_window.txt").lines() {
            let (direction, bits) = line.split_once(':').expect("oracle line");
            let (dx, dy) = direction.split_once(',').expect("oracle direction");
            let to = (
                dx.parse::<i32>().expect("dx"),
                dy.parse::<i32>().expect("dy"),
            );
            assert_eq!(bits.len(), 1024);
            for (mask, expected) in bits.bytes().enumerate() {
                let mut collision = Collision::default();
                for (bit, name) in names.iter().enumerate() {
                    if mask & (1 << bit) != 0 {
                        collision.add(to.0, to.1, 0, tile_collision_flags(&defs, name));
                    }
                }
                assert_eq!(
                    !collision.can_step((0, 0), to, 0),
                    expected == b'1',
                    "direction={direction} mask={mask}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 8192);
    }

    #[test]
    fn water_and_solidtrans_match_original_cardinal_samples() {
        // The original-side fixture uses water, solidtrans and solidfloor on
        // the source and destination; model each property as a separate tile.
        let mut defs = TileDefs::default();
        for name in ["water", "solidtrans", "solidfloor"] {
            defs.tiles
                .insert(name.into(), Props::from([(name.into(), String::new())]));
        }
        let mut checked = 0;
        for line in include_str!("../../../tests/golden/collision_water.txt").lines() {
            let (direction, bits) = line.split_once(':').expect("oracle line");
            let (dx, dy) = direction.split_once(',').expect("oracle direction");
            let to = (
                dx.parse::<i32>().expect("dx"),
                dy.parse::<i32>().expect("dy"),
            );
            assert_eq!(bits.len(), 64);
            for (mask, expected) in bits.bytes().enumerate() {
                let mut collision = Collision::default();
                for (square, shift) in [((0, 0), 0), (to, 3)] {
                    for (bit, name) in ["water", "solidtrans", "solidfloor"]
                        .into_iter()
                        .enumerate()
                    {
                        if mask & (1 << (shift + bit)) != 0 {
                            collision.add(square.0, square.1, 0, tile_collision_flags(&defs, name));
                        }
                    }
                }
                assert_eq!(
                    !collision.can_step((0, 0), to, 0),
                    expected == b'1',
                    "direction={direction} mask={mask}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 256);
    }

    #[test]
    fn static_square_flags_match_original_adjacent_samples() {
        // Expected blocked states came from tests/original_probe/CollisionProbe.java.
        // Each pair is forward/reverse, with the flags on the eastern or southern square.
        type Case = (&'static [&'static str], (bool, bool), (bool, bool));
        let cases: &[Case] = &[
            (&[], (false, false), (false, false)),
            (&["solid"], (true, false), (true, false)),
            (&["solidtrans"], (true, false), (true, false)),
            (&["collideW"], (true, true), (false, false)),
            (&["collideN"], (false, false), (true, true)),
            (&["WallW"], (false, false), (false, false)),
            (&["WallN"], (false, false), (false, false)),
            (&["doorW"], (false, false), (false, false)),
            (&["windowW"], (false, false), (false, false)),
            (&["collideW", "doorW"], (true, true), (false, false)),
            (&["collideW", "windowW"], (true, true), (false, false)),
            (&["WallW", "collideW"], (true, true), (false, false)),
            (&["WallN", "collideN"], (false, false), (true, true)),
        ];
        for (properties, east, south) in cases {
            let mut defs = TileDefs::default();
            let props: Props = properties
                .iter()
                .map(|p| (p.to_string(), String::new()))
                .collect();
            defs.tiles.insert("probe".into(), props);
            for (to, expected) in [((1, 0), east), ((0, 1), south)] {
                let mut collision = Collision::default();
                collision.add(to.0, to.1, 0, tile_collision_flags(&defs, "probe"));
                let result = (
                    !collision.can_step((0, 0), to, 0),
                    !collision.can_step(to, (0, 0), 0),
                );
                assert_eq!(result, *expected, "properties={properties:?}, to={to:?}");
            }
        }
    }
}
