//! Map zone file (`media/maps/<map>/objects.lua`): a Lua table literal `objects = { {name=..,type=..,x=..,..}, ... }`.
//! Parsed with a small constructor-only Lua reader (strings, numbers, booleans, nil, nested tables, `k = v`, `[k] = v`).
//! It does NOT evaluate expressions; it is checked against the real Lua count in tests/ (see docs/COMPATIBILITY.md).
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Nil,
    Bool(bool),
    Num(f64),
    Str(String),
    Table(Vec<Val>, BTreeMap<String, Val>),
}

impl Val {
    pub fn get(&self, k: &str) -> Option<&Val> {
        if let Val::Table(_, m) = self {
            m.get(k)
        } else {
            None
        }
    }
    pub fn num(&self, k: &str) -> Option<f64> {
        if let Some(Val::Num(n)) = self.get(k) {
            Some(*n)
        } else {
            None
        }
    }
    pub fn str(&self, k: &str) -> Option<&str> {
        if let Some(Val::Str(s)) = self.get(k) {
            Some(s)
        } else {
            None
        }
    }
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        loop {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.b[self.i..].starts_with(b"--") {
                while self.i < self.b.len() && self.b[self.i] != b'\n' {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }
    fn ident(&mut self) -> String {
        let s = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'_')
        {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.b[s..self.i]).into_owned()
    }
    fn value(&mut self) -> Result<Val, String> {
        self.ws();
        let c = *self.b.get(self.i).ok_or("eof")?;
        match c {
            b'{' => self.table(),
            b'"' | b'\'' => {
                self.i += 1;
                let mut s = Vec::new();
                while self.i < self.b.len() && self.b[self.i] != c {
                    if self.b[self.i] == b'\\' {
                        self.i += 1;
                    }
                    s.push(self.b[self.i]);
                    self.i += 1;
                }
                self.i += 1;
                Ok(Val::Str(String::from_utf8_lossy(&s).into_owned()))
            }
            b'-' | b'.' | b'0'..=b'9' => {
                let s = self.i;
                self.i += 1;
                while self.i < self.b.len()
                    && (self.b[self.i].is_ascii_digit()
                        || matches!(self.b[self.i], b'.' | b'e' | b'E' | b'+' | b'-'))
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[s..self.i])
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .map(Val::Num)
                    .ok_or_else(|| format!("bad number at {s}"))
            }
            _ => match self.ident().as_str() {
                "true" => Ok(Val::Bool(true)),
                "false" => Ok(Val::Bool(false)),
                "nil" => Ok(Val::Nil),
                w => Err(format!("unsupported expression `{w}` at byte {}", self.i)),
            },
        }
    }
    fn table(&mut self) -> Result<Val, String> {
        self.i += 1; // {
        let (mut arr, mut map) = (Vec::new(), BTreeMap::new());
        loop {
            self.ws();
            match self.b.get(self.i) {
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                Some(b',') | Some(b';') => {
                    self.i += 1;
                    continue;
                }
                None => return Err("eof in table".into()),
                _ => {}
            }
            let save = self.i;
            if self.b[self.i] == b'[' {
                self.i += 1;
                let k = self.value()?;
                self.ws();
                self.i += 1; // ]
                self.ws();
                self.i += 1; // =
                let v = self.value()?;
                map.insert(
                    match k {
                        Val::Str(s) => s,
                        Val::Num(n) => n.to_string(),
                        _ => return Err("bad key".into()),
                    },
                    v,
                );
                continue;
            }
            if self.b[self.i].is_ascii_alphabetic() || self.b[self.i] == b'_' {
                let id = self.ident();
                self.ws();
                if self.b.get(self.i) == Some(&b'=') && self.b.get(self.i + 1) != Some(&b'=') {
                    self.i += 1;
                    let v = self.value()?;
                    map.insert(id, v);
                    continue;
                }
                self.i = save;
            }
            arr.push(self.value()?);
        }
        Ok(Val::Table(arr, map))
    }
}

#[derive(Debug, Clone)]
pub struct Zone {
    pub name: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    /// polygon points (x,y) when `geometry = "polygon"`, else empty
    pub points: Vec<(i32, i32)>,
    pub has_properties: bool,
}

/// Parse an `objects.lua`; returns every zone in file order.
pub fn parse_objects(src: &[u8]) -> Result<Vec<Zone>, String> {
    let mut p = P { b: src, i: 0 };
    p.ws();
    if p.ident() != "objects" {
        return Err("file does not start with `objects =`".into());
    }
    p.ws();
    p.i += 1; // =
    let Val::Table(arr, _) = p.value()? else {
        return Err("objects is not a table".into());
    };
    let mut out = Vec::with_capacity(arr.len());
    for v in &arr {
        let pts = match v.get("points") {
            Some(Val::Table(a, _)) => a
                .chunks(2)
                .filter_map(|c| match c {
                    [Val::Num(x), Val::Num(y)] => Some((*x as i32, *y as i32)),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };
        out.push(Zone {
            name: v.str("name").unwrap_or("").to_string(),
            kind: v.str("type").unwrap_or("").to_string(),
            x: v.num("x").unwrap_or(0.0) as i32,
            y: v.num("y").unwrap_or(0.0) as i32,
            z: v.num("z").unwrap_or(0.0) as i32,
            w: v.num("width").unwrap_or(0.0) as i32,
            h: v.num("height").unwrap_or(0.0) as i32,
            points: pts,
            has_properties: v.get("properties").is_some(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_rect_polygon_and_properties() {
        let z = parse_objects(br#"objects = {
          { name = "a", type = "TownZone", x = 1, y = 2, z = 0, width = 3, height = 4 }, -- c
          { name = "", type = "Vegitation", z = 0, geometry = "polygon", points = { 1, 2, 3, 4, 5, 6 } },
          { name = "m", type = "Mannequin", x = 9, y = 8, z = 1, width = 1, height = 1, properties = { Pose = "x", [1] = true } },
        }"#).unwrap();
        assert_eq!(z.len(), 3);
        assert_eq!(
            (z[0].kind.as_str(), z[0].x, z[0].w, z[0].h),
            ("TownZone", 1, 3, 4)
        );
        assert_eq!(z[1].points, vec![(1, 2), (3, 4), (5, 6)]);
        assert!(z[2].has_properties && !z[0].has_properties);
    }
    #[test]
    fn real_muldraugh_matches_the_lua_oracle() {
        let p = std::path::Path::new("../../../media/maps/Muldraugh, KY/objects.lua");
        let Ok(b) = std::fs::read(p) else { return };
        let z = parse_objects(&b).unwrap();
        // 37,389 zone rows counted with `grep -c '^  {'` and by the real Lua (#objects, see docs/COMPATIBILITY.md)
        assert_eq!(z.len(), 37389);
        let bad: Vec<_> = z
            .iter()
            .filter(|z| z.kind == "Mannequin" && !z.has_properties)
            .collect();
        assert_eq!(bad.len(), 1);
        assert_eq!(
            (bad[0].x, bad[0].y, bad[0].z),
            (13583, 1299, 0),
            "same coords as the real client's error"
        );
    }
}
