//! zombie.iso.IsoWorld: so far the tile-definition loading part of `init`.
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::java;
use crate::zombie::core::math::pz_math;
use crate::zombie::core::properties::fascia_edge::FasciaEdge;
use crate::zombie::core::properties::iso_property_type as pt;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use crate::zombie::iso::sprite::iso_sprite::{self, RL_DEFAULT, RL_FLOOR};
use crate::zombie::iso::sprite::iso_sprite_grid::IsoSpriteGrid;
use crate::zombie::iso::sprite::iso_sprite_manager::{GridRef, IsoSpriteManager, SpriteRef};
use crate::zombie::iso::sprite::sprite_grid_parse_data::SpriteGridParseData;
use crate::zombie::iso::sprite_details::iso_flag_type::IsoFlagType as F;
use crate::zombie::iso::sprite_details::iso_object_type::IsoObjectType as T;
use crate::zombie::util::string_utils;

/// zombie.iso.IsoWorld.TDEF_FILE_MAGIC
const TDEF_FILE_MAGIC: &[u8; 4] = b"tdef";

/// The vanilla files in the order `IsoWorld.init` loads them, with their file numbers (-1 = patch file).
pub const VANILLA_TILE_DEFINITION_FILES: [(&str, i32); 7] = [
    ("newtiledefinitions.tiles", 1),
    ("tiledefinitions_erosion.tiles", 2),
    ("tiledefinitions_overlays.tiles", 4),
    ("tiledefinitions_b42chunkcaching.tiles", 5),
    ("tiledefinitions_noiseworks.patch.tiles", -1),
    ("jumbo_trees_big.tiles", 6),
    ("jumbo_trees.tiles", 8),
];

/// Byte cursor with the semantics of `IsoWorld.readInt(InputStream)` / `readString(InputStream, StringBuilder)`.
struct In<'a> {
    b: &'a [u8],
    p: usize,
}
impl In<'_> {
    /// zombie.iso.IsoWorld.readInt: little-endian; EOFException at end of data.
    fn read_int(&mut self) -> Result<i32, String> {
        let s = self
            .b
            .get(self.p..self.p + 4)
            .ok_or("java.io.EOFException")?;
        self.p += 4;
        Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    /// zombie.iso.IsoWorld.readString: bytes up to '\n' as Latin-1 chars; `None` (Java null) at end of data with nothing read.
    fn read_string(&mut self) -> Result<Option<String>, String> {
        let mut out = String::new();
        loop {
            match self.b.get(self.p) {
                None => return Ok(if out.is_empty() { None } else { Some(out) }),
                Some(b'\n') => {
                    self.p += 1;
                    return Ok(Some(out));
                }
                Some(b'\r') => {
                    return Err("java.lang.IllegalStateException: \\r\\n unsupported".into())
                }
                Some(c) => {
                    out.push(*c as char);
                    self.p += 1;
                }
            }
        }
    }
    /// `readString(...).trim()`: a null string is the NullPointerException the original runs into.
    fn read_trimmed(&mut self) -> Result<String, String> {
        Ok(java::trim(
            &self
                .read_string()?
                .ok_or("java.lang.NullPointerException")?,
        )
        .to_string())
    }
}

fn parse_int(s: &str) -> Result<i32, String> {
    java::parse_int(s)
        .ok_or_else(|| format!("java.lang.NumberFormatException: For input string: \"{s}\""))
}

fn header(input: &mut In, filename: &str, file_number: i32) -> Result<(i32, i32, i32), String> {
    let max_tilesets_per_file = if file_number == 1 { 1024 } else { 512 };
    let max_tiles_per_tileset = if file_number == 1 { 1024 } else { 512 };
    if input.b.get(..4) != Some(TDEF_FILE_MAGIC) {
        return Err("java.io.IOException: not magic".into());
    }
    input.p = 4;
    let version = input.read_int()?;
    if version != 1 {
        return Err(format!(
            "{filename}: invalid version {version}, must be from 1 to 1"
        ));
    }
    let num_tilesheets = input.read_int()?;
    if num_tilesheets < 0 || num_tilesheets > max_tilesets_per_file {
        return Err(format!("{filename}: invalid number of tilesets {num_tilesheets}, must be from 0 to {max_tilesets_per_file}"));
    }
    Ok((num_tilesheets, max_tilesets_per_file, max_tiles_per_tileset))
}

