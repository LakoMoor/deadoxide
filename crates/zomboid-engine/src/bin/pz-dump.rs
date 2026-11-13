//! Dumps engine state in the exact text format of the Java probes in `tests/original_probe`, for byte-for-byte diffs.
//!   pz-dump tiledefs <game root>                  (pairs with TileDefsProbe.java)
//!   pz-dump lotheaders <game root> <map folder>   (pairs with LotHeadersProbe.java)
//!   pz-dump lotpack <game root> <map folder> x1 y1 x2 y2   (pairs with LotPackProbe.java)
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use zomboid_engine::java;
use zomboid_engine::zombie::core::random::rand_abstract::RandAbstract;
use zomboid_engine::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use zomboid_engine::zombie::iso::cell_loader::{self, LoadContext};
use zomboid_engine::zombie::iso::iso_chunk::IsoChunk;
use zomboid_engine::zombie::iso::iso_grid_square::{line_clear, GetSquare, IsoGridSquare};
use zomboid_engine::zombie::iso::iso_lot::IsoLot;
use zomboid_engine::zombie::iso::iso_meta_grid::{AlarmSettings, IsoMetaGrid};
use zomboid_engine::zombie::iso::iso_object::ObjectKind;
use zomboid_engine::zombie::iso::iso_world::{
    self, IsoWorld, TileDefinitions, VANILLA_TILE_DEFINITION_FILES,
};
use zomboid_engine::zombie::iso::sprite::iso_sprite_manager::IsoSpriteManager;
use zomboid_engine::zombie::iso::sprite_details::iso_flag_type::IsoFlagType;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("tiledefs") if a.len() == 3 => tiledefs(Path::new(&a[2])),
        Some("lotheaders") if a.len() == 4 => lotheaders(Path::new(&a[2]), &a[3]),
        Some("lotpack") if a.len() == 8 => lotpack(Path::new(&a[2]), &a[3], &a[4..8]),
        Some("metagrid") if a.len() == 8 => metagrid(Path::new(&a[2]), &a[3], &a[4..8]),
        Some("chunkobjects") if a.len() == 7 => chunkobjects(Path::new(&a[2]), &a[3..7]),
        Some("collide") if a.len() == 7 => collide(Path::new(&a[2]), &a[3..7]),
        Some("entitysprites") if a.len() == 3 => entitysprites(Path::new(&a[2])),
        Some("sandbox") if a.len() == 4 => sandbox(Path::new(&a[2]), Path::new(&a[3])),
        Some("specialcollide") if a.len() == 5 || a.len() == 6 => {
            specialcollide(Path::new(&a[2]), &a[3..], false)
        }
        Some("stairs") if a.len() == 5 => specialcollide_mode(Path::new(&a[2]), &a[3..], 2),
        Some("los") if a.len() == 5 || a.len() == 6 => {
            specialcollide(Path::new(&a[2]), &a[3..], true)
        }
        Some("basements") if a.len() == 7 => basements(Path::new(&a[2]), &a[3..7]),
        _ => Err("usage: see the header of pz-dump.rs".into()),
    };
    match r {
        Ok(text) => {
            if let Err(e) = std::io::stdout().lock().write_all(text.as_bytes()) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

/// The loading sequence of IsoWorld.init without mods and without entity scripts, then one line per sprite.
pub fn tiledefs(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    for (f, n) in VANILLA_TILE_DEFINITION_FILES {
        let p = root.join("media").join(f);
        files.push((
            f,
            n,
            std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?,
        ));
    }
    let mut defs = TileDefinitions::default();
    let mut alias = TilePropertyAliasMap::default();
    let mut sm = IsoSpriteManager::new();
    for (f, n, data) in &files {
        if let Err(e) = defs.load_tile_definitions_property_strings(data, f, *n) {
            eprintln!("PROBE strings {f}: {e}");
        }
    }
    defs.set_custom_property_values()?;
    defs.generate_tile_property_lookup_tables(&mut alias, &[]);
    for (f, n, data) in &files {
        if let Err(e) = iso_world::load_tile_definitions(&mut sm, &alias, data, f, *n) {
            eprintln!("PROBE load {f}: {e}");
        }
    }
    iso_world::register_fake_jumbo_tree(&mut sm, &alias, 7);

    let mut names: Vec<&String> = sm.named_map.keys().collect();
    names.sort_unstable();
    let mut sb = String::new();
    let null = |o: Option<&str>| o.unwrap_or("null").to_string();
    for n in &names {
        let r = sm.named_map[*n];
        let s = sm.sprite(r);
        let p = &s.properties;
        let flags: Vec<&str> = p.get_flags_list().iter().map(|f| f.name()).collect();
        let _ = write!(
            sb,
            "{n}\t{}\t{}\t{}\t",
            s.id,
            s.tile_type.name(),
            flags.join(",")
        );
        for k in p.get_property_names(&alias) {
            let _ = write!(sb, "{k}={};", null(p.get(&alias, k)));
        }
        sb.push('\t');
        for x in [
            s.force_ambient,
            s.solidfloor,
            s.can_be_removed,
            s.attached_floor,
            s.cut_w,
            s.cut_n,
            s.solid,
            s.solid_trans,
            s.invisible,
            s.always_draw,
            s.force_render,
            s.move_with_wind,
            s.is_bush,
            s.treat_as_wall_order,
        ] {
            sb.push(if x { '1' } else { '0' });
        }
        let _ = write!(
            sb,
            "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            s.firerequirement,
            null(s.burnt_tile.as_deref()),
            s.wind_type,
            s.render_layer,
            null(s.tileset_name.as_deref()),
            s.tile_sheet_index,
            null(s.parent_object_name.as_deref()),
            s.fascia_edge.map_or("null", |f| f.name())
        );
        match s.sprite_grid {
            None => sb.push_str("\t-"),
            Some(g) => {
                let g = sm.grid(g);
                let anchor = g
                    .get_anchor_sprite()
                    .map(|a| null(sm.sprite(a).name.as_deref()))
                    .unwrap_or("null".into());
                let _ = write!(
                    sb,
                    "\t{}x{}x{}@{}:{}",
                    g.get_width(),
                    g.get_height(),
                    g.get_levels(),
                    g.get_sprite_index(r).map_or(-1, |i| i as i32),
                    anchor
                );
            }
        }
        let u = p.init_surface(&alias);
        let b = |x: bool| if x { 1 } else { 0 };
        let _ = writeln!(
            sb,
            "\t{},{}{}{},{},{},{},{},{}",
            u.surface,
            b(u.is_surface_offset),
            b(u.is_table),
            b(u.is_table_top),
            u.stack_replace_tile_offset,
            u.item_height,
            u.sloped_surface_direction.map_or("null", |d| d.name()),
            u.sloped_surface_height_min,
            u.sloped_surface_height_max
        );
    }
    eprintln!("PROBE sprites={} intMap={}", names.len(), sm.int_map.len());
    Ok(sb)
}

const NO_WORLD_GEN: (i32, i32, i32, i32) = (10000000, 10000000, -10000000, -10000000);

fn mix(h: &mut u64, v: i32) {
    for b in v.to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}
fn mix_long(h: &mut u64, v: i64) {
    mix(h, v as i32);
    mix(h, (v >> 32) as i32);
}

/// Every lotheader of one map folder through the port of MetaGridLoaderThread.loadCell, cells in x-then-y order.
/// `lightsActive` is random in the original and not printed.
pub fn lotheaders(root: &Path, dir: &str) -> Result<String, String> {
    let media = root.join("media");
    let mut g = IsoMetaGrid::default();
    g.create_step1(&media, &[dir.to_string()], NO_WORLD_GEN)?;
    let mut sb = String::new();
    {
        let mf = &g.map_files[0];
        let _ = writeln!(
            sb,
            "M {} {} {} {} {} {} {} {}",
            mf.min_x,
            mf.min_y,
            mf.max_x,
            mf.max_y,
            mf.min_cell300_x,
            mf.min_cell300_y,
            mf.max_cell300_x,
            mf.max_cell300_y
        );
        for y in mf.min_y..=mf.max_y {
            for x in mf.min_x..=mf.max_x {
                sb.push(if mf.has_cell(x, y) { '1' } else { '0' });
            }
            sb.push('\n');
        }
        for y in mf.min_cell300_y..=mf.max_cell300_y {
            for x in mf.min_cell300_x..=mf.max_cell300_x {
                sb.push(if mf.has_cell300(x, y) { '1' } else { '0' });
            }
            sb.push('\n');
        }
    }
    let (x1, y1, x2, y2) = (
        g.map_files[0].min_x,
        g.map_files[0].min_y,
        g.map_files[0].max_x,
        g.map_files[0].max_y,
    );
    for x in x1..=x2 {
        for y in y1..=y2 {
            if !g.map_files[0].has_cell(x, y) {
                continue;
            }
            let path = g.map_files[0]
                .get_lot_header(x, y)
                .ok_or("header missing")?
                .absolute_file_path
                .clone();
            let data = std::fs::read(&path).map_err(|e| e.to_string())?;
            if let Err(e) = g.load_cell(0, &data, x, y, &mut |_| 1, &[]) {
                eprintln!("ERROR loading {}: {e}", path.display());
            }
            let h = g.map_files[0]
                .get_lot_header(x, y)
                .ok_or("header missing")?;
            let c = g.get_cell_data(x, y).ok_or("cell missing")?;
            let _ = writeln!(
                sb,
                "C {x} {y} {} {} {} {} {} {} {} {} {}",
                h.version,
                h.width,
                h.height,
                h.min_level,
                h.max_level,
                h.fixed2x,
                h.tiles_used.len(),
                c.room_list.len(),
                c.buildings.len()
            );
            for t in &h.tiles_used {
                let _ = writeln!(sb, "T {t}");
            }
            for r in &c.room_list {
                let r = g.room(*r);
                let _ = write!(
                    sb,
                    "R {} {} {} {} {} {} {} {} {} [{}]",
                    r.id,
                    r.meta_id,
                    r.level,
                    r.x,
                    r.y,
                    r.x2,
                    r.y2,
                    r.area,
                    r.building.map_or(-1, |b| g.building(b).id),
                    r.name
                );
                for q in &r.rects {
                    let _ = write!(sb, " {},{},{},{}", q.x, q.y, q.w, q.h);
                }
                sb.push_str(" |");
                for o in &r.objects {
                    let _ = write!(sb, " {},{},{}", o.r#type, o.x, o.y);
                }
                sb.push('\n');
            }
            for b in &c.buildings {
                let b = g.building(*b);
                let _ = write!(
                    sb,
                    "B {} {} {} {} {} {} r",
                    b.id, b.meta_id, b.x, b.y, b.x2, b.y2
                );
                for r in &b.rooms {
                    let _ = write!(sb, " {}", g.room(*r).id);
                }
                sb.push_str(" e");
                for r in &b.emptyoutside {
                    let _ = write!(sb, " {}", g.room(*r).id);
                }
                sb.push_str(" c");
                for v in &b.overlapped_chunks {
                    let _ = write!(sb, " {v}");
                }
                sb.push('\n');
            }
            sb.push_str("Z ");
            for i in 0..1024 {
                let _ = write!(sb, "{:02x}", h.get_zombie_intensity(i));
            }
            sb.push('\n');
        }
    }
    Ok(sb)
}

/// Every chunk of a cell range through the port of IsoLot.load; pairs with LotPackProbe.java.
pub fn lotpack(root: &Path, dir: &str, range: &[String]) -> Result<String, String> {
    let r: Vec<i32> = range
        .iter()
        .map(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let media = root.join("media");
    let mut g = IsoMetaGrid::default();
    g.create_step1(&media, &[dir.to_string()], NO_WORLD_GEN)?;
    let mut sb = String::new();
    let mut lot = IsoLot::default();
    for cx in r[0]..=r[2] {
        for cy in r[1]..=r[3] {
            if !g.map_files[0].has_cell(cx, cy) {
                continue;
            }
            let path = g.map_files[0]
                .get_lot_header(cx, cy)
                .ok_or("header missing")?
                .absolute_file_path
                .clone();
            let data = std::fs::read(&path).map_err(|e| e.to_string())?;
            g.load_cell(0, &data, cx, cy, &mut |_| 1, &[])?;
            let info = g.map_files[0]
                .get_lot_header(cx, cy)
                .ok_or("header missing")?;
            let pack_path = g.map_files[0]
                .info_file_names
                .get(&format!("world_{cx}_{cy}.lotpack"))
                .ok_or("no lotpack")?;
            let pack = std::fs::read(pack_path).map_err(|e| e.to_string())?;
            let mut h: u64 = 0xcbf29ce484222325;
            let n = ((info.max_level.min(31) - info.min_level.max(-32) + 1) * 64) as usize;
            let base = ((info.min_level.max(-32) - info.min_level) * 64) as usize;
            for lx in 0..32 {
                for ly in 0..32 {
                    if let Some(e) = lot.load(info, &pack, cx, cy, cx * 32 + lx, cy * 32 + ly) {
                        eprintln!("ERROR {cx},{cy} chunk {lx},{ly}: {e}");
                    }
                    mix(&mut h, lot.min_level);
                    mix(&mut h, lot.max_level);
                    mix(&mut h, lot.data.len() as i32);
                    for v in &lot.offset_in_data[base..base + n] {
                        mix(&mut h, *v);
                    }
                    for v in &lot.data {
                        mix(&mut h, *v);
                    }
                    if lx == 16 && ly == 16 {
                        let _ = write!(sb, "D {cx} {cy} {} {}", lot.min_level, lot.max_level);
                        for v in &lot.offset_in_data[base..base + n] {
                            let _ = write!(sb, " {v}");
                        }
                        sb.push_str(" |");
                        for v in &lot.data {
                            let _ = write!(sb, " {v}");
                        }
                        sb.push('\n');
                    }
                }
            }
            let _ = writeln!(sb, "L {cx} {cy} {h:x}");
        }
    }
    Ok(sb)
}

/// The whole meta grid as IsoMetaGrid.Create builds it for `Core.gameMap`, then rooms per chunk and room lookups per
/// square for a cell range; pairs with MetaGridProbe.java (run inside the booted original).
pub fn metagrid(root: &Path, game_map: &str, range: &[String]) -> Result<String, String> {
    let r: Vec<i32> = range
        .iter()
        .map(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let media = root.join("media");
    let mut g = IsoMetaGrid::default();
    let dirs = IsoMetaGrid::lot_directories(&media, game_map);
    g.create_step1(&media, &dirs, (-250, -250, 250, 250))?;
    for e in g.load_all_cells(&mut |_| 1, &mut |_, _| Vec::new()) {
        eprintln!("{e}");
    }
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 1,
        alarm_decay: 1,
    };
    g.create_step2(&alarm, &mut |_| 1, &mut |a, _| a);
    let mut sb = String::new();
    let _ = writeln!(
        sb,
        "G {} {} {} {} dirs={}",
        g.min_x,
        g.min_y,
        g.max_x,
        g.max_y,
        dirs.join(";")
    );
    let mut h: u64 = 0xcbf29ce484222325;
    for b in &g.buildings {
        mix_long(&mut h, g.building(*b).id);
    }
    let _ = writeln!(sb, "B {} {h:x}", g.buildings.len());
    for cx in r[0]..=r[2] {
        for cy in r[1]..=r[3] {
            let Some(info) = g.get_header(cx, cy) else {
                continue;
            };
            let adj: String = info
                .adjacent_cells
                .iter()
                .map(|b| if *b { '1' } else { '0' })
                .collect();
            let _ = writeln!(sb, "A {cx} {cy} {adj}");
            let cell = g.get_cell_data(cx, cy).ok_or("cell missing")?;
            for ly in 0..32 {
                for lx in 0..32 {
                    if let Some(ch) = cell.chunk((ly * 32 + lx) as usize) {
                        if ch.get_rooms_size() > 0 {
                            let _ = write!(sb, "K {cx} {cy} {lx} {ly}");
                            for rr in ch.rooms() {
                                let _ = write!(sb, " {}", g.room(*rr).id);
                            }
                            sb.push('\n');
                        }
                    }
                }
            }
            let mut h: u64 = 0xcbf29ce484222325;
            for z in info.min_level..=info.max_level {
                for y in cy * 256..cy * 256 + 256 {
                    for x in cx * 256..cx * 256 + 256 {
                        mix_long(
                            &mut h,
                            g.get_room_at(x, y, z).map_or(-1, |rr| g.room(rr).id),
                        );
                        mix_long(
                            &mut h,
                            g.get_empty_outside_at(x, y, z)
                                .map_or(-1, |rr| g.room(rr).id),
                        );
                    }
                }
            }
            let _ = writeln!(sb, "H {cx} {cy} {h:x}");
        }
    }
    Ok(sb)
}

/// Tile definitions as IsoWorld.init loads them (vanilla files, no mods, no entity scripts).
fn load_tile_definitions(
    root: &Path,
) -> Result<(IsoSpriteManager, TilePropertyAliasMap, TileDefinitions), String> {
    let mut files = Vec::new();
    for (f, n) in VANILLA_TILE_DEFINITION_FILES {
        let p = root.join("media").join(f);
        files.push((
            f,
            n,
            std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?,
        ));
    }
    let mut defs = TileDefinitions::default();
    let mut alias = TilePropertyAliasMap::default();
    let mut sm = IsoSpriteManager::new();
    for (f, n, data) in &files {
        if let Err(e) = defs.load_tile_definitions_property_strings(data, f, *n) {
            eprintln!("PROBE strings {f}: {e}");
        }
    }
    defs.set_custom_property_values()?;
    defs.generate_tile_property_lookup_tables(&mut alias, &[]);
    for (f, n, data) in &files {
        if let Err(e) = iso_world::load_tile_definitions(&mut sm, &alias, data, f, *n) {
            eprintln!("PROBE load {f}: {e}");
        }
    }
    iso_world::register_fake_jumbo_tree(&mut sm, &alias, 7);
    Ok((sm, alias, defs))
}

/// Squares and objects of a chunk range through the port of CellLoader.LoadCellBinaryChunk; pairs with
/// ChunkObjectsProbe.java (run inside the booted original).
pub fn chunkobjects(root: &Path, range: &[String]) -> Result<String, String> {
    let r: Vec<i32> = range
        .iter()
        .map(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let media = root.join("media");
    let (mut sm, alias, mut defs) = load_tile_definitions(root)?;
    sm.add_sprite("media/ui/missing-tile.png");
    defs.loaded_tile_definitions(&mut sm, &alias, &|_| false)?;
    let mut g = IsoMetaGrid::default();
    let dirs = IsoMetaGrid::lot_directories(&media, "Muldraugh, KY");
    g.create_step1(&media, &dirs, (-250, -250, 250, 250))?;
    for e in g.load_all_cells(&mut |_| 1, &mut |_, _| Vec::new()) {
        eprintln!("{e}");
    }
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 1,
        alarm_decay: 1,
    };
    g.create_step2(&alarm, &mut |_| 1, &mut |a, _| a);
    let mut rand = RandAbstract::new(java::Random::new(12345));
    // LockedHouses is 6 in the dedicated server's default sandbox preset, which the probe runs under.
    let mut ctx = LoadContext {
        sprites: &mut sm,
        alias: &alias,
        glass_removed_window_sprite_map: &defs.glass_removed_window_sprite_map,
        smashed_window_sprite_map: &defs.smashed_window_sprite_map,
        meta_grid: &mut g,
        rand: &mut rand,
        locked_houses: 6,
        missing_tiles: Vec::new(),
        lotpack_cache: None,
    };
    let mut sb = String::new();
    let _ = writeln!(sb, "O lockedHouses={}", ctx.locked_houses);
    let b = |v: bool| if v { 1 } else { 0 };
    for wx in r[0]..=r[2] {
        for wy in r[1]..=r[3] {
            *ctx.rand = RandAbstract::new(java::Random::new(12345));
            let mut ch = IsoChunk::new(wx, wy);
            let mut errors = Vec::new();
            let ok = cell_loader::load_cell_binary_chunk(&mut ctx, wx, wy, &mut ch, &mut errors);
            for e in errors {
                eprintln!("chunk {wx},{wy}: {e}");
            }
            let mut taint = false;
            for z in ch.get_min_level()..=ch.get_max_level() {
                for i in 0..64 {
                    if let Some(sq) = ch.get_grid_square(i % 8, i / 8, z) {
                        taint |= sq.objects.iter().any(|o| {
                            matches!(o.kind, ObjectKind::IsoRadio | ObjectKind::IsoTelevision)
                        });
                    }
                }
            }
            let _ = writeln!(
                sb,
                "C {wx} {wy} {ok} {} {}",
                ch.get_min_level(),
                ch.get_max_level()
            );
            for z in ch.get_min_level()..=ch.get_max_level() {
                for y in 0..8 {
                    for x in 0..8 {
                        let Some(sq) = ch.get_grid_square(x, y, z) else {
                            continue;
                        };
                        let _ = write!(sb, "S {x} {y} {z} {}", sq.room_id);
                        let name = |s: Option<_>| {
                            s.and_then(|s| ctx.sprites.sprite(s).name.clone())
                                .unwrap_or_else(|| "null".to_string())
                        };
                        for o in &sq.objects {
                            let _ = write!(sb, " {}:{}", o.kind.class_name(), name(o.sprite));
                            for at in o.attached_anim_sprite.iter().flatten() {
                                let _ = write!(sb, "+{}", name(Some(*at)));
                            }
                            if o.special {
                                sb.push('*');
                            }
                            if o.render_y_offset != 0.0 {
                                let _ = write!(sb, "@{:x}", o.render_y_offset.to_bits());
                            }
                            match &o.kind {
                                ObjectKind::IsoDoor(d) if taint => {
                                    let _ = write!(sb, "{{o{}n{}?}}", b(d.open), b(d.north));
                                }
                                ObjectKind::IsoDoor(d) => {
                                    let _ = write!(
                                        sb,
                                        "{{o{}n{}l{}k{}}}",
                                        b(d.open),
                                        b(d.north),
                                        b(d.locked),
                                        b(d.locked_by_key)
                                    );
                                }
                                ObjectKind::IsoWindow(w) if taint => {
                                    let _ = write!(
                                        sb,
                                        "{{n{}?p{}s{}}}",
                                        b(w.north),
                                        b(w.perma_locked),
                                        b(w.smashed)
                                    );
                                }
                                ObjectKind::IsoWindow(w) => {
                                    let _ = write!(
                                        sb,
                                        "{{n{}l{}p{}s{}}}",
                                        b(w.north),
                                        b(w.locked),
                                        b(w.perma_locked),
                                        b(w.smashed)
                                    );
                                }
                                ObjectKind::IsoTree(t) => {
                                    let _ = write!(sb, "{{s{}}}", t.size);
                                }
                                _ => {}
                            }
                            if o.sprite.is_some_and(|s| {
                                ctx.sprites
                                    .sprite(s)
                                    .properties
                                    .has(IsoFlagType::vegitation)
                            }) {
                                if taint {
                                    sb.push_str("{t?}");
                                } else {
                                    let _ = write!(
                                        sb,
                                        "{{t{:x},{:x},{:x}}}",
                                        o.tintr.to_bits(),
                                        o.tintg.to_bits(),
                                        o.tintb.to_bits()
                                    );
                                }
                            }
                        }
                        sb.push('\n');
                    }
                }
            }
            if !taint {
                let _ = writeln!(sb, "X {}", ctx.rand.next(1000000));
            }
        }
    }
    for t in &ctx.missing_tiles {
        eprintln!("Missing tile definition: {t}");
    }
    Ok(sb)
}

struct ChunkGrid<'a>(std::collections::HashMap<(i32, i32, i32), &'a IsoGridSquare>);
impl GetSquare for ChunkGrid<'_> {
    fn get_grid_square(&self, x: i32, y: i32, z: i32) -> Option<&IsoGridSquare> {
        self.0.get(&(x, y, z)).copied()
    }
}

/// Square properties and collide/path/vision matrices of a chunk range; pairs with CollideProbe.java.
pub fn collide(root: &Path, range: &[String]) -> Result<String, String> {
    let r: Vec<i32> = range
        .iter()
        .map(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let mut rand = RandAbstract::new(java::Random::new(12345));
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 1,
        alarm_decay: 1,
    };
    let mut world = IsoWorld::init(root, "Muldraugh, KY", &|_| false, &alarm, None, &mut rand)?;
    let mut chunks = Vec::new();
    {
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
            locked_houses: 6,
            missing_tiles: Vec::new(),
            lotpack_cache: None,
        };
        for wx in r[0]..=r[2] {
            for wy in r[1]..=r[3] {
                *ctx.rand = RandAbstract::new(java::Random::new(12345));
                let mut ch = IsoChunk::new(wx, wy);
                cell_loader::load_cell_binary_chunk(&mut ctx, wx, wy, &mut ch, &mut Vec::new());
                chunks.push(ch);
            }
        }
    }
    for ch in &mut chunks {
        for z in ch.get_min_level()..=ch.get_max_level() {
            for i in 0..64 {
                if let Some(sq) = ch.get_grid_square_mut(i % 8, i / 8, z) {
                    sq.recalc_properties(&world.sprites);
                }
            }
        }
    }
    let mut all = Vec::new();
    for ch in &chunks {
        for z in ch.get_min_level()..=ch.get_max_level() {
            for y in 0..8 {
                for x in 0..8 {
                    all.extend(ch.get_grid_square(x, y, z));
                }
            }
        }
    }
    let grid = ChunkGrid(all.iter().map(|s| ((s.x, s.y, s.z), *s)).collect());
    let mut sb = String::new();
    for sq in &all {
        let flags: Vec<&str> = sq
            .properties
            .get_flags_list()
            .iter()
            .map(|f| f.name())
            .collect();
        let _ = write!(
            sb,
            "Q {} {} {} {} {} ",
            sq.x,
            sq.y,
            sq.z,
            if sq.have_roof { 1 } else { 0 },
            flags.join(",")
        );
        for k in sq.properties.get_property_names(&world.alias) {
            let _ = write!(
                sb,
                "{k}={};",
                sq.properties.get(&world.alias, k).unwrap_or("null")
            );
        }
        let (c, p, v) = sq.calculate_matrices(&world.alias, &grid);
        let _ = writeln!(sb, " {:x} {c:x} {p:x} {v:x}", sq.has_types);
    }
    Ok(sb)
}

/// Entity scripts in ScriptManager order and the sprites they mark; pairs with EntitySpritesProbe.java.
pub fn entitysprites(root: &Path) -> Result<String, String> {
    let mut rand = RandAbstract::new(java::Random::new(12345));
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 1,
        alarm_decay: 1,
    };
    let world = IsoWorld::init(root, "Muldraugh, KY", &|_| false, &alarm, None, &mut rand)?;
    for l in &world.log {
        eprintln!("{l}");
    }
    let mut sb = String::new();
    for e in &world.scripts.entities {
        let _ = writeln!(sb, "E {}", e.name);
    }
    let mut names: Vec<&String> = world.sprites.named_map.keys().collect();
    names.sort_unstable();
    for n in names {
        let p = &world.sprites.sprite(world.sprites.named_map[n]).properties;
        let name = p.get(&world.alias, "EntityScriptName");
        if p.has(IsoFlagType::EntityScript) || name.is_some() {
            let _ = writeln!(
                sb,
                "S {n} {} {}",
                if p.has(IsoFlagType::EntityScript) {
                    1
                } else {
                    0
                },
                name.unwrap_or("null")
            );
        }
    }
    Ok(sb)
}

