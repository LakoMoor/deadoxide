//! zombie.scripting.ScriptManager: so far reading the script files into modules and loading `entity` scripts'
//! SpriteConfig components. Items, recipes, vehicles and the other script types are collected as raw text only.
use std::collections::HashMap;
use std::path::Path;

use super::script_parser::{self, Block};
use super::sprite_config::SpriteConfigScript;
use crate::java;

/// zombie.scripting.entity.GameEntityScript, the ported part.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GameEntityScript {
    pub name: String,
    pub enabled: bool,
    pub debug_only: bool,
    pub sprite_config: Option<SpriteConfigScript>,
}

/// zombie.scripting.objects.ScriptModule with its ScriptBucket load data: script bodies by (tag, name), in file order.
#[derive(Clone, Debug, Default)]
pub struct ScriptModule {
    pub name: String,
    pub imports: Vec<String>,
    /// tag -> names in first-seen order
    pub names: HashMap<String, Vec<String>>,
    /// (tag, name) -> bodies in load order (later files add bodies to an existing script)
    pub bodies: HashMap<(String, String), Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct ScriptManager {
    /// zombie.scripting.ScriptManager.moduleList
    pub module_list: Vec<ScriptModule>,
    /// Files in load order, relative lower-case paths.
    pub load_file_names: Vec<String>,
    /// zombie.scripting.ScriptManager.getAllGameEntities
    pub entities: Vec<GameEntityScript>,
}

/// zombie.scripting.ScriptManager.searchFolders: every `.txt` below `fo`, as lower-case paths relative to the game
/// root, skipping folders whose path contains "tempNotWorking".
fn search_folders(root: &Path, fo: &Path, load_list: &mut Vec<(String, std::path::PathBuf)>) {
    if fo.is_dir() {
        if fo.to_string_lossy().contains("tempNotWorking") {
            return;
        }
        let Ok(rd) = std::fs::read_dir(fo) else {
            return;
        };
        for e in rd.flatten() {
            search_folders(root, &e.path(), load_list);
        }
    } else if fo.to_string_lossy().to_lowercase().ends_with(".txt") {
        if let Ok(rel) = fo.strip_prefix(root) {
            let rel = rel.to_string_lossy().replace('\\', "/").to_lowercase();
            load_list.push((rel, fo.to_path_buf()));
        }
    }
}

/// zombie.scripting.objects.ScriptModule.GetTokenType: the first word of a token's header.
fn get_token_type(token: &str) -> Option<&str> {
    let p = token.find('{')?;
    let header = java::trim(&token[..p]);
    let end = header.find([' ', '\t']).unwrap_or(header.len());
    Some(&header[..end])
}

/// The script tags `ScriptModule.CreateFromTokenPP` hands to a bucket; anything else is "unknown script object".
/// `template` tokens go to the template buckets and are kept under the tag "template".
fn token_name(tag: &str, token: &str) -> String {
    // token.split("[{}]")[0], minus the first occurrence of the tag text anywhere in it (String.replace replaces all).
    let head = token.split(['{', '}']).next().unwrap_or("");
    java::trim(&head.replace(tag, "")).to_string()
}

impl ScriptManager {
    /// zombie.scripting.ScriptManager.Load, the ported part: the vanilla script files in the original's order
    /// (`template_*` files first, then by path), parsed into modules, then entity scripts loaded.
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut game_files = Vec::new();
        search_folders(root, &root.join("media").join("scripts"), &mut game_files);
        let file_name = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
        game_files.sort_by(|a, b| {
            let (t1, t2) = (
                file_name(&a.0).starts_with("template_"),
                file_name(&b.0).starts_with("template_"),
            );
            // java.lang.String.compareTo: UTF-16 code unit order.
            t2.cmp(&t1)
                .then_with(|| a.0.encode_utf16().cmp(b.0.encode_utf16()))
        });
        game_files.dedup_by(|a, b| a.0 == b.0);
        let mut m = ScriptManager::default();
        for (rel, path) in game_files {
            // LoadFile: lines joined with '\n' (BufferedReader.readLine drops line terminators).
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let text = String::from_utf8_lossy(&bytes);
            let mut buf = String::with_capacity(text.len() + 1);
            for line in text.split('\n') {
                buf.push_str(line.strip_suffix('\r').unwrap_or(line));
                buf.push('\n');
            }
            if text.ends_with('\n') {
                buf.pop();
            }
            m.load_file_names.push(rel);
            m.parse_script(&script_parser::strip_comments(&buf));
        }
        m.load_entities();
        Ok(m)
    }

