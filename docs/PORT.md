# Zomboid-rs — compatible Rust runtime for Project Zomboid (Horizon-style)

Model: https://github.com/horizon-rust/horizon (Minecraft Java reimplemented in Rust; reads the user's own install; ships no game assets; differential harnesses against the original).

## Mode / assumptions
- Compatible runtime, loads original content from the user's install (build 25485521, see `reference.lock.json`).
- Goal stated by user: full working game. This is a multi-year-scale scope; work proceeds in measured slices.
- Decompiling `projectzomboid.jar` (REA) is allowed as a hint only; no decompiled output is copied or committed.
- No GPU deps until a slice needs them (wgpu planned for the render slice).
- Toolchain: cargo 1.99.1 at `~/.cargo/bin` (not on PATH).

## Slices
- [x] S0 workspace + `zomboid-ctl check <root>` content inventory (4112 lotheader/lotpack pairs, 2 map sets).
- [x] S1 lotheader (names,width,height,minLevel,maxLevel) + lotpack chunk decode (roomId + tile ids per 8x8 square, skip runs). Verified structurally on all Muldraugh cells: 4,162,560 chunks, 0 length mismatches. Not yet decoded: rooms/buildings/zombie-intensity tail of lotheader; tile-id→name/texture mapping (S2).
- [~] S2 texture packs decoded (PZPK v0/v1; 24/24 packs, 65,236 subtextures, exact end-of-file). Map tile ids index lotheader tile_names (verified on 3 cells; max id = names-1). Muldraugh names: 26063 distinct, 82 unresolved vs packs (camping_03/04_*; cause unknown). TODO: *.tiles.txt tile properties loader, PNG decode, name→atlas rect lookup.
- [~] S3 wgpu client (crates/zomboid-client): multi-cell region loader (`zomboid-client <root> 40_38:42_40`), 1,121,953 instanced sprites for 3x3 cells (town of Muldraugh) in a layered 8192x8192 texture array, ~2.7 s load; window with pan/zoom/level cut; offscreen readback via --shot (verified by eye, seams between cells continuous). Window ran 12 s without crash earlier (single cell); region window not interactively verified. 82 tile names (camping_*, vegetation_*...) are defined in newtiledefinitions.tiles.txt but absent from all packs (88,432 instances in the 3x3 region); assumed blank slots, UNVERIFIED. TODO: streaming instead of fixed region, tiles.txt properties, game-screenshot diff.
- [~] S4 tile props parsed (33,568 defs). media/scripts: generic block parser (`zomboid-content::script`), 1004/1004 files parse with balanced braces; block counts agree with grep for craftRecipe (969) and vehicle (241); 5,105 `item` blocks (incl. nested ingredient forms), 4,090 model, 3,140 sound. Structure only, property semantics (items/recipes/vehicles) not interpreted. Lua harness `crates/zomboid-lua` (mlua Lua 5.1): compile 1394/1395 (`0.0f` Kahlua literal in ISFluidEmptyAction.lua:48); execution under a permissive Java-API stub shared 372/2, client 717/10, server 260/34 — stubs hide failures, NOT compatibility; 132 undefined globals touched. NEXT: ScriptManager over parsed scripts exposed to Lua, real Events/getText/sandbox options, rerun and diff vs a real-game Lua log (not yet captured).
- [~] S5 first sim step: window-free `sim.rs` (collision grid from tile props, axis-separated player step, 2 unit tests) + player marker (red block, not a real character) walking with WASD in screen directions, camera follows. Collision rules are SIMPLIFIED (doors/windows passable, no diagonal rule, no objects/furniture flags) and not diffed against the original. No zombies, no time/tick loop, no oracle dumps.
- [ ] S6 saves, vehicles (media/models_X), audio (FMOD banks), UI, multiplayer.

