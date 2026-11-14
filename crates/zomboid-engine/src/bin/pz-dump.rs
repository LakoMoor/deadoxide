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
        Some("items") if a.len() == 4 => items(Path::new(&a[2]), Path::new(&a[3])),
        Some("sandbox") if a.len() == 4 => sandbox(Path::new(&a[2]), Path::new(&a[3])),
        Some("specialcollide") if a.len() == 5 || a.len() == 6 => {
            specialcollide(Path::new(&a[2]), &a[3..], false)
        }
        Some("stairs") if a.len() == 5 => specialcollide_mode(Path::new(&a[2]), &a[3..], 2),
        Some("los") if a.len() == 5 || a.len() == 6 => {
            specialcollide(Path::new(&a[2]), &a[3..], true)
        }
        Some("basements") if a.len() == 7 => basements(Path::new(&a[2]), &a[3..7]),
        Some("worlddictionary") if a.len() == 4 => worlddictionary(Path::new(&a[3])),
        Some("chunksave") if a.len() == 4 => chunksave(Path::new(&a[2]), Path::new(&a[3])),
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

/// Item scripts, pairs with ItemsProbe.java: every loaded item sorted by full name, with the fields named in `fields`
/// (one per line, from tools/gen_item_params.py); floats as raw bits, null as "null".
pub fn items(root: &Path, fields: &Path) -> Result<String, String> {
    use zomboid_engine::zombie::scripting::item::Value;
    use zomboid_engine::zombie::scripting::script_manager::ScriptManager;
    let names =
        std::fs::read_to_string(fields).map_err(|e| format!("{}: {e}", fields.display()))?;
    let names: Vec<&str> = names.lines().filter(|l| !l.is_empty()).collect();
    let m = ScriptManager::load(root)?;
    for (n, e) in &m.item_errors {
        eprintln!("dropped {n}: {e}");
    }
    let mut items: Vec<_> = m.items.iter().collect();
    // java.lang.String.compareTo: UTF-16 order; item names are ASCII.
    items.sort_by_key(|i| i.full_name());
    let mut sb = String::new();
    for it in items {
        sb.push_str(&it.full_name());
        for n in &names {
            let v = it.get(n).ok_or_else(|| format!("no field {n}"))?;
            sb.push('\t');
            match v {
                Value::F32(f) => {
                    let _ = write!(sb, "{:x}", f.to_bits());
                }
                Value::I32(i) => {
                    let _ = write!(sb, "{i}");
                }
                Value::Bool(b) => {
                    let _ = write!(sb, "{b}");
                }
                Value::Str(x) => sb.push_str(x.as_deref().unwrap_or("null")),
            }
        }
        sb.push('\n');
    }
    Ok(sb)
}

/// Every table of a save's WorldDictionary.bin; pairs with WorldDictionaryProbe.java.
pub fn worlddictionary(path: &Path) -> Result<String, String> {
    use zomboid_engine::zombie::world::dictionary_data::{DictionaryData, DictionaryInfo};
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let d = DictionaryData::load_from_byte_buffer(&data)?;
    let info = |sb: &mut String, tag: &str, i: &DictionaryInfo| {
        let _ = writeln!(
            sb,
            "{tag}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            i.registry_id,
            i.full_type,
            i.module_name,
            i.name,
            i.mod_id,
            i.is_modded,
            i.exists_as_vanilla,
            i.obsolete,
            i.removed,
            i.mod_overrides
                .as_ref()
                .map_or("null".to_string(), |v| v.join(","))
        );
    };
    let mut sb = String::new();
    let _ = writeln!(
        sb,
        "N\t{}\t{}\t{}",
        d.next_info_id, d.next_object_name_id, d.next_sprite_name_id
    );
    for i in d.items.values() {
        info(&mut sb, "I", i);
    }
    let _ = writeln!(sb, "IT\t{}", d.item_types.len());
    for i in d.entities.values() {
        info(&mut sb, "E", i);
    }
    let _ = writeln!(sb, "ET\t{}", d.entity_types.len());
    for (k, v) in &d.object_names {
        let _ = writeln!(sb, "O\t{k}\t{v}");
    }
    let _ = writeln!(sb, "OT\t{}", d.object_ids.len());
    for (k, v) in &d.sprite_names {
        let _ = writeln!(sb, "S\t{k}\t{v}");
    }
    let _ = writeln!(sb, "ST\t{}", d.sprite_ids.len());
    for r in &d.string_registers {
        let _ = writeln!(sb, "R\t{}\t{}", r.name, r.next_id);
        for i in r.by_id.values() {
            let _ = writeln!(sb, "r\t{}\t{}\t{}", i.registry_id, i.string, i.is_loaded);
        }
    }
    for r in &d.script_registers {
        let _ = writeln!(sb, "C\t{}\t{}", r.name, r.next_id);
        for i in r.by_id.values() {
            let _ = writeln!(
                sb,
                "c\t{}\t{}\t{}\t{}",
                i.registry_id, i.version, i.name, i.is_loaded
            );
        }
    }
    Ok(sb)
}

mod save_dump {
    use std::fmt::Write as _;

