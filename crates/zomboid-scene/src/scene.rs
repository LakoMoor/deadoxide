//! Window-free scene: turns real cells into sprite instances (world-pixel rects + atlas UVs) and a collision grid.
use crate::sim::{
    Collision, BLOCK, EDGE_N, EDGE_W, HOPPABLE, SOLID, SOLID_TRANS, WATER, WINDOW, WINDOW_N,
    WINDOW_W,
};
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;
use zomboid_content::atlas::{build_atlas_from, pack_order, Atlas};
#[cfg(test)]
use zomboid_content::tiledefs::TileDefs;
use zomboid_engine::java;
use zomboid_engine::vfs;
use zomboid_engine::zombie::core::random::rand_abstract::RandAbstract;
use zomboid_engine::zombie::iso::cell_loader::{self, LoadContext};
use zomboid_engine::zombie::iso::iso_chunk::IsoChunk;
use zomboid_engine::zombie::iso::iso_grid_square::{
    line_clear, GetSquare, IsoGridSquare, TestResults,
};
use zomboid_engine::zombie::iso::iso_meta_grid::AlarmSettings;
use zomboid_engine::zombie::iso::iso_object::ObjectKind;
use zomboid_engine::zombie::iso::iso_world::IsoWorld;
use zomboid_engine::zombie::iso::sprite::iso_sprite_manager::SpriteRef;
use zomboid_engine::zombie::iso::sprite_details::iso_flag_type::IsoFlagType;

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
    /// World z corresponding to the first entry in level_ranges.
    pub min_level: i32,
    pub unresolved: u64,
    pub collision: Collision,
    /// per instance: (gx, gy, z, room_id), parallel to `instances` (used by `retain_room`)
    pub meta: Vec<(i32, i32, i32, i32)>,
    /// room id of every square that has one (room_id >= 0)
    pub rooms: std::collections::HashMap<(i32, i32, i32), i32>,
    pub clear: [f32; 3],
    /// Present for scenes built by `load_region`.
    pub state: Option<SceneState>,
}

pub const TILE_W: i32 = 128;
pub const TILE_H: i32 = 64;
pub const LEVEL_PX: i32 = 192;

/// zombie.iso.IsoGridSquare.CalculateCollide: collect the measured static square flags.
/// Doors, windows, stairs and dynamic objects still need their own collision rules.
fn collision_flags(has: impl Fn(&str) -> bool) -> u16 {
    let mut flags = 0;
    if has("solid") || has("solidtrans") {
        flags |= BLOCK;
    }
    if has("solid") {
        flags |= SOLID;
    }
    if has("solidtrans") {
        flags |= SOLID_TRANS;
    }
    if has("water") {
        flags |= WATER;
    }
    if ["windowN", "windowW", "WindowN", "WindowW"]
        .iter()
        .any(|prop| has(prop))
    {
        flags |= WINDOW;
    }
    if has("windowN") || has("WindowN") {
        flags |= WINDOW_N;
    }
    if has("windowW") || has("WindowW") {
        flags |= WINDOW_W;
    }
    if has("HoppableN") || has("HoppableW") {
        flags |= HOPPABLE;
    }
    if has("collideN") {
        flags |= EDGE_N;
    }
    if has("collideW") {
        flags |= EDGE_W;
    }
    flags
}

/// The same mapping over raw `.tiles.txt` properties (the synthetic oracle tests use it).
#[cfg(test)]
fn tile_collision_flags(defs: &TileDefs, name: &str) -> u16 {
    collision_flags(|prop| defs.has(name, prop))
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

/// Collision flags of one object's sprite, read from the properties the ported tile-definition loader derived
/// (`IsoWorld.LoadTileDefinitions`), e.g. `WallN` implies `collideN` there.
fn sprite_collision_flags(world: &IsoWorld, spr: SpriteRef) -> u16 {
    let p = &world.sprites.sprite(spr).properties;
    collision_flags(|name| match IsoFlagType::from_string(name) {
        IsoFlagType::MAX => p.has_str(&world.alias, name),
        f => p.has(f) || p.has_str(&world.alias, name),
    })
}

/// World seed used until the client reads `map_worldgen.bin` of a save or creates worlds itself: the Java hash of
/// the oracle's seed text "DeadOxideOracle", so basements match the reference server world.
const WORLD_SEED: i64 = -1594728997;

struct RegionGrid<'a>(std::collections::HashMap<(i32, i32, i32), &'a IsoGridSquare>);
impl GetSquare for RegionGrid<'_> {
    fn get_grid_square(&self, x: i32, y: i32, z: i32) -> Option<&IsoGridSquare> {
        self.0.get(&(x, y, z)).copied()
    }
}