/// Collide matrices and door/window collision objects of one 64-square server cell; pairs with SpecialCollideProbe.java.
pub fn specialcollide(root: &Path, cell: &[String], los: bool) -> Result<String, String> {
    specialcollide_mode(root, cell, if los { 1 } else { 0 })
}

/// mode 0: collision objects, 1: line of sight (LosProbe.java), 2: stair heights (StairsProbe.java).
fn specialcollide_mode(root: &Path, cell: &[String], mode: u8) -> Result<String, String> {
    let los = mode == 1;
    let toggle = cell.len() == 3;
    let c: Vec<i32> = cell[..2]
        .iter()
        .map(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let mut rand = RandAbstract::new(java::Random::new(12345));
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 1,
        alarm_decay: 1,
    };
    let mut world = IsoWorld::init(root, "Muldraugh, KY", &|_| false, &alarm, None, &mut rand)?;
    let mut chunks = Vec::new();
    {
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
            locked_houses: 6,
            missing_tiles: Vec::new(),
            lotpack_cache: None,
        };
        for cx in 0..8 {
            for cy in 0..8 {
                *ctx.rand = RandAbstract::new(java::Random::new(12345));
                let (wx, wy) = (c[0] * 8 + cx, c[1] * 8 + cy);
                let mut ch = IsoChunk::new(wx, wy);
                cell_loader::load_cell_binary_chunk(&mut ctx, wx, wy, &mut ch, &mut Vec::new());
                chunks.push(ch);
            }
        }
    }
    for ch in &mut chunks {
        for z in ch.get_min_level()..=ch.get_max_level() {
            for i in 0..64 {
                if let Some(sq) = ch.get_grid_square_mut(i % 8, i / 8, z) {
                    if toggle {
                        for o in &mut sq.objects {
                            o.toggle_door_silent();
                        }
                    }
                    sq.recalc_properties(&world.sprites);
                }
            }
        }
    }
    if los {
        // Store the vision matrices (RecalcAllWithNeighbours), which testVisionAdjacent reads.
        let mut vm = Vec::new();
        {
            let mut all = Vec::new();
            for ch in &chunks {
                for z in ch.get_min_level()..=ch.get_max_level() {
                    for i in 0..64 {
                        all.extend(ch.get_grid_square(i % 8, i / 8, z));
                    }
                }
            }
            let grid = ChunkGrid(all.iter().map(|s| ((s.x, s.y, s.z), *s)).collect());
            for sq in &all {
                vm.push(sq.calculate_matrices(&world.alias, &grid).2);
            }
        }
        let mut it = vm.into_iter();
        for ch in &mut chunks {
            for z in ch.get_min_level()..=ch.get_max_level() {
                for i in 0..64 {
                    if let Some(sq) = ch.get_grid_square_mut(i % 8, i / 8, z) {
                        sq.vision_matrix = it.next().unwrap_or(0);
                    }
                }
            }
        }
    }
    let mut all = Vec::new();
    for ch in &chunks {
        for z in ch.get_min_level()..=ch.get_max_level() {
            for y in 0..8 {
                for x in 0..8 {
                    all.extend(ch.get_grid_square(x, y, z));
                }
            }
        }
    }
    let grid = ChunkGrid(all.iter().map(|s| ((s.x, s.y, s.z), *s)).collect());
    if mode == 2 {
        let mut sb = String::from("F true\n");
        for sq in &all {
            if !sq.has_stairs() && sq.get_sloped_surface_direction(&world.alias).is_none() {
                continue;
            }
            let (cm, _, _) = sq.calculate_matrices(&world.alias, &grid);
            let _ = write!(
                sb,
                "Z {} {} {} {} {cm:x}",
                sq.x,
                sq.y,
                sq.z,
                sq.get_stairs().name()
            );
            for py in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                for px in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                    let _ = write!(
                        sb,
                        " {:x}",
                        sq.get_apparent_z(&world.alias, px, py, true).to_bits()
                    );
                }
            }
            sb.push('\n');
        }
        return Ok(sb);
    }
    if los {
        let mut sb = String::new();
        for sq in &all {
            if sq.z != 0 || sq.x % 8 != 3 || sq.y % 8 != 3 {
                continue;
            }
            let _ = write!(sb, "L {} {} ", sq.x, sq.y);
            for z in 0..=1 {
                for dy in -10..=10 {
                    for dx in -10..=10 {
                        let r = line_clear(
                            &world.sprites,
                            &world.alias,
                            &grid,
                            sq.x,
                            sq.y,
                            0,
                            sq.x + dx,
                            sq.y + dy,
                            z,
                            false,
                            10000,
                            true,
                        );
                        let _ = write!(sb, "{}", r as u8);
                    }
                }
            }
            sb.push('\n');
        }
        return Ok(sb);
    }
    let mut sb = String::new();
    for sq in &all {
        let (cm, _, _) = sq.calculate_matrices(&world.alias, &grid);
        let _ = write!(sb, "P {} {} {} {cm:x}", sq.x, sq.y, sq.z);
        for (dx, dy) in [
            (0, -1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
        ] {
            let next = grid.get_grid_square(sq.x + dx, sq.y + dy, sq.z);
            match sq.test_collide_special_objects(&world.alias, next, &grid) {
                None => sb.push_str(" -"),
                Some((x, y, z, i)) => {
                    let o = &grid
                        .get_grid_square(x, y, z)
                        .ok_or("object square")?
                        .objects[i];
                    let name = o
                        .sprite
                        .and_then(|s| world.sprites.sprite(s).name.clone())
                        .unwrap_or_else(|| "null".into());
                    let _ = write!(sb, " {}:{name}@{x},{y}", o.kind.class_name());
                }
            }
        }
        sb.push('\n');
    }
    Ok(sb)
}

