//! Translations: media/lua/shared/Translate/<LANG>/*.json (flat key -> string). `getText` falls back to the key.
use mlua::{Lua, MultiValue, Result, Value};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
enum FormatError {
    Missing,
    Invalid,
}

// The subset needed by the measured Translator samples. Unsupported Java
// Formatter conversions return Invalid instead of guessing a result.
struct FormatArg {
    text: String,
    boolean: bool,
}

fn java_format(template: &str, args: &[FormatArg]) -> std::result::Result<String, FormatError> {
    let bytes = template.as_bytes();
    let mut out = String::new();
    let mut pos = 0;
    let mut next_arg = 0;
    let mut previous_arg = None;
    while pos < bytes.len() {
        if bytes[pos] != b'%' {
            let start = pos;
            while pos < bytes.len() && bytes[pos] != b'%' {
                pos += 1;
            }
            out.push_str(&template[start..pos]);
            continue;
        }
        pos += 1;
        if bytes.get(pos) == Some(&b'%') {
            out.push('%');
            pos += 1;
            continue;
        }
        if bytes.get(pos) == Some(&b'n') {
            out.push_str(if cfg!(windows) { "\r\n" } else { "\n" });
            pos += 1;
            continue;
        }
        let digits = pos;
        while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
        }
        let reuse = bytes.get(pos) == Some(&b'<');
        if reuse {
            if pos != digits {
                return Err(FormatError::Invalid);
            }
            pos += 1;
        }
        let index = if reuse {
            previous_arg.ok_or(FormatError::Missing)?
        } else if pos > digits && bytes.get(pos) == Some(&b'$') {
            let n = template[digits..pos]
                .parse::<usize>()
                .map_err(|_| FormatError::Invalid)?;
            pos += 1;
            n.checked_sub(1).ok_or(FormatError::Invalid)?
        } else {
            if pos != digits {
                return Err(FormatError::Invalid);
            }
            let n = next_arg;
            next_arg += 1;
            n
        };
        if !matches!(bytes.get(pos), Some(b's' | b'S' | b'b' | b'B')) {
            return Err(FormatError::Invalid);
        }
        let arg = args.get(index).ok_or(FormatError::Missing)?;
        let formatted = match bytes.get(pos) {
            Some(b's') => arg.text.clone(),
            Some(b'S') => arg.text.to_uppercase(),
            Some(b'b') => arg.boolean.to_string(),
            Some(b'B') => arg.boolean.to_string().to_uppercase(),
            _ => return Err(FormatError::Invalid),
        };
        previous_arg = Some(index);
        pos += 1;
        out.push_str(&formatted);
    }
    Ok(out)
}

/// zombie.core.Translator.reportMissingArgumentsFromPastAbuse: format a translation.
fn substitute(template: &str, values: &[Value]) -> String {
    let args: Vec<FormatArg> = values
        .iter()
        .map(|v| FormatArg {
            text: match v {
                Value::Nil => String::new(),
                Value::String(s) => s.to_string_lossy(),
                Value::Integer(n) => n.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                _ => String::new(), // Other Java object string conversions are not yet ported.
            },
            // Translator normalizes null to an empty string before formatting.
            boolean: !matches!(v, Value::Boolean(false)),
        })
        .collect();
    match java_format(template, &args) {
        Ok(s) => return s,
        Err(FormatError::Missing) => return template.to_owned(),
        Err(FormatError::Invalid) => {}
    }
    // Translator's legacy retry escapes certain percent signs, then rewrites
    // %%<digits> to a positional Java string conversion.
    let chars: Vec<char> = template.chars().collect();
    let mut escaped = String::new();
    for (i, c) in chars.iter().enumerate() {
        if *c == '%' && !matches!(chars.get(i + 1), Some('d' | 's' | '.')) {
            escaped.push('%');
        }
        escaped.push(*c);
    }
    let mut rewritten = String::new();
    let mut tail = escaped.as_str();
    while let Some(index) = tail.find("%%") {
        rewritten.push_str(&tail[..index]);
        tail = &tail[index + 2..];
        let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 {
            rewritten.push('%');
            rewritten.push_str(&tail[..digits]);
            rewritten.push_str("$s");
            tail = &tail[digits..];
        } else {
            rewritten.push_str("%%");
        }
    }
    rewritten.push_str(tail);
    java_format(&rewritten, &args).unwrap_or_else(|_| template.to_owned())
}

