//! Narrow compatibility shim for numeric literals accepted by the game's Kahlua compiler.

fn long_open(src: &[u8], i: usize) -> Option<(usize, usize)> {
    if src.get(i) != Some(&b'[') {
        return None;
    }
    let mut j = i + 1;
    while src.get(j) == Some(&b'=') {
        j += 1;
    }
    (src.get(j) == Some(&b'[')).then_some((j + 1, j - i - 1))
}

fn long_end(src: &[u8], mut i: usize, equals: usize) -> usize {
    while i < src.len() {
        if src[i] == b']' {
            let mut j = i + 1;
            while src.get(j) == Some(&b'=') {
                j += 1;
            }
            if j - i - 1 == equals && src.get(j) == Some(&b']') {
                return j + 1;
            }
        }
        i += 1;
    }
    i
}

/// se.krka.kahlua.luaj.compiler.LexState: adapt observed decimal suffixes for Lua 5.1.
/// This is a narrow lexical shim, not a complete Kahlua lexer.
pub(crate) fn strip_float_suffixes(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        let start = i;
        if src[i] == b'\'' || src[i] == b'"' {
            let quote = src[i];
            i += 1;
            while i < src.len() {
                if src[i] == b'\\' {
                    i = (i + 2).min(src.len());
                } else if src[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if src[i..].starts_with(b"--") {
            i += 2;
            if let Some((body, equals)) = long_open(src, i) {
                i = long_end(src, body, equals);
            } else {
                while i < src.len() && src[i] != b'\n' && src[i] != b'\r' {
                    i += 1;
                }
            }
        } else if let Some((body, equals)) = long_open(src, i) {
            i = long_end(src, body, equals);
        } else if src[i].is_ascii_alphabetic() || src[i] == b'_' {
            i += 1;
            while i < src.len() && (src[i].is_ascii_alphanumeric() || src[i] == b'_') {
                i += 1;
            }
        } else if src[i].is_ascii_digit()
            || (src[i] == b'.' && src.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < src.len() {
                if src[i].is_ascii_alphanumeric()
                    || src[i] == b'_'
                    || src[i] == b'.'
                    || (matches!(src[i], b'+' | b'-') && matches!(src[i - 1], b'e' | b'E'))
                {
                    i += 1;
                } else {
                    break;
                }
            }
            let token = &src[start..i];
            if token.len() > 1
                && matches!(token[token.len() - 1], b'f' | b'F' | b'd' | b'D')
                && !token.starts_with(b"0x")
                && !token.starts_with(b"0X")
                && std::str::from_utf8(&token[..token.len() - 1])
                    .ok()
                    .is_some_and(|number| number.parse::<f64>().is_ok())
            {
                out.extend_from_slice(&token[..token.len() - 1]);
                continue;
            }
        } else {
            i += 1;
        }
        out.extend_from_slice(&src[start..i]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_float_suffixes;
    use mlua::Lua;

    #[test]
    fn decimal_suffix_values_observed_in_original_kahlua() -> mlua::Result<()> {
        let lua = Lua::new();
        for (literal, expected) in [
            ("0.0f", 0.0),
            ("1f", 1.0),
            (".5f", 0.5),
            ("1.2e2f", 120.0),
            ("1.2F", 1.2),
            ("1e-2F", 0.01),
            ("1d", 1.0),
            ("1D", 1.0),
            ("1.23456789f", 1.23456789),
            ("0x10f", 271.0),
        ] {
            let source = strip_float_suffixes(format!("return {literal}").as_bytes());
            let value: f64 = lua.load(&source).eval()?;
            assert_eq!(value, expected, "{literal}");
        }
        Ok(())
    }

    #[test]
    fn preserves_text_comments_and_invalid_numbers() -> mlua::Result<()> {
        let lua = Lua::new();
        let source = br#"local s = '1f\'2f'; local t = [=[3f]=] -- 4f
--[==[ 5f ]==]
return s .. t .. 0.0f"#;
        let normalized = strip_float_suffixes(source);
        assert_eq!(lua.load(&normalized).eval::<String>()?, "1f'2f3f0");
        for invalid in ["1ff", "1.0fabc", "0x1p2f"] {
            let normalized = strip_float_suffixes(format!("return {invalid}").as_bytes());
            assert!(lua.load(&normalized).into_function().is_err(), "{invalid}");
        }
        Ok(())
    }
}
