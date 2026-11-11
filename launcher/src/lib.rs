//! Launcher logic without any UI: registry types, download + sha256 + safe extract, game discovery.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const SITE: &str = "https://deadoxide-rust.github.io/deadoxide/";

#[derive(Deserialize, Clone, Debug)]
pub struct Mod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub url: String,
    pub sha256: String,
}
#[derive(Deserialize)]
pub struct Registry {
    pub mods: Vec<Mod>,
}
#[derive(Deserialize, Clone)]
pub struct Area {
    pub name: String,
    pub percent: f32,
    pub measured: bool,
}
#[derive(Deserialize, Clone)]
pub struct Progress {
    pub overall: f32,
    pub generated: String,
    pub areas: Vec<Area>,
}

#[derive(Deserialize, Clone)]
pub struct Release {
    pub version: String,
    pub date: String,
    #[serde(default)]
    pub new: Vec<String>,
    #[serde(default)]
    pub fixed: Vec<String>,
}
#[derive(Deserialize)]
pub struct Changelog {
    pub releases: Vec<Release>,
}

pub fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    if let Some(p) = url.strip_prefix("file:") {
        let mut v = std::fs::read(p).map_err(|e| e.to_string())?;
        if v.starts_with(&[0xEF, 0xBB, 0xBF]) {
            v.drain(..3);
        }
        return Ok(v);
    }
    let mut r = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let mut v = Vec::new();
    r.body_mut()
        .as_reader()
        .take(64 << 20)
        .read_to_end(&mut v)
        .map_err(|e| e.to_string())?;
    if v.starts_with(&[0xEF, 0xBB, 0xBF]) {
        v.drain(..3);
    }
    Ok(v)
}

/// Verify sha256, then extract. Entries that would escape `dest` (zip-slip) are rejected.
pub fn install(m: &Mod, base: &str, mods_dir: &Path) -> Result<PathBuf, String> {
    let url = if m.url.contains("://") || m.url.starts_with("file:") {
        m.url.clone()
    } else {
        format!("{base}{}", m.url)
    };
    let data = get_bytes(&url)?;
    let got: String = Sha256::digest(&data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !got.eq_ignore_ascii_case(&m.sha256) {
        return Err(format!("sha256 mismatch: expected {}, got {got}", m.sha256));
    }
    let dest = mods_dir.join(&m.id);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        let rel = f
            .enclosed_name()
            .ok_or_else(|| format!("unsafe path in archive: {}", f.name()))?;
        let out = dest.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut w).map_err(|e| e.to_string())?;
        }
    }
    Ok(dest)
}

pub fn find_game() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = vec![];
    if let Ok(p) = std::env::var("PZ_ROOT") {
        c.push(p.into());
    }
    let mut d = std::env::current_dir().ok();
    while let Some(p) = d {
        c.push(p.clone());
        d = p.parent().map(Path::to_path_buf);
    }
    for pf in [
        "C:/Program Files (x86)/Steam",
        "C:/Program Files/Steam",
        "D:/Steam",
        "D:/SteamLibrary",
        "E:/Games",
        "E:/SteamLibrary",
    ] {
        c.push(Path::new(pf).join("steamapps/common/ProjectZomboid"));
    }
    c.into_iter()
        .find(|p| p.join("projectzomboid.jar").exists())
}

pub fn find_client() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "zomboid-client.exe"
    } else {
        "zomboid-client"
    };
    let mut c = vec![];
    if let Ok(e) = std::env::current_exe() {
        if let Some(d) = e.parent() {
            c.push(d.join(name));
        }
    }
    c.push(PathBuf::from("target/release").join(name));
    c.push(PathBuf::from("zomboid-rs/target/release").join(name));
    c.into_iter().find(|p| p.exists())
}

pub fn find_players_db(dir: &Path) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_players_db(&p) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == "players.db") {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> String {
        format!(
            "file:{}/../site/",
            env!("CARGO_MANIFEST_DIR").replace(std::path::MAIN_SEPARATOR, "/")
        )
    }
    fn catalog_mod() -> Mod {
        let b = get_bytes(&format!("{}registry.json", site())).unwrap();
        serde_json::from_slice::<Registry>(&b)
            .unwrap()
            .mods
            .remove(0)
    }

    #[test]
    fn installs_the_catalog_mod_into_a_fresh_dir() {
        let dir = std::env::temp_dir().join(format!("deadoxide-test-{}", std::process::id()));
        let out = install(&catalog_mod(), &site(), &dir).unwrap();
        let lua = std::fs::read_to_string(out.join("media/lua/client/HelloDeadOxide.lua")).unwrap();
        assert!(lua.contains("mod loaded"));
        assert!(out.join("mod.info").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_sha256_installs_nothing() {
        let dir = std::env::temp_dir().join(format!("deadoxide-test-bad-{}", std::process::id()));
        let mut m = catalog_mod();
        m.sha256 = "0".repeat(64);
        assert!(install(&m, &site(), &dir).unwrap_err().contains("sha256"));
        assert!(!dir.exists());
    }
}
