//! zombie.scripting.objects.ModelScript: the `model` script fields that loading sets (attachments, bone weights and
//! the renderer-side state are not ported yet).
use super::item::parse_float;
use super::script_parser;
use crate::java;

/// zombie.scripting.objects.ModelScript
#[derive(Clone, Debug, PartialEq)]
pub struct ModelScript {
    pub module: String,
    /// BaseScriptObject.getScriptObjectName: the bucket key, set once.
    pub script_object_name: String,
    /// ModelScript.name: set by InitLoadPP, cleared by `reset`, so a model defined twice has the full type `Module.null`.
    pub name: Option<String>,
    pub enabled: bool,
    pub debug_only: bool,
    pub mesh_name: Option<String>,
    pub texture_name: Option<String>,
    pub shader_name: Option<String>,
    pub is_static: bool,
    pub scale: f32,
}

impl ModelScript {
    pub fn new(module: &str, name: &str) -> Self {
        ModelScript {
            module: module.to_string(),
            script_object_name: name.to_string(),
            name: Some(name.to_string()),
            enabled: true,
            debug_only: false,
            mesh_name: None,
            texture_name: None,
            shader_name: None,
            is_static: true,
            scale: 1.0,
        }
    }

    /// zombie.scripting.objects.ModelScript.getFullType
    pub fn get_full_type(&self) -> String {
        format!("{}.{}", self.module, self.name.as_deref().unwrap_or("null"))
    }

    /// zombie.scripting.objects.ModelScript.reset (`name` too; `enabled`/`debugOnly` are not reset).
    pub fn reset(&mut self) {
        self.name = None;
        self.mesh_name = None;
        self.texture_name = None;
        self.shader_name = None;
        self.is_static = true;
        self.scale = 1.0;
    }

    /// zombie.scripting.objects.ModelScript.Load: `Err` where the original throws (`scale` or an attachment's
    /// `offset`/`rotate` that `Float.parseFloat` rejects).
    pub fn load(&mut self, body: &str) -> Result<(), String> {
        let parsed = script_parser::parse(body);
        let block = parsed
            .children
            .first()
            .ok_or("java.lang.IndexOutOfBoundsException: no model block")?;
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
        // zombie.scripting.objects.ModelScript.LoadAttachment -> LoadVector3f: three floats split on " ".
        for child in &block.children {
            if child.r#type != "attachment" {
                continue;
            }
            for value in &child.values {
                let (k, v) = (java::trim(value.get_key()), java::trim(value.get_value()));
                if k == "offset" || k == "rotate" {
                    let ss: Vec<&str> = v.split(' ').collect();
                    if ss.len() < 3 {
                        return Err(format!(
                            "java.lang.ArrayIndexOutOfBoundsException: {k} = {v}"
                        ));
                    }
                    for s in &ss[..3] {
                        parse_float(s)
                            .ok_or_else(|| format!("java.lang.NumberFormatException: {k} = {v}"))?;
                    }
                }
            }
        }
        let mut undo_core_scale = false;
        for value in &block.values {
            let (k, v) = (java::trim(value.get_key()), java::trim(value.get_value()));
            if java::equals_ignore_case(k, "mesh") {
                self.mesh_name = Some(v.to_string());
            } else if java::equals_ignore_case(k, "scale") {
                self.scale = parse_float(v)
                    .ok_or_else(|| format!("java.lang.NumberFormatException: scale = {v}"))?;
            } else if java::equals_ignore_case(k, "shader") {
                self.shader_name = Some(v.to_string());
            } else if java::equals_ignore_case(k, "static") {
                self.is_static = java::equals_ignore_case(v, "true");
            } else if java::equals_ignore_case(k, "texture") {
                self.texture_name = Some(v.to_string());
            } else if java::equals_ignore_case(k, "undoCoreScale") {
                undo_core_scale = java::equals_ignore_case(v, "true");
            }
        }
        if undo_core_scale {
            self.scale *= 0.6666667;
        }
        Ok(())
    }
}
