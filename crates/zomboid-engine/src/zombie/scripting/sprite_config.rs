//! zombie.scripting.entity.components.spriteconfig.SpriteConfigScript and the sprite marking of
//! zombie.entity.components.spriteconfig.SpriteConfigManager.
use std::collections::HashSet;

use super::script_parser::Block;
use crate::java;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use crate::zombie::iso::sprite::iso_sprite_manager::{IsoSpriteManager, SpriteRef};
use crate::zombie::iso::sprite_details::iso_flag_type::IsoFlagType;

/// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript.TileScript
#[derive(Clone, Debug, PartialEq)]
pub struct TileScript {
    pub tile_name: Option<String>,
    pub is_empty_space: bool,
    pub blocks_square: bool,
}
/// One `row = ...` line: tiles along x.
pub type XRow = Vec<TileScript>;
/// One `layer { }`: rows along y.
pub type ZLayer = Vec<XRow>;

/// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript.FaceScript
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FaceScript {
    pub face_name: String,
    pub face_id: usize,
    pub total_width: usize,
    pub total_height: usize,
    pub layers: Vec<ZLayer>,
}

/// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript: which tiles make up a buildable/world entity,
/// per facing. Only the tile layout and validity are ported; health, lights and callbacks are not.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SpriteConfigScript {
    pub faces: [Option<FaceScript>; 6],
    pub is_single_face: bool,
    pub is_multi_tile: bool,
    pub is_valid: bool,
    pub all_tile_names: Vec<String>,
}

/// zombie.entity.components.spriteconfig.SpriteConfigManager.GetFaceIdForString
pub fn get_face_id_for_string(face: &str) -> Option<usize> {
    let eq = |s: &str| java::equals_ignore_case(face, s);
    if eq("n") || eq("single") {
        Some(0)
    } else if eq("n_open") {
        Some(4)
    } else if eq("e") {
        Some(3)
    } else if eq("s") {
        Some(2)
    } else if eq("w") {
        Some(1)
    } else if eq("w_open") {
        Some(5)
    } else {
        None
    }
}

impl SpriteConfigScript {
    /// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript.load: reads the `face` blocks of a
    /// `component SpriteConfig` block. Loading a second body adds to the first, as in the original.
    pub fn load(&mut self, block: &Block) {
        for child in &block.children {
            if java::equals_ignore_case(&child.r#type, "face") && !self.is_single_face {
                // A face block without an id throws in the original (child.id is null).
                let id = child.id.as_deref().unwrap_or("");
                let Some(face_id) = get_face_id_for_string(id) else {
                    continue;
                };
                let mut face = FaceScript {
                    face_name: id.to_string(),
                    face_id,
                    ..FaceScript::default()
                };
                Self::load_face(&mut face, child);
                self.faces[face_id] = Some(face);
                if java::equals_ignore_case(id, "single") {
                    self.is_single_face = true;
                }
            }
        }
        self.check_scripts();
    }

    /// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript.loadFace: rows directly inside the face, or
    /// one layer per `layer` block.
    fn load_face(face: &mut FaceScript, block: &Block) {
        let layer = Self::load_layer(block);
        if !layer.is_empty() {
            face.layers.push(layer);
        } else {
            for child in &block.children {
                if java::equals_ignore_case(&child.r#type, "layer") {
                    face.layers.push(Self::load_layer(child));
                }
            }
        }
    }

    /// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript.loadLayer: `row = tile tile ...`, where
    /// `true`/`false` stand for an empty cell that does or does not block its square.
    fn load_layer(block: &Block) -> ZLayer {
        let mut layer = ZLayer::new();
        for value in &block.values {
            if !java::equals_ignore_case(java::trim(value.get_key()), "row") {
                continue;
            }
            let val = java::trim(value.get_value());
            // java.lang.String.split("\\s+") of a trimmed string; an empty string gives one empty tile name.
            let tiles: Vec<&str> = if val.is_empty() {
                vec![""]
            } else {
                val.split(char::is_whitespace)
                    .filter(|t| !t.is_empty())
                    .collect()
            };
            let row = tiles
                .into_iter()
                .map(|t| {
                    if java::equals_ignore_case(t, "true") || java::equals_ignore_case(t, "false") {
                        TileScript {
                            tile_name: None,
                            is_empty_space: true,
                            blocks_square: java::equals_ignore_case(t, "true"),
                        }
                    } else {
                        TileScript {
                            tile_name: Some(t.to_string()),
                            is_empty_space: false,
                            blocks_square: true,
                        }
                    }
                })
                .collect();
            layer.push(row);
        }
        layer
    }

    /// zombie.scripting.entity.components.spriteconfig.SpriteConfigScript.checkScripts: sizes, the list of all tile
    /// names, and validity (at least one face, no tile named twice).
    fn check_scripts(&mut self) {
        if self.is_single_face {
            for f in &mut self.faces {
                if f.as_ref().is_some_and(|f| f.face_id != 0) {
                    *f = None;
                }
            }
        }
        let (mut has_face, mut has_duplicates, mut face_count) = (false, false, 0);
        for face in self.faces.iter_mut().flatten() {
            has_face = true;
            face_count += 1;
            for layer in &face.layers {
                face.total_height = face.total_height.max(layer.len());
                for row in layer {
                    face.total_width = face.total_width.max(row.len());
                    for tile in row {
                        if let Some(name) = &tile.tile_name {
                            if self.all_tile_names.contains(name) {
                                has_duplicates = true;
                            } else {
                                self.all_tile_names.push(name.clone());
                            }
                        }
                    }
                }
            }
            if face_count > 1 || face.total_height > 1 || face.total_width > 1 {
                self.is_multi_tile = true;
            }
        }
        if has_face && !has_duplicates {
            self.is_valid = true;
        }
    }
}

/// zombie.entity.components.spriteconfig.SpriteConfigManager.InitScriptsPostTileDef: marks every tile of every valid
/// SpriteConfig with the `EntityScript` flag and its entity's name. `entities` are (entity name, its SpriteConfig) in
/// `ScriptManager.getAllGameEntities()` order. An entity that names a sprite already claimed is skipped as a whole,
/// but the sprites it claimed before the clash stay claimed. Returns the names of skipped entities.
pub fn init_scripts_post_tile_def(
    sm: &mut IsoSpriteManager,
    alias: &TilePropertyAliasMap,
    entities: &[(String, Option<SpriteConfigScript>)],
) -> Vec<String> {
    let mut registered: HashSet<SpriteRef> = HashSet::new();
    let mut failed = Vec::new();
    let mut object_infos: HashSet<&str> = HashSet::new();
    'entity: for (name, config) in entities {
        let Some(config) = config else { continue };
        // parseSpriteConfigScript: invalid scripts and repeated entity names are errors.
        if !config.is_valid || !object_infos.insert(name.as_str()) {
            failed.push(name.clone());
            continue;
        }
        let mut sprites = Vec::new();
        for s in &config.all_tile_names {
            let spr = sm.get_sprite(s);
            if !registered.insert(spr) {
                failed.push(name.clone());
                continue 'entity;
            }
            sprites.push(spr);
        }
        for spr in sprites {
            let p = &mut sm.sprite_mut(spr).properties;
            p.set_str(alias, "EntityScriptName", name, true);
            p.set(IsoFlagType::EntityScript);
            p.create_key_set();
        }
    }
    failed
}