    use zomboid_engine::se::krka::kahlua::j2se::kahlua_table_impl::{KahluaTable, LuaValue};
    use zomboid_engine::zombie::core::skinnedmodel::visual::human_visual::HumanVisual;
    use zomboid_engine::zombie::inventory::inventory_item::{InventoryItem, ItemKind, ItemType};
    use zomboid_engine::zombie::inventory::item_container::ItemContainer;
    use zomboid_engine::zombie::inventory::types::alarm_clock::AlarmClockSave;
    use zomboid_engine::zombie::inventory::types::clothing::ClothingSave;
    use zomboid_engine::zombie::inventory::types::moveable::MoveableSave;
    use zomboid_engine::zombie::iso::chunk_save::{ClassSave, ObjectSave};
    use zomboid_engine::zombie::iso::iso_chunk::IsoChunk;
    use zomboid_engine::zombie::iso::iso_grid_square::IsoGridSquare;
    use zomboid_engine::zombie::iso::iso_object::{IsoObject, ObjectKind};
    use zomboid_engine::zombie::iso::objects::iso_dead_body::IsoDeadBody;
    use zomboid_engine::zombie::iso::sprite::iso_sprite_manager::{IsoSpriteManager, SpriteRef};
    use zomboid_engine::zombie::world::dictionary_data::DictionaryData;
    use zomboid_engine::zombie::world_map::symbols::world_map_symbols::WorldMapSymbol;

