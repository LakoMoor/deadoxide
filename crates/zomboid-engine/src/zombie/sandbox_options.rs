//! zombie.SandboxOptions: the world's rule settings. The option list is generated from the original class
//! (`sandbox_options_table.rs`); this module holds values, parsing and the `map_sand.bin` reader.
use std::collections::HashMap;

use super::sandbox_options_table::OPTIONS;
use crate::java;

/// Definition of one option, as its `new...Option(...)` call declares it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Def {
    pub name: &'static str,
    pub kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Boolean(bool),
    Double {
        min: f64,
        max: f64,
        default: f64,
    },
    Integer {
        min: i32,
        max: i32,
        default: i32,
    },
    /// Values 1..=num_values.
    Enum {
        num_values: i32,
        default: i32,
    },
    Str {
        default: &'static str,
        max_length: i32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Boolean(bool),
    Double(f64),
    Integer(i32),
    Str(String),
}

/// java.lang.Double.parseDouble for the forms the game writes (decimal and exponent notation, surrounding blanks,
/// an optional f/d suffix); `None` is NumberFormatException. Hexadecimal floats, "NaN" and "Infinity" are not handled.
fn parse_double(s: &str) -> Option<f64> {
    let t = java::trim(s);
    let t = t.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(t);
    if t.is_empty()
        || !t
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
    {
        return None;
    }
    t.parse::<f64>().ok()
}

