//! zombie.scripting.objects.Item: an `item` script. So far the parameters whose handling in `DoParam` is a plain typed
//! assignment (table generated from the original, `item_params_table.rs`); the other parameters are kept as text.
use super::item_params_table::{COMPLEX, FIELDS, PARAMS};
use super::script_parser::{self, BlockElement};
use crate::java;
use crate::zombie::util::string_utils;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    F32(f32),
    I32(i32),
    Bool(bool),
    Str(Option<&'static str>),
}

/// How a `DoParam` branch assigns the (trimmed) value to its field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `Float.parseFloat(val)`, `Integer.parseInt(val)`, `val`, `val.equalsIgnoreCase("true")` or
    /// `Boolean.parseBoolean(val)`, by the field's type.
    Set,
    /// `!val.equalsIgnoreCase("true")`
    Not,
    /// `StringUtils.discardNullOrWhitespace(val)`
    Discard,
    /// `Integer.parseInt(val)` into a float field.
    IntToFloat,
}

/// A field of the original class with its declared default.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDef {
    pub name: &'static str,
    pub default: FieldValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    F32(f32),
    I32(i32),
    Bool(bool),
    Str(Option<String>),
}

/// zombie.scripting.objects.Item
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub module: String,
    pub name: String,
    pub enabled: bool,
    pub debug_only: bool,
    /// Values of `FIELDS`, same order.
    pub values: Vec<Value>,
    /// Parameters with non-trivial handling in the original (`COMPLEX`), as (lower-case name, value text), in script order.
    pub unported: Vec<(String, String)>,
    /// Parameters the original has no branch for at all.
    pub unknown: Vec<String>,
}

/// java.lang.Float.parseFloat for decimal text (surrounding blanks and an f/d suffix allowed).
pub(crate) fn parse_float(s: &str) -> Option<f32> {
    let t = java::trim(s);
    let t = t.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(t);
    if t.is_empty()
        || !t
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
    {
        return None;
    }
    t.parse::<f32>().ok()
}

/// Attribute names (`zombie.entity.components.attributes.Attribute`), which `DoParam` routes to the attribute
/// component before anything else. Lower case.
const ATTRIBUTES: [&str; 17] = [
    "testquality",
    "testuses",
    "testcondition",
    "testbool",
    "teststring",
    "teststring2",
    "testitemtype",
    "testcategories",
    "testtags",
    "sharpness",
    "headcondition",
    "headconditionmax",
    "timesheadrepaired",
    "quality",
    "originx",
    "originy",
    "originz",
];

impl Item {
    pub fn new(module: &str, name: &str) -> Self {
        let values = FIELDS
            .iter()
            .map(|f| match &f.default {
                FieldValue::F32(v) => Value::F32(*v),
                FieldValue::I32(v) => Value::I32(*v),
                FieldValue::Bool(v) => Value::Bool(*v),
                FieldValue::Str(v) => Value::Str(v.map(str::to_string)),
            })
            .collect();
        Item {
            module: module.to_string(),
            name: name.to_string(),
            enabled: true,
            debug_only: false,
            values,
            unported: Vec::new(),
            unknown: Vec::new(),
        }
    }

    /// `module.name`, the original's `moduleDotType`.
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.module, self.name)
    }

    pub fn get(&self, field: &str) -> Option<&Value> {
        FIELDS
            .iter()
            .position(|f| f.name == field)
            .map(|i| &self.values[i])
    }

    pub fn set(&mut self, field: &str, value: Value) {
        if let Some(i) = FIELDS.iter().position(|f| f.name == field) {
            self.values[i] = value;
        }
    }

    /// zombie.scripting.objects.Item.Load: every `param = value,` of the script body goes through `do_param`.
    /// `Err` is the exception that makes the original drop the whole item (a value without `=`, an unparsable number).
    pub fn load(&mut self, body: &str) -> Result<(), String> {
        let parsed = script_parser::parse(body);
        let block = parsed
            .children
            .first()
            .ok_or("java.lang.IndexOutOfBoundsException: no item block")?;
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
        for element in &block.elements {
            let BlockElement::Value(v) = element else {
                continue;
            };
            if java::trim(&v.string).is_empty() {
                continue;
            }
            // java.lang.String.split("="): pieces between '=' signs, trailing empty pieces dropped.
            let mut p: Vec<&str> = v.string.split('=').collect();
            while p.last().is_some_and(|x| x.is_empty()) {
                p.pop();
            }
            if p.len() < 2 {
                return Err(format!(
                    "java.lang.ArrayIndexOutOfBoundsException: \"{}\"",
                    java::trim(&v.string)
                ));
            }
            self.do_param(java::trim(p[0]), java::trim(p[1]))?;
        }
        Ok(())
    }

    /// zombie.scripting.objects.Item.DoParam (String, String) for the tabulated parameters.
    pub fn do_param(&mut self, param: &str, val: &str) -> Result<(), String> {
        let lower = param.to_lowercase();
        if ATTRIBUTES.contains(&lower.as_str()) {
            self.unported.push((lower, val.to_string()));
            return Ok(());
        }
        let Some(&(_, i, op)) = PARAMS.iter().find(|p| p.0 == lower) else {
            if COMPLEX.contains(&lower.as_str()) {
                self.unported.push((lower, val.to_string()));
            } else {
                self.unknown.push(param.to_string());
            }
            return Ok(());
        };
        let nfe =
            || format!("java.lang.NumberFormatException: For input string: \"{val}\" ({param})");
        self.values[i] = match (&FIELDS[i].default, op) {
            (FieldValue::F32(_), Op::IntToFloat) => {
                Value::F32(java::parse_int(val).ok_or_else(nfe)? as f32)
            }
            (FieldValue::F32(_), _) => Value::F32(parse_float(val).ok_or_else(nfe)?),
            (FieldValue::I32(_), _) => Value::I32(java::parse_int(val).ok_or_else(nfe)?),
            (FieldValue::Bool(_), Op::Not) => Value::Bool(!java::equals_ignore_case(val, "true")),
            (FieldValue::Bool(_), _) => Value::Bool(java::equals_ignore_case(val, "true")),
            (FieldValue::Str(_), Op::Discard) if string_utils::is_null_or_whitespace(Some(val)) => {
                Value::Str(None)
            }
            (FieldValue::Str(_), _) => Value::Str(Some(val.to_string())),
        };
        Ok(())
    }
}
