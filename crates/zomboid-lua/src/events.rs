//! Lua event dispatch shared by the client and the standalone harness.
use mlua::{Function, Lua, Result};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventError {
    pub event: String,
    pub message: String,
}

type ErrorBuffer = Rc<RefCell<Vec<EventError>>>;

/// Install the measured subset of zombie.Lua.LuaEventManager behavior.
pub fn install(lua: &Lua) -> Result<ErrorBuffer> {
    let errors = Rc::new(RefCell::new(Vec::new()));
    let sink = errors.clone();
    lua.globals().raw_set(
        "__recordEventError",
        lua.create_function(move |_, (event, message): (String, String)| {
            sink.borrow_mut().push(EventError { event, message });
            Ok(())
        })?,
    )?;
    lua.load(include_str!("events.lua"))
        .set_name("events.lua")
        .exec()?;
    Ok(errors)
}

/// A single-threaded Lua event environment. Game APIs and general script loading
/// are not implemented here; callers explicitly choose which scripts to load.
pub struct EventRuntime {
    lua: Lua,
    errors: ErrorBuffer,
}

impl EventRuntime {
    pub fn new() -> Result<Self> {
        let lua = Lua::new();
        let errors = install(&lua)?;
        Ok(Self { lua, errors })
    }

    pub fn run_source(&self, source: &str, name: &str) -> Result<()> {
        self.lua.load(source).set_name(name).exec()
    }

    pub fn load_file(&self, path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(path).map_err(mlua::Error::external)?;
        self.run_source(&source, &path.to_string_lossy())
    }

    pub fn eval_string(&self, source: &str) -> Result<String> {
        self.lua.load(source).eval()
    }

    pub fn dispatch(&self, name: &str) -> Result<Vec<EventError>> {
        let trigger: Function = self.lua.globals().raw_get("triggerEvent")?;
        trigger.call::<()>(name)?;
        Ok(std::mem::take(&mut *self.errors.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn event_mutations_and_errors_match_original() -> Result<()> {
        let expected: HashMap<_, _> = include_str!("../../../tests/golden/lua_event_api.txt")
            .lines()
            .map(|line| {
                line.strip_prefix("EVENT ")
                    .unwrap()
                    .split_once('=')
                    .unwrap()
            })
            .collect();
        let cases = [
            ("duplicate", "local function a() trace=trace..'A' end; local function b() trace=trace..'B' end; E.Add(a); E.Add(a); E.Add(b)", 1),
            ("remove", "local function a() trace=trace..'A' end; local function b() trace=trace..'B' end; E.Add(a); E.Add(b); E.Remove(a)", 1),
            ("remove_next", "local b; local function a() trace=trace..'A'; E.Remove(b) end; b=function() trace=trace..'B' end; local function c() trace=trace..'C' end; E.Add(a); E.Add(b); E.Add(c)", 1),
            ("add_during", "local function c() trace=trace..'C' end; local function a() trace=trace..'A'; E.Add(c) end; local function b() trace=trace..'B' end; E.Add(a); E.Add(b)", 2),
            ("remove_self", "local a; a=function() trace=trace..'A'; E.Remove(a) end; local function b() trace=trace..'B' end; E.Add(a); E.Add(b)", 2),
            ("error", "local function a() trace=trace..'A'; error('probe error') end; local function b() trace=trace..'B' end; E.Add(a); E.Add(b)", 1),
        ];
        for (name, setup, rounds) in cases {
            let runtime = EventRuntime::new()?;
            runtime.run_source(&format!("trace=''; local E=Events.Test; {setup}"), name)?;
            for round in 1..=rounds {
                let errors = runtime.dispatch("Test")?;
                assert_eq!(errors.len(), usize::from(name == "error"));
                let actual: String = runtime.lua.globals().get("trace")?;
                assert_eq!(actual, expected[format!("{name}.{round}").as_str()]);
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires the installed original game Lua files"]
    fn real_fishing_lua_daily_callback_matches_original_with_test_double() -> Result<()> {
        let runtime = EventRuntime::new()?;
        runtime.run_source("Vector2={new=function() return {} end}; updates=0; local manager={}; function manager:updateSeed() updates=updates+1 end; FishSchoolManager={getInstance=function() return manager end}", "test-double")?;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../media/lua/shared/Fishing/FishingUtils.lua");
        runtime.load_file(&root)?;
        let expected = include_str!("../../../tests/golden/fishing_event.txt");
        for (index, event) in [
            None,
            Some("EveryHours"),
            Some("EveryDays"),
            Some("EveryDays"),
        ]
        .iter()
        .enumerate()
        {
            if let Some(event) = event {
                assert!(runtime.dispatch(event)?.is_empty());
            }
            let actual: f64 = runtime.lua.globals().get("updates")?;
            let recorded = expected
                .lines()
                .nth(index)
                .unwrap()
                .split_once('=')
                .unwrap()
                .1
                .parse::<f64>()
                .unwrap();
            assert_eq!(actual, recorded);
        }
        Ok(())
    }
}