/// Load cells `x0..=x1` x `y0..=y1` (cell coordinates) into one scene in global painter's order.
/// Squares and objects come from the direct port (`zomboid-engine`): `CellLoader.LoadCellBinaryChunk` per chunk, so
/// object order, overlays, open curtains, removed trees and table stacking are the original's. `Rand` is seeded with a
/// fixed value here; random fields (locks, tints) are therefore the same on every run and tints are not drawn yet.
pub fn load_region(root: &Path, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<Scene> {
    let mut rand = RandAbstract::new(java::Random::new(1));
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 4,
        alarm_decay: 1,
    };
    let mut world = IsoWorld::init(
        root,
        "Muldraugh, KY",
        &|_| true,
        &alarm,
        Some(WORLD_SEED),
        &mut rand,
    )
    .map_err(|e| anyhow!(e))?;
    for l in &world.log {
        log::warn!("{l}");
    }
    let IsoWorld {
        sprites,
        alias,
        tile_definitions,
        meta_grid,
        ..
    } = &mut world;
    let mut ctx = LoadContext {
        sprites,
        alias,
        glass_removed_window_sprite_map: &tile_definitions.glass_removed_window_sprite_map,
        smashed_window_sprite_map: &tile_definitions.smashed_window_sprite_map,
        meta_grid,
        rand: &mut rand,
        locked_houses: 4,
        missing_tiles: Vec::new(),
        lotpack_cache: None,
    };
    let mut chunks = Vec::new();
    let (mut zmin, mut zmax) = (i32::MAX, i32::MIN);
    for cx in x0..=x1 {
        for cy in y0..=y1 {
            if ctx.meta_grid.get_header(cx, cy).is_none() {
                continue;
            }
            for wx in cx * 32..cx * 32 + 32 {
                for wy in cy * 32..cy * 32 + 32 {
                    let mut ch = IsoChunk::new(wx, wy);
                    let mut errors = Vec::new();
                    cell_loader::load_cell_binary_chunk(&mut ctx, wx, wy, &mut ch, &mut errors);
                    for e in errors {
                        log::warn!("chunk {wx},{wy}: {e}");
                    }
                    zmin = zmin.min(ch.get_min_level());
                    zmax = zmax.max(ch.get_max_level());
                    chunks.push(ch);
                }
            }
        }
    }
    if chunks.is_empty() {
        return Err(anyhow!("no cells in region {x0},{y0}..{x1},{y1}"));
    }
    let missing = ctx.missing_tiles.len() as u64;
    drop(ctx);
    // zombie.iso.IsoGridSquare.RecalcProperties for every square, then the collide matrices over the loaded region.
    for ch in &mut chunks {
        for z in ch.get_min_level()..=ch.get_max_level() {
            for i in 0..64 {
                if let Some(sq) = ch.get_grid_square_mut(i % 8, i / 8, z) {
                    sq.recalc_properties(&world.sprites);
                }
            }
        }
    }

    let mut wanted: HashSet<String> = HashSet::new();
    for ch in &chunks {
        for z in ch.get_min_level()..=ch.get_max_level() {
            for i in 0..64 {
                let Some(sq) = ch.get_grid_square(i % 8, i / 8, z) else {
                    continue;
                };
                for o in &sq.objects {
                    for s in o
                        .sprite
                        .iter()
                        .chain(o.attached_anim_sprite.iter().flatten())
                    {
                        wanted.extend(world.sprites.sprite(*s).name.clone());
                    }
                    // both states of a door, so that it can be opened later
                    if let ObjectKind::IsoDoor(d) = &o.kind {
                        for s in d.open_sprite.iter().chain(d.closed_sprite.iter()) {
                            wanted.extend(world.sprites.sprite(*s).name.clone());
                        }
                    }
                }
            }
        }
    }
    // Texture packs through the engine's file access (disk natively, the mounted files in the browser), 2x packs first.
    let pack_dir = root.join("media").join("texturepacks");
    let mut names: Vec<String> = vfs::read_dir(&pack_dir)?
        .into_iter()
        .filter(|n| n.ends_with(".pack"))
        .collect();
    names.sort_by_key(|n| pack_order(n));
    let mut packs = names.into_iter().map(|n| {
        let p = pack_dir.join(&n);
        // In the browser a pack is dropped from memory once read.
        vfs::take(&p)
            .map(Ok)
            .unwrap_or_else(|| vfs::read(&p))
            .map(|b| (n, b))
            .map_err(|e| e.to_string())
    });
    let atlas = build_atlas_from(&mut packs, &wanted, 8192, 8192).map_err(|e| anyhow!(e))?;
    let state = SceneState {
        world,
        chunks,
        bounds: (x0, y0, x1, y1),
        levels: (zmin, zmax),
        missing,
    };
    let mut scene = Scene {
        atlas,
        instances: Vec::new(),
        level_ranges: Vec::new(),
        min_level: zmin,
        unresolved: 0,
        collision: Collision::default(),
        meta: Vec::new(),
        rooms: std::collections::HashMap::new(),
        clear: [20.0 / 255.0, 24.0 / 255.0, 20.0 / 255.0],
        state: Some(state),
    };
    scene.rebuild();
    Ok(scene)
}

