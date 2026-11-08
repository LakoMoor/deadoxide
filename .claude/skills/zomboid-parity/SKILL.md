---
name: zomboid-parity
description: Fix a gap in the Zomboid Rust port so it behaves like the real Project Zomboid 42.21.0, starting from the compatibility ledger (docs/COMPATIBILITY.md) and the oracle tools instead of guessing. Use for any "X doesn't look/work like in the real game" report, a Lua warning that differs from the real client log, a render that does not match a real screenshot, or a Java API the Lua scripts call that is still stubbed.
---

# Zomboid parity (adapted from Horizon's `vanilla-parity`)

Horizon asks a *port ledger* first and greps second. Here the ledger is `docs/COMPATIBILITY.md` (what matches,
what is measured, what is UNKNOWN) plus the executable parity checks. The real game is the specification; the
decompiled `projectzomboid.jar` (REA) is a **hint** about behaviour, never something to copy.

## 0. Read the ledger and run the checks

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -q --release                         # unit tests (client sim, content parsers, zones vs the Lua count)
sh tests/lua_require_parity.sh                  # harness Lua warnings vs the REAL client log (tests/golden/lua_real)
cargo run -q --release -p zomboid-lua -- .. run # shared/client/server Lua execution counts + errors
cargo run -q --release -p zomboid-ctl -- resolve ..   # unresolved tile names
```

Then open `docs/COMPATIBILITY.md` for the area you touch. A row that says UNKNOWN or "not verified" is a lead.

## 1. From the report to a measurement

| Report says | Oracle | Tool |
|---|---|---|
| a Lua warning/error differs | real client log `profile/stdout.txt` / `tests/golden/lua_real/client_lua_warnings.txt` | `tests/lua_require_parity.sh`, `zomboid-lua run` |
| picture differs | a real screenshot in `screenshots/` | `zomboid-ctl imgdiff <game.png> <port.png> [x0 y0 x1 y1]`, `tools/region_luma.mjs` |
| a map/zone/tile fact | the real file + the real Lua (`loadfile`) | `zomboid-ctl zones`, `zomboid-content::zones` test vs `#objects` |
| a behaviour number (speed, range, damage) | the game running (see `zomboid-oracle`) | sample it; do not invent it |

If there is no oracle for the area, **make one first** (see the `zomboid-oracle` skill), then fix the port.

## 2. Port it the original way

- Translate the behaviour, do not reinvent it: same order of operations, constants and random draws where output depends on them.
- Open the fn doc with the Java class/method it follows: `/// zombie.iso.IsoGridSquare.getRoom: ...`. One line, so the next agent finds it.
- Lua scripts are the cheapest oracle: the game's own `media/lua` runs unchanged in `zomboid-lua`; supply only the **Java** functions
  they call (see `crates/zomboid-lua/src/prelude.lua` and `scripts.rs`). Prefer running the real Lua (as `doMapZones` does)
  over re-implementing what the Lua already does.
- Keep stubs honest: a stub returns a placeholder and its comment says what is NOT ported.
- Determinism: fixed 30 Hz step, explicit seeded RNG (`sim::Rng`), no wall clock in the core.

## 3. Before you report

- Re-run section 0. Record the new numbers in `docs/COMPATIBILITY.md` (measured / UNKNOWN / wrong-and-corrected). If an earlier note
  turns out wrong, correct it in place and say so; do not leave both.
- Never state "matches the original" without an oracle diff you ran. Say which sample (zoom, floor, time of day, position) it covers.
- Screenshots go to `screenshots/`.

## Finding work

```sh
grep -n "UNKNOWN\|NOT \|not verified" docs/COMPATIBILITY.md   # open leads
cargo run -q --release -p zomboid-lua -- .. run | sed -n '/undefined globals/,$p'   # Java API the Lua touches that is still stubbed
```