/// One tileset header as both loading passes read it.
struct Tileset {
    name: String,
    image_name: Option<String>,
    tileset_number: i32,
    n_tiles: i32,
}
fn tileset(
    input: &mut In,
    filename: &str,
    max_sets: i32,
    max_tiles: i32,
) -> Result<Tileset, String> {
    let name = input.read_trimmed()?;
    let image_name = input.read_string()?;
    let _w_tiles = input.read_int()?;
    let _h_tiles = input.read_int()?;
    let tileset_number = input.read_int()?;
    if tileset_number < 1 || tileset_number > max_sets {
        return Err(format!("{filename}: invalid tileset number {tileset_number}, must be from 1 to {max_sets} in {name}"));
    }
    let n_tiles = input.read_int()?;
    if n_tiles < 0 || n_tiles > max_tiles {
        return Err(format!("{filename}: invalid number of tiles {n_tiles}, must be from 0 to {max_tiles} in {name}"));
    }
    Ok(Tileset {
        name,
        image_name,
        tileset_number,
        n_tiles,
    })
}

/// The tile-definition state of `IsoWorld` (`PropertyValueMap`, `tileImages`) and the lookups built after loading.
#[derive(Default, Debug)]
pub struct TileDefinitions {
    /// zombie.iso.IsoWorld.PropertyValueMap: every value seen for each property, in first-seen order.
    pub property_value_map: BTreeMap<String, Vec<String>>,
    seen: HashSet<(String, String)>,
    /// zombie.iso.IsoWorld.tileImages
    pub tile_images: Vec<Option<String>>,
    /// zombie.iso.CellLoader.glassRemovedWindowSpriteMap: glass-removed sprite -> intact window sprite.
    pub glass_removed_window_sprite_map: HashMap<SpriteRef, SpriteRef>,
    /// zombie.iso.CellLoader.smashedWindowSpriteMap: smashed sprite -> intact window sprite.
    pub smashed_window_sprite_map: HashMap<SpriteRef, SpriteRef>,
}

impl TileDefinitions {
    /// zombie.iso.IsoWorld.LoadTileDefinitionsPropertyStrings: first pass, collects the value strings of every property.
    /// `Err` is the exception the original logs and swallows; what was read before it stays.
    pub fn load_tile_definitions_property_strings(
        &mut self,
        data: &[u8],
        filename: &str,
        file_number: i32,
    ) -> Result<(), String> {
        let mut input = In { b: data, p: 0 };
        let (num_tilesheets, max_sets, max_tiles) = header(&mut input, filename, file_number)?;
        for _ in 0..num_tilesheets {
            let ts = tileset(&mut input, filename, max_sets, max_tiles)?;
            self.tile_images.push(ts.image_name);
            for _ in 0..ts.n_tiles {
                let n_props = input.read_int()?;
                for _ in 0..n_props {
                    let prop = input.read_trimmed()?;
                    let val = input.read_trimmed()?;
                    // `values.contains(val)` of the original, without the linear scan.
                    if self.seen.insert((prop.clone(), val.clone())) {
                        self.property_value_map.entry(prop).or_default().push(val);
                    } else {
                        self.property_value_map.entry(prop).or_default();
                    }
                }
            }
        }
        Ok(())
    }

    /// zombie.iso.IsoWorld.SetCustomPropertyValues: values the engine sets itself and therefore must be registered.
    /// `Err` where the original would hit a NullPointerException because the tile files never mention the property.
    pub fn set_custom_property_values(&mut self) -> Result<(), String> {
        let m = &mut self.property_value_map;
        fn add(m: &mut BTreeMap<String, Vec<String>>, k: &str, v: &str) -> Result<(), String> {
            m.get_mut(k)
                .ok_or_else(|| {
                    format!("java.lang.NullPointerException: PropertyValueMap.get(\"{k}\")")
                })?
                .push(v.to_string());
            Ok(())
        }
        add(m, "WindowN", "WindowN")?;
        add(m, "WindowW", "WindowW")?;
        add(m, "DoorWallN", "DoorWallN")?;
        add(m, "DoorWallW", "DoorWallW")?;
        add(m, "WallSE", "WallSE")?;
        let offsets: Vec<String> = (-96..=96).map(|i: i32| i.to_string()).collect();
        for k in ["Noffset", "Soffset", "Woffset", "Eoffset"] {
            m.insert(k.to_string(), offsets.clone());
        }
        for v in ["5", "6", "7", "8"] {
            add(m, "tree", v)?;
        }
        add(m, pt::RED_LIGHT, "0")?;
        add(m, pt::GREEN_LIGHT, "0")?;
        add(m, pt::BLUE_LIGHT, "0")?;
        for i in 0..=96 {
            let value = i.to_string();
            for k in ["ItemHeight", "Surface"] {
                let values = m.get_mut(k).ok_or_else(|| {
                    format!("java.lang.NullPointerException: PropertyValueMap.get(\"{k}\")")
                })?;
                if !values.contains(&value) {
                    values.push(value.clone());
                }
            }
        }
        Ok(())
    }

    /// zombie.iso.IsoWorld.GenerateTilePropertyLookupTables
    pub fn generate_tile_property_lookup_tables(
        &mut self,
        alias: &mut TilePropertyAliasMap,
        entity_script_names: &[String],
    ) {
        alias.generate(&self.property_value_map, entity_script_names);
        self.property_value_map.clear();
        self.seen.clear();
    }