    /// `Integer.toHexString(Float.floatToRawIntBits(f))`
    pub fn fh(f: f32) -> String {
        format!("{:x}", f.to_bits())
    }
    fn dh(d: f64) -> String {
        format!("{:x}", d.to_bits())
    }
    fn b(v: bool) -> char {
        if v {
            '1'
        } else {
            '0'
        }
    }
    fn s(v: Option<&str>) -> &str {
        v.unwrap_or("null")
    }
    /// `sp == null ? "-" : String.valueOf(sp.name)`
    fn sn(sm: &IsoSpriteManager, sp: Option<SpriteRef>) -> String {
        sp.map_or("-".to_string(), |r| {
            s(sm.sprite(r).name.as_deref()).to_string()
        })
    }
    fn lua(v: &LuaValue) -> String {
        match v {
            LuaValue::String(x) => format!("s:{x}"),
            LuaValue::Number(x) => format!("n:{}", dh(*x)),
            LuaValue::Boolean(x) => format!("b:{x}"),
            LuaValue::Table(t) => table(t),
        }
    }
    fn table(t: &KahluaTable) -> String {
        let mut l: Vec<String> = t
            .entries
            .iter()
            .map(|(k, v)| format!("{}={}", lua(k), lua(v)))
            .collect();
        l.sort();
        format!("{{{}}}", l.join(","))
    }
    /// `hasModData() ? lua(getModData()) : "-"`
    fn mod_data(t: Option<&KahluaTable>) -> String {
        t.filter(|t| !t.entries.is_empty())
            .map_or("-".to_string(), table)
    }
    fn color(c: Option<[i32; 3]>) -> String {
        c.map_or("null".to_string(), |c| {
            c.map(|v| fh(v as f32 / 255.0)).join(",")
        })
    }
    fn bytes(a: Option<&[i8]>) -> String {
        a.map_or("null".to_string(), |a| {
            a.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
    }
    fn human_visual(sb: &mut String, v: &HumanVisual) {
        let white = Some([255, 255, 255]);
        let _ = writeln!(
            sb,
            "HV\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            color(v.skin_color.or(white)),
            v.skin_texture,
            s(v.skin_texture_name.as_deref()),
            v.zombie_rot_stage,
            color(v.hair_color),
            color(v.beard_color),
            color(v.natural_hair_color),
            color(v.natural_beard_color),
            s(v.hair_model.as_deref()),
            s(v.beard_model.as_deref()),
            v.body_hair,
            bytes(Some(&v.blood)),
            bytes(Some(&v.dirt)),
            bytes(Some(&v.holes)),
            s(v.non_attached_hair.as_deref())
        );
        for iv in &v.body_visuals {
            let _ = write!(
                sb,
                "IV\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                iv.full_type,
                iv.alternate_model_name,
                iv.clothing_item_name,
                color(iv.tint),
                iv.base_texture,
                iv.texture_choice,
                fh(iv.hue),
                s(iv.decal.as_deref())
            );
            for a in [
                &iv.blood,
                &iv.dirt,
                &iv.holes,
                &iv.basic_patches,
                &iv.denim_patches,
                &iv.leather_patches,
            ] {
                let _ = write!(sb, "\t{}", bytes(a.as_ref().map(|a| &a[..])));
            }
            sb.push('\n');
        }
    }
    fn container(sb: &mut String, dict: &DictionaryData, c: &ItemContainer) {
        let _ = write!(
            sb,
            "C\t{}\t{}\t{}\t{}",
            c.r#type,
            b(c.explored),
            b(c.has_been_looted),
            c.capacity
        );
        for it in &c.items {
            let _ = write!(sb, "\t{}:{}", it.full_type, it.id);
        }
        sb.push('\n');
        for it in &c.items {
            item(sb, dict, it);
        }
    }

    /// `o == null ? "null" : "s:" + o`
    fn st(v: Option<&str>) -> String {
        v.map_or("null".to_string(), |v| format!("s:{v}"))
    }
    /// java.util.List.toString
    fn list(v: Option<&[String]>) -> String {
        v.map_or("null".to_string(), |v| format!("[{}]", v.join(", ")))
    }

    /// The I and J lines ChunkSaveProbe.java prints for an item (and its bag's contents).
    pub fn item(sb: &mut String, dict: &DictionaryData, it: &InventoryItem) {
        let cls = it.item_type.class_name().unwrap_or("?");
        let tinted = matches!(
            it.item_type,
            ItemType::Clothing | ItemType::AlarmClockClothing | ItemType::Container
        );
        let moveable = matches!(it.item_type, ItemType::Moveable | ItemType::Radio);
        let names: Vec<String> = it
            .extra_items
            .iter()
            .map(|id| {
                dict.items
                    .get(id)
                    .map_or("null".to_string(), |i| i.full_type.clone())
            })
            .collect();
        let extra = format!("[{}]", names.join(", "));
        let _ = write!(
            sb,
            "I\t{}:{}\t{cls}\t{}\t{}/{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{extra}\t{}\t{}\t{}\t{},{}\t{},{},{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{},{},{}\t{}\t{}",
            it.full_type,
            it.id,
            it.uses,
            it.condition,
            it.condition_max,
            b(it.custom_color),
            if tinted { "~".to_string() } else { it.col.map(fh).join(",") },
            fh(it.item_capacity),
            b(it.activated),
            it.have_been_repaired,
            if moveable || it.recorded_media_id.is_some() {
                "~".to_string()
            } else {
                it.name.as_deref().map_or("=".to_string(), |n| format!("s:{n}"))
            },
            it.byte_data.as_ref().map_or("-".to_string(), |d| d.len().to_string()),
            b(it.custom_name),
            if moveable { "~".to_string() } else { it.custom_weight.map_or("-".to_string(), fh) },
            it.key_id,
            it.remote_control_id,
            it.remote_range,
            fh(it.color_red),
            fh(it.color_green),
            fh(it.color_blue),
            st(it.worker.as_deref()),
            fh(it.wet_cooldown),
            b(it.favorite),
            st(it.stash_map.as_deref()),
            b(it.zombie_infected),
            it.current_ammo_count,
            it.attached_slot,
            st(it.attached_slot_type.as_deref()),
            st(it.attached_to_model.as_deref()),
            it.max_capacity,
            it.recorded_media_id.as_deref().map_or(it.recorded_media_index.to_string(), |id| format!(
                "{}={id}",
                it.recorded_media_index
            )),
            fh(it.world_x_rotation),
            fh(it.world_y_rotation),
            fh(it.world_z_rotation),
            fh(it.world_scale),
            b(it.is_initialised)
        );
        let _ = writeln!(
            sb,
            "\t{}\t{}",
            mod_data(it.table.as_ref()),
            it.custom_name_entry.as_ref().map_or("-".to_string(), |n| {
                if moveable {
                    "~".to_string()
                } else {
                    n.as_deref().map_or("=".to_string(), |n| format!("s:{n}"))
                }
            })
        );
        let mut x = String::from("J");
        match &it.kind {
            ItemKind::Plain => {}
            ItemKind::Key(k) => {
                let _ = write!(x, "\t{}", k.number_of_key);
            }
            ItemKind::AlarmClock(a) => alarm(&mut x, a),
            ItemKind::Clothing(c, a) => {
                clothing(&mut x, c);
                if let Some(a) = a {
                    alarm(&mut x, a);
                }
            }
            ItemKind::Container(c) => {
                let _ = write!(x, "\t{}", c.weight_reduction);
            }
            ItemKind::Food(f) => {
                let la = if f.last_aged == -1.0 {
                    "W".to_string()
                } else {
                    fh(f.last_aged)
                };
                let _ = write!(x, "\t{}\t{la}", fh(f.age));
                for v in [
                    f.calories,
                    f.proteins,
                    f.lipids,
                    f.carbohydrates,
                    f.hung_change,
                    f.base_hunger,
                    f.unhappy_change,
                    f.boredom_change,
                    f.thirst_change,
                    f.heat,
                ] {
                    let _ = write!(x, "\t{}", fh(v));
                }
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    f.last_cook_minute,
                    fh(f.cooking_time),
                    b(f.cooked),
                    b(f.burnt),
                    b(f.is_cookable),
                    b(f.dangerous_uncooked),
                    f.poison_detection_level,
                    list(f.spices.as_deref()),
                    f.poison_power,
                    st(f.chef.as_deref()),
                    f.off_age,
                    f.off_age_max,
                    fh(f.pain_reduction),
                    f.flu_reduction,
                    f.food_sickness_change,
                    b(f.poison),
                    f.use_for_poison,
                    fh(f.freezing_time),
                    b(f.frozen),
                    fh(f.last_frozen_update),
                    fh(f.rotten_time),
                    fh(f.compost_time),
                    b(f.cooked_in_microwave),
                    fh(f.fatigue_change),
                    fh(f.end_change),
                    f.milk.as_ref().map_or(0, |m| m.0),
                    st(f.milk.as_ref().map(|m| m.1.as_str())),
                    fh(f.stress_change)
                );
            }
            ItemKind::Literature(l) => {
                let pages = l.custom_pages.as_ref().map_or(String::new(), |p| {
                    p.iter()
                        .enumerate()
                        .map(|(i, t)| format!("{}={t}", i + 1))
                        .collect::<Vec<_>>()
                        .join(", ")
                });
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{{{pages}}}\t{}\t{}",
                    l.number_of_pages,
                    l.already_read_pages,
                    b(l.can_be_write),
                    st(l.locked_by.as_deref()),
                    list(l.learned_recipes.as_deref())
                );
            }
            ItemKind::Weapon(w) => {
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t",
                    fh(w.max_range),
                    fh(w.min_range_ranged),
                    w.clip_size,
                    fh(w.min_damage),
                    fh(w.max_damage),
                    w.recoil_delay,
                    w.aiming_time,
                    w.reload_time,
                    w.hit_chance,
                    fh(w.min_angle),
                    w.explosion_timer,
                    fh(w.max_angle),
                    fh(w.blood_level),
                    b(w.is_jammed)
                );
                for p in &w.parts {
                    let _ = write!(x, "{p};");
                }
            }
            ItemKind::Moveable(m) => moveable_fields(&mut x, m),
            ItemKind::Radio(m, r) => {
                moveable_fields(&mut x, m);
                let d = r.device_data.clone().unwrap_or_default();
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    d.device_name,
                    b(d.two_way),
                    d.transmit_range,
                    d.mic_range,
                    b(d.mic_is_muted),
                    fh(d.base_volume_range),
                    fh(d.device_volume),
                    b(d.is_portable),
                    b(d.is_television),
                    b(d.is_high_tier),
                    b(d.is_turned_on),
                    d.channel,
                    d.min_channel_range,
                    d.max_channel_range,
                    b(d.is_battery_powered),
                    b(d.has_battery),
                    fh(d.power_delta),
                    fh(d.use_delta),
                    d.headphone_type,
                    d.media_index,
                    d.media_type,
                    s(d.media_item.as_deref()),
                    b(d.no_transmit)
                );
            }
            ItemKind::Map(m) => {
                let _ = write!(x, "\t{}", st(Some(&m.map_id)));
                for sy in &m.symbols.symbols {
                    let (name, base) = match sy {
                        WorldMapSymbol::Text { base, .. } => ("WorldMapTextSymbol", base),
                        WorldMapSymbol::Texture { base, .. } => ("WorldMapTextureSymbol", base),
                    };
                    let _ = write!(x, "\t{name}");
                    for v in [
                        base.x,
                        base.y,
                        base.anchor_x,
                        base.anchor_y,
                        base.scale,
                        base.rotation,
                        base.r,
                        base.g,
                        base.b,
                        base.a,
                    ] {
                        let _ = write!(x, ",{}", fh(v));
                    }
                    let _ = write!(
                        x,
                        ",{}{}{},{},{}",
                        b(base.collide),
                        b(base.match_perspective),
                        b(base.apply_zoom),
                        fh(base.min_zoom.unwrap_or(0.0)),
                        fh(base.max_zoom.unwrap_or(24.0))
                    );
                    match sy {
                        WorldMapSymbol::Text {
                            text,
                            translated,
                            layer_id,
                            ..
                        } => {
                            let _ = write!(
                                x,
                                ",s:{text},{},{}",
                                b(*translated),
                                layer_id.as_deref().unwrap_or("text-note")
                            );
                        }
                        WorldMapSymbol::Texture { symbol_id, .. } => {
                            let _ = write!(x, ",s:{symbol_id}");
                        }
                    }
                }
            }
        }
        if x.len() > 1 {
            sb.push_str(&x);
            sb.push('\n');
        }
        if let ItemKind::Container(c) = &it.kind {
            container(sb, dict, &c.container);
        }
    }

    fn alarm(x: &mut String, a: &AlarmClockSave) {
        let _ = write!(
            x,
            "\t{}\t{}\t{}\t{}\t{}",
            a.alarm_hour,
            a.alarm_minutes,
            b(a.alarm_set),
            dh(f64::from(a.ring_since)),
            a.force_dont_ring
        );
    }

    fn clothing(x: &mut String, c: &ClothingSave) {
        let _ = write!(x, "\t{}\t", fh(c.wetness));
        for (k, p) in c.patches.iter().flatten() {
            let _ = write!(
                x,
                "{k}:{}/{}/{}/{}/{}/{};",
                p.tailor_lvl,
                p.fabric_type,
                p.scratch_defense,
                p.bite_defense,
                b(p.has_hole),
                p.condition_gain
            );
        }
    }

    fn moveable_fields(x: &mut String, m: &MoveableSave) {
        let _ = write!(
            x,
            "\t{}\t{}",
            st(Some(&m.world_sprite)),
            b(m.light.is_some())
        );
        if let Some(l) = &m.light {
            let _ = write!(
                x,
                "\t{}\t{}\t{}",
                b(l.use_battery),
                b(l.has_battery),
                st(Some(l.bulb_item.as_deref().unwrap_or("Base.LightBulb")))
            );
            for v in [l.power, l.delta, l.r, l.g, l.b] {
                let _ = write!(x, "\t{}", fh(v));
            }
        }
    }

    fn object(
        sb: &mut String,
        sm: &IsoSpriteManager,
        dict: &DictionaryData,
        sq: &IsoGridSquare,
        index: usize,
        o: &IsoObject,
    ) {
        let default = ObjectSave::default();
        let os = o.save.as_deref().unwrap_or(&default);
        let world = sq
            .save
            .as_ref()
            .is_some_and(|s| s.world_objects.contains(&index));
        let key_id = match &os.class {
            ClassSave::Door { key_id, .. } => *key_id,
            _ => os.key_id,
        };
        let mut comps: Vec<String> = os.components.iter().map(|(id, _)| id.to_string()).collect();
        comps.sort_by_key(|c| c.parse::<i32>().unwrap_or(0));
        let _ = writeln!(
            sb,
            "O\t{index}\t{}\t{}\t{}\t{}\t{}{}\t{}{}{}{}{}\t{}\t{key_id}\t{}\t{}\t{}\t{}\t{}\t[{}]\t{}",
            o.kind.class_name(),
            sn(sm, o.sprite),
            s(os.name.as_deref()),
            s(os.sprite_name.as_deref()),
            b(o.special),
            b(world),
            b(o.outline_on_mouseover),
            b(os.do_not_sync),
            b(os.special_tooltip),
            b(os.uses_external_water_source),
            b(os.moved_thumpable),
            fh(o.render_y_offset),
            os.custom_color.map_or("-".to_string(), |c| c.map(fh).join(",")),
            os.sheet_rope_health.map_or("-".to_string(), fh),
            sn(sm, os.overlay_sprite),
            os.overlay_sprite_color.map_or("-".to_string(), |c| c.map(fh).join(",")),
            s(os.sprite_model_name.as_deref()),
            comps.join(", "),
            mod_data(os.mod_data.as_ref())
        );
        for (spr, a) in o.attached_anim_sprite.iter().flatten().zip(&os.attached) {
            let d = a.data.unwrap_or([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
            let _ = writeln!(
                sb,
                "A\t{}\t{}{}{}\t{},{},{}\t{},{},{}\t{}",
                sn(sm, Some(*spr)),
                b(a.flip),
                b(a.copy_target_alpha),
                b(a.multiply_object_alpha),
                fh(d[0]),
                fh(d[1]),
                fh(d[2]),
                fh(d[3]),
                fh(d[4]),
                fh(d[5]),
                fh(a.alpha.unwrap_or(1.0))
            );
        }
        for w in &os.wall_blood_splats {
            let _ = writeln!(sb, "W\t{}", sn(sm, w.sprite));
        }
        for c in &os.containers {
            container(sb, dict, c);
        }
        let mut x = String::from("X");
        match (&o.kind, &os.class) {
            (
                ObjectKind::IsoDoor(d),
                ClassSave::Door {
                    health,
                    max_health,
                    key_id,
                    has_curtain,
                    curtain_open,
                    curtain_inside,
                },
            ) => {
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{health}\t{max_health}\t{}\t{}\t{key_id}\t{}\t{}\t{}\t{}",
                    b(d.open),
                    b(d.locked),
                    b(d.north),
                    sn(sm, d.closed_sprite),
                    sn(sm, d.open_sprite),
                    b(d.locked_by_key),
                    b(*has_curtain),
                    b(*curtain_open),
                    b(*curtain_inside)
                );
            }
            (ObjectKind::IsoWindow(w), ClassSave::Window { health, max_health }) => {
                let _ = write!(
                    x,
                    "\t{}\t{}\t{health}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{max_health}",
                    b(w.open),
                    b(w.north),
                    b(w.locked),
                    b(w.perma_locked),
                    b(w.smashed),
                    b(w.glass_removed),
                    sn(sm, w.open_sprite),
                    sn(sm, w.closed_sprite),
                    sn(sm, w.smashed_sprite),
                    sn(sm, w.glass_removed_sprite)
                );
            }
            (
                ObjectKind::IsoCurtain(c),
                ClassSave::Curtain {
                    health,
                    barricade_strength,
                },
            ) => {
                let _ = write!(
                    x,
                    "\t{}\t{}\t{health}\t{barricade_strength}\t{}\t{}",
                    b(c.open),
                    b(c.north),
                    sn(sm, c.closed_sprite),
                    sn(sm, c.open_sprite)
                );
            }
            (ObjectKind::IsoWindowFrame { north }, _) => {
                let _ = write!(x, "\t{}", b(*north));
            }
            (ObjectKind::IsoTree(t), _) => {
                let _ = write!(x, "\t{}\t{}\t{}", t.size, t.log_yield, t.damage);
            }
            (_, ClassSave::LightSwitch(l)) => {
                let activated = if l.activated_from_room {
                    "?".to_string()
                } else {
                    b(l.activated).to_string()
                };
                let _ = write!(
                    x,
                    "\t{}\t{}\t{activated}\t{}\t{}\t{}\t{}",
                    b(l.light_room),
                    l.room_id,
                    b(l.can_be_modified),
                    b(l.use_battery),
                    b(l.has_battery),
                    s(l.bulb_item.as_deref())
                );
                for f in [l.power, l.delta, l.primary[0], l.primary[1], l.primary[2]] {
                    let _ = write!(x, "\t{}", fh(f));
                }
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}",
                    l.last_minute_stamp,
                    l.bulb_burn_minutes,
                    b(l.street_light)
                );
            }
            (
                ObjectKind::IsoBarbecue {
                    has_propane_tank,
                    fuel_amount_minutes,
                },
                ClassSave::Barbecue {
                    lit,
                    last_update_time,
                    minutes_since_extinguished,
                    normal,
                    no_tank,
                },
            ) => {
                let _ = write!(
                    x,
                    "\t{}\t{fuel_amount_minutes}\t{}\t{}\t{minutes_since_extinguished}\t{}\t{}",
                    b(*has_propane_tank),
                    b(*lit),
                    fh(*last_update_time),
                    sn(sm, *normal),
                    sn(sm, *no_tank)
                );
            }
            (
                _,
                ClassSave::Fireplace {
                    fuel_amount_minutes,
                    lit,
                    last_update_time,
                    minutes_since_extinguished,
                },
            ) => {
                let _ = write!(
                    x,
                    "\t{fuel_amount_minutes}\t{}\t{}\t{minutes_since_extinguished}",
                    b(*lit),
                    fh(*last_update_time)
                );
            }
            (
                _,
                ClassSave::Stove {
                    activated,
                    seconds_timer,
                    max_temperature,
                    first_turn_on,
                    broken,
                },
            ) => {
                let _ = write!(
                    x,
                    "\t{}\t{seconds_timer}\t{}\t{}\t{}",
                    b(*activated),
                    fh(*max_temperature),
                    b(*first_turn_on),
                    b(*broken)
                );
            }
            (
                _,
                ClassSave::Washer {
                    activated,
                    last_update,
                },
            ) => {
                let _ = write!(x, "\t{}\t{}", b(*activated), fh(*last_update));
            }
            (_, ClassSave::Dryer { activated }) => {
                let _ = write!(x, "\t{}", b(*activated));
            }
            (
                _,
                ClassSave::WasherDryer {
                    dryer,
                    washer_activated,
                    washer_last_update,
                    dryer_activated,
                },
            ) => {
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{}",
                    b(*dryer),
                    b(*washer_activated),
                    fh(*washer_last_update),
                    b(*dryer_activated)
                );
            }
            (_, ClassSave::WaveSignal(d)) => {
                let d = d.clone().unwrap_or_default();
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    d.device_name,
                    b(d.two_way),
                    d.transmit_range,
                    d.mic_range,
                    b(d.mic_is_muted),
                    fh(d.base_volume_range),
                    fh(d.device_volume),
                    b(d.is_portable),
                    b(d.is_television),
                    b(d.is_high_tier),
                    b(d.is_turned_on),
                    d.channel,
                    d.min_channel_range,
                    d.max_channel_range,
                    b(d.is_battery_powered),
                    b(d.has_battery),
                    fh(d.power_delta),
                    fh(d.use_delta),
                    d.headphone_type,
                    d.media_index,
                    d.media_type,
                    s(d.media_item.as_deref()),
                    b(d.no_transmit)
                );
                let p = d.presets.unwrap_or_default();
                let _ = write!(
                    x,
                    "\t{}",
                    if p.max_presets == 0 && p.presets.is_empty() {
                        10
                    } else {
                        p.max_presets
                    }
                );
                for (name, freq) in &p.presets {
                    let _ = write!(x, "\t{name}:{freq}");
                }
            }
            (_, ClassSave::Mannequin(m)) => {
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    m.dir & 7,
                    b(m.init),
                    b(m.female),
                    b(m.zombie),
                    b(m.skeleton),
                    m.mannequin_script_name,
                    m.pose
                );
            }
            (_, ClassSave::WorldInventory(w)) => {
                let (ft, id) = w
                    .item
                    .as_ref()
                    .map_or(("null".to_string(), 0), |i| (i.full_type.clone(), i.id));
                let _ = write!(
                    x,
                    "\t{}\t{}\t{}\t{ft}:{id}\t{}\t{}\t{}",
                    fh(w.xoff),
                    fh(w.yoff),
                    fh(w.zoff),
                    dh(w.drop_time),
                    b(w.ignore_remove_sandbox),
                    b(w.extended_placement)
                );
            }
            _ => {}
        }
        if x.len() > 1 {
            sb.push_str(&x);
            sb.push('\n');
        }
        if let ClassSave::Mannequin(m) = &os.class {
            human_visual(sb, &m.human_visual);
        }
        if let ClassSave::WorldInventory(w) = &os.class {
            if let Some(i) = &w.item {
                item(sb, dict, i);
            }
        }
    }

    fn corpse(sb: &mut String, dict: &DictionaryData, d: &IsoDeadBody) {
        let _ = write!(
            sb,
            "D\t{},{},{}\t{}{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            fh(d.x),
            fh(d.y),
            fh(d.z),
            b(d.female),
            b(d.was_zombie),
            d.object_id.0,
            d.persistent_outfit_id,
            fh(d.death_time),
            fh(d.reanimate_time),
            b(d.fall_on_front),
            b(d.killed_by_fall),
            b(d.was_skeleton),
            fh(d.angle),
            d.zombie_rot_stage_at_death,
            d.animal_rot_stage_at_death,
            s(d.rotten_texture.as_deref()),
            s(d.skel_inv_icon.as_deref())
        );
        let _ = writeln!(
            sb,
            "\t{}\t{}\t{}\t{}",
            b(d.crawling),
            b(d.fake_dead),
            b(d.ragdoll_fall),
            mod_data(d.mod_data.as_ref())
        );
        if let Some(desc) = &d.desc {
            let _ = writeln!(
                sb,
                "SD\t{}\t{}\t{}\t{}\t{}\t[{}]\t{}\t{}\t{}",
                desc.id,
                desc.forename,
                desc.surname,
                desc.torso,
                b(desc.female),
                desc.extra.join(", "),
                desc.voice_prefix,
                fh(desc.voice_pitch),
                desc.voice_type
            );
        }
        human_visual(sb, &d.human_visual);
        if let Some(c) = &d.container {
            container(sb, dict, c);
        }
    }

    /// The text ChunkSaveProbe.java prints for one loaded chunk.
    pub fn chunk(sb: &mut String, sm: &IsoSpriteManager, dict: &DictionaryData, ch: &IsoChunk) {
        let Some(c) = ch.save.as_deref() else { return };
        let _ = write!(
            sb,
            "K\t{}\t{}\t{}\t{}\t{}{}",
            ch.wx,
            ch.wy,
            ch.get_min_level(),
            ch.get_max_level(),
            b(c.blending_done_full),
            b(c.blending_done_partial)
        );
        for f in c.blending_modified {
            sb.push(b(f));
        }
        let _ = write!(
            sb,
            "\t{}\t{}",
            bytes(Some(&c.blending_depth)),
            b(c.attachments_done_full)
        );
        for f in c.attachments_state {
            sb.push(b(f));
        }
        let e = &c.erosion;
        let _ = write!(
            sb,
            "\t{}\t{},{},{},{},{},{}\t{}\t{}",
            c.attachments_partial
                .as_ref()
                .map_or("-".to_string(), |v| v.len().to_string()),
            b(e.init),
            e.e_tick_stamp,
            e.epoch,
            fh(e.moisture),
            fh(e.minerals),
            e.soil,
            c.generators_touching_this_chunk
                .as_ref()
                .map_or("-".to_string(), |v| v.len().to_string()),
            c.loot_respawn_hour
        );
        for (i, r) in c.spawned_rooms.iter().enumerate() {
            let _ = write!(sb, "{}{r}", if i == 0 { '\t' } else { ',' });
        }
        sb.push('\n');
        for s in &c.floor_blood_splats {
            let _ = writeln!(
                sb,
                "B\t{},{},{}\t{}\t{}",
                fh(s.x),
                fh(s.y),
                fh(s.z),
                s.r#type,
                s.index
            );
        }
        for z in ch.get_min_level()..=ch.get_max_level() {
            for i in 0..64 {
                let Some(sq) = ch.get_grid_square(i % 8, i / 8, z) else {
                    continue;
                };
                let default = Default::default();
                let ss = sq.save.as_deref().unwrap_or(&default);
                let es = &ss.erosion;
                let _ = write!(
                    sb,
                    "Q\t{}\t{}\t{}\t{}\t{}{}\t{}{}{}{}\t{}{},{},{},{}",
                    sq.x,
                    sq.y,
                    sq.z,
                    sq.room_id,
                    b(sq.room_id != -1 && sq.room.is_some()),
                    b(sq.roof_hide_building.is_some()),
                    b(ss.overlay_done),
                    b(sq.have_roof),
                    b(ss.burnt_out),
                    b(ss.have_sheet_rope),
                    b(es.init),
                    b(es.do_nothing),
                    es.noise_main_byte,
                    es.soil,
                    es.magic_num_byte
                );
                for r in &es.regions {
                    let _ = write!(
                        sb,
                        ",{}/{}/{}{}{}/{}/{}/{}",
                        r.region_id,
                        r.category_id,
                        b(r.do_nothing),
                        b(r.has_spawned),
                        b(r.disp_bloom),
                        r.stage,
                        r.disp_season,
                        r.game_obj
                    );
                }
                let _ = writeln!(sb, "\t{}", mod_data(ss.mod_data.as_ref()));
                for (oi, o) in sq.objects.iter().enumerate() {
                    object(sb, sm, dict, sq, oi, o);
                }
                for d in &ss.static_moving_objects {
                    corpse(sb, dict, d);
                }
            }
        }
    }
}