/// The meta grid with procedural basements merged in for the oracle seed; pairs with BasementsProbe.java.
pub fn basements(root: &Path, range: &[String]) -> Result<String, String> {
    use zomboid_engine::zombie::basements::Basements;
    let r: Vec<i32> = range
        .iter()
        .map(|s| s.parse::<i32>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let media = root.join("media");
    let mut g = IsoMetaGrid::default();
    let dirs = IsoMetaGrid::lot_directories(&media, "Muldraugh, KY");
    g.create_step1(&media, &dirs, (-250, -250, 250, 250))?;
    for e in g.load_all_cells(&mut |_| 1, &mut |_, _| Vec::new()) {
        eprintln!("{e}");
    }
    let alarm = AlarmSettings {
        last_stand: false,
        power_grid_exists: true,
        alarm: 1,
        alarm_decay: 1,
    };
    g.create_step2(&alarm, &mut |_| 1, &mut |a, _| a);
    // WorldGenParams.setSeedString: the seed is the Java hash of the seed text.
    let seed = java::string_hash_code("DeadOxideOracle") as i64;
    let b = Basements::before_load_meta_grid(&media, &mut g, &dirs, seed)?;
    for l in &b.log {
        eprintln!("{l}");
    }
    let mut sb = String::new();
    let _ = writeln!(sb, "SEED {seed}");
    for p in &b.basement_placements {
        let _ = writeln!(sb, "P {} {} {} {} {} {}", p.name, p.x, p.y, p.z, p.w, p.h);
    }
    let mut h: u64 = 0xcbf29ce484222325;
    for bd in &g.buildings {
        mix_long(&mut h, g.building(*bd).id);
    }
    let _ = writeln!(sb, "B {} {h:x}", g.buildings.len());
    for cx in r[0]..=r[2] {
        for cy in r[1]..=r[3] {
            if g.get_header(cx, cy).is_none() {
                continue;
            }
            let Some(cell) = g.get_cell_data(cx, cy) else {
                continue;
            };
            let _ = writeln!(
                sb,
                "C {cx} {cy} {} {}",
                cell.room_list.len(),
                cell.buildings.len()
            );
            for rr in &cell.room_list {
                let room = g.room(*rr);
                let _ = writeln!(
                    sb,
                    "R {} {} {} {} {} {} {} {} [{}]",
                    room.id,
                    room.meta_id,
                    room.level,
                    room.x,
                    room.y,
                    room.x2,
                    room.y2,
                    room.building.map_or(-1, |b| g.building(b).id),
                    room.name
                );
            }
            for bd in &cell.buildings {
                let d = g.building(*bd);
                let _ = writeln!(
                    sb,
                    "D {} {} {} {} {} {} {} {}",
                    d.id,
                    d.meta_id,
                    d.x,
                    d.y,
                    d.x2,
                    d.y2,
                    d.rooms.len(),
                    d.emptyoutside.len()
                );
            }
            for ly in 0..32 {
                for lx in 0..32 {
                    let Some(ch) = cell.chunk((ly * 32 + lx) as usize) else {
                        continue;
                    };
                    if ch.get_rooms_size() == 0 {
                        continue;
                    }
                    let mut ids: Vec<i64> = ch.rooms().iter().map(|rr| g.room(*rr).id).collect();
                    ids.sort_unstable();
                    let _ = write!(sb, "K {cx} {cy} {lx} {ly}");
                    for id in ids {
                        let _ = write!(sb, " {id}");
                    }
                    sb.push('\n');
                }
            }
            let mut h: u64 = 0xcbf29ce484222325;
            for z in -32..=8 {
                for y in cy * 256..cy * 256 + 256 {
                    for x in cx * 256..cx * 256 + 256 {
                        mix_long(
                            &mut h,
                            g.get_room_at(x, y, z).map_or(-1, |rr| g.room(rr).id),
                        );
                        mix_long(
                            &mut h,
                            g.get_empty_outside_at(x, y, z)
                                .map_or(-1, |rr| g.room(rr).id),
                        );
                    }
                }
            }
            let _ = writeln!(sb, "H {cx} {cy} {h:x}");
        }
    }
    Ok(sb)
}

/// Sandbox options at their defaults and after loading a save's map_sand.bin; pairs with SandboxProbe.java.
pub fn sandbox(root: &Path, file: &Path) -> Result<String, String> {
    use zomboid_engine::zombie::sandbox_options::{Kind, SandboxOptions};
    let mut s = SandboxOptions::new();
    let preset = std::fs::read_to_string(root.join("media/lua/shared/Sandbox/Apocalypse.lua"))
        .map_err(|e| e.to_string())?;
    s.load_game_file(&preset);
    let mut sb = String::new();
    let dump = |sb: &mut String, tag: &str, s: &SandboxOptions| {
        for (i, d) in s.defs().iter().enumerate() {
            let ty = match d.kind {
                Kind::Boolean(_) => "boolean",
                Kind::Double { .. } => "double",
                Kind::Integer { .. } => "integer",
                Kind::Enum { .. } => "enum",
                Kind::Str { .. } => "string",
            };
            let _ = writeln!(sb, "{tag}\t{}\t{ty}\t{}", d.name, s.value_as_string(i));
        }
    };
    dump(&mut sb, "D", &s);
    s.load(&std::fs::read(file).map_err(|e| e.to_string())?)?;
    for l in &s.log {
        eprintln!("{l}");
    }
    dump(&mut sb, "L", &s);
    let _ = writeln!(sb, "P\t{}", s.preset);
    Ok(sb)
}