    /// zombie.iso.IsoWorld.loadedTileDefinitions: window sprite lookups, railings never cut away, and the
    /// `HasLightOnSprite` flag for sprites that have a textured `<tileset>_on_<index>` twin.
    /// `has_texture` answers `!sprite.hasNoTextures()` for a sprite's texture name.
    /// The managers the original notifies afterwards (sprite models, depth textures, geometry, attachments) are not ported.
    pub fn loaded_tile_definitions(
        &mut self,
        spr_man: &mut IsoSpriteManager,
        alias: &TilePropertyAliasMap,
        has_texture: &dyn Fn(&str) -> bool,
    ) -> Result<(), String> {
        self.glass_removed_window_sprite_map.clear();
        self.smashed_window_sprite_map.clear();
        let all: Vec<SpriteRef> = spr_man.named_map.values().copied().collect();
        for sprite in all {
            let props = &spr_man.sprite(sprite).properties;
            if props.has(F::windowW) || props.has(F::windowN) {
                for (key, glass) in [
                    (pt::GLASS_REMOVED_OFFSET, true),
                    (pt::SMASHED_TILE_OFFSET, false),
                ] {
                    let Some(val) = spr_man.sprite(sprite).properties.get(alias, key) else {
                        continue;
                    };
                    let offset = pz_math::try_parse_int(Some(val), 0);
                    if offset != 0 {
                        if let Some(sprite2) =
                            iso_sprite::get_sprite_offset(spr_man, sprite, offset)?
                        {
                            if glass {
                                self.glass_removed_window_sprite_map.insert(sprite2, sprite);
                            } else {
                                self.smashed_window_sprite_map.insert(sprite2, sprite);
                            }
                        }
                    }
                }
            }
            let s = spr_man.sprite(sprite);
            let railing = s
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with("fixtures_railings_01"));
            // Java concatenates a null tilesetName as the text "null".
            let on_name = format!(
                "{}_on_{}",
                s.tileset_name.as_deref().unwrap_or("null"),
                s.tile_sheet_index
            );
            let lit = spr_man.named_map.get(&on_name).is_some_and(|r| {
                spr_man
                    .sprite(*r)
                    .texture_name
                    .as_deref()
                    .is_some_and(has_texture)
            });
            let s = spr_man.sprite_mut(sprite);
            if railing {
                s.properties.set(F::NeverCutaway);
            }
            if lit {
                s.properties.set(F::HasLightOnSprite);
            } else {
                s.properties.unset(F::HasLightOnSprite);
            }
        }
        Ok(())
    }
}

/// zombie.iso.IsoWorld.getSpriteID
pub fn get_sprite_id(file_number: i32, tileset_number: i32, tile_index: i32) -> i32 {
    if file_number == 1 {
        (tileset_number - 1) * 1024 + tile_index
    } else {
        // Wrapping as Java ints do: the patch file is number -1.
        1048576i32
            .wrapping_add((file_number - 2).wrapping_mul(262144))
            .wrapping_add((tileset_number - 1) * 512)
            .wrapping_add(tile_index)
    }
}

/// zombie.iso.IsoWorld.registerFakeJumboTree
pub fn register_fake_jumbo_tree(
    spr_man: &mut IsoSpriteManager,
    alias: &TilePropertyAliasMap,
    file_number: i32,
) {
    let r = spr_man.add_sprite_with_id("jumbo_tree_01_0", file_number * 512 * 512 + 6144);
    let spr = spr_man.sprite_mut(r);
    spr.name = Some("jumbo_tree_01_0".into());
    spr.set_tile_type(T::tree);
    spr.properties.set_str(alias, "tree", "4", true);
    spr.properties.unset(F::solid);
    spr.properties.set(F::blocksight);
}

