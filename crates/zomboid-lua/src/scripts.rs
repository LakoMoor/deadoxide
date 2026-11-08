//! Exposes parsed media/scripts to Lua as `ScriptManager.instance` (items only for now) and a real `Events` registry.
use mlua::{Lua, Result};
use std::path::{Path, PathBuf};
use zomboid_content::script::{parse_script, Block};

fn walk(d: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out)
            } else if p.extension().and_then(|x| x.to_str()) == Some("txt") {
                out.push(p)
            }
        }
    }
}

/// Returns number of item scripts registered.
pub fn install(lua: &Lua, root: &Path) -> Result<usize> {
    let mut files = vec![];
    walk(&root.join("media/scripts"), &mut files);
    files.sort();
    let items = lua.create_table()?;
    let order = lua.create_table()?;
    let mut n = 0usize;
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let Ok(blocks) = parse_script(&text) else {
            continue;
        };
        for m in blocks.iter().filter(|b| b.kind == "module") {
            for it in m.children.iter().filter(|b: &&Block| b.kind == "item") {
                let t = lua.create_table()?;
                let props = lua.create_table()?;
                for (k, v) in &it.props {
                    props.set(k.as_str(), v.as_str())?;
                }
                t.set("props", props)?;
                t.set("module", m.name.as_str())?;
                t.set("name", it.name.as_str())?;
                let full = format!("{}.{}", m.name, it.name);
                t.set("fullType", full.as_str())?;
                items.set(full.as_str(), t)?;
                n += 1;
                order.set(n, full)?;
            }
        }
    }
    // Java WorldGenUtils: file listing used by media/lua/server/WorldGen/*.lua (recursive, sorted; assumption)
    let media = root.join("media");
    lua.globals().set(
        "__listdir",
        lua.create_function(move |_, d: String| {
            let mut v = vec![];
            fn w(p: &Path, o: &mut Vec<PathBuf>) {
                if let Ok(rd) = std::fs::read_dir(p) {
                    for e in rd.flatten() {
                        let q = e.path();
                        if q.is_dir() {
                            w(&q, o)
                        } else if q.extension().and_then(|x| x.to_str()) == Some("lua") {
                            o.push(q)
                        }
                    }
                }
            }
            w(&media.join(&d), &mut v);
            v.sort();
            Ok(v.iter()
                .map(|p| {
                    format!(
                        "media/{}",
                        p.strip_prefix(&media)
                            .unwrap()
                            .to_string_lossy()
                            .replace(std::path::MAIN_SEPARATOR, "/")
                    )
                })
                .collect::<Vec<_>>())
        })?,
    )?;
    lua.globals().set("__ScriptItems", items)?;
    lua.globals().set("__ScriptItemOrder", order)?;
    lua.load(r#"
-- Item script objects: Java-style getters resolve to script properties (getWeight -> "Weight").
local ItemMT = {}
ItemMT.__index = function(self, k)
  local v = rawget(ItemMT, k); if v then return v end
  local prop = type(k) == "string" and k:match("^get(%u.*)$")
  if prop then return function(s) local p = rawget(s, "props")[prop]; return tonumber(p) or p end end
  return nil
end
function ItemMT.getFullName(s) return s.fullType end
function ItemMT.getModuleName(s) return s.module end
function ItemMT.getName(s) return s.name end
function ItemMT.getTypeString(s) return s.props.Type end
for _, it in pairs(__ScriptItems) do setmetatable(it, ItemMT) end
local SM = {}
function SM:getItem(full) return __ScriptItems[full] end
function SM:FindItem(name) return __ScriptItems[name] or __ScriptItems["Base." .. tostring(name)] end
function SM:getAllItems() local t = {}; for i, f in ipairs(__ScriptItemOrder) do t[i] = __ScriptItems[f] end; return t end
ScriptManager = rawset and { instance = SM }
rawset(_G, "ScriptManager", { instance = SM, getInstance = function() return SM end })
-- Real event registry (LuaEventManager semantics: Add/Remove/Trigger by event name).
local ev = {}
local function event(name)
  local e = rawget(ev, name); if e then return e end
  local handlers = {}
  e = { Add = function(f) handlers[#handlers + 1] = f end,
        Remove = function(f) for i, h in ipairs(handlers) do if h == f then table.remove(handlers, i) break end end end,
        Trigger = function(...) for _, h in ipairs(handlers) do h(...) end end,
        count = function() return #handlers end }
  ev[name] = e; return e
end
rawset(_G, "Events", setmetatable({}, { __index = function(_, k) return event(k) end }))
rawset(_G, "WorldGenUtils", { INSTANCE = { _files = {},
  getFiles = function(self, d) self._files = __listdir(d); return self._files end,
  getFilesNum = function(self) return #self._files end,
  getFile = function(self, i) return self._files[i + 1] end,
  generateSeed = function() return "0" end } })
rawset(_G, "triggerEvent", function(name, ...) event(name).Trigger(...) end)
rawset(_G, "LuaEventManager", { AddEvent = function(name) event(name) end, triggerEvent = function(name, ...) event(name).Trigger(...) end })
"#).exec()?;
    Ok(n)
}
