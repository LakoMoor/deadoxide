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
                // Item.getActualWeight stores Java float precision; absent Weight defaults to 1.
                let weight = it
                    .props
                    .iter()
                    .rev()
                    .find(|(key, _)| key.eq_ignore_ascii_case("Weight"))
                    .and_then(|(_, value)| value.parse::<f32>().ok())
                    .unwrap_or(1.0);
                t.set("actualWeight", weight as f64)?;
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
-- These defaults and three built-in types were measured on original script items.
function ItemMT.getActualWeight(s) return s.actualWeight end
function ItemMT.getObsolete(s) return string.lower(s.props.Obsolete or s.props.obsolete or "false") == "true" end
function ItemMT.isHidden(s) return string.lower(s.props.hidden or s.props.Hidden or "false") == "true" end
function ItemMT.getSkillTrained(s) return s.props.SkillTrained or "" end
function ItemMT.getRecordedMediaCat(s) return s.props.RecordedMediaCat end
function ItemMT.isItemType(s, itemType) return itemType ~= nil and s.props.ItemType == itemType end
rawset(_G, "ItemType", {
  ALARM_CLOCK="base:alarmclock", ALARM_CLOCK_CLOTHING="base:alarmclockclothing",
  ANIMAL="base:animal", CLOTHING="base:clothing", CONTAINER="base:container",
  DRAINABLE="base:drainable", FOOD="base:food", KEY="base:key", KEY_RING="base:keyring",
  LITERATURE="base:literature", MAP="base:map", MOVEABLE="base:moveable", NORMAL="base:normal",
  RADIO="base:radio", WEAPON="base:weapon", WEAPON_PART="base:weaponpart"
})
for _, it in pairs(__ScriptItems) do setmetatable(it, ItemMT) end
local SM = {}
function SM:getItem(full) return __ScriptItems[full] end
function SM:FindItem(name) return __ScriptItems[name] or __ScriptItems["Base." .. tostring(name)] end
-- ScriptManager.getAllItems returns a Java ArrayList. Retain the Lua array
-- representation for existing harness users but expose the measured list API.
local ItemList = {}
function ItemList:size() return #self end
function ItemList:isEmpty() return #self == 0 end
function ItemList:get(index)
  if type(index) ~= "number" or index ~= math.floor(index) or index < 0 or index >= #self then
    error("IndexOutOfBoundsException", 2)
  end
  return self[index + 1]
end
function ItemList:contains(item)
  for i = 1, #self do if self[i] == item then return true end end
  return false
