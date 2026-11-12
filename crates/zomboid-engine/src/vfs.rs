//! File access for the engine. Natively this is `std::fs`. In the browser there is no file system, so the page mounts
//! the files the player picked from their own install into memory and the same engine code reads them from there.
//! The mounted set is per thread (logged in docs/PORT.md as the one piece of global state of the engine).
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;

thread_local! {
    static MEM: RefCell<Option<BTreeMap<String, Vec<u8>>>> = const { RefCell::new(None) };
}

fn key(p: &Path) -> String {
    p.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

/// Add a file to the in-memory file system; from the first call on, all reads of this thread go to memory.
pub fn mount(path: &str, data: Vec<u8>) {
    MEM.with(|m| {
        m.borrow_mut()
            .get_or_insert_with(BTreeMap::new)
            .insert(path.trim_end_matches('/').to_string(), data)
    });
}

/// Remove and return a mounted file (frees memory for large one-time inputs such as texture packs).
pub fn take(path: &Path) -> Option<Vec<u8>> {
    MEM.with(|m| m.borrow_mut().as_mut().and_then(|m| m.remove(&key(path))))
}

fn mounted() -> bool {
    MEM.with(|m| m.borrow().is_some())
}

fn not_found(p: &Path) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, p.display().to_string())
}

pub fn read(p: &Path) -> io::Result<Vec<u8>> {
    if !mounted() {
        return std::fs::read(p);
    }
    MEM.with(|m| m.borrow().as_ref().and_then(|m| m.get(&key(p)).cloned()))
        .ok_or_else(|| not_found(p))
}

pub fn exists(p: &Path) -> bool {
    if !mounted() {
        return p.exists();
    }
    let k = key(p);
    MEM.with(|m| m.borrow().as_ref().is_some_and(|m| m.contains_key(&k))) || is_dir(p)
}

pub fn is_dir(p: &Path) -> bool {
    if !mounted() {
        return p.is_dir();
    }
    let prefix = format!("{}/", key(p));
    MEM.with(|m| {
        m.borrow().as_ref().is_some_and(|m| {
            m.range(prefix.clone()..)
                .next()
                .is_some_and(|(k, _)| k.starts_with(&prefix))
        })
    })
}

/// Names of the entries directly inside a directory. Natively in the order the OS lists them (as Java's `File.list`);
/// in memory in name order.
pub fn read_dir(p: &Path) -> io::Result<Vec<String>> {
    if !mounted() {
        let mut names = Vec::new();
        for e in std::fs::read_dir(p)? {
            names.push(e?.file_name().to_string_lossy().into_owned());
        }
        return Ok(names);
    }
    let prefix = format!("{}/", key(p));
    let names: Vec<String> = MEM.with(|m| {
        let m = m.borrow();
        let mut out: Vec<String> = Vec::new();
        for (k, _) in m
            .as_ref()
            .into_iter()
            .flat_map(|m| m.range(prefix.clone()..))
            .take_while(|(k, _)| k.starts_with(&prefix))
        {
            let name = k[prefix.len()..]
                .split('/')
                .next()
                .unwrap_or("")
                .to_string();
            if out.last() != Some(&name) {
                out.push(name);
            }
        }
        out
    });
    if names.is_empty() {
        return Err(not_found(p));
    }
    Ok(names)
}
