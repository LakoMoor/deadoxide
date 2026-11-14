//! zombie.core.Translator: so far the `ItemName` table, for the default language (EN) only.
use std::collections::HashMap;
use std::path::Path;

/// zombie.core.Translator, the ported part: `itemName` (full type -> display name).
#[derive(Clone, Debug, Default)]
pub struct Translator {
    pub item_name: HashMap<String, String>,
}

/// zombie.core.Translator.formatFixer: `%%` stays, `%N` (N in 1..9) becomes `%N$s`.
pub fn format_fixer(input: &str) -> String {
    let b = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            match b.get(i + 1) {
                Some(b'%') => {
                    out.push_str("%%");
                    i += 2;
                    continue;
                }
                Some(d @ b'1'..=b'9') => {
                    out.push('%');
                    out.push(*d as char);
                    out.push_str("$s");
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        // Copy one whole UTF-8 character.
        let len = input[i..].chars().next().map_or(1, char::len_utf8);
        out.push_str(&input[i..i + len]);
        i += len;
    }
    out
}

/// org.json.JSONObject(String).toMap() for an object whose values are all strings: entries in file order.
/// `Err` where org.json throws (syntax, duplicate key) or where the original's `(String) v` cast fails.
pub fn parse_string_object(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut p = JsonReader {
        s: text.as_bytes(),
        i: 0,
    };
    p.ws();
    p.expect(b'{')?;
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    p.ws();
    if p.peek() == Some(b'}') {
        return Ok(out);
    }
    loop {
        p.ws();
        let k = p.string()?;
        p.ws();
        p.expect(b':')?;
        p.ws();
        if p.peek() != Some(b'"') {
            return Err(format!(
                "java.lang.ClassCastException: value of \"{k}\" is not a String"
            ));
        }
        let v = p.string()?;
        if !seen.insert(k.clone()) {
            return Err(format!("org.json.JSONException: Duplicate key \"{k}\""));
        }
        out.push((k, v));
        p.ws();
        match p.next() {
            Some(b',') => {
                p.ws();
                // org.json accepts a trailing comma before '}'.
                if p.peek() == Some(b'}') {
                    return Ok(out);
                }
            }
            Some(b'}') => return Ok(out),
            _ => {
                return Err(format!(
                    "org.json.JSONException: Expected a ',' or '}}' at {}",
                    p.i
                ))
            }
        }
    }
}

struct JsonReader<'a> {
    s: &'a [u8],
    i: usize,
}

impl JsonReader<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn next(&mut self) -> Option<u8> {
        let c = self.peek();
        self.i += 1;
        c
    }
    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.next() == Some(c) {
            Ok(())
        } else {
            Err(format!(
                "org.json.JSONException: expected '{}' at {}",
                c as char, self.i
            ))
        }
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or("org.json.JSONException: Illegal escape.")?;
        let h = std::str::from_utf8(h).map_err(|e| e.to_string())?;
        self.i += 4;
        u32::from_str_radix(h, 16)
            .map_err(|_| "org.json.JSONException: Illegal escape.".to_string())
    }
    /// A quoted string; escapes as in JSONTokener.nextString.
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let start = self.i;
            while matches!(self.peek(), Some(c) if c != b'"' && c != b'\\' && c != b'\n' && c != b'\r')
            {
                self.i += 1;
            }
            let run = std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?;
            units.extend(run.encode_utf16());
            match self.next() {
                Some(b'"') => break,
                Some(b'\\') => {
                    let c = match self.next() {
                        Some(b'b') => 8,
                        Some(b't') => 9,
                        Some(b'n') => 10,
                        Some(b'f') => 12,
                        Some(b'r') => 13,
                        Some(b'u') => self.hex4()?,
                        Some(c @ (b'"' | b'\'' | b'\\' | b'/')) => u32::from(c),
                        _ => return Err("org.json.JSONException: Illegal escape.".into()),
                    };
                    units.push(c as u16);
                }
                _ => return Err("org.json.JSONException: Unterminated string".into()),
            }
        }
        Ok(String::from_utf16_lossy(&units))
    }
}

impl Translator {
    /// zombie.core.Translator.loadFiles for the `ItemName` table and the language stack of EN (the server's default):
    /// zombie.core.Translator.tryFillMapFromFile reads `media/lua/shared/Translate/EN/ItemName.json`; a missing file
    /// is skipped, a malformed one is the original's RuntimeException. Mods are not loaded.
    pub fn load_files(root: &Path) -> Result<Self, String> {
        let mut t = Translator::default();
        let file = root.join("media/lua/shared/Translate/EN/ItemName.json");
        if crate::vfs::exists(&file) {
            let bytes = crate::vfs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let text = String::from_utf8_lossy(&bytes);
            let entries = parse_string_object(&text)
                .map_err(|e| format!("JSON Error in: {}: {e}", file.display()))?;
            for (k, v) in entries {
                if !t.item_name.contains_key(&k) || !v.is_empty() {
                    t.item_name.insert(k, format_fixer(&v));
                }
            }
        }
        Ok(t)
    }

    /// zombie.core.Translator.getItemNameFromFullType: the table entry, else the script's display name (its full name
    /// when it has none), else the full type; the answer is cached in the table. `script_display_name` is
    /// `Some(item.getDisplayName())` when ScriptManager knows the item.
    pub fn get_item_name_from_full_type(
        &mut self,
        full_type: &str,
        script_display_name: Option<String>,
    ) -> Result<String, String> {
        if !full_type.contains('.') {
            return Err(
                "java.lang.IllegalArgumentException: fullType must contain \".\" i.e. module.type"
                    .into(),
            );
        }
        if let Some(name) = self.item_name.get(full_type) {
            return Ok(name.clone());
        }
        let name = script_display_name.unwrap_or_else(|| full_type.to_string());
        self.item_name.insert(full_type.to_string(), name.clone());
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_fixer_rewrites_numbered_placeholders_only() {
        assert_eq!(
            format_fixer("%1 of %2, 100%% and %a %0"),
            "%1$s of %2$s, 100%% and %a %0"
        );
    }

    #[test]
    fn json_strings_unescape_and_duplicates_fail() {
        let v = parse_string_object(r#"{ "a": "x\"yé", "b": "" }"#).unwrap_or_default();
        assert_eq!(
            v,
            vec![
                ("a".to_string(), "x\"y\u{e9}".to_string()),
                ("b".to_string(), String::new())
            ]
        );
        assert!(parse_string_object(r#"{"a":"1","a":"2"}"#).is_err());
        assert!(parse_string_object(r#"{"a":1}"#).is_err());
    }
}