/// The engine-side world a scene was built from, kept so that the scene can change (doors) and be rebuilt.
pub struct SceneState {
    pub world: IsoWorld,
    pub chunks: Vec<IsoChunk>,
    bounds: (i32, i32, i32, i32),
    levels: (i32, i32),
    missing: u64,
}

impl SceneState {
    fn chunk(&self, x: i32, y: i32) -> Option<usize> {
        // Chunks were loaded cell by cell, 32 x 32 per cell, x outer and y inner.
        let (x0, y0, _, y1) = self.bounds;
        let (wx, wy) = (x.div_euclid(8), y.div_euclid(8));
        let (cx, cy) = (wx.div_euclid(32) - x0, wy.div_euclid(32) - y0);
        if cx < 0 || cy < 0 || cy > y1 - y0 {
            return None;
        }
        let i = ((cx * (y1 - y0 + 1) + cy) * 1024 + wx.rem_euclid(32) * 32 + wy.rem_euclid(32))
            as usize;
        self.chunks
            .get(i)
            .filter(|c| c.wx == wx && c.wy == wy)
            .map(|_| i)
    }
    fn square_mut(&mut self, x: i32, y: i32, z: i32) -> Option<&mut IsoGridSquare> {
        let i = self.chunk(x, y)?;
        self.chunks[i].get_grid_square_mut(x.rem_euclid(8), y.rem_euclid(8), z)
    }
}
impl GetSquare for SceneState {
    fn get_grid_square(&self, x: i32, y: i32, z: i32) -> Option<&IsoGridSquare> {
        let i = self.chunk(x, y)?;
        self.chunks[i].get_grid_square(x.rem_euclid(8), y.rem_euclid(8), z)
    }
}

impl Scene {
    /// zombie.iso.LosUtil.lineClear between two squares of one level, as a client computes it; a closed window or
    /// door in the way counts as seen through only where the original says so. `false` without engine state.
    pub fn line_clear(&self, from: (i32, i32), to: (i32, i32), z: i32) -> bool {
        let Some(st) = &self.state else { return false };
        line_clear(
            &st.world.sprites,
            &st.world.alias,
            st,
            from.0,
            from.1,
            z,
            to.0,
            to.1,
            z,
            false,
            10000,
            false,
        ) != TestResults::Blocked
    }

