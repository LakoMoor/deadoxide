//! Translations: media/lua/shared/Translate/<LANG>/*.json (flat key -> string). `getText` falls back to the key.
use mlua::{Lua, MultiValue, Result, Value};
use std::collections::HashMap;
use std::path::Path;

pub fn install(lua: &Lua, root: &Path, lang: &str) -> Result<usize> {
    let dir = root.join("media/lua/shared/Translate").join(lang);
    let mut map: HashMap<String, String> = HashMap::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut files: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();
        files.sort();
        for f in files {
            let Ok(s) = std::fs::read_to_string(&f) else {
                continue;
            };
            if let Ok(serde_json::Value::Object(o)) = serde_json::from_str::<serde_json::Value>(&s)
            {
                for (k, v) in o {
                    if let serde_json::Value::String(t) = v {
                        map.insert(k, t);
                    }
                }
            }
        }
    }
    let n = map.len();
    let map = std::rc::Rc::new(map);
    let subst = |tpl: &str, args: &[Value]| -> String {
        let mut out = tpl.to_string();
        for (i, a) in args.iter().take(4).enumerate() {
            let s = match a {
                Value::String(s) => s.to_string_lossy().to_string(),
                Value::Integer(n) => n.to_string(),
                Value::Number(n) => n.to_string(),
                _ => continue,
            };
            out = out.replace(&format!("%{}", i + 1), &s);
        }
        out
    };
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
                Some(t) => subst(t, &v[1..]),
                None => k,
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
            Ok(m2.get(&*k.to_string_lossy()).map(|t| subst(t, &v[1..])))
        })?,
    )?;
    Ok(n)
}