/// zombie.iso.IsoWorld.LoadTileDefinitions: second pass, creates (or for `.patch.tiles` files, amends) one sprite per
/// tile and derives engine flags from the tile properties. `Err` is the exception the original logs and swallows:
/// loading of this file stops there and everything done before it stays.
pub fn load_tile_definitions(
    spr_man: &mut IsoSpriteManager,
    alias: &TilePropertyAliasMap,
    data: &[u8],
    filename: &str,
    file_number: i32,
) -> Result<(), String> {
    let b_patch = filename.ends_with(".patch.tiles");
    let mut input = In { b: data, p: 0 };
    let (num_tilesheets, max_sets, max_tiles) = header(&mut input, filename, file_number)?;
    let face_names = ["N", "E", "S", "W"];
    let mut buffer: Vec<SpriteRef> = Vec::new();

    for _ in 0..num_tilesheets {
        let ts = tileset(&mut input, filename, max_sets, max_tiles)?;
        let name = ts.name.as_str();
        for m in 0..ts.n_tiles {
            let tile_name = format!("{name}_{m}");
            let r = if b_patch {
                match spr_man.named_map.get(&tile_name) {
                    Some(r) => *r,
                    None => {
                        let n_props = input.read_int()?;
                        for _ in 0..n_props {
                            input.read_string()?;
                            input.read_string()?;
                        }
                        continue;
                    }
                }
            } else {
                spr_man.add_sprite_with_id(
                    &tile_name,
                    get_sprite_id(file_number, ts.tileset_number, m),
                )
            };
            buffer.push(r);
            let spr = spr_man.sprite_mut(r);
            if !b_patch {
                spr.name = Some(tile_name.clone());
                spr.tileset_name = Some(name.to_string());
                spr.tile_sheet_index = m;
            }
            // A patched sprite always came from AddSprite(String, int) plus setName, so its name is set.
            let spr_name = spr
                .name
                .clone()
                .ok_or("java.lang.NullPointerException: spr.name")?;
            if spr_name.contains("damaged") || spr_name.contains("trash_") {
                spr.attached_floor = true;
                spr.properties.set_str(alias, "attachedFloor", "true", true);
            }
            if spr_name.starts_with("f_bushes") && m <= 31 {
                spr.is_bush = true;
                spr.attached_floor = true;
            }

            let n_props = input.read_int()?;
            for _ in 0..n_props {
                let prop_owned = input.read_trimmed()?;
                let prop = prop_owned.as_str();
                let mut val_owned = input.read_trimmed()?;
                let p = &mut spr.properties;
                let ty = T::from_string(prop);
                if ty != T::MAX {
                    if (spr.tile_type != T::doorW && spr.tile_type != T::doorN) || ty != T::wall {
                        spr.tile_type = ty;
                    }
                    if ty == T::doorW {
                        p.set(F::doorW);
                    } else if ty == T::doorN {
                        p.set(F::doorN);
                    }
                } else {
                    let val = val_owned.as_str();
                    match prop {
                        "firerequirement" | "fireRequirement" => {
                            spr.firerequirement = parse_int(val)?
                        }
                        "BurntTile" => spr.burnt_tile = Some(val.to_string()),
                        "FloorHeight" => {
                            if val == "OneThird" {
                                p.set(F::FloorHeightOneThird);
                            } else if val == "TwoThirds" {
                                p.set(F::FloorHeightTwoThirds);
                            }
                        }
                        _ => {
                            match prop {
                                "ForceAmbient" => spr.force_ambient = true,
                                "solidfloor" => spr.solidfloor = true,
                                "canBeRemoved" => spr.can_be_removed = true,
                                "attachedFloor" => spr.attached_floor = true,
                                "cutW" => spr.cut_w = true,
                                "cutN" => spr.cut_n = true,
                                "solid" => spr.solid = true,
                                "solidtrans" => spr.solid_trans = true,
                                "invisible" => spr.invisible = true,
                                "alwaysDraw" => spr.always_draw = true,
                                "FasciaEdge" => spr.fascia_edge = FasciaEdge::value_of(val),
                                "forceRender" => spr.force_render = true,
                                "MoveWithWind" => spr.move_with_wind = true,
                                "WindType" => spr.wind_type = parse_int(val)?,
                                "RenderLayer" => {}
                                "TreatAsWallOrder" => spr.treat_as_wall_order = true,
                                _ => {}
                            }
                            p.set_str(alias, prop, val, true);
                            match prop {
                                "RenderLayer" => {
                                    if val == "Default" {
                                        spr.render_layer = RL_DEFAULT;
                                    } else if val == "Floor" {
                                        spr.render_layer = RL_FLOOR;
                                    }
                                }
                                "WindowN" | "WindowW" => p.set_str(alias, prop, val, false),
                                _ => {}
                            }
                        }
                    }
                }

                if ty == T::tree {
                    if spr_name == "e_riverbirch_1_1" {
                        val_owned = "1".to_string();
                    }
                    p.set_str(alias, "tree", &val_owned, true);
                    p.set(F::blocksight);
                    let mut size = parse_int(&val_owned)?;
                    if name.starts_with("vegetation_trees") {
                        size = 4;
                    }
                    size = size.clamp(1, 4);
                    if size == 1 || size == 2 {
                        p.unset(F::blocksight);
                    }
                    p.set_str(alias, "MaterialType", "Wood_Solid", true);
                }
                let val = val_owned.as_str();

                if prop == "interior" && val == "false" {
                    p.set(F::exterior);
                }
                if prop == "HoppableN" {
                    p.set(F::collideN);
                    p.set(F::canPathN);
                    p.set(F::transparentN);
                }
                if prop == "HoppableW" {
                    p.set(F::collideW);
                    p.set(F::canPathW);
                    p.set(F::transparentW);
                }
                if prop == "WallN" {
                    p.set(F::collideN);
                    p.set(F::cutN);
                    spr.tile_type = T::wall;
                    spr.cut_n = true;
                    p.set_str(alias, "WallN", "", false);
                }

                if prop == "CantClimb" {
                    p.set(F::CantClimb);
                } else if java::equals_ignore_case(pt::CONTAINER, prop) {
                    p.set_str(alias, prop, val, false);
                } else {
                    match prop {
                        "WallNTrans" => {
                            p.set(F::collideN);
                            p.set(F::cutN);
                            p.set(F::transparentN);
                            spr.tile_type = T::wall;
                            spr.cut_n = true;
                            p.set_str(alias, "WallNTrans", "", false);
                        }
                        "WallW" => {
                            p.set(F::collideW);
                            p.set(F::cutW);
                            spr.tile_type = T::wall;
                            spr.cut_w = true;
                            p.set_str(alias, "WallW", "", false);
                        }
                        "windowN" => {
                            p.set_str(alias, "WindowN", "WindowN", true);
                            p.set(F::transparentN);
                            p.set_str(alias, "WindowN", "WindowN", false);
                        }
                        "windowW" => {
                            p.set_str(alias, "WindowW", "WindowW", true);
                            p.set(F::transparentW);
                            p.set_str(alias, "WindowW", "WindowW", false);
                        }
                        "cutW" => {
                            p.set(F::cutW);
                            spr.cut_w = true;
                        }
                        "cutN" => {
                            p.set(F::cutN);
                            spr.cut_n = true;
                        }
                        "WallWTrans" => {
                            p.set(F::collideW);
                            p.set(F::transparentW);
                            p.set(F::cutW);
                            spr.tile_type = T::wall;
                            spr.cut_w = true;
                            p.set_str(alias, "WallWTrans", "", false);
                        }
                        "DoorWallN" => {
                            p.set(F::cutN);
                            spr.cut_n = true;
                            p.set_str(alias, "DoorWallN", "", false);
                        }
                        "DoorWallNTrans" => {
                            p.set(F::cutN);
                            p.set(F::transparentN);
                            spr.cut_n = true;
                            p.set_str(alias, "DoorWallNTrans", "", false);
                        }
                        "DoorWallW" => {
                            p.set(F::cutW);
                            spr.cut_w = true;
                            p.set_str(alias, "DoorWallW", "", false);
                        }
                        "DoorWallWTrans" => {
                            p.set(F::cutW);
                            p.set(F::transparentW);
                            spr.cut_w = true;
                            p.set_str(alias, "DoorWallWTrans", "", false);
                        }
                        "WallNW" => {
                            p.set(F::collideN);
                            p.set(F::cutN);
                            p.set(F::collideW);
                            p.set(F::cutW);
                            spr.tile_type = T::wall;
                            spr.cut_w = true;
                            spr.cut_n = true;
                            p.set_str(alias, "WallNW", "", false);
                        }
                        "WallNWTrans" => {
                            p.set(F::collideN);
                            p.set(F::cutN);
                            p.set(F::collideW);
                            p.set(F::transparentN);
                            p.set(F::transparentW);
                            p.set(F::cutW);
                            spr.tile_type = T::wall;
                            spr.cut_w = true;
                            spr.cut_n = true;
                            p.set_str(alias, "WallNWTrans", "", false);
                        }
                        "WallSE" => {
                            p.set(F::cutW);
                            p.set(F::WallSE);
                            p.set_str(alias, "WallSE", "WallSE", true);
                            spr.cut_w = true;
                        }
                        "WindowW" => {
                            p.set(F::canPathW);
                            p.set(F::collideW);
                            p.set(F::cutW);
                            p.set(F::transparentW);
                            spr.tile_type = T::windowFW;
                            if p.has(F::HoppableW) {
                                p.unset(F::HoppableW);
                            }
                            spr.cut_w = true;
                        }
                        "WindowN" => {
                            p.set(F::canPathN);
                            p.set(F::collideN);
                            p.set(F::cutN);
                            p.set(F::transparentN);
                            spr.tile_type = T::windowFN;
                            if p.has(F::HoppableN) {
                                p.unset(F::HoppableN);
                            }
                            spr.cut_n = true;
                        }
                        "UnbreakableWindowW" => {
                            p.set(F::canPathW);
                            p.set(F::collideW);
                            p.set(F::cutW);
                            p.set(F::transparentW);
                            spr.tile_type = T::wall;
                            spr.cut_w = true;
                        }
                        "UnbreakableWindowN" => {
                            p.set(F::canPathN);
                            p.set(F::collideN);
                            p.set(F::cutN);
                            p.set(F::transparentN);
                            spr.tile_type = T::wall;
                            spr.cut_n = true;
                        }
                        "UnbreakableWindowNW" => {
                            p.set(F::cutN);
                            p.set(F::transparentN);
                            p.set(F::collideN);
                            p.set(F::collideW);
                            p.set(F::cutW);
                            spr.tile_type = T::wall;
                            spr.cut_w = true;
                            spr.cut_n = true;
                        }
                        "NoWallLighting" => p.set(F::NoWallLighting),
                        "ForceAmbient" => p.set(F::ForceAmbient),
                        _ => {}
                    }
                }

                if prop == "name" {
                    spr.parent_object_name = Some(val.to_string());
                }
            }

            let p = &mut spr.properties;
            if p.has_str(alias, pt::RED_LIGHT)
                || p.has_str(alias, pt::GREEN_LIGHT)
                || p.has_str(alias, pt::BLUE_LIGHT)
            {
                for k in [pt::RED_LIGHT, pt::GREEN_LIGHT, pt::BLUE_LIGHT] {
                    if !p.has_str(alias, k) {
                        p.set_str(alias, k, "0", true);
                    }
                }
            }
            p.create_key_set();
        }

        set_open_door_properties(spr_man, alias, name, &buffer);

        // Movable objects: sprites sharing GroupName + CustomName are the faces (or the grid cells) of one object.
        // The original keeps the groups in a HashMap; groups do not affect each other, so their order is irrelevant.
        let mut group_map: BTreeMap<String, Vec<SpriteRef>> = BTreeMap::new();
        for &r in &buffer {
            let sprx = spr_man.sprite_mut(r);
            if sprx.properties.has_str(alias, "StopCar") && sprx.tile_type != T::tree {
                sprx.tile_type = T::isMoveAbleObject;
            }
            let p = &sprx.properties;
            let custom = p.get(alias, pt::CUSTOM_NAME);
            let group = || {
                format!(
                    "{} {}",
                    p.get(alias, pt::GROUP_NAME).unwrap_or("null"),
                    custom.unwrap_or("null")
                )
            };
            if p.has_str(alias, "IsMoveAble") {
                if custom.is_some_and(|c| !c.is_empty()) && p.has_str(alias, pt::GROUP_NAME) {
                    group_map.entry(group()).or_default().push(r);
                }
            } else if p.has_str(alias, "SpriteGridPos")
                && !string_utils::is_null_or_whitespace(custom)
                && p.has_str(alias, pt::GROUP_NAME)
            {
                group_map.entry(group()).or_default().push(r);
            }
        }

        for members in group_map.values() {
            let grid_pos = |r: SpriteRef| spr_man.sprite(r).properties.get(alias, "SpriteGridPos");
            let test_is_sprite_grid = |r: SpriteRef| grid_pos(r).is_some_and(|v| v != "None");
            let is_sprite_grid = test_is_sprite_grid(members[0]);
            let mut is_valid = true;
            let mut face_map: [Vec<SpriteRef>; 4] = Default::default();
            for &current in members {
                if is_sprite_grid != test_is_sprite_grid(current) {
                    is_valid = false;
                    break;
                }
                match spr_man.sprite(current).properties.get(alias, pt::FACING) {
                    None => is_valid = false,
                    Some(face) => match face_names.iter().position(|f| *f == face) {
                        Some(i) => face_map[i].push(current),
                        None => is_valid = false,
                    },
                }
                if !is_valid {
                    break;
                }
            }
            if !is_valid {
                continue;
            }

            if is_sprite_grid {
                let mut spr_count = 0usize;
                let mut grids: [Option<IsoSpriteGrid>; 4] = Default::default();
                let mut parse = SpriteGridParseData::default();
                'faces: for (i, direction) in face_map.iter().enumerate() {
                    if direction.is_empty() {
                        continue;
                    }
                    if spr_count == 0 {
                        spr_count = direction.len();
                    }
                    if spr_count != direction.len() {
                        is_valid = false;
                        break;
                    }
                    parse.clear();
                    for &mem in direction {
                        let props = &spr_man.sprite(mem).properties;
                        let pos = props.get(alias, "SpriteGridPos").unwrap_or("");
                        let parts = java_split_comma(pos);
                        if parts.len() < 2 || parts.len() > 3 {
                            is_valid = false;
                            break 'faces;
                        }
                        let x = parse_int(parts[0])?;
                        let y = parse_int(parts[1])?;
                        let mut sprite_grid_z = 0;
                        if parts.len() == 3 {
                            sprite_grid_z = parse_int(parts[2])?;
                        }
                        if let Some(level) = props.get(alias, "SpriteGridLevel") {
                            sprite_grid_z = parse_int(level)?;
                            if sprite_grid_z < 0 {
                                is_valid = false;
                                break 'faces;
                            }
                        }
                        let level_data = parse
                            .get_or_create_level(sprite_grid_z)
                            .ok_or("java.lang.IndexOutOfBoundsException: SpriteGrid level")?;
                        if level_data.xy_to_sprite.contains_key(pos) {
                            is_valid = false;
                            break 'faces;
                        }
                        level_data.xy_to_sprite.insert(pos.to_string(), mem);
                        level_data.width = level_data.width.max(x + 1);
                        level_data.height = level_data.height.max(y + 1);
                        let (w, h) = (level_data.width, level_data.height);
                        parse.width = parse.width.max(w);
                        parse.height = parse.height.max(h);
                    }
                    if !parse.is_valid() {
                        is_valid = false;
                        break;
                    }
                    let mut grid =
                        IsoSpriteGrid::new(parse.width, parse.height, parse.levels.len() as i32);
                    for level_data in &parse.levels {
                        for (posx, sprite) in &level_data.xy_to_sprite {
                            let parts = java_split_comma(posx);
                            grid.set_sprite(
                                parse_int(parts[0])?,
                                parse_int(parts[1])?,
                                level_data.z,
                                *sprite,
                            );
                        }
                    }
                    if !grid.validate() {
                        is_valid = false;
                        break;
                    }
                    grids[i] = Some(grid);
                }

                if is_valid && spr_count != 0 {
                    let refs: Vec<Option<GridRef>> = grids
                        .into_iter()
                        .map(|g| g.map(|g| spr_man.add_grid(g)))
                        .collect();
                    for ix in 0..face_names.len() {
                        let Some(grid) = refs[ix] else { continue };
                        let cells: Vec<SpriteRef> = spr_man
                            .grid(grid)
                            .get_sprites()
                            .iter()
                            .flatten()
                            .copied()
                            .collect();
                        for member in cells {
                            spr_man.sprite_mut(member).sprite_grid = Some(grid);
                            for j in 0..face_names.len() {
                                let Some(other) = refs[j].filter(|_| j != ix) else {
                                    continue;
                                };
                                let anchor = spr_man
                                    .grid(other)
                                    .get_anchor_sprite()
                                    .ok_or("java.lang.NullPointerException: anchorSprite")?;
                                let d = spr_man.sprite(anchor).tile_sheet_index
                                    - spr_man.sprite(member).tile_sheet_index;
                                spr_man.sprite_mut(member).properties.set_str(
                                    alias,
                                    &format!("{}offset", face_names[j]),
                                    &d.to_string(),
                                    true,
                                );
                            }
                        }
                    }
                }
            } else if members.len() <= 4 {
                if face_map.iter().any(|f| f.len() > 1) {
                    continue;
                }
                let index_of =
                    |r: SpriteRef| buffer.iter().position(|b| *b == r).map_or(-1, |i| i as i32);
                for &current in members {
                    for (face_name, direction) in face_names.iter().zip(&face_map) {
                        if let Some(&first) = direction.first().filter(|f| **f != current) {
                            let d = index_of(first) - index_of(current);
                            spr_man.sprite_mut(current).properties.set_str(
                                alias,
                                &format!("{face_name}offset"),
                                &d.to_string(),
                                true,
                            );
                        }
                    }
                }
            }
        }
        buffer.clear();
    }
    Ok(())
}

