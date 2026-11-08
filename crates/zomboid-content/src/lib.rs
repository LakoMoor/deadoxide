//! Read-only view of an original Project Zomboid installation.
//! No GPU, no window. The install root is never written to.
pub mod atlas;
pub mod lot;
pub mod pack;
pub mod save;
pub mod script;
pub mod tiledefs;
pub mod zones;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Inventory {
    /// extension (lowercase, no dot) -> file count
    pub by_ext: BTreeMap<String, u64>,
    /// first directory under `media/` -> file count
    pub by_dir: BTreeMap<String, u64>,
    /// map name -> (lotheader count, lotpack count, headers without pack, packs without header)
    pub maps: BTreeMap<String, MapStats>,
    pub unreadable: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone)]
pub struct MapStats {
    pub lotheaders: u64,
    pub lotpacks: u64,
    pub headers_without_pack: Vec<String>,
    pub packs_without_header: Vec<String>,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, bad: &mut Vec<PathBuf>) {
    match std::fs::read_dir(dir) {
        Ok(rd) => {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out, bad);
                } else {
                    out.push(p);
                }
            }
        }
        Err(_) => bad.push(dir.to_path_buf()),
    }
}

/// Walk `<root>/media` and summarize it. Parsing success is not execution compatibility.
pub fn inventory(root: &Path) -> Inventory {
    let media = root.join("media");
    let mut files = Vec::new();
    let mut inv = Inventory::default();
    walk(&media, &mut files, &mut inv.unreadable);
    let mut headers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut packs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in &files {
        let rel = f.strip_prefix(&media).unwrap_or(f);
        let top = rel
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default();
        let top = if rel.components().count() == 1 {
            "(root)".to_string()
        } else {
            top
        };
        *inv.by_dir.entry(top).or_default() += 1;
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        *inv.by_ext.entry(ext.clone()).or_default() += 1;
        if rel.starts_with("maps") && (ext == "lotheader" || ext == "lotpack") {
            let map = rel
                .components()
                .nth(1)
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .unwrap_or_default();
            let stem = f
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            // pack files are named world_X_Y.lotpack, headers X_Y.lotheader
            let key = stem.strip_prefix("world_").unwrap_or(&stem).to_string();
            let m = if ext == "lotheader" {
                &mut headers
            } else {
                &mut packs
            };
            m.entry(map).or_default().push(key);
        }
    }
    let names: std::collections::BTreeSet<_> =
        headers.keys().chain(packs.keys()).cloned().collect();
    for n in names {
        let h = headers.remove(&n).unwrap_or_default();
        let p = packs.remove(&n).unwrap_or_default();
        let hs: std::collections::BTreeSet<_> = h.iter().cloned().collect();
        let ps: std::collections::BTreeSet<_> = p.iter().cloned().collect();
        inv.maps.insert(
            n,
            MapStats {
                lotheaders: h.len() as u64,
                lotpacks: p.len() as u64,
                headers_without_pack: hs.difference(&ps).cloned().collect(),
                packs_without_header: ps.difference(&hs).cloned().collect(),
            },
        );
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_root_is_reported_unreadable() {
        let inv = inventory(Path::new("definitely/not/here"));
        assert_eq!(inv.unreadable.len(), 1);
        assert!(inv.maps.is_empty());
    }
}