end
-- Item.getLearnedRecipes returns a Java list, or nil when the property is absent.
function ItemMT.getLearnedRecipes(s)
  local value = s.props.LearnedRecipes
  if not value then return nil end
  if not s.learnedRecipes then
    local recipes = {}
    for recipe in string.gmatch(value, "[^;]+") do recipes[#recipes + 1] = recipe end
    s.learnedRecipes = setmetatable(recipes, { __index = ItemList })
  end
  return s.learnedRecipes
end
function SM:getAllItems()
  local t = {}; for i, f in ipairs(__ScriptItemOrder) do t[i] = __ScriptItems[f] end
  return setmetatable(t, { __index = ItemList })
end
rawset(_G, "ScriptManager", { instance = SM, getInstance = function() return SM end })
rawset(_G, "getScriptManager", function() return SM end)
rawset(_G, "getAllItems", function() return SM:getAllItems() end)
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

#[cfg(test)]
mod tests {
    use super::install;
    use mlua::Lua;
    use std::path::Path;

    #[test]
    fn real_learned_recipes_match_original_samples() -> mlua::Result<()> {
        let lua = Lua::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        install(&lua, &root)?;
        let inspect: mlua::Function = lua
            .load(
                r#"
            return function(name)
                local recipes = getScriptManager():getItem(name):getLearnedRecipes()
                if recipes == nil then return nil end
                local result = {}
                -- This is the loop used by ISLiteratureUI:setLists.
                for j = 1, recipes:size() do result[j] = recipes:get(j - 1) end
                assert(not pcall(function() recipes:get(-1) end))
                assert(not pcall(function() recipes:get(recipes:size()) end))
                return table.concat(result, "|")
            end
        "#,
            )
            .eval()?;
        let mut checked = 0;
        for line in include_str!("../../../tests/golden/item_recipes_partial.txt")
            .lines()
            .skip(1)
        {
            let (key, expected) = line
                .strip_prefix("PROBE ")
                .expect("oracle prefix")
                .split_once('=')
                .expect("oracle entry");
            let actual: Option<String> = inspect.call(key)?;
            assert_eq!(
                actual.as_deref(),
                if expected == "null" {
                    None
                } else {
                    Some(expected)
                },
                "{key}"
            );
            checked += 1;
        }
        assert_eq!(checked, 4);
        Ok(())
    }

    #[test]
    fn real_item_properties_match_original_samples() -> mlua::Result<()> {
        let lua = Lua::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        install(&lua, &root)?;
        let inspect: mlua::Function = lua
            .load(
                r#"
            return function(name)
                local item = getScriptManager():getItem(name)
                return item:getFullName(), item:getActualWeight(), item:getObsolete(),
                    item:isHidden(), item:getSkillTrained(), item:getRecordedMediaCat(),
                    item:isItemType(ItemType.LITERATURE), item:isItemType(ItemType.WEAPON),
                    item:isItemType(ItemType.CLOTHING)
            end
        "#,
            )
            .eval()?;
        let item_types: mlua::Table = lua.globals().get("ItemType")?;
        for line in include_str!("../../../tests/golden/item_type_names.txt").lines() {
            let (name, value) = line.split_once('=').expect("type oracle");
            assert_eq!(item_types.get::<String>(name)?, value);
        }
        let oracle = include_str!("../../../tests/golden/item_properties_partial.txt");
        for line in oracle.lines().skip(2) {
            let (key, values) = line
                .strip_prefix("PROBE ")
                .expect("oracle prefix")
                .split_once('=')
                .expect("oracle entry");
            let fields: Vec<_> = values.split(',').collect();
            assert_eq!(fields.len(), 9);
            let actual: (
                String,
                f64,
                bool,
                bool,
                String,
                Option<String>,
                bool,
                bool,
                bool,
            ) = inspect.call(key)?;
            assert_eq!(actual.0, fields[0]);
            assert_eq!(actual.1, fields[1].parse::<f64>().expect("weight"));
            assert_eq!(actual.2.to_string(), fields[2]);
            assert_eq!(actual.3.to_string(), fields[3]);
            assert_eq!(actual.4, fields[4]);
            assert_eq!(
                actual.5.as_deref(),
                if fields[5] == "null" {
                    None
                } else {
                    Some(fields[5])
                }
            );
            assert_eq!(actual.6.to_string(), fields[6]);
            assert_eq!(actual.7.to_string(), fields[7]);
            assert_eq!(actual.8.to_string(), fields[8]);
        }
        Ok(())
    }

    #[test]
    fn real_item_list_supports_original_lua_iterator() -> mlua::Result<()> {
        let lua = Lua::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let count = install(&lua, &root)?;
        let original =
            std::fs::read_to_string(root.join("media/lua/shared/Foraging/forageSystem.lua"))
                .map_err(mlua::Error::external)?;
        let start = original
            .find("local function iterList(_list)")
            .expect("iterator start");
        let end = original[start..]
            .find("local function isInRect")
            .expect("iterator end")
            + start;
        let chunk = format!(
            "{}\n{}",
            &original[start..end],
            r#"
            local manager = getScriptManager()
            local list = manager:getAllItems()
            assert(manager == ScriptManager.instance)
            assert(list:size() == getAllItems():size())
            assert(not list:isEmpty())
            assert(list:contains(manager:getItem("Base.Axe")))
            assert(not pcall(function() list:get(-1) end))
            assert(not pcall(function() list:get(list:size()) end))
            local n, found, visibleHidden = 0, false, false
            for item, i in iterList(list) do
                assert(item == list:get(i))
                n = n + 1
                -- The same filter used by forageSystem.doItemDefCheck.
                if not item:getObsolete() and not item:isHidden() then
                    found = found or item:getFullName() == "Base.Axe"
                    visibleHidden = visibleHidden or item:getFullName() == "Base.F_Hair_Stubble"
                end
            end
            assert(not visibleHidden)
            return n, found
            "#
        );
        let (iterated, found): (usize, bool) = lua.load(&chunk).eval()?;
        assert_eq!(iterated, count);
        assert!(found);
        Ok(())
    }
}
