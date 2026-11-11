//! Lua 5.1 harness over the game's media/lua tree (PZ uses Kahlua, a Lua 5.1 dialect).
//! `zomboid-lua <root> compile` : compile-only check.
//! `zomboid-lua <root> run [lua-expr]` : execute shared, client, server with a stubbed Java API and PZ-style `require`.
//!
//! `require` rules (taken from the real client's log, see tests/golden/lua_real): a module is only visible from
//! its own directory tree or `shared`; requiring a module that is still loading logs `recursive require()`;
//! an unresolvable module logs `require("x") failed`. Warnings are printed in the real log's normalised form.
mod kahlua;
mod scripts;
mod text;
use mlua::{Lua, Table};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn walk(d: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out)
            } else if p.extension().and_then(|x| x.to_str()) == Some("lua") {
                out.push(p)
            }
        }
    }
}

#[derive(Default)]
struct State {
    /// "sub/name" -> loaded
    loaded: HashSet<String>,
    loading: HashSet<String>,
    stack: Vec<String>,
    warnings: Vec<String>,
}

fn rel_name(base: &Path, sub: &str, p: &Path) -> String {
    p.strip_prefix(base.join(sub))
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn main() -> mlua::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(a.get(1).map(String::as_str).unwrap_or(".."));
    let base = root.join("media/lua");
    let mode = a.get(2).map(String::as_str).unwrap_or("compile");
    let lua = Lua::new();
    if mode == "compile" {
        let mut files = vec![];
        walk(&base, &mut files);
        files.sort();
        let (mut ok, mut bad) = (0, 0);
        for f in &files {
            let src = std::fs::read(f).map_err(mlua::Error::external)?;
            let name = f.to_string_lossy();
            let mut compiled = lua.load(&src[..]).set_name(name.as_ref()).into_function();
            if matches!(&compiled, Err(e) if e.to_string().contains("malformed number")) {
                let normalized = kahlua::strip_float_suffixes(&src);
                compiled = lua
                    .load(&normalized[..])
                    .set_name(name.as_ref())
                    .into_function();
            }
            match compiled {
                Ok(_) => ok += 1,
                Err(e) => {
                    bad += 1;
                    println!(
                        "{}: {}",
                        f.display(),
                        e.to_string().lines().next().unwrap_or("")
                    );
                }
            }
        }
        println!("lua files: {} compiled ok={ok} failed={bad}", files.len());
        return Ok(());
    }
    let nitems = scripts::install(&lua, &root)?;
    let lang = std::env::var("ZOMBOID_LANG").unwrap_or_else(|_| "EN".to_owned());
    println!(
        "ScriptManager: {nitems} item scripts; {lang} strings: {}",
        text::install(&lua, &root, &lang)?
    );
    lua.load(include_str!("prelude.lua")).exec()?;

    let mut all: Vec<(String, PathBuf)> = vec![];
    for sub in ["shared", "client", "server"] {
        let mut v = vec![];
        walk(&base.join(sub), &mut v);
        // real client order is case-insensitive (lowercase dir "attachments" loads before "Biomes.lua"): matches the golden log
        v.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        for p in v {
            all.push((sub.to_string(), p));
        }
    }
    // per-directory index: (sub, relative name) -> path
    let mut index: HashMap<(String, String), PathBuf> = HashMap::new();
    for (sub, p) in &all {
        index.insert(
            (sub.clone(), rel_name(&base, sub, p).to_lowercase()),
            p.clone(),
        );
    }
    let index = Rc::new(index);
    let st = Rc::new(RefCell::new(State::default()));
    let errs: Rc<RefCell<BTreeMap<String, u32>>> = Rc::new(RefCell::new(BTreeMap::new()));

    // Executes one file; records the first-line error text.
    let run_file = {
        let st = st.clone();
        let errs = errs.clone();
        let base = base.clone();
        move |lua: &Lua, sub: &str, p: &Path| -> bool {
            let name = rel_name(&base, sub, p);
            let key = format!("{sub}/{name}");
            st.borrow_mut().loading.insert(key.clone());
            st.borrow_mut().stack.push(sub.to_string());
            let src = std::fs::read(p).unwrap_or_default();
            let path = p.to_string_lossy().to_string();
            let mut r = lua.load(&src[..]).set_name(path.clone()).exec();
            // Kahlua accepts Java float literals (`0.0f`); stock Lua 5.1 does not. Retry with the suffix stripped,
            // only for files that fail with "malformed number".
            if matches!(&r, Err(e) if e.to_string().contains("malformed number")) {
                let out = kahlua::strip_float_suffixes(&src);
                r = lua.load(&out[..]).set_name(path.clone()).exec();
            }
            st.borrow_mut().stack.pop();
            let mut s = st.borrow_mut();
            s.loading.remove(&key);
            s.loaded.insert(key);
            match r {
                Ok(()) => true,
                Err(err) => {
                    let m: String = err
                        .to_string()
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(110)
                        .collect();
                    *errs
                        .borrow_mut()
                        .entry(m.replace(&path, "<file>"))
                        .or_default() += 1;
                    false
                }
            }
        }
    };
    let run_file = Rc::new(run_file);

    {
        let (st, index, run_file, base) =
            (st.clone(), index.clone(), run_file.clone(), base.clone());
        lua.globals().raw_set(
            "require",
            lua.create_function(move |lua, name: String| {
                let name = name.strip_suffix(".lua").unwrap_or(&name).to_string();
                let caller = st
                    .borrow()
                    .stack
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "shared".into());
                let visible: &[&str] = match caller.as_str() {
                    "client" => &["client", "shared"],
                    "server" => &["server", "shared"],
                    _ => &["shared"],
                };
                let found = visible.iter().find_map(|s| {
                    index
                        .get(&(s.to_string(), name.to_lowercase()))
                        .map(|p| (s.to_string(), p.clone()))
                });
                let Some((sub, path)) = found else {
                    st.borrow_mut()
                        .warnings
                        .push(format!("require(\"{name}\") failed"));
                    return Ok(());
                };
                let key = format!("{sub}/{name}");
                if st.borrow().loading.contains(&key) {
                    st.borrow_mut()
                        .warnings
                        .push(format!("recursive require(): media/lua/{sub}/{name}.lua"));
                    return Ok(());
                }
                if st.borrow().loaded.contains(&key) {
                    return Ok(());
                }
                run_file(lua, &sub, &path);
                let _ = &base;
                Ok(())
            })?,
        )?;
    }
    if let Some(code) = a.get(3).filter(|_| mode == "run") {
        println!("eval: {}", lua.load(code.as_str()).eval::<String>()?);
    }
    let mut stats = BTreeMap::<String, (u32, u32)>::new();
    for (sub, p) in &all {
        let key = format!("{sub}/{}", rel_name(&base, sub, p));
        if st.borrow().loaded.contains(&key) {
            stats.entry(sub.clone()).or_default().0 += 1;
            continue;
        }
        let ok = run_file(&lua, sub, p);
        let e = stats.entry(sub.clone()).or_default();
        if ok {
            e.0 += 1
        } else {
            e.1 += 1
        }
    }
    for (k, (ok, bad)) in &stats {
        println!("{k:7} executed ok={ok} errored={bad}");
    }
    let mut top: Vec<_> = errs.borrow().iter().map(|(k, v)| (k.clone(), *v)).collect();
    top.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("-- top errors --");
    for (m, n) in top.iter().take(12) {
        println!("{n:4}  {m}");
    }
    let missing: Table = lua.globals().get("MISSING")?;
    let mut ms: Vec<(String, i64)> = missing.pairs::<String, i64>().flatten().collect();
    ms.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("-- {} distinct undefined globals touched --", ms.len());
    // warnings in the same normalised form as tests/golden/lua_real/client_lua_warnings.txt
    // Run the REAL metazoneHandler.doMapZones() with only the Java file-access functions supplied.
    {
        let g = lua.globals();
        let wr = st.clone();
        g.raw_set(
            "__warn",
            lua.create_function(move |_, m: String| {
                wr.borrow_mut().warnings.push(m);
                Ok(())
            })?,
        )?;
        let r2 = root.clone();
        g.raw_set(
            "fileExists",
            lua.create_function(move |_, p: String| Ok(r2.join(p).exists()))?,
        )?;
        let (r3, rf) = (root.clone(), run_file.clone());
        g.raw_set(
            "reloadLuaFile",
            lua.create_function(move |lua, p: String| {
                let src = std::fs::read(r3.join(&p)).map_err(mlua::Error::external)?;
                lua.load(&src[..]).set_name(p).exec()?;
                let _ = &rf;
                Ok(())
            })?,
        )?;
        let mut ds: Vec<String> = std::fs::read_dir(root.join("media/maps"))
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().join("map.info").exists())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        ds.sort_by_key(|d| d.to_lowercase());
        let dl = lua.create_table()?;
        for (i, d) in ds.iter().enumerate() {
            dl.raw_set(i + 1, d.as_str())?;
        }
        g.raw_set("__lotdirs", dl)?;
        let r = lua.load(r#"
          getLotDirectories = function() local t = __lotdirs; return { size = function() return #t end, get = function(_, i) return t[i + 1] end } end
          DebugType = DebugType and type(DebugType.Lua) == "table" and DebugType or {}
          DebugType.Lua = { error = function(_, m) __warn(m) end, log = function() end, warn = function(_, m) __warn(m) end }
          if doMapZones then local ok, e = pcall(doMapZones); if not ok then __warn("doMapZones failed: " .. tostring(e)) end end
        "#).exec();
        if let Err(e) = r {
            println!("doMapZones glue error: {e}");
        }
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for w in &st.borrow().warnings {
        *counts.entry(w.clone()).or_default() += 1;
    }
    println!("-- lua warnings (harness) --");
    for (w, n) in counts {
        println!("{n:7} {w}");
    }
    Ok(())
}