    /// zombie.scripting.ScriptManager.ParseScript / CreateFromToken: top-level `module Name { ... }` tokens.
    fn parse_script(&mut self, total_file: &str) {
        for token in script_parser::parse_tokens(total_file) {
            let token = java::trim(&token);
            if !token.starts_with("module") {
                continue;
            }
            let (Some(first_open), Some(last_close)) = (token.find('{'), token.rfind('}')) else {
                continue;
            };
            let name = java::trim(
                &token
                    .split(['{', '}'])
                    .next()
                    .unwrap_or("")
                    .replace("module", ""),
            )
            .to_string();
            if first_open + 1 > last_close {
                continue;
            }
            let actual = &token[first_open + 1..last_close];
            let idx = match self.module_list.iter().position(|w| w.name == name) {
                Some(i) => i,
                None => {
                    self.module_list.push(ScriptModule {
                        name: name.clone(),
                        ..ScriptModule::default()
                    });
                    self.module_list.len() - 1
                }
            };
            // ScriptModule.Load -> ParseScriptPP -> CreateFromTokenPP
            for t in script_parser::parse_tokens(java::trim(actual)) {
                let t = java::trim(&t).to_string();
                let Some(tag) = get_token_type(&t).map(str::to_string) else {
                    continue;
                };
                let module = &mut self.module_list[idx];
                if tag == "imports" {
                    if let Some(list) = t.split(['{', '}']).nth(1) {
                        for c in list.split(',') {
                            let c = java::trim(c);
                            if !c.is_empty() && c != module.name {
                                module.imports.push(c.to_string());
                            }
                        }
                    }
                    continue;
                }
                let name = token_name(&tag, &t);
                let names = module.names.entry(tag.clone()).or_default();
                if !names.contains(&name) {
                    names.push(name.clone());
                }
                module.bodies.entry((tag, name)).or_default().push(t);
            }
        }
    }

    /// ScriptBucket.LoadScripts for the entity buckets, then ScriptBucketCollection.getAllScripts: per module, entities
    /// in the iteration order of the original's `HashMap` of names.
    fn load_entities(&mut self) {
        let mut all = Vec::new();
        for module in &self.module_list {
            let Some(names) = module.names.get("entity") else {
                continue;
            };
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            let mut loaded: Vec<GameEntityScript> = Vec::new();
            for k in java::hash_map_order(&refs) {
                let name = &names[k];
                let mut script = GameEntityScript {
                    name: name.clone(),
                    enabled: true,
                    debug_only: false,
                    sprite_config: None,
                };
                let mut ok = true;
                for (index, body) in module.bodies[&("entity".to_string(), name.clone())]
                    .iter()
                    .enumerate()
                {
                    // ScriptType.Flags.ResetExisting: a later body (an override) starts from a reset script.
                    if index >= 1 {
                        script.sprite_config = None;
                    }
                    if !script.load(body) {
                        ok = false;
                        break;
                    }
                }
                // A script that failed to load is still registered by the original (no RemoveLoadError flag here).
                let _ = ok;
                if script.enabled && !script.debug_only {
                    loaded.push(script);
                }
            }
            // scriptMap.values(): the HashMap order of the names that were kept.
            let kept: Vec<&str> = loaded.iter().map(|s| s.name.as_str()).collect();
            let order = java::hash_map_order(&kept);
            all.extend(order.into_iter().map(|i| loaded[i].clone()));
        }
        self.entities = all;
    }
}

impl GameEntityScript {
    /// zombie.scripting.entity.GameEntityScript.Load (String, String): `false` where the original throws.
    /// Entity templates (`entitytemplate = ...`) are not applied; they cannot carry a SpriteConfig in the original.
    fn load(&mut self, body: &str) -> bool {
        let parsed = script_parser::parse(body);
        let Some(block) = parsed.children.first() else {
            return false;
        };
        // BaseScriptObject.LoadCommonBlock
        for value in &block.values {
            let (key, val) = (java::trim(value.get_key()), java::trim(value.get_value()));
            if !key.is_empty() && !val.is_empty() {
                if java::equals_ignore_case(key, "enabled") {
                    self.enabled = java::equals_ignore_case(val, "true");
                } else if java::equals_ignore_case(key, "debugOnly") {
                    self.debug_only = java::equals_ignore_case(val, "true");
                }
            }
        }
        for child in &block.children {
            self.load_component_block(child);
        }
        true
    }

    /// zombie.scripting.entity.GameEntityScript.LoadComponentBlock, for the one component type ported so far.
    fn load_component_block(&mut self, block: &Block) {
        if java::equals_ignore_case(&block.r#type, "component")
            && block.id.as_deref() == Some("SpriteConfig")
        {
            self.sprite_config
                .get_or_insert_with(SpriteConfigScript::default)
                .load(block);
        }
    }
}
