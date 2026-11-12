//! zombie.scripting.ScriptParser: the brace/comma syntax of `media/scripts/*.txt`.

/// zombie.scripting.ScriptParser.Value: one comma-terminated entry, usually `key = value`.
#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    pub string: String,
}
impl Value {
    /// zombie.scripting.ScriptParser.Value.getKey
    pub fn get_key(&self) -> &str {
        self.string.split_once('=').map_or(&self.string, |p| p.0)
    }
    /// zombie.scripting.ScriptParser.Value.getValue
    pub fn get_value(&self) -> &str {
        self.string.split_once('=').map_or("", |p| p.1)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockElement {
    Value(Value),
    /// Index into `Block::children`.
    Block(usize),
}

/// zombie.scripting.ScriptParser.Block: `type id { ... }`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Block {
    pub r#type: String,
    pub id: Option<String>,
    /// Values and child blocks in source order.
    pub elements: Vec<BlockElement>,
    pub values: Vec<Value>,
    pub children: Vec<Block>,
}

/// zombie.scripting.ScriptParser.readBlock. Positions are char indices, as Java's are UTF-16 units (the same for the
/// basic-plane text of the game's scripts).
fn read_block(s: &[char], mut start: usize, block: &mut Block) -> usize {
    let mut i = start;
    while i < s.len() {
        match s[i] {
            '{' => {
                let header: String = s[start..i].iter().collect();
                let header = crate::java::trim(&header);
                // java.lang.String.split("\\s+"): a leading empty string cannot occur after trim.
                let mut ss = header.split(char::is_whitespace).filter(|x| !x.is_empty());
                let mut child = Block {
                    r#type: ss.next().unwrap_or("").to_string(),
                    id: ss.next().map(str::to_string),
                    ..Block::default()
                };
                i = read_block(s, i + 1, &mut child);
                block.children.push(child);
                block
                    .elements
                    .push(BlockElement::Block(block.children.len() - 1));
                start = i;
                continue;
            }
            '}' => return i + 1,
            ',' => {
                let v = Value {
                    string: s[start..i].iter().collect(),
                };
                block.values.push(v.clone());
                block.elements.push(BlockElement::Value(v));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    i
}

/// zombie.scripting.ScriptParser.parse
pub fn parse(s: &str) -> Block {
    let chars: Vec<char> = s.chars().collect();
    let mut block = Block::default();
    read_block(&chars, 0, &mut block);
    block
}

fn last_index_of(s: &[char], pat: [char; 2], from: isize) -> Option<usize> {
    if from < 0 || s.len() < 2 {
        return None;
    }
    let mut i = (from as usize).min(s.len() - 2);
    loop {
        if s[i] == pat[0] && s[i + 1] == pat[1] {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

/// zombie.scripting.ScriptParser.stripComments: removes `/* ... */`, innermost handling as in the original (scanning
/// from the end of the text).
pub fn strip_comments(total_file: &str) -> String {
    let mut sb: Vec<char> = total_file.chars().collect();
    let (open, close) = (['/', '*'], ['*', '/']);
    let mut end = last_index_of(&sb, close, sb.len() as isize);
    while let Some(e) = end {
        let Some(mut start) = last_index_of(&sb, open, e as isize - 1) else {
            break;
        };
        let mut inner_end = last_index_of(&sb, close, e as isize - 1);
        let mut broken = false;
        while inner_end.is_some_and(|ie| ie > start) {
            let inner_start = start;
            match last_index_of(&sb, open, start as isize - 2) {
                Some(s2) => start = s2,
                None => {
                    broken = true;
                    break;
                }
            }
            inner_end = last_index_of(&sb, close, inner_start as isize - 2);
        }
        if broken {
            break;
        }
        sb.drain(start..e + 2);
        end = last_index_of(&sb, close, start as isize);
    }
    sb.into_iter().collect()
}

/// zombie.scripting.ScriptParser.parseTokens: splits text into top-level `header { ... }` pieces (trimmed).
pub fn parse_tokens(total_file: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut rest: Vec<char> = total_file.chars().collect();
    let find = |s: &[char], c: char, from: usize| (from..s.len()).find(|i| s[*i] == c);
    loop {
        if find(&rest, '}', 1).is_none() {
            let t: String = rest.iter().collect();
            let t = crate::java::trim(&t);
            if !t.is_empty() {
                tokens.push(t.to_string());
            }
            return tokens;
        }
        let mut depth = 0i32;
        // -1 stands for Java's indexOf "not found"; searching resumes from index + 1.
        let (mut next_open, mut next_closed): (isize, isize) = (0, 0);
        loop {
            next_open = find(&rest, '{', (next_open + 1) as usize).map_or(-1, |v| v as isize);
            next_closed = find(&rest, '}', (next_closed + 1) as usize).map_or(-1, |v| v as isize);
            if (next_closed >= next_open || next_closed == -1) && next_open != -1 {
                next_closed = next_open;
                depth += 1;
            } else {
                next_open = next_closed;
                depth -= 1;
            }
            if depth <= 0 {
                break;
            }
        }
        // A stray closing brace gives -1 here; the original then takes an empty token and loops on.
        let cut = (next_open + 1).max(0) as usize;
        let t: String = rest[..cut.min(rest.len())].iter().collect();
        tokens.push(crate::java::trim(&t).to_string());
        rest.drain(..cut.min(rest.len()));
        if cut == 0 {
            return tokens;
        }
    }
}
