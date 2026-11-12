-- zombie.Lua.Event.trigger/Add/Remove (42.21.0, CFR + original probes).
-- Partial LuaEventManager: selected mutation and error behavior is checked
-- against the original. Event names and callback type validation remain incomplete.
local registry = {}
local function event(name)
    local existing = rawget(registry, name)
    if existing then return existing end
    local handlers = {}
    local e = {}
    e.Add = function(fn) handlers[#handlers + 1] = fn end
    e.Remove = function(fn)
        for i, callback in ipairs(handlers) do
            if callback == fn then table.remove(handlers, i); break end
        end
    end
    e.Trigger = function(...)
        local i = 1
        while i <= #handlers do
            local callback = handlers[i]
            local ok, err = pcall(callback, ...)
            if not ok then __recordEventError(name, tostring(err)) end
            -- Original continues at the same slot when the invoked callback
            -- was removed entirely. Duplicate registrations remain distinct.
            local present = false
            for _, value in ipairs(handlers) do
                if value == callback then present = true; break end
            end
            if present then i = i + 1 end
        end
    end
    e.count = function() return #handlers end
    registry[name] = e
    return e
end
rawset(_G, "Events", setmetatable({}, { __index = function(_, name) return event(name) end }))
rawset(_G, "triggerEvent", function(name, ...) event(name).Trigger(...) end)
rawset(_G, "LuaEventManager", { AddEvent = function(name) event(name) end,
    triggerEvent = function(name, ...) event(name).Trigger(...) end })
