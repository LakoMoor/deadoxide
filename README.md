<p align="center"><img src="assets/icon-256.png" width="128" alt="DeadOxide"></p>

# DeadOxide

An unofficial compatible runtime for **Project Zomboid** in Rust, modelled on [Horizon](https://github.com/horizon-rust/horizon) for Minecraft.
It reads the files of your **own** copy of the game (Steam build 25485521, version 42.21.0). No game files are in this repository.

**Progress, comparison with the original and mod catalog:** https://deadoxide-rust.github.io/deadoxide/

Roughly 20% is ported (a weighted estimate; some areas are author estimates, and the site and `tools/gen_progress.mjs` show how each number is computed).
It is not playable yet: no zombies, inventory, sound or networking. What exists: readers for the game's formats, running the game's Lua, and a picture of the world.

## Run

```sh
git clone https://github.com/deadoxide-rust/deadoxide    # place the folder inside your ProjectZomboid folder
cargo build --release
cargo run --release -p deadoxide-launcher           # launcher: play, mods, progress (GPUI)
```

The Lua harness defaults to English; set `ZOMBOID_LANG=RU` to load Russian translations over English fallback (other languages are supported experimentally). This does not yet change the client UI language.

Client without the launcher: `play.cmd`. The client can read a saved player position with `cargo run --release -p zomboid-client -- .. --save <save-folder>`; add `--shot screenshots/save.png` for an offscreen image. The player row is opened read-only through SQLite, including committed WAL data; SQLite may use transient sidecars. This reads the player position, floor and the supported save-time prefix; saved time advances in an experimental normal-speed clock when the sandbox day length is available; clock events are dispatched to explicitly loaded Lua handlers, but weather is not connected, and the rest of the saved world is not restored. To inspect time separately, use `cargo run --release -p zomboid-ctl -- save-time <save-folder>`. For a deterministic clock trace through the actual world step, use `cargo run --release -p zomboid-client -- .. --save <save-folder> --headless-ticks 100`; add `--trace-clock-events` to list the emitted signals. An experimental `--lua-script <path>` loads one script into the client event environment; original game APIs and automatic script loading are not yet available. Checks: `sh tools/check.sh`; on Windows with PowerShell 7 and the Windows SDK, use `pwsh -NoProfile -File tools/check.ps1` (add `-Quick` to skip Lua log parity). The Windows script uses a local fxc wrapper for the GPUI shader include issue and leaves the installed dependency untouched.

## Mods

The catalog is `registry/index.json` (url, sha256). The launcher downloads the archive, verifies the sha256 and extracts it into `profile/mods`.
To add a mod, open a pull request with a catalog entry; the archive must be yours or published with the author's permission.

## Layout

See [CLAUDE.md](CLAUDE.md), [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) and [docs/PORT.md](docs/PORT.md).

Not affiliated with The Indie Stone. Screenshots of the original game in `screenshots/` and on the site are included as evidence for the comparison; they will be removed at the rights holder's request.