/// java.lang.String.split(","): trailing empty strings are removed; an empty input gives one empty string.
fn java_split_comma(s: &str) -> Vec<&str> {
    let mut v: Vec<&str> = s.split(',').collect();
    while v.len() > 1 && v.last().is_some_and(|x| x.is_empty()) {
        v.pop();
    }
    v
}

/// zombie.iso.IsoWorld.setOpenDoorProperties: marks which door sprites are the open state. Double and garage doors
/// say so by index; for a plain door the sprite two tiles further in the sheet is its open twin.
fn set_open_door_properties(
    spr_man: &mut IsoSpriteManager,
    alias: &TilePropertyAliasMap,
    tilesheet_name: &str,
    sprites: &[SpriteRef],
) {
    for &r in sprites {
        let spr = spr_man.sprite(r);
        let ty = spr.tile_type;
        if (ty == T::doorN || ty == T::doorW) && !spr.properties.has(F::open) {
            if let Some(double_door) = spr.properties.get(alias, pt::DOUBLE_DOOR) {
                if pz_math::try_parse_int(Some(double_door), -1) >= 5 {
                    spr_man.sprite_mut(r).properties.set(F::open);
                }
            } else if let Some(garage_door) = spr.properties.get(alias, pt::GARAGE_DOOR) {
                if pz_math::try_parse_int(Some(garage_door), -1) >= 4 {
                    spr_man.sprite_mut(r).properties.set(F::open);
                }
            } else if let Some(&open) = spr_man
                .named_map
                .get(&format!("{tilesheet_name}_{}", spr.tile_sheet_index + 2))
            {
                let open_sprite = spr_man.sprite_mut(open);
                open_sprite.tile_type = ty;
                open_sprite
                    .properties
                    .set(if ty == T::doorN { F::doorN } else { F::doorW });
                open_sprite.properties.set(F::open);
            }
        }
    }
}

