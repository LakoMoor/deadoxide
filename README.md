<p align="center"><img src="assets/icon-256.png" width="128" alt="DeadOxide"></p>

# DeadOxide

An unofficial compatible runtime for **Project Zomboid** in Rust, modelled on [Horizon](https://github.com/horizon-rust/horizon) for Minecraft.
It reads the files of your **own** copy of the game (Steam build 25485521, version 42.21.0). No game files are in this repository.

**Progress, comparison with the original and mod catalog:** https://lakomoor.github.io/deadoxide/

Roughly 20% is ported (a weighted estimate; some areas are author estimates, and the site and `tools/gen_progress.mjs` show how each number is computed).
It is not playable yet: no zombies, inventory, sound or networking. What exists: readers for the game's formats, running the game's Lua, and a picture of the world.

## Run

```sh
git clone https://github.com/LakoMoor/deadoxide    # place the folder inside your ProjectZomboid folder
cargo build --release
cargo run --release -p deadoxide-launcher           # launcher: play, mods, progress (GPUI)
```

Client without the launcher: `play.cmd`. Checks: `sh tools/check.sh`.

## Mods

The catalog is `registry/index.json` (url, sha256). The launcher downloads the archive, verifies the sha256 and extracts it into `profile/mods`.
To add a mod, open a pull request with a catalog entry; the archive must be yours or published with the author's permission.

## Layout

See [CLAUDE.md](CLAUDE.md), [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) and [docs/PORT.md](docs/PORT.md).

Not affiliated with The Indie Stone. Screenshots of the original game in `screenshots/` and on the site are included as evidence for the comparison; they will be removed at the rights holder's request.
