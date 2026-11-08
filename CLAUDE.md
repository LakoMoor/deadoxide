# Zomboid-rs: a map for agents

Project Zomboid 42.21.0 (Steam build 25485521) as a compatible Rust runtime over the player's **own** install in `..`
(same idea as Horizon for Minecraft). The real game is the specification. Reply to the user in Russian.
Status and every measured/UNKNOWN fact: [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md); plan: [docs/PORT.md](docs/PORT.md).

## Rules that bite

- **Never commit the game's files**: `media/`, `projectzomboid.jar`, decompiled sources (REA output), the `profile/` folder. The decompiled jar is a *hint*, never copied.
- **Isolate runs from real player data**: the game always starts with `-cachedir=<repo>/profile`. Ask before launching it (window focus); capture the client area only; kill only by exact PID. See the `zomboid-oracle` skill.
- **No claim without an oracle diff.** "Matches" needs a command you ran and the conditions it covers (zoom, floor, time of day, position). Write UNKNOWN when unmeasured; correct wrong notes in place.
- **Screenshots go to `screenshots/`.**
- No `unsafe`; no `unwrap()` on I/O; no wall clock or global RNG in the simulation core (30 Hz fixed step, `sim::Rng`).
- Placeholder behaviour (zombie speed/vision/wander) is labelled PLACEHOLDER in the code until measured.
- Open each ported fn doc with the Java class/method it follows (`/// zombie.iso.IsoGridSquare.getRoom: ...`).

## Skills (in `.claude/skills/`, adapted from Horizon)

| Skill | Use |
|---|---|
| `zomboid-parity` | fix a gap so the port behaves like the real game: ledger → measurement → port the original way → record |
| `zomboid-oracle` | launch/drive the real game in the isolated profile and capture screenshots, logs, player position |

## Crates

| Crate | Owns |
|---|---|
| `zomboid-content` | Real-file readers: `lot` (LOTH/LOTP), `pack` (PZPK), `atlas` (sprite atlas), `script` (script blocks), `tiledefs`, `zones` (objects.lua). Window-free. |
| `zomboid-client` | `scene.rs` (cells → instances, collision, zone/marker overlays), `sim.rs` (deterministic world, collision, RNG), `main.rs` + `shader.wgsl` (wgpu window and offscreen `--shot`). |
| `zomboid-lua` | Lua 5.1 harness over the game's own `media/lua`: PZ-style `require`, `prelude.lua` (Java stubs), `scripts.rs` (ScriptManager, Events), `text.rs` (translations). Runs the real `doMapZones`. |
| `zomboid-ctl` | Inspection/oracle CLI: `check lots hist packs resolve tiledefs scripts render zones imgdiff`. |

## Commands

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -q --release
sh tests/lua_require_parity.sh
cargo run -q --release -p zomboid-lua -- .. run
cargo run -q --release -p zomboid-client -- .. 32_45 --shot screenshots/p.png -216704 639720 1.0 1920 1050 --player 8303.5 11689.5 --level 2 [--zones TownZone] [--lit-room]
cargo run -q --release -p zomboid-ctl -- imgdiff .. screenshots/game.png screenshots/p.png 650 300 1280 600
```

## Known traps (found the hard way)

- Windows shell: Bash heredocs lose backslashes; use the Write tool for files. `cargo` may be missing from PATH: `$HOME/.cargo/bin`.
- `iso()` of a fractional position is already the tile centre (feet); sprites use the tile's top vertex.
- `.5`-style numbers occur in `objects.lua`; Kahlua accepts `0.0f` literals and Java String methods on Lua strings.
- Lua `require` lookups and load order are case-insensitive in the real client.

## Write tests with signal (from Horizon)

- Test observable behaviour, not words in docs, comments or source. Keep checks for machine-consumed contracts.
- Use independent expected values or an oracle (the real game's file, the real Lua, a real screenshot); never derive the answer from the code under test.
  Examples here: zones count vs `#objects` from the real Lua; player square inside `livingroom`; Lua warnings vs the real client log.
- Test outcomes, not only calls to stubs. Skip trivial constants, derives and getters.
- Never delete the parity checks (`tests/lua_require_parity.sh`, the real-file tests): they are the spec.

## Build and test fast

```sh
sh tools/check.sh            # fmt, clippy -D warnings, all tests, Lua parity vs the real client log (paste its result in your report)
sh tools/check.sh --quick    # without the Lua parity run
play.cmd                     # build + open the Rust client (window); `play.cmd shot a.png` = headless screenshot into screenshots\
```

## Attribution (from Horizon, adapted)

Commits end with a `Co-Authored-By:` line for the agent that wrote them; PR bodies end with the agent's own "Generated with" line (use whatever the
session's instructions give). Anything an agent writes on someone's behalf says so (`🤖 Agent:` prefix on comments, "Opened by <person>'s AI agent" in a PR body).
Horizon's issue-claim workflow (`/claim`) and release labels do not apply here: there is no public tracker yet.
