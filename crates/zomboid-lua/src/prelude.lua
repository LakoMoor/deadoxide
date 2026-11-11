-- Permissive stub for Java-exposed API: indexable, callable, arithmetic-safe. Records every first-touch global.
MISSING = {}
local stub
local function mk(name)
  return setmetatable({__name = name}, stub)
end
stub = {
  __index = function(t, k) local v = mk((rawget(t, "__name") or "?") .. "." .. tostring(k)); rawset(t, k, v); return v end,
  __call = function(t, ...) return mk((rawget(t, "__name") or "?") .. "()") end,
  __add = function() return 0 end, __sub = function() return 0 end, __mul = function() return 0 end,
  __div = function() return 0 end, __mod = function() return 0 end, __unm = function() return 0 end,
  __concat = function(a, b) return tostring(a) .. tostring(b) end,
  __lt = function() return false end, __le = function() return false end, __eq = function() return false end,
  __len = function() return 0 end,
  __tostring = function(t) return rawget(t, "__name") or "stub" end,
}
setmetatable(_G, { __index = function(g, k)
  MISSING[k] = (MISSING[k] or 0) + 1
  local v = mk(tostring(k)); rawset(g, k, v); return v
end })

-- Bind the measured translator method onto the existing permissive Java stub;
-- unimplemented Translator members remain stubs.
Translator.getRecipeName = __getRecipeName

-- Globals the Java side provides (values observed in Lua usage; not yet diffed against the real client).
UI_BORDER_SPACING = 10
BUTTON_HGT = 25
CGlobalObjects = { getSystemCount = function() return 0 end, getSystemByIndex = function() return nil end }
-- Text metrics: placeholder constants (real values come from the font renderer, not ported yet).
do
  local tm = { MeasureStringX = function() return 100 end, MeasureStringY = function() return 18 end,
               getFontHeight = function() return 18 end, MeasureFont = function() return 18 end }
  local orig = rawget(_G, "getTextManager")
  tm.__index = function(t, k) local v = tm[k]; if v ~= nil then return v end; local s = setmetatable({ __name = "TextManager." .. tostring(k) }, stub); rawset(t, k, s); return s end
  getTextManager = function() return setmetatable({}, tm) end
end
-- Kahlua exposes some java.lang.String methods on Lua strings.
string.contains = function(s, sub) return string.find(s, sub, 1, true) ~= nil end
string.isEmpty = function(s) return #s == 0 end
string.trim = function(s) return (string.gsub(s, "^%s*(.-)%s*$", "%1")) end
string.equals = function(a, b) return a == b end
string.startsWith = function(s, p) return string.sub(s, 1, #p) == p end
string.endsWith = function(s, p) return p == "" or string.sub(s, -#p) == p end
-- ServerOptions: option count is data-driven in the real game (not ported); empty list keeps loops well-formed.
-- ServerOptions: the option list is data-driven in the real game (not ported). Option count is 0, every named lookup yields a permissive stub.
ServerOptions = { new = function()
  local o = { getNumOptions = function() return 0 end, getOptionByIndex = function() return nil end }
  return setmetatable(o, { __index = function(t, k) local s = setmetatable({ __name = "ServerOptions." .. tostring(k) }, stub); rawset(t, k, s); return s end })
end }
-- namedColorToTable: the real client logs "namedColorToTable color not found: <name>" for unknown names (ProgressYellow is one in 42.21).
-- Known set below is a SUBSET (names seen in media/lua); the full Java colour table is not ported.
do
  local known = { CornFlowerBlue = {r=0.39,g=0.58,b=0.93,a=1}, White = {r=1,g=1,b=1,a=1}, Black = {r=0,g=0,b=0,a=1},
                  Red = {r=1,g=0,b=0,a=1}, Green = {r=0,g=1,b=0,a=1}, Blue = {r=0,g=0,b=1,a=1} }
  WARNINGS = WARNINGS or {}
  namedColorToTable = function(name)
    local c = known[name]
    if c then return { r = c.r, g = c.g, b = c.b, a = c.a } end
    WARNINGS[#WARNINGS + 1] = "namedColorToTable color not found: " .. tostring(name)
    return { r = 1, g = 1, b = 1, a = 1 }
  end
end
-- Kahlua table extensions (not in stock Lua 5.1).
table.wipe = function(t) for k in pairs(t) do t[k] = nil end return t end
table.isempty = function(t) return next(t) == nil end
table.newarray = function(...) return { ... } end