pub fn install(lua: &Lua, root: &Path, lang: &str) -> Result<usize> {
    let translations = root.join("media/lua/shared/Translate");
    if lang.is_empty() || !lang.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
        return Err(mlua::Error::external("invalid translation language code"));
    }
    if !translations.join(lang).is_dir() {
        return Err(mlua::Error::external(format!(
            "translation language not found: {lang}"
        )));
    }
    let mut map: HashMap<String, String> = HashMap::new();
    let mut item_names: HashMap<String, String> = HashMap::new();
    // The original's measured Russian stack is EN then RU; later entries override.
    let languages = if lang == "EN" {
        vec!["EN"]
    } else {
        vec!["EN", lang]
    };
    for language in languages {
        let dir = translations.join(language);
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let mut files: Vec<_> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
                .filter(|p| p.file_name().and_then(|x| x.to_str()) != Some("language.json"))
                .collect();
            files.sort();
            for f in files {
                let filename = f.file_name().and_then(|name| name.to_str()).unwrap_or("");
                // Map descriptions and evolved-recipe names have their own
                // lookup APIs and are not part of getText's namespace.
                if filename.ends_with(", KY.json") || filename == "EvolvedRecipeName.json" {
                    continue;
                }
                let destination = if filename == "ItemName.json" {
                    &mut item_names
                } else {
                    &mut map
                };
                let Ok(s) = std::fs::read_to_string(&f) else {
                    continue;
                };
                if let Ok(serde_json::Value::Object(o)) =
                    serde_json::from_str::<serde_json::Value>(&s)
                {
                    for (k, v) in o {
                        if let serde_json::Value::String(t) = v {
                            destination.insert(k, t);
                        }
                    }
                }
            }
        }
    }
    lua.globals().set(
        "getItemNameFromFullType",
        lua.create_function(move |_, key: String| {
            if !key.contains('.') {
                return Err(mlua::Error::external(
                    "fullType must contain '.' i.e. module.type",
                ));
            }
            Ok(item_names.get(&key).cloned().unwrap_or(key))
        })?,
    )?;
    let n = map.len();
    let map = std::rc::Rc::new(map);
    let m1 = map.clone();
    lua.globals().set(
        "getText",
        lua.create_function(move |_, mv: MultiValue| {
            let v: Vec<Value> = mv.into_iter().collect();
            let Some(Value::String(k)) = v.first() else {
                return Ok(String::new());
            };
            let k = k.to_string_lossy().to_string();
            Ok(match m1.get(&k) {
                Some(t) => substitute(t, &v[1..]),
                None => substitute(&k, &v[1..]),
            })
        })?,
    )?;
    let m2 = map;
    lua.globals().set(
        "getTextOrNull",
        lua.create_function(move |_, mv: MultiValue| {
            let v: Vec<Value> = mv.into_iter().collect();
            let Some(Value::String(k)) = v.first() else {
                return Ok(None);
            };
            Ok(m2
                .get(&*k.to_string_lossy())
                .map(|t| substitute(t, &v[1..])))
        })?,
    )?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::{install, substitute};
    use mlua::{Lua, Value};

    #[test]
    fn item_and_text_namespaces_match_original() -> mlua::Result<()> {
        let lua = Lua::new();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        install(&lua, &root, "RU")?;
        let get: mlua::Function = lua.globals().get("getText")?;
        let nullable: mlua::Function = lua.globals().get("getTextOrNull")?;
        let item: mlua::Function = lua.globals().get("getItemNameFromFullType")?;
        let oracle: Vec<_> = include_str!("../../../tests/golden/translator_categories_ru.txt")
            .lines()
            .collect();
        for (key, text_line, item_line) in [
            ("Base.CannedCarrotsOpen", 2, 4),
            ("Base.Axe", 6, 8),
            ("Base.MissingProbe", 10, 12),
        ] {
            let expected_text = oracle[text_line].split_once('=').expect("text oracle").1;
            let expected_item = oracle[item_line].split_once('=').expect("item oracle").1;
            assert_eq!(get.call::<String>(key)?, expected_text);
            assert_eq!(nullable.call::<Option<String>>(key)?, None);
            assert_eq!(item.call::<String>(key)?, expected_item);
        }
        assert_eq!(get.call::<String>("title")?, "title");
        assert_eq!(nullable.call::<Option<String>>("title")?, None);
        assert!(item.call::<String>("Axe").is_err());
        Ok(())
    }

    #[test]
    fn russian_real_content_lookup_matches_original() -> mlua::Result<()> {
        let lua = Lua::new();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        install(&lua, &root, "RU")?;
        let get: mlua::Function = lua.globals().get("getText")?;
        let nullable: mlua::Function = lua.globals().get("getTextOrNull")?;
        let oracle: Vec<_> = include_str!("../../../tests/golden/translator_load_ru.txt")
            .lines()
            .collect();
        for (key, line) in [
            ("Challenge_YouHaveOneDay_name", 4),
            ("UI_servers_last_wipe", 6),
        ] {
            let expected = oracle[line].split_once('=').expect("oracle value").1;
            assert_eq!(get.call::<String>(key)?, expected);
            assert_eq!(
                nullable.call::<Option<String>>(key)?.as_deref(),
                Some(expected)
            );
        }
        assert_eq!(get.call::<String>("UI_ProbeAbsent")?, "UI_ProbeAbsent");
        assert_eq!(nullable.call::<Option<String>>("UI_ProbeAbsent")?, None);
        Ok(())
    }

    #[test]
    fn translator_value_formats_match_original_probe() -> mlua::Result<()> {
        let lua = Lua::new();
        let string = |s: &str| lua.create_string(s).map(Value::String);
        let cases = [
            (0, "%s", vec![Value::Nil]),
            (1, "%s", vec![Value::Number(1.0)]),
            (2, "%s", vec![Value::Number(1.5)]),
            (3, "%s", vec![Value::Boolean(true)]),
            (4, "%1", vec![Value::Nil]),
            (8, "%b", vec![Value::Boolean(false)]),
            (9, "%S", vec![string("hello")?]),
            (12, "%s %<s", vec![string("repeat")?]),
            (13, "%2$s %s %s", vec![string("a")?, string("b")?]),
            (14, "%1 %2", vec![Value::Nil, string("x")?]),
            (15, "%b", vec![Value::Nil]),
            (16, "%b", vec![string("false")?]),
            (17, "%B", vec![Value::Boolean(true)]),
        ];
        let oracle: Vec<_> = include_str!("../../../tests/golden/translator_values.txt")
            .lines()
            .collect();
        for (index, template, values) in cases {
            let (number, expected) = oracle[index].split_once(':').expect("oracle line");
            assert_eq!(number.parse::<usize>().expect("oracle index"), index);
            assert_eq!(substitute(template, &values), expected, "case {index}");
        }
        Ok(())
    }

    #[test]
    fn translator_formats_observed_in_original() -> mlua::Result<()> {
        let lua = Lua::new();
        let cases: &[(&str, &[&str], &str)] = &[
            ("Hello %s", &["Alex"], "Hello Alex"),
            ("%1 / %2", &["one", "two"], "one / two"),
            ("%s %s", &["one", "two"], "one two"),
            ("100%% %s", &["done"], "100% done"),
            ("%2$s %1$s", &["one", "two"], "two one"),
            ("%1 %2", &["one"], "%1 %2"),
            ("%s %s", &["one"], "%s %s"),
            ("plain", &["extra"], "plain"),
            ("%s", &["a%2"], "a%2"),
            ("%1 %s", &["one", "two"], "one one"),
            ("%1 %1", &["A"], "A A"),
            ("%1%%", &["50"], "50%%"),
            ("%s %q", &["one", "two"], "one %q"),
            ("%5", &["a", "b", "c", "d", "e"], "e"),
            ("%s", &[], "%s"),
            ("literal 50%", &["unused"], "literal 50%"),
            (
                "Wave %1 in : %2 (press Enter to skip)",
                &["4", "9"],
                "Wave 4 in : 9 (press Enter to skip)",
            ),
            (
                "average temperature %1. Minimum: %2. Maximum: %3. Humidity: %4%%...",
                &["10", "5", "15", "50"],
                "average temperature %1. Minimum: %2. Maximum: %3. Humidity: %4%%...",
            ),
        ];
        for (template, args, expected) in cases {
            let values: Vec<Value> = args
                .iter()
                .map(|s| lua.create_string(*s).map(Value::String))
                .collect::<mlua::Result<_>>()?;
            assert_eq!(substitute(template, &values), *expected, "{template:?}");
        }
        Ok(())
    }
}
