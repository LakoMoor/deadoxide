//! Dumps engine state in the exact text format of the Java probes in `tests/original_probe`, for byte-for-byte diffs.
//!   pz-dump tiledefs <game root>                  (pairs with TileDefsProbe.java)
//!   pz-dump lotheaders <game root> <map folder>   (pairs with LotHeadersProbe.java)
//!   pz-dump lotpack <game root> <map folder> x1 y1 x2 y2   (pairs with LotPackProbe.java)
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use zomboid_engine::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use zomboid_engine::zombie::game_states::choose_game_info;
use zomboid_engine::zombie::iso::iso_lot::IsoLot;
use zomboid_engine::zombie::iso::iso_meta_cell::IsoMetaCell;
use zomboid_engine::zombie::iso::iso_meta_grid;
use zomboid_engine::zombie::iso::iso_world::{
    self, TileDefinitions, VANILLA_TILE_DEFINITION_FILES,
};
use zomboid_engine::zombie::iso::map_files::MapFiles;
use zomboid_engine::zombie::iso::sprite::iso_sprite_manager::IsoSpriteManager;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("tiledefs") if a.len() == 3 => tiledefs(Path::new(&a[2])),
        Some("lotheaders") if a.len() == 4 => lotheaders(Path::new(&a[2]), &a[3]),
        Some("lotpack") if a.len() == 8 => lotpack(Path::new(&a[2]), &a[3], &a[4..8]),
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

/// Every lotheader of one map folder through the port of MetaGridLoaderThread.loadCell, cells in x-then-y order.
/// `lightsActive` is random in the original and not printed.
pub fn lotheaders(root: &Path, dir: &str) -> Result<String, String> {
    let media = root.join("media");
    let details = choose_game_info::get_map_details(&media, dir).ok_or("no map.info")?;
    let mut mf = MapFiles::new(dir, &media.join("maps").join(dir), 0);
    if !mf.load(details.fixed2x)? {
        return Err("MapFiles.load false".into());
    }
    mf.post_load();
    let mut sb = String::new();
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
    let mut all = vec![mf];
    for x in all[0].min_x..=all[0].max_x {
        for y in all[0].min_y..=all[0].max_y {
            if !all[0].has_cell(x, y) {
                continue;
            }
            let name = format!("{x}_{y}.lotheader");
            let mut h = all[0].info_headers.remove(&name).ok_or("header missing")?;
            let data = std::fs::read(&h.absolute_file_path).map_err(|e| e.to_string())?;
            let mut c = IsoMetaCell::new(x, y);
            if let Err(e) =
                iso_meta_grid::load_cell(&all, 0, &mut h, &mut c, &data, x, y, &mut |_| 1, &[])
            {
                eprintln!("ERROR loading {name}: {e}");
            }
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
                    r.building.unwrap_or(-1),
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
                let _ = write!(
                    sb,
                    "B {} {} {} {} {} {} r",
                    b.id, b.meta_id, b.x, b.y, b.x2, b.y2
                );
                for r in &b.rooms {
                    let _ = write!(sb, " {}", c.room_list[*r].id);
                }
                sb.push_str(" e");
                for r in &b.emptyoutside {
                    let _ = write!(sb, " {}", c.room_list[*r].id);
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
            all[0].info_headers.insert(name, h);
        }
    }
    Ok(sb)
}

/// Every chunk of a cell range through the port of IsoLot.load; pairs with LotPackProbe.java.
pub fn lotpack(root: &Path, dir: &str, range: &[String]) -> Result<String, String> {
    let r: Vec<i32> = range.iter().map(|s| s.parse::<i32>().map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let media = root.join("media");
    let details = choose_game_info::get_map_details(&media, dir).ok_or("no map.info")?;
    let mut mf = MapFiles::new(dir, &media.join("maps").join(dir), 0);
    mf.load(details.fixed2x)?;
    mf.post_load();
    let mut all = vec![mf];
    let mut sb = String::new();
    let mut lot = IsoLot::default();
    for cx in r[0]..=r[2] {
        for cy in r[1]..=r[3] {
            if !all[0].has_cell(cx, cy) {
                continue;
            }
            let name = format!("{cx}_{cy}.lotheader");
            let mut info = all[0].info_headers.remove(&name).ok_or("header missing")?;
            let data = std::fs::read(&info.absolute_file_path).map_err(|e| e.to_string())?;
            let mut c = IsoMetaCell::new(cx, cy);
            iso_meta_grid::load_cell(&all, 0, &mut info, &mut c, &data, cx, cy, &mut |_| 1, &[])?;
            let pack_path = all[0].info_file_names.get(&format!("world_{cx}_{cy}.lotpack")).ok_or("no lotpack")?;
            let pack = std::fs::read(pack_path).map_err(|e| e.to_string())?;
            let mut h: u64 = 0xcbf29ce484222325;
            let mut mix = |v: i32| {
                for b in v.to_le_bytes() {
                    h ^= b as u64;
                    h = h.wrapping_mul(0x100000001b3);
                }
            };
            let n = ((info.max_level.min(31) - info.min_level.max(-32) + 1) * 64) as usize;
            let base = ((info.min_level.max(-32) - info.min_level) * 64) as usize;
            for lx in 0..32 {
                for ly in 0..32 {
                    if let Some(e) = lot.load(&info, &pack, cx, cy, cx * 32 + lx, cy * 32 + ly) {
                        eprintln!("ERROR {cx},{cy} chunk {lx},{ly}: {e}");
                    }
                    mix(lot.min_level);
                    mix(lot.max_level);
                    mix(lot.data.len() as i32);
                    for v in &lot.offset_in_data[base..base + n] {
                        mix(*v);
                    }
                    for v in &lot.data {
                        mix(*v);
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
            all[0].info_headers.insert(name, info);
        }
    }
    Ok(sb)
}