    /// Recompute instances and collision from the engine squares (after an object changed).
    pub fn rebuild(&mut self) {
        let Some(mut state) = self.state.take() else {
            return;
        };
        let pending;
        {
            let SceneState {
                world,
                chunks,
                bounds,
                levels,
                missing,
            } = &state;
            let (x0, y0, x1, y1) = *bounds;
            let (zmin, zmax) = *levels;
            let missing = *missing;
            let atlas = &self.atlas;
            let mut visions: Vec<(i32, i32, i32, i32)> = Vec::new();

            let mut collision = Collision::default();
            {
                let mut grid = RegionGrid(std::collections::HashMap::new());
                for ch in chunks.iter() {
                    for z in ch.get_min_level()..=ch.get_max_level() {
                        for i in 0..64 {
                            if let Some(sq) = ch.get_grid_square(i % 8, i / 8, z) {
                                grid.0.insert((sq.x, sq.y, sq.z), sq);
                            }
                        }
                    }
                }
                for sq in grid.0.values() {
                    let (c, _path, vision) = sq.calculate_matrices(&world.alias, &grid);
                    collision.set_collide_matrix(sq.x, sq.y, sq.z, c);
                    visions.push((sq.x, sq.y, sq.z, vision));
                    let mut mask = 0u16;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let next = grid.get_grid_square(sq.x + dx, sq.y + dy, sq.z);
                            if (dx != 0 || dy != 0)
                                && sq
                                    .test_collide_special_objects(&world.alias, next, &grid)
                                    .is_some()
                            {
                                mask |= 1 << ((dx + 1) + (dy + 1) * 3);
                            }
                        }
                    }
                    collision.set_special_blocks(sq.x, sq.y, sq.z, mask);
                }
            }
            let (aw, ah) = (atlas.width as f32, atlas.height as f32);
            let mut instances = Vec::new();
            let mut level_ranges = vec![];
            let mut unresolved = missing;
            let mut meta = Vec::new();
            // Room of every square inside a room rect (also squares without tiles), numbered by the meta grid's room handle.
            let mut rooms = std::collections::HashMap::new();
            for cy in y0..=y1 {
                for cx in x0..=x1 {
                    let Some(cell) = world.meta_grid.get_cell_data(cx, cy) else {
                        continue;
                    };
                    for r in &cell.room_list {
                        let def = world.meta_grid.room(*r);
                        for q in &def.rects {
                            for yy in q.y..q.y + q.h {
                                for xx in q.x..q.x + q.w {
                                    rooms.insert((xx, yy, def.level), r.0 as i32);
                                }
                            }
                        }
                    }
                }
            }
            let by_pos: std::collections::HashMap<(i32, i32), &IsoChunk> =
                chunks.iter().map(|c| ((c.wx, c.wy), c)).collect();
            for z in zmin..=zmax {
                level_ranges.push(instances.len() as u32);
                for gy in y0 * 256..(y1 + 1) * 256 {
                    for gx in x0 * 256..(x1 + 1) * 256 {
                        let Some(ch) = by_pos.get(&(gx.div_euclid(8), gy.div_euclid(8))) else {
                            continue;
                        };
                        let Some(sq) = ch.get_grid_square(gx.rem_euclid(8), gy.rem_euclid(8), z)
                        else {
                            continue;
                        };
                        let p = iso(gx as f32, gy as f32, z);
                        let (px, py) = (p[0] as i32, p[1] as i32);
                        let room = rooms.get(&(gx, gy, z)).copied().unwrap_or(-1);
                        for o in &sq.objects {
                            for (k, spr) in o
                                .sprite
                                .iter()
                                .chain(o.attached_anim_sprite.iter().flatten())
                                .enumerate()
                            {
                                if k == 0 {
                                    let fl = sprite_collision_flags(world, *spr);
                                    if fl != 0 {
                                        collision.add(gx, gy, z, fl);
                                    }
                                }
                                let Some(s) = world
                                    .sprites
                                    .sprite(*spr)
                                    .name
                                    .as_ref()
                                    .and_then(|n| atlas.sprites.get(n))
                                else {
                                    unresolved += 1;
                                    continue;
                                };
                                let x = (px - s.orig_w / 2 + s.off_x) as f32;
                                // renderYOffset (table stacking) is in unscaled pixels; the original multiplies by Core.tileScale (2).
                                let y = (py + TILE_H - s.orig_h + s.off_y) as f32
                                    - o.render_y_offset * 2.0;
                                meta.push((gx, gy, z, room));
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
            }
            level_ranges.push(instances.len() as u32);
            self.instances = instances;
            self.level_ranges = level_ranges;
            self.unresolved = unresolved;
            self.collision = collision;
            self.meta = meta;
            self.rooms = rooms;
            pending = visions;
        }
        // zombie.iso.IsoGridSquare.visionMatrix, read by the ported line of sight.
        for (x, y, z, v) in pending {
            if let Some(sq) = state.square_mut(x, y, z) {
                sq.vision_matrix = v;
            }
        }
        self.state = Some(state);
    }

    /// The player's "interact" on doors: flips the nearest ordinary door within reach of (x, y) on level z
    /// (zombie.iso.objects.IsoDoor.ToggleDoorSilent: no locks, keys, sounds or obstruction checks yet; double and
    /// garage doors are left alone) and rebuilds the scene. Returns whether a door was toggled.
    pub fn toggle_door_near(&mut self, x: f32, y: f32, z: i32) -> bool {
        let Some(state) = self.state.as_mut() else {
            return false;
        };
        let SceneState { world, chunks, .. } = state;
        let mut best: Option<(f32, usize, i32, i32, usize)> = None;
        for (ci, ch) in chunks.iter().enumerate() {
            if ((ch.wx * 8 + 4) as f32 - x).abs() > 8.0 || ((ch.wy * 8 + 4) as f32 - y).abs() > 8.0
            {
                continue;
            }
            for i in 0..64 {
                let Some(sq) = ch.get_grid_square(i % 8, i / 8, z) else {
                    continue;
                };
                for (oi, o) in sq.objects.iter().enumerate() {
                    let ObjectKind::IsoDoor(d) = &o.kind else {
                        continue;
                    };
                    let props = o.sprite.map(|s| &world.sprites.sprite(s).properties);
                    if props.is_some_and(|p| {
                        p.has_str(&world.alias, "DoubleDoor")
                            || p.has_str(&world.alias, "GarageDoor")
                    }) {
                        continue;
                    }
                    // The door stands on the north or west edge of its square.
                    let (dx, dy) = if d.north {
                        (sq.x as f32 + 0.5, sq.y as f32)
                    } else {
                        (sq.x as f32, sq.y as f32 + 0.5)
                    };
                    let dist = ((dx - x).powi(2) + (dy - y).powi(2)).sqrt();
                    if dist <= 1.5 && best.is_none_or(|b| dist < b.0) {
                        best = Some((dist, ci, i % 8, i / 8, oi));
                    }
                }
            }
        }
        let Some((_, ci, lx, ly, oi)) = best else {
            return false;
        };
        if let Some(sq) = chunks[ci].get_grid_square_mut(lx, ly, z) {
            sq.objects[oi].toggle_door_silent();
            sq.recalc_properties(&world.sprites);
        }
        self.rebuild();
        true
    }
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
        // Squares within `radius` the player has line of sight to: the ported LosUtil.lineClear (the radius itself
        // and what the original does with the result when drawing are still approximations).
        let mut seen = std::collections::HashSet::new();
        if let Some(rad) = los {
            let r = rad.ceil() as i32;
            for x in (px.floor() as i32 - r)..=(px.floor() as i32 + r) {
                for y in (py.floor() as i32 - r)..=(py.floor() as i32 + r) {
                    let (cx, cy) = (x as f32 + 0.5 - px, y as f32 + 0.5 - py);
                    let seen_sq = if self.state.is_some() {
                        self.line_clear((px.floor() as i32, py.floor() as i32), (x, y), z)
                    } else {
                        self.collision.los((px, py), (x, y), z)
                    };
                    if cx * cx + cy * cy <= rad * rad && seen_sq {
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
        let (zmin, n) = (self.min_level, self.level_ranges.len() - 1);
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
    fn spawn_house_steps_follow_original_collide_matrices() -> anyhow::Result<()> {
        // collideMatrix of four squares of the Rosewood spawn house, printed by the ORIGINAL through
        // tests/original_probe/CollideProbe.java (2026-11-13). Bit (dx+1) + (dy+1)*3 + 9 set = step blocked.
        let observed = [
            ((8303, 11689), 0x7fc2fff),
            ((8300, 11690), 0x7fe69ff),
            ((8301, 11692), 0x7feffff),
            ((8304, 11688), 0x7fcbfff),
        ];
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let scene = load_region(&root, 32, 45, 32, 45)?;
        for ((x, y), matrix) in observed {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let blocked = (matrix >> ((dx + 1) + (dy + 1) * 3 + 9)) & 1 == 1;
                    assert_eq!(
                        !scene.collision.can_step((x, y), (x + dx, y + dy), 0),
                        blocked,
                        "from {x},{y} by {dx},{dy}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "real-content acceptance: loads a complete cell and texture atlas"]
    fn opening_the_spawn_house_door_lets_the_player_through() -> anyhow::Result<()> {
        // (8288, 11702, 0) holds the closed west-edge door fixtures_doors_02_4 of the house's back room
        // (ChunkObjectsProbe output of the original, chunk 1036,1462, square 0,6).
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut scene = load_region(&root, 32, 45, 32, 45)?;
        let (inside, outside) = ((8288, 11702), (8287, 11702));
        assert!(
            !scene.collision.can_step(inside, outside, 0),
            "closed door blocks"
        );
        assert!(scene.toggle_door_near(8288.4, 11702.5, 0));
        assert!(
            scene.collision.can_step(inside, outside, 0),
            "open door lets through"
        );
        assert!(scene.collision.can_step(outside, inside, 0));
        assert!(scene.toggle_door_near(8288.4, 11702.5, 0));
        assert!(
            !scene.collision.can_step(inside, outside, 0),
            "closed again"
        );
        Ok(())
    }

    #[test]
    #[ignore = "real-content acceptance: loads a complete cell and texture atlas"]
    fn real_water_square_is_wired_into_loaded_scene() -> anyhow::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        // (1, 4803, 0) holds the water tile blends_natural_02_0: found by scanning the lotpack of cell 0_18 (2026-11-11).
        let (x, y) = (1, 4803);
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
