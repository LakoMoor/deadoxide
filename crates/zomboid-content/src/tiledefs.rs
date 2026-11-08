//! Tile property definitions (`media/*.tiles.txt`, text form of the binary `.tiles`).
//! Layout (observed): `tileset { file = NAME  size = COLS,ROWS  id = N  tile { xy = X,Y  Prop = Value ... } ... }`.
//! Tile name = `NAME_{Y*COLS+X}` (matches the `// NAME_i` comments in the files).
use std::collections::HashMap;

pub type Props = HashMap<String, String>;
#[derive(Default)]
pub struct TileDefs {
    pub tiles: HashMap<String, Props>,
}

pub fn parse_tiles_txt(src: &str, into: &mut TileDefs) {
    let (mut file, mut cols) = (String::new(), 1i32);
    let mut cur: Option<(i32, i32, Props)> = None;
    let mut in_tile = false;
    for line in src.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with("//") {
            continue;
        }
        if l == "tile" {
            in_tile = true;
            cur = Some((0, 0, Props::new()));
            continue;
        }
        if l == "tileset" {
            file.clear();
            continue;
        }
        if l == "{" {
            continue;
        }
        if l == "}" {
            if in_tile {
                if let Some((x, y, p)) = cur.take() {
                    let name = format!("{file}_{}", y * cols + x);
                    into.tiles.entry(name).or_default().extend(p);
                }
                in_tile = false;
            }
            continue;
        }
        let Some((k, v)) = l.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if in_tile {
            if let Some((_, _, p)) = &mut cur {
                if k == "xy" {
                    if let Some((a, b)) = v.split_once(',') {
                        if let (Ok(a), Ok(b)) = (a.trim().parse(), b.trim().parse()) {
                            p.insert("xy".into(), v.into());
                            cur.as_mut().unwrap().0 = a;
                            cur.as_mut().unwrap().1 = b;
                        }
                    }
                } else {
                    p.insert(k.into(), v.into());
                }
            }
        } else if k == "file" {
            file = v.into();
        } else if k == "size" {
            cols = v
                .split(',')
                .next()
                .and_then(|c| c.trim().parse().ok())
                .unwrap_or(1);
        }
    }
}

impl TileDefs {
    pub fn load(root: &std::path::Path) -> std::io::Result<Self> {
        let mut d = TileDefs::default();
        // later files patch earlier ones
        for f in [
            "newtiledefinitions.tiles.txt",
            "tiledefinitions_erosion.tiles.txt",
            "tiledefinitions_noiseworks.patch.tiles.txt",
            "jumbo_trees.tiles.txt",
            "jumbo_trees_big.tiles.txt",
        ] {
            parse_tiles_txt(
                &std::fs::read_to_string(root.join("media").join(f))?,
                &mut d,
            );
        }
        Ok(d)
    }
    pub fn has(&self, name: &str, prop: &str) -> bool {
        self.tiles.get(name).is_some_and(|p| p.contains_key(prop))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_names_and_props() {
        let src = "version = 1\ntileset\n{\n file = foo\n size = 8,16\n id = 3\n // foo_9\n tile\n {\n  xy = 1,1\n  solid = \n  Facing = S\n }\n}\n";
        let mut d = TileDefs::default();
        parse_tiles_txt(src, &mut d);
        assert!(d.has("foo_9", "solid"));
        assert_eq!(d.tiles["foo_9"]["Facing"], "S");
    }
}
