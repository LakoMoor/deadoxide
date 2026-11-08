//! Generic parser for PZ script files (`media/scripts/**/*.txt`).
//! Observed grammar: nested blocks `kind [name...] { ... }`; inside, `key = value,` properties and other raw lines
//! (e.g. `item 1 tags[base:hammer] mode:keep,` ingredient lines). `/* */` and `//` comments are skipped.
//! This is structure only: property semantics belong to the consumers (items, recipes, vehicles...).
#[derive(Debug, Default, Clone)]
pub struct Block {
    pub kind: String,
    pub name: String,
    pub props: Vec<(String, String)>,
    pub lines: Vec<String>,
    pub children: Vec<Block>,
}

#[derive(Debug)]
pub struct ScriptError {
    pub line: usize,
    pub msg: &'static str,
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                if b[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
        } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// Parse a whole file into top-level blocks (usually `module X { ... }`).
pub fn parse_script(src: &str) -> Result<Vec<Block>, ScriptError> {
    let text = strip_comments(src);
    let mut stack: Vec<Block> = vec![Block::default()]; // root
    let mut pending: Option<String> = None; // header line waiting for `{`
    for (ln, raw) in text.lines().enumerate() {
        let mut l = raw.trim();
        if l.is_empty() {
            continue;
        }
        // a line may contain `{` / `}` at its start or end (`kind name {`, `}`, `kind {` ...)
        loop {
            if l.is_empty() {
                break;
            }
            if l == "{" || l.starts_with('{') {
                let hdr = pending.take().unwrap_or_default();
                let mut it = hdr.splitn(2, char::is_whitespace);
                let kind = it.next().unwrap_or("").to_string();
                let name = it.next().unwrap_or("").trim().to_string();
                stack.push(Block {
                    kind,
                    name,
                    ..Default::default()
                });
                l = l[1..].trim();
                continue;
            }
            if l.starts_with('}') {
                if let Some(h) = pending.take() {
                    stack.last_mut().unwrap().lines.push(h);
                }
                if stack.len() < 2 {
                    return Err(ScriptError {
                        line: ln + 1,
                        msg: "unbalanced }",
                    });
                }
                let done = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(done);
                l = l[1..].trim_start_matches(',').trim();
                continue;
            }
            // up to the next structural char
            let end = l.find(['{', '}']).unwrap_or(l.len());
            let seg = l[..end].trim();
            let seg_is_header = end < l.len() && l.as_bytes()[end] == b'{';
            if !seg.is_empty() {
                if seg_is_header {
                    pending = Some(seg.trim_end_matches(',').to_string());
                } else if let Some((k, v)) = seg.split_once('=') {
                    if let Some(h) = pending.take() {
                        stack.last_mut().unwrap().lines.push(h);
                    }
                    stack.last_mut().unwrap().props.push((
                        k.trim().to_string(),
                        v.trim().trim_end_matches(',').trim().to_string(),
                    ));
                } else if end == l.len()
                    && !seg.contains(' ')
                    && !seg.ends_with(',')
                    && !seg.contains(':')
                {
                    // possible header whose `{` is on the next line
                    if let Some(h) = pending.take() {
                        stack.last_mut().unwrap().lines.push(h);
                    }
                    pending = Some(seg.to_string());
                } else {
                    if let Some(h) = pending.take() {
                        stack.last_mut().unwrap().lines.push(h);
                    }
                    if end == l.len()
                        && !seg.ends_with(',')
                        && !seg.contains(':')
                        && !seg.contains('[')
                    {
                        pending = Some(seg.to_string());
                    } else {
                        stack
                            .last_mut()
                            .unwrap()
                            .lines
                            .push(seg.trim_end_matches(',').trim().to_string());
                    }
                }
            }
            l = l[end..].trim();
        }
    }
    if let Some(h) = pending.take() {
        stack.last_mut().unwrap().lines.push(h);
    }
    if stack.len() != 1 {
        return Err(ScriptError {
            line: text.lines().count(),
            msg: "unclosed {",
        });
    }
    Ok(stack.pop().unwrap().children)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn module_item_props_and_nested() {
        let s = "module Base\n{\n  item Axe\n  {\n    Weight = 2.5,\n    Type = Weapon, // c\n    craftRecipe X { inputs { item 1 tags[base:hammer] mode:keep, } }\n  }\n}";
        let t = parse_script(s).unwrap();
        assert_eq!(t[0].kind, "module");
        assert_eq!(t[0].name, "Base");
        let item = &t[0].children[0];
        assert_eq!((item.kind.as_str(), item.name.as_str()), ("item", "Axe"));
        assert_eq!(item.props[0], ("Weight".into(), "2.5".into()));
        assert_eq!(item.children[0].kind, "craftRecipe");
        assert_eq!(
            item.children[0].children[0].lines[0],
            "item 1 tags[base:hammer] mode:keep"
        );
    }
    #[test]
    fn unbalanced_is_error() {
        assert!(parse_script("module A {").is_err());
        assert!(parse_script("}").is_err());
    }
}