/// java.lang.Double.toString: shortest digits that round-trip, plain notation for 1e-3 <= |v| < 1e7, otherwise
/// "d.dddE±n".
pub fn double_to_string(v: f64) -> String {
    if v == 0.0 {
        return if v.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    let a = v.abs();
    if (1.0e-3..1.0e7).contains(&a) {
        let s = format!("{v}");
        return if s.contains('.') { s } else { format!("{s}.0") };
    }
    // Rust's {:e} prints the shortest round-trip digits as d.ddde±n
    let s = format!("{v:e}");
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let m = if m.contains('.') {
        m.to_string()
    } else {
        format!("{m}.0")
    };
    format!("{m}E{e}")
}

/// zombie.SandboxOptions
#[derive(Clone, Debug, PartialEq)]
pub struct SandboxOptions {
    values: Vec<Value>,
    by_name: HashMap<&'static str, usize>,
    /// `IsoWorld.getPreset()` as stored at the end of `map_sand.bin`.
    pub preset: String,
    /// "ERROR ..." lines the original would log while loading.
    pub log: Vec<String>,
}

impl Default for SandboxOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxOptions {
    /// Every option at its default value.
    pub fn new() -> Self {
        let values = OPTIONS
            .iter()
            .map(|d| match d.kind {
                Kind::Boolean(b) => Value::Boolean(b),
                Kind::Double { default, .. } => Value::Double(default),
                Kind::Integer { default, .. } | Kind::Enum { default, .. } => {
                    Value::Integer(default)
                }
                Kind::Str { default, .. } => Value::Str(default.to_string()),
            })
            .collect();
        // A repeated name keeps its last registration, as `optionByName.put` does.
        let by_name = OPTIONS
            .iter()
            .enumerate()
            .map(|(i, d)| (d.name, i))
            .collect();
        SandboxOptions {
            values,
            by_name,
            preset: String::new(),
            log: Vec::new(),
        }
    }

    pub fn defs(&self) -> &'static [Def] {
        &OPTIONS
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.by_name.get(name).map(|i| &self.values[*i])
    }
    /// Value of an integer or enum option.
    pub fn get_int(&self, name: &str) -> Option<i32> {
        match self.get(name)? {
            Value::Integer(v) => Some(*v),
            _ => None,
        }
    }
    /// zombie.config.DoubleConfigOption.getValueAsString (and its siblings) of option `i`.
    pub fn value_as_string(&self, i: usize) -> String {
        match &self.values[i] {
            Value::Boolean(b) => b.to_string(),
            Value::Double(d) => double_to_string(*d),
            Value::Integer(v) => v.to_string(),
            Value::Str(s) => s.clone(),
        }
    }

    /// zombie.config.IntegerConfigOption.parse (and the Boolean/Double/String ones) for option `i`: an unparsable or out-of-range value is logged and the
    /// option keeps its value.
    pub fn parse(&mut self, i: usize, s: &str) {
        let d = &OPTIONS[i];
        match d.kind {
            Kind::Boolean(_) => {
                let is = |x: &str| s.eq_ignore_ascii_case(x);
                if is("true") || is("false") || is("1") || is("0") {
                    self.values[i] = Value::Boolean(is("true") || is("1"));
                } else {
                    self.log.push(format!(
                        "ERROR BooleanConfigOption.parse() \"{}\" string={s}\"",
                        d.name
                    ));
                }
            }
            Kind::Double { min, max, .. } => match parse_double(s) {
                Some(v) if v < min => self.log.push(format!(
                    "ERROR: DoubleConfigOption.setValue() \"{}\" {} is less than min={}",
                    d.name,
                    double_to_string(v),
                    double_to_string(min)
                )),
                Some(v) if v > max => self.log.push(format!(
                    "ERROR: DoubleConfigOption.setValue() \"{}\" {} is greater than max={}",
                    d.name,
                    double_to_string(v),
                    double_to_string(max)
                )),
                Some(v) => self.values[i] = Value::Double(v),
                None => self.log.push(format!(
                    "ERROR DoubleConfigOption.parse() \"{}\" string=\"{s}\"",
                    d.name
                )),
            },
            Kind::Integer { .. } | Kind::Enum { .. } => {
                let (min, max) = match d.kind {
                    Kind::Integer { min, max, .. } => (min, max),
                    Kind::Enum { num_values, .. } => (1, num_values),
                    _ => (0, 0),
                };
                match parse_double(s) {
                    // (int) of a double saturates in Java as `as i32` does in Rust.
                    Some(v) if (v as i32) < min => self.log.push(format!(
                        "ERROR: IntegerConfigOption.setValue() \"{}\" {} is less than min={min}",
                        d.name, v as i32
                    )),
                    Some(v) if (v as i32) > max => self.log.push(format!(
                        "ERROR: IntegerConfigOption.setValue() \"{}\" {} is greater than max={max}",
                        d.name, v as i32
                    )),
                    Some(v) => self.values[i] = Value::Integer(v as i32),
                    None => self.log.push(format!(
                        "ERROR IntegerConfigOption.parse() \"{}\" string=\"{s}\"",
                        d.name
                    )),
                }
            }
            // Length limits of string options are not applied yet.
            Kind::Str { .. } => self.values[i] = Value::Str(s.to_string()),
        }
    }

    /// zombie.SandboxOptions.loadGameFile: applies a preset table (`media/lua/shared/Sandbox/<name>.lua`, a Lua file
    /// that returns a table of options, with one level of group tables such as `ZombieLore = { ... }`). The file is
    /// read as data here, not executed. The original constructor applies "Apocalypse" and takes the result as its
    /// defaults.
    pub fn load_game_file(&mut self, text: &str) {
        let mut group = String::new();
        for line in text.lines() {
            let l = java::trim(line);
            let l = l.strip_suffix(',').unwrap_or(l);
            if l == "}" {
                group.clear();
                continue;
            }
            let Some((k, v)) = l.split_once('=') else {
                continue;
            };
            let (k, v) = (java::trim(k), java::trim(v));
            if v == "{" {
                group = format!("{k}.");
                continue;
            }
            let name = format!("{group}{k}");
            if let Some(i) = self.by_name.get(name.as_str()).copied() {
                // A Lua string value arrives without its quotes.
                let v = v
                    .strip_prefix('"')
                    .and_then(|x| x.strip_suffix('"'))
                    .unwrap_or(v);
                self.parse(i, v);
            }
        }
    }

    /// zombie.SandboxOptions.load (ByteBuffer): the contents of a save's `map_sand.bin` ("SAND", world version, options
    /// version, count, then name/value strings, then the preset name). Strings are `GameWindow.ReadString`: a
    /// big-endian short length and UTF-8 bytes. The value upgrades for option versions below 6 are not ported; such a
    /// file is refused.
    pub fn load(&mut self, input: &[u8]) -> Result<(), String> {
        let mut p = 4usize;
        let eof = "java.nio.BufferUnderflowException";
        let int = |p: &mut usize| -> Result<i32, String> {
            let b = input.get(*p..*p + 4).ok_or(eof)?;
            *p += 4;
            Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        };
        let _world_version = int(&mut p)?;
        let version = int(&mut p)?;
        if version < 6 {
            return Err(format!(
                "sandbox options version {version}: upgrades from versions below 6 are not ported"
            ));
        }
        let count = int(&mut p)?;
        let string = |p: &mut usize| -> Result<String, String> {
            let b = input.get(*p..*p + 2).ok_or(eof)?;
            let n = i16::from_be_bytes([b[0], b[1]]).max(0) as usize;
            *p += 2;
            let s = input.get(*p..*p + n).ok_or(eof)?;
            *p += n;
            Ok(String::from_utf8_lossy(s).into_owned())
        };
        for _ in 0..count {
            let name = string(&mut p)?;
            let value = string(&mut p)?;
            match self.by_name.get(name.as_str()).copied() {
                Some(i) => self.parse(i, &value),
                None => self
                    .log
                    .push(format!("ERROR unknown SandboxOption \"{name}\"")),
            }
        }
        self.preset = string(&mut p)?;
        Ok(())
    }
}