/// Every chunk of a save loaded through the port of IsoChunk.LoadFromDiskOrBuffer, in the world the oracle server
/// runs (seeded basements merged into the meta grid); pairs with ChunkSaveProbe.java.
pub fn chunksave(root: &Path, save: &Path) -> Result<String, String> {
    use zomboid_engine::zombie::basements::Basements;
    use zomboid_engine::zombie::inventory::inventory_item::{ItemFactory, ItemTemplate, ItemType};
    use zomboid_engine::zombie::iso::chunk_save::SaveLoadContext;
    use zomboid_engine::zombie::radio::media::recorded_media::RecordedMedia;
    use zomboid_engine::zombie::scripting::item::Value;
    use zomboid_engine::zombie::scripting::script_manager::ScriptManager;
    use zomboid_engine::zombie::world::dictionary_data::DictionaryData;

    /// What the factory needs from an item script: obsolete, ItemType (`None` when unknown) and conditionMax.
    struct ScriptInfo {
        obsolete: bool,
        item_type: Option<ItemType>,
        condition_max: i32,
        hunger_change: f32,
    }
    /// InventoryItemFactory.CreateItem(short): a valid dictionary entry whose item script exists.
    struct Factory<'a> {
        dictionary: &'a DictionaryData,
        scripts: std::collections::HashMap<String, ScriptInfo>,
        recorded_media: RecordedMedia,
        /// The `RecMedia["<id>"]` keys of media/lua/shared/RecordedMedia/recorded_media.lua, which the Lua
        /// `OnInitRecordedMedia` handler registers.
        media_ids: std::collections::HashSet<String>,
    }
    impl ItemFactory for Factory<'_> {
        fn create_item(&self, registry_id: i16) -> Option<ItemTemplate> {
            let info = self.dictionary.items.get(&registry_id)?;
            if info.obsolete || info.removed {
                return None;
            }
            let script = if info.full_type.starts_with("Moveables.") {
                "Moveables.Moveable"
            } else {
                info.full_type.as_str()
            };
            let si = self.scripts.get(script)?;
            Some(ItemTemplate {
                full_type: info.full_type.clone(),
                item_type: si.item_type?,
                condition_max: si.condition_max,
                // Item.InstanceItem: food.setBaseHunger(this.hungerChange / 100.0F)
                base_hunger: if si.item_type == Some(ItemType::Food) {
                    si.hunger_change / 100.0
                } else {
                    0.0
                },
            })
        }
        fn is_obsolete(&self, full_type: &str) -> bool {
            self.scripts.get(full_type).is_some_and(|s| s.obsolete)
        }
        fn media_data_from_index(&self, index: i16) -> Option<String> {
            self.recorded_media
                .get_media_data_from_index(index, &|id| self.media_ids.contains(id))
                .map(str::to_string)
        }
    }

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
    let seed = java::string_hash_code("DeadOxideOracle") as i64;
    Basements::before_load_meta_grid(&media, &mut g, &dirs, seed)?;
    let dict_path = save.join("WorldDictionary.bin");
    let dictionary = DictionaryData::load_from_byte_buffer(
        &std::fs::read(&dict_path).map_err(|e| format!("{}: {e}", dict_path.display()))?,
    )?;
    let scripts = ScriptManager::load(root)?;
    let media_path = save.join("recorded_media.bin");
    let recorded_media = RecordedMedia::load(
        &std::fs::read(&media_path).map_err(|e| format!("{}: {e}", media_path.display()))?,
    )?;
    let lua_path = media.join("lua/shared/RecordedMedia/recorded_media.lua");
    let media_ids = std::fs::read_to_string(&lua_path)
        .map_err(|e| format!("{}: {e}", lua_path.display()))?
        .lines()
        .filter_map(|l| {
            l.strip_prefix("RecMedia[\"")?
                .split('"')
                .next()
                .map(str::to_string)
        })
        .collect();
    let factory = Factory {
        recorded_media,
        media_ids,
        dictionary: &dictionary,
        scripts: scripts
            .items
            .iter()
            .map(|i| {
                let item_type = i
                    .unported
                    .iter()
                    .rev()
                    .find(|(k, _)| k == "itemtype")
                    .and_then(|(_, v)| ItemType::from_script(java::trim(v)));
                let condition_max = match i.get("conditionMax") {
                    Some(Value::I32(v)) => *v,
                    _ => 10,
                };
                let hunger_change = match i.get("hungerChange") {
                    Some(Value::F32(v)) => *v,
                    _ => 0.0,
                };
                (
                    i.full_name(),
                    ScriptInfo {
                        obsolete: matches!(i.get("obsolete"), Some(Value::Bool(true))),
                        item_type,
                        condition_max,
                        hunger_change,
                    },
                )
            })
            .collect(),
    };
    let mut files = std::collections::BTreeMap::new();
    let map = save.join("map");
    for xd in std::fs::read_dir(&map).map_err(|e| format!("{}: {e}", map.display()))? {
        let xd = xd.map_err(|e| e.to_string())?.path();
        let Some(wx) = xd
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };
        for yf in std::fs::read_dir(&xd).map_err(|e| format!("{}: {e}", xd.display()))? {
            let yf = yf.map_err(|e| e.to_string())?.path();
            let Some(wy) = yf
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".bin"))
                .and_then(|n| n.parse::<i32>().ok())
            else {
                continue;
            };
            files.insert((wx, wy), yf);
        }
    }
    let mut rand = RandAbstract::new(java::Random::new(0));
    let mut sb = String::new();
    for ((wx, wy), path) in files {
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let has_lot_header = g.get_header(wx * 8 / 256, wy * 8 / 256).is_some();
        let mut ctx = SaveLoadContext {
            sprites: &mut sm,
            alias: &alias,
            dictionary: &dictionary,
            items: &factory,
            meta_grid: &mut g,
            rand: &mut rand,
            has_lot_header,
        };
        match IsoChunk::load_from_disk_or_buffer(&data, wx, wy, &mut ctx) {
            Ok(ch) => save_dump::chunk(&mut sb, ctx.sprites, &dictionary, &ch),
            Err(e) => {
                let _ = writeln!(sb, "K\t{wx}\t{wy}\tFAIL\t{e}");
            }
        }
    }
    Ok(sb)
}