## S4 update — ScriptManager/Events in Lua
- `zomboid-lua` now installs a REAL `ScriptManager.instance` (5,105 item scripts from media/scripts; `getItem`, `getAllItems`, Java-style `getFoo()` -> script property `Foo`) and a real `Events` registry (Add/Remove/Trigger, `triggerEvent`) before the permissive global stub. Verified from Lua: `Base.Axe` getWeight=3, event dispatch accumulates handler results, getAllItems n=5105.
- Item `Type` returns nil for Base.Axe (property absent in B42 scripts; semantics of B42 item fields not yet mapped).
- Execution results with these in place: shared 370 ok/4 err, client 717/10, server 259/35. New real failures surfaced (e.g. `FindItem` missing on ScriptManager). Still stubbed: all other Java globals (129 touched), getText, sandbox options, world generation data.
- No real-game Lua log captured yet, so no differential check exists.
- `getText`/`getTextOrNull` over media/lua/shared/Translate/EN/*.json (47,260 strings; `%1..%4` substitution; missing key -> key, getTextOrNull -> nil). Semantics inferred from usage, not diffed with the real game. `ScriptManager:FindItem(name)` (full or `Base.` short name). Harness result now: shared 372 ok/2 err, client 717/10, server 260/34.
- `WorldGenUtils.INSTANCE` (getFiles/getFilesNum/getFile; recursive sorted listing under media/, ASSUMPTION about recursion/ordering) + `require` accepting `.lua` suffix: server WorldGen biome/feature files now load through Biomes.lua's own require chain. Harness: server 294 ok/0 err (some still rely on the global stub), client 717/10, shared 372/2. Remaining errors are stub artefacts (number-vs-stub in CGlobalObjectSystem:123, math.max on stubs, `contains` on a stub) plus the Kahlua-only `0.0f` literal.

## S5 update — fixed-step world
- `sim.rs`: 30 Hz fixed step with accumulator, explicit xorshift64* RNG, `World { tick, rng, player, zombies }`, `state_hash()` (FNV-1a over bit patterns). 4 unit tests: same seed => same hash, different seed differs, zombies stay inside walls.
- Zombies in the window (40, green blocks) are PLACEHOLDERS: chase within 12 tiles at 1.2 tiles/s, otherwise random wander at 0.5 tiles/s, player 4 tiles/s. None of these numbers come from the original game. No attacks, health, animation, sound, noise or sight model.
- Window smoke run: 10 s without crash (exit 124 from `timeout`), not visually inspected; no render interpolation.

## Oracle capture (started)
- Real game launched with an isolated profile: `java ... zombie.gameStates.MainScreenState -nosteam -cachedir=zomboid-rs/profile` (user's ~/Zomboid untouched). It runs and renders (window title "Project Zomboid", build 25485521, Russian UI locale), loads texture packs/fmod, reaches TISLogoState; screenshot `tests/out/oracle_screen.png` = legal-terms dialog of a fresh profile.
- BLOCKER: the terms/privacy dialog must be accepted by the user; not clicked by the agent. Until then no in-game screenshots or Lua console logs of the real client exist.
- Observed oracle facts so far (stderr): `AngelCodeFont failed to load page 0 texture media/mainfont_0.png` (x5, fonts); `FluidContainerScript.load Sanitizing container name 'Large Bucket' / 'Fuel Pump'`.
- User accepted the terms in the isolated profile. Log: `STATE: exit TermsOfServiceState`, intro videos (binkId 1, 2), WARN `Lua(Vanilla).namedColorToTable color not found: ProgressYellow` (real-game Lua warning, first oracle datum for the Lua harness). Game window not capturable without taking focus from the user (full-screen capture also shows the user's other windows; do not repeat it).

## Oracle capture — result (2026-11-07)
Real game driven through the UI (isolated profile `zomboid-rs/profile`, user accepted the terms and allowed window focus): Single player -> Sandbox -> Rosewood -> sandbox settings -> default character -> Play. Game closed afterwards (PID 2880 stopped).
Evidence in `tests/golden/spawn_rosewood/`:
- `meta.json`: player position read from the save's `players.db` (copy, via node:sqlite): x=8303.5 y=11689.5 z=0 = exactly spawnpoint `{8303, 11689, 0}` of Rosewood's spawnpoints.lua -> cell 32_45 of the Muldraugh map folder.
- `game_ingame.png` (real game, night, roof cut away) vs `port_z0.png` (port, level z=0, cmd in meta.json): same house, same living room — leather sofas, coffee table, floor lamp, bookshelf, window, poster/dartboard, rug under the player. This is a QUALITATIVE match of content and placement (it confirms: lotpack decoding, tile-name->sprite resolution, tile index->name, cell 32_45 coordinate mapping `x=cell*256+sx`, level index = z - minLevel with minLevel -2). It is NOT a pixel diff: zoom/scale, lighting (night), roof cutaway and the character sprite differ; port draws a red block as the player.
- Level range of 32_45 header: width 8, height 8, minLevel -2, maxLevel 3 (6 levels) -> port `--level` index 2 = z 0.
Other oracle data: sandbox defaults (day length 1 h 30 min, start month July, start day 9, start time 09:00), spawn regions listed: Brandenburg, Echo Creek, Ekron, Fallas Lake, Irvington, March Ridge, Riverside, Rosewood, Valley Station (no Muldraugh/West Point in the list), real-game Lua warning `env.lua namedColorToTable color not found: ProgressYellow`, `ISInventoryTransferUtil require("ISUI/PlayerData/ISPlayerData") failed` (WARN from the real client).
Still missing: numeric screenshot diff, real-game Lua console log for the full client load, timing/zombie behaviour traces.