/// zombie.iso.IsoWorld: the loaded world's tables. So far what `init` builds before the first cell is loaded:
/// tile definitions and the meta grid. Mods, scripts, Lua events, world generation and basements are not part of it yet.
pub struct IsoWorld {
    pub sprites: IsoSpriteManager,
    pub alias: TilePropertyAliasMap,
    pub tile_definitions: TileDefinitions,
    pub meta_grid: crate::zombie::iso::iso_meta_grid::IsoMetaGrid,
    pub scripts: crate::zombie::scripting::script_manager::ScriptManager,
    /// Problems the original would log and continue past.
    pub log: Vec<String>,
}

impl IsoWorld {
    /// zombie.iso.IsoWorld.init, the ported part: the tile-definition sequence, then `IsoMetaGrid.Create` and the procedural basements for
    /// `game_map` (`Core.gameMap`). `has_texture` answers whether a texture of that name exists (always false on a
    /// dedicated server). `rand` is `Rand`.
    pub fn init<R: crate::java::JavaRandom>(
        root: &std::path::Path,
        game_map: &str,
        has_texture: &dyn Fn(&str) -> bool,
        alarm: &crate::zombie::iso::iso_meta_grid::AlarmSettings,
        // `WorldGenParams.getSeed()`; `None` skips the procedural basements (used by probes of the bare grid).
        world_seed: Option<i64>,
        rand: &mut crate::zombie::core::random::rand_abstract::RandAbstract<R>,
    ) -> Result<Self, String> {
        use crate::zombie::iso::iso_meta_grid::IsoMetaGrid;
        let media = root.join("media");
        let mut log = Vec::new();
        let mut files = Vec::new();
        for (f, n) in VANILLA_TILE_DEFINITION_FILES {
            let p = media.join(f);
            files.push((
                f,
                n,
                crate::vfs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?,
            ));
        }
        let mut tile_definitions = TileDefinitions::default();
        let mut alias = TilePropertyAliasMap::default();
        let mut sprites = IsoSpriteManager::new();
        for (f, n, data) in &files {
            if let Err(e) = tile_definitions.load_tile_definitions_property_strings(data, f, *n) {
                log.push(format!("{f}: {e}"));
            }
        }
        tile_definitions.set_custom_property_values()?;
        // ScriptManager.Load runs before the world loads; entity names become the values of "EntityScriptName".
        let scripts = crate::zombie::scripting::script_manager::ScriptManager::load(root)?;
        let entity_names: Vec<String> = scripts.entities.iter().map(|e| e.name.clone()).collect();
        tile_definitions.generate_tile_property_lookup_tables(&mut alias, &entity_names);
        for (f, n, data) in &files {
            if let Err(e) = load_tile_definitions(&mut sprites, &alias, data, f, *n) {
                log.push(format!("{f}: {e}"));
            }
        }
        register_fake_jumbo_tree(&mut sprites, &alias, 7);
        sprites.add_sprite("media/ui/missing-tile.png");
        // ScriptManager.PostTileDefinitions
        let entities: Vec<(
            String,
            Option<crate::zombie::scripting::sprite_config::SpriteConfigScript>,
        )> = scripts
            .entities
            .iter()
            .map(|e| (e.name.clone(), e.sprite_config.clone()))
            .collect();
        for name in crate::zombie::scripting::sprite_config::init_scripts_post_tile_def(
            &mut sprites,
            &alias,
            &entities,
        ) {
            log.push(format!("SpriteConfig of entity {name} not applied"));
        }
        tile_definitions.loaded_tile_definitions(&mut sprites, &alias, has_texture)?;

        let mut meta_grid = IsoMetaGrid::default();
        let dirs = IsoMetaGrid::lot_directories(&media, game_map);
        meta_grid.create_step1(&media, &dirs, (-250, -250, 250, 250))?;
        log.extend(meta_grid.load_all_cells(&mut |n| rand.next(n), &mut |_, _| Vec::new()));
        // Two closures over one generator: draws happen strictly one after another.
        let cell = std::cell::RefCell::new(rand);
        meta_grid.create_step2(alarm, &mut |n| cell.borrow_mut().next(n), &mut |a, b| {
            cell.borrow_mut().next_range(a, b)
        });
        // Basements.beforeLoadMetaGrid (new world: no map_basements.bin yet).
        if let Some(seed) = world_seed {
            let b = crate::zombie::basements::Basements::before_load_meta_grid(
                &media,
                &mut meta_grid,
                &dirs,
                seed,
            )?;
            log.extend(b.log);
        }
        Ok(IsoWorld {
            sprites,
            alias,
            tile_definitions,
            meta_grid,
            scripts,
            log,
        })
    }
}
