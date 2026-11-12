//! Dumps engine state in the exact text format of the Java probes in `tests/original_probe`, for byte-for-byte diffs.
//!   pz-dump tiledefs <game root>      (pairs with TileDefsProbe.java)
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use zomboid_engine::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use zomboid_engine::zombie::iso::iso_world::{
    self, TileDefinitions, VANILLA_TILE_DEFINITION_FILES,
};
use zomboid_engine::zombie::iso::sprite::iso_sprite_manager::IsoSpriteManager;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = match a.get(1).map(String::as_str) {
        Some("tiledefs") if a.len() == 3 => tiledefs(Path::new(&a[2])),
        _ => Err("usage: pz-dump tiledefs <game root>".into()),
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
