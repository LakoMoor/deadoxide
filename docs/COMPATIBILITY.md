| Behavior | Original evidence | Rust location | Impl | Wired | Real-content verified | Differential | Next |
|---|---|---|---|---|---|---|---|
| Content inventory / lotheader-lotpack pairing | file walk of media/maps | zomboid-content::inventory | yes | zomboid-ctl check | yes (Muldraugh 4065/4065, challengemaps 47/47) | n/a | S1 decode headers |
| LOTP index / LOTH tile names | hexdump of 0_18.lotheader, world_0_18.lotpack | zomboid-content::lot | yes | zomboid-ctl lots | yes, 8130/8130 files parse (structure only) | none | decode LOTH tail + chunk payload |
| LOTP chunk payload (z,x,y squares, skip runs) | IsoLot.load control flow (javap hint) + bytes | zomboid-content::lot::parse_chunk | yes | zomboid-ctl lots | yes: 4162560/4162560 chunks consume exactly to next offset | none (no oracle dump yet) | S2 tile defs; oracle dump of a chunk from real game |
| Chunk tile ids = index into lotheader tile_names | min/max tile id vs name count on cells 0_18/0_20/30_20 | zomboid-ctl hist | observed | n/a | 3 cells only | none | check all cells |
| PZPK texture packs v0/v1 | TexturePackDevice.initMetaData/readPage control flow (javap hint) + bytes | zomboid-content::pack | yes | zomboid-ctl packs | yes: 24/24 parse to exact EOF, 65,236 subtextures (no PNG decode yet; no unit test yet) | none | PNG decode, atlas lookup |
| Tile name → pack texture resolution | resolve over Muldraugh headers | zomboid-ctl resolve | partial | – | 26063 used, 82 unresolved (camping_*) | none | find where those sprites come from |
| Isometric tile placement (128x64 2x tiles, orig/off rect) | visual check only | zomboid-ctl::render | yes | zomboid-ctl render | by eye: cell 41_39 window, 1982 sprites, 3 unresolved | none (no game screenshot diff) | screenshot comparison against real game
| wgpu sprite renderer, offscreen shot path | – | zomboid-client | yes | zomboid-client [--shot] | cell 41_39 by eye only | none | stream neighbouring cells; diff vs real game
| Multi-cell region + layered atlas | – | zomboid-client::scene::load_region | yes | zomboid-client | 3x3 cells by eye (tests/out/region.png) | none | verify missing sprites vs real game; streaming
| Tile properties (tiles.txt) | newtiledefinitions.tiles.txt etc. (text) | zomboid-content::tiledefs | yes (1 unit test) | zomboid-ctl tiledefs, client collision | 33,568 defs parsed on real files | none | check against binary .tiles; property semantics |
| Collision from solid/solidtrans/Wall*/collide* | tile property names only | zomboid-client::sim | yes (2 unit tests) | window WASD | player marker near Pizza Whirled by eye; walls not walk-tested vs real game | none | doors, windows, furniture, diagonal rules, oracle trace |
| Lua 5.1 compile of media/lua | stock Lua 5.1 compile | zomboid-lua compile | yes | cargo run -p zomboid-lua -- .. compile | 1394/1395 | none | Kahlua literal 0.0f |
| Lua execution with stubbed Java API | – | zomboid-lua run | harness only | – | 1349/1395 run w/o error under stubs (not meaningful) | none | real Events/ScriptManager/etc.
| Script file grammar (media/scripts) | file inspection + grep counts | zomboid-content::script | yes (2 tests) | zomboid-ctl scripts | 1004/1004 parse; counts match grep for craftRecipe/vehicle | none | property typing, item/recipe interpretation |
| ScriptManager items + Events registry (Lua) | media/scripts content only; Events semantics assumed (Add/Remove/Trigger) | zomboid-lua::scripts | yes | zomboid-lua run | spot check: Axe weight, event dispatch, 5105 items | none | method surface (FindItem...), recipes, B42 item fields; capture real Lua log
| getText / getTextOrNull (EN) | Translate/EN/*.json; fallback-to-key behaviour assumed | zomboid-lua::text | yes | zomboid-lua run | spot check: UI_mainscreen_survival=SURVIVAL, missing key | none | language selection, plural/format rules, other languages |
| WorldGenUtils file listing for Lua | usage in WorldGen/Biomes.lua + Features.lua only; recursion/order assumed | zomboid-lua::scripts | yes | zomboid-lua run | server WorldGen chain loads: 294/294 files execute | none | confirm against real game
| Fixed-step world, RNG, placeholder zombies | none (values invented) | zomboid-client::sim::World | yes (4 tests) | window | determinism only | none | measure real speeds/vision/spawn; oracle trace
| Cell/tile/level decoding end-to-end vs real game | real game screenshot + players.db position (Rosewood spawn 8303,11689) | zomboid-client::scene + content::lot/pack | yes | zomboid-client --shot | qualitative content+placement match of one room (tests/golden/spawn_rosewood) | no pixel diff | numeric diff with equal zoom; more locations

## Lua `require` oracle diff (zomboid-lua run vs tests/golden/lua_real/client_lua_warnings.txt)
Measured: harness executes shared 373/374, client 716/727, server 294/294 files.
- MATCH: 5x `recursive require(): server/WorldGen/WorldGen.lua`; all 10 `require("...") failed` lines of the real client (ISLootWindowControlHandler x2, pzapi/ui/ui, ISInventoryTransferAction, ISPlayerData, ISVehicleMenu, ISInventoryPaneContextMenu, ISContextMenu, CCampfireSystem, ISAnimalPickMateCursor).
- MISSING in harness: `recursive require(): server/WorldGen/attachments/Priorities.lua` (load order differs).
- EXTRA in harness (not in real log): 2x `require("DebugUIS/...") failed` (debug UI is not loaded by the real client), 1x recursive `server/WorldGen/Biomes.lua`.
- Not reproduced: namedColorToTable ProgressYellow, "can't find map objects file" x10, Mannequin zone error (need map-objects loader / color table).
- 11 client + 1 shared files still error from stubbed Java API (numeric `for`/`math.max` on stub values, Java `.contains`, `0.0f` literal is a Kahlua-only syntax).
This is a partial match, not full parity; load order is not yet confirmed identical.

## Screenshot numeric diff (zomboid-ctl imgdiff, tests/golden/spawn_rosewood)
`zomboid-ctl imgdiff .. game_ingame.png port_z0.png` (4x box downsample, luma): mean |diff| = 80.4/255 at zero shift;
the best shift found (76.8) sits on the search bound (dy=-120 px), i.e. the search is inconclusive and the images are NOT close.
Causes not yet separated: game HUD/lighting/zombies/day-night, 82 unresolved tile names, the known ~29 px vertical offset, scale.
Consequence: the earlier "qualitative match" only supports the cell/level/coordinate mapping, not visual fidelity.
Widened search (±320 px x, ±240 px y): best 68.2 at (-240,-320) px, still far from 0 and far from the claimed ~29 px offset,
so a plain luma diff does not align these frames. Needs masked comparison (exclude HUD, compare structure/edges) before it is a usable oracle.

## Unresolved tile names (82) breakdown (zomboid-ctl resolve)
camping_04 x35, vegetation_groundcover_01 x23, vegetation_trees_01 x11, lighting_outdoor_01_off x4, rooftop_furniture x2,
furniture_seating_indoor_03FIX x2, camping_03 x2, street_decoration_01 x1, jumbo_tree_01 x1, carpentry_02 x1.
The sheets themselves are present in the packs (camping_04_ strings occur in Tiles2x.pack); only these specific indices are absent from
every pack's index, so they are likely sparse/blank slots, but that is NOT verified against the real renderer (UNKNOWN: whether the game draws anything for them).

## Camera/offset result (measured, one sample)
Region-restricted imgdiff (room area x650..1280,y300..600) found best shift dx=0, dy=-88 px; re-rendering with cy 639808 -> 639720 gives
best shift (0,0) and mean|luma diff| 50.5 -> 35.6 (remaining diff = game lighting/fog-of-war/HUD/characters, not geometry).
Derivation: tile-centre feet iso y = (8303.5+11689.5)*32 = 639776; the real camera centre is 56 px above the feet (639720); horizontal exact.
Fixed in the port: follow camera now = feet - 56 px (CAM_OVER_FEET), and the player marker bottom is at iso() (it was 32 px too low).
Caveat: single sample at zoom 1, z=0; other zoom levels / floors / movement not verified. Golden: screenshots/port_z0_aligned.png.
The older "~29 px" figure was an eyeball estimate and was wrong.

## UPDATE: require-warning parity (supersedes the 16/18 note above) — tests/lua_require_parity.sh
All 11 `require` / `recursive require()` warning lines now match the real client log EXACTLY (diff empty). Two real rules found by the diff:
1. load order is case-insensitive path order (lowercase `attachments/` before `Biomes.lua`), which produces `recursive require(): .../attachments/Priorities.lua`;
2. module lookup is case-insensitive (`DebugUIS/...` resolves to `DebugUIs/...`).
Still not reproduced (non-require lines): ProgressYellow color, 10x "can't find map objects file", Mannequin zone error. Client Lua errors: 2/727 files.

## Non-load-time warnings (cannot be reproduced by executing Lua files alone)
- `namedColorToTable color not found: ProgressYellow` comes from CoopOptionsScreen.lua:225 inside a function, i.e. when that screen is opened. The harness now has `namedColorToTable` with the same warning text (values for known colours are a SUBSET/approximate); needs a scenario that opens the screen to reproduce.
- `can't find map objects file: media/maps/<map>/objects.lua` (10x) and the Mannequin zone error are written by Java map loading / metazoneHandler, not by Lua load. Needs the map loader port.

## UPDATE: map-objects warnings reproduced (tests/lua_require_parity.sh now covers 21 kinds, IDENTICAL as a SET of lines with counts; emission order is NOT compared: the harness prints sorted, and the golden file order may itself be an artefact of how it was produced)
Rule found: for every maps/ subfolder with map.info (a lot directory), in reverse alphabetical order, a missing objects.lua logs
`can't find map objects file: media/maps/<dir>/objects.lua`. Exactly the 10 maps without objects.lua (all but Muldraugh) match the 10 real log lines.
Implemented Rust-side in zomboid-lua main.rs as a stand-in for the Java loader (it is NOT the real loader; it only reproduces the log).
Remaining non-reproduced: Mannequin zone error (needs real objects.lua zone parsing for Muldraugh), ProgressYellow (needs screen scenario).

## UPDATE 2: map-objects + Mannequin warnings now come from the REAL Lua (tests/lua_require_parity.sh: 22 kinds IDENTICAL as a set)
The Rust-side shim described above was WRONG about the source and has been removed: `can't find map objects file` is emitted by the game's own
server/metazones/metazoneHandler.lua `doMapZones()`. The harness now runs that real function, supplying only the Java file-access functions
(getLotDirectories, fileExists, reloadLuaFile, DebugType.Lua:error) — no hand-written knowledge of which maps lack objects.lua.
The Mannequin error (`ERROR: Mannequin zone missing properties in media/maps/Muldraugh, KY/objects.lua coords: 13583, 1299, 0`) is produced by
real Lua parsing the real 40,831-line Muldraugh objects.lua. Needed Kahlua extensions: table.wipe / isempty / newarray (added to prelude.lua).
Remaining unreproduced client-log line: ProgressYellow (screen scenario needed). Order of emission still not compared.

## Zones (objects.lua) in Rust — zomboid-content::zones, `zomboid-ctl zones <root> [map]`
Constructor-only Lua table reader (no expression evaluation). Verified against the real Lua oracle: Muldraugh has 37,389 zones both in the harness
(`#objects` after running the real file) and in the Rust parser; the one Mannequin zone without `properties` has the same coords (13583,1299,0) as the real log.
Bug found by the real file, not by the synthetic test: numbers like `.5` (no leading zero).
NOT done: the simulation does not use zones yet; zone-type semantics (TownZone, Forest, ZombiesType, Vegitation, Nav, ...) are not ported.

## Zone overlay (zomboid-client --zones <Type>, screenshots/port_zones_*.png)
Debug overlay draws zone outlines (rect/polygon edges, red 5 px dots every tile) from objects.lua at the zone's z. It RUNS (TownZone 201 dots, Vegitation 823, Nav 137, Forest 0 near the Rosewood spawn)
but the screenshots at zoom 0.35 show only sparse tiny dots; whether zone shapes line up with the real map features is NOT verified (no oracle for zone placement yet).
Side result: with the aligned camera the player marker's feet land at y~580 vs ~585 in the real game screenshot (visual check, one sample).

## Lighting oracle (first measurements; real game 42.21.0, night, paused, same spawn) — screenshots/game_clean2.png vs port_z0_aligned.png
Mean luma (0..255) by region, game vs port: room interior 56.9 vs 71.6 (ratio 0.79); room left wall 55.2 vs 88.2; lit road (light cone) 131.9 vs 133.2 (ratio ~1.0);
unlit outdoors 0.0 vs 89-116. So: (a) squares not in the player's room and not reached by a light are fully black; (b) lit ground matches the unlit-port brightness;
(c) the room interior is ~0.8 of the port. The light cone on the road comes from the building light spilling through the door — real light propagation (Java LightingJNI) is NOT ported.
One sample only (one time of day, one position). Zone-debug oracle not captured (the game was run without -debug).
Correction: `namedColorToTable ... ProgressYellow` is logged while the MAIN MENU is built (observed at menu load in stdout.txt), not when CoopOptionsScreen is opened as written above.

## Visibility mask (`--lit-room`, Scene::retain_room) — NOT working yet
Implemented as "keep only the player's room + touching room-less squares". On the Rosewood spawn it does nothing: `Square.room_id` read from the lotpack
is >= 0 for 1,121 squares in the 3x3 region but for NONE within 12 tiles of the player on any floor (z -2..3), so the player's square has no room.
Hypothesis (unverified): rooms are defined by rectangles in the .lotheader, and the per-square id in the pack is something else. Needs a lotheader room parse + an oracle.

## Agents/skills adapted from Horizon (github.com/horizon-rust/horizon)
Horizon ships `.claude/skills/vanilla-parity` (ledger-first fixes), `CLAUDE.md`/`AGENTS.md` (map, rules) and an in-game agent-control API.
Adapted here: `.claude/skills/zomboid-parity`, `.claude/skills/zomboid-oracle`, `CLAUDE.md`, `AGENTS.md`, `tools/pzdrive.ps1`.
NOT adapted: Horizon's `tools/port_ledger.py` (a method-level ledger needs the decompiled jar; ours is docs/COMPATIBILITY.md) and the control API (PZ has none; input is Win32).

## UPDATE: rooms parsed from the .lotheader; visibility mask WORKS on the spawn sample
Header layout after the 4 dimension ints (bytes of real 32_45.lotheader, parsed to byte 120,687 of 123,391; the remaining ~2.7 KB is NOT decoded):
`u32 room_count`, then per room: `name\n`, `i32 level (= floor z, observed -2..2)`, `u32 rect_count`, rect_count x (x,y,w,h as i32, cell-local),
`u32 object_count`, object_count x (type,x,y as i32; type meaning UNKNOWN). (My first reading called the level a "tag" and the object count a "trailer" — wrong, corrected here.)
Per-square `room_id` in the lotpack is NOT the room (earlier hypothesis confirmed wrong for the purpose of visibility).
Check against the real game: cell-local (111,169) z=0 is inside `livingroom` — where the real character spawned. Tests: lot::room_tests (synthetic + real).
Visibility mask (`zomboid-client --lit-room`, Scene::retain_room): keep the player's room + touching room-less squares; the rest is black. Measured vs the real night screenshot
(screenshots/game_clean2.png, 1 sample, paused, player indoors): whole-frame mean|luma diff| 79.7 -> 18.1 /255 and the best shift is now exactly (0,0) (independent confirmation of the camera alignment).
Still different: the lit road cone (game 131.9 vs port 0), room brightness (56.9 vs 67.8), a strip of outer wall/bench and grass patches the game hides, the room's left neighbour. Day-time, other rooms, upper floors: not measured.

## Header tail after the rooms (32_45.lotheader, 2,704 bytes from byte 120,687) — first look, NOT decoded
It starts `57, 9, 1, 0, 2, 3, 6, 9, 5, 7, 8, 1, 4, 11, 10, 11, 14, 252, 253, 254, 255, 12, 257, 256, ...` as i32: a count (57?) followed by id-like numbers, some 252..257.
No strings seen in the first 200 bytes. Not building data as far as can be told; meaning UNKNOWN. Do not build on it.

## Horizon's Claude files, translated for Zomboid (complete list of what Horizon ships for Claude)
Horizon has exactly three: `CLAUDE.md`, `AGENTS.md` (identical text) and `.claude/skills/vanilla-parity/SKILL.md`.
Here: `CLAUDE.md` + `AGENTS.md` (rules, crate map, "write tests with signal", "build and test fast", attribution) and `.claude/skills/zomboid-parity` (the vanilla-parity workflow with the compatibility ledger instead of port_ledger.py);
plus `.claude/skills/zomboid-oracle` (driving the real game). Their `tools/pr_ready.py` became `tools/check.sh` (fmt, clippy -D warnings, tests, Lua parity vs the real client log): ALL CHECKS PASSED on this tree.
Not translated because they do not apply: issue claiming, release labels/batches, launcher-home env isolation (our isolation = `-cachedir=profile`).

## Client launcher
`play.cmd` (PowerShell: `.\play.cmd`): builds release and opens the window over the player's own install; `.\play.cmd 41_39` another cell, `.\play.cmd shot a.png` headless screenshot into screenshots\.
Verified: the window opens and responds, the 3x3... cell 32_45 loads in ~2.1 s, 40 PLACEHOLDER zombies at 30 Hz, no wgpu validation errors on stderr (12 s run, killed by PID).
Not a game launcher yet: it does not start or manage the real Project Zomboid, and there is no menu, save or settings UI in the Rust client.
Second oracle run (2026-11-08): the real game process exited by itself during the time-of-day capture attempt (GameThread exited in profile/stdout2.txt) — cause not investigated; no daytime sample was captured.
Oracle reproducibility: two separate real-game sessions of the same save give identical frames (mean|luma diff| 0.22/255, game_clean.png vs game_s2_a.png).

## Saves (first slice): the saved character position
`zomboid-content::save::load_player` reads `localPlayers` of a save's `players.db` (SQLite, read-only; rusqlite bundled): id, name, x, y, z, isDead.
`zomboid-client --save <save folder|players.db>` (or `.\play.cmd save`) reads it from a temp copy, picks the map cell from the position and puts the character there
(printed: `save: Тейлор Столлворт at (8303.5, 11689.5) z0`). Test (independent oracle): the saved square lies in a `livingroom` of the cell header at the saved z — matches the real screenshot.
NOT done: everything else in the save (inventory, health, skills, world modifications, `map/` chunk data, time of day, zombies); writing saves; loading any save other than the first local player; z > 0 starting levels untested.

## Save time-of-day: first look at `map_t.bin` (566 bytes) — UNRESOLVED, NOT used by the client
Big-endian. Bytes: magic `GMTM`, i32 249 (= worldversion in players.db), f32 1.0, i32 0, i32 750, f32 10.951, f32 10.952, i32 8, i32 6, i32 1993, ...
A reading that fits the layout: multiplier 1.0, 750 = ?, time of day 10.95 h, month index 8, day index 6, year 1993.
PROBLEM: the real-game screenshots of this exact save are NIGHT-dark outdoors (unlit squares fully black, a light cone on the road), which does not fit 10:57. So at least one field is misread
(or the screenshot is dark for another reason, e.g. the character spawned in a closed room with the outdoor sun hidden by the room visibility rules). No in-game clock oracle was captured. Do not build on these fields.

## Visibility mask parameter sweep (same single night sample; whole-frame mean|luma diff| vs screenshots/game_clean2.png)
Which room-less squares to keep next to the player's room: 8-neighbours 18.14; 4-neighbours 17.34 (CHOSEN); room + east/south neighbours 16.27; room squares only 16.18.
"Room squares only" scores best but drops the exterior walls that the real game still draws (right side, dark-red wall with window; checked visually, screenshots/exp_room_only.png),
so the luma metric alone is misleading here: it does not see missing walls. 4-neighbours is the compromise. Overfitting risk: one frame, one position, one room.

## Second oracle sample (2026-11-08): the character moved, with a held-out check
Real game, same save, pressed S (1.5 s, unpaused then paused), then Esc > "Выйти" so the game wrote players.db: new position (8305.825, 11691.658) z0 (screenshots/game_s3_moved_S.png).
1. Camera model validated at a second position WITHOUT refitting: with camera centre = feet - 56 px, the room corner and posters land on the same pixels as in the real frame (visual check, screenshots/port_s3.png).
   (The imgdiff best-shift search was misleading on this frame — the room is repetitive; the position was taken from the save, not from the search.)
2. Found the "wall cutaway" rule: the real game does not draw walls on the camera-facing sides (south/east) of the player's room. Rule in Scene::retain_room: keep a room-less square only when the player's room is on its +x or +y side.
   Whole-frame mean|luma diff| (0..255): frame 1 17.34 -> 16.27, frame 2 (held out, not used to choose the rule) 28.12 -> 27.09.
3. What still differs: lighting (the road light cone and the dark vignette), grass strips, the room's far-side outside details. Frame 2 residual 27.1 is dominated by lighting.
Still one scene, one room, night. The time-of-day question (map_t.bin) stays unresolved.

## Night ambient (`--lit-room` => x0.8) — measured, partial

Command: `node tools/ratio_map.mjs game.png port.png 80` prints game/port luma per 80 px cell where the port draws. Inside the player's room the ratio is 0.72-0.99 on both oracle frames (median about 0.8); toward the lower-left it drops to 0 (the real game keeps those squares black: a visibility rule not modelled) and outside the door it exceeds 1 (the light patch, not modelled).
Sweep of the flat multiplier (`--ambient k`, whole-frame mean|luma diff| of 255): k=1.0 16.27 / 27.09, k=0.9 15.54 / 26.41, k=0.8 15.16 / 26.09, k=0.7 15.20 / 26.31 (frame 1 / frame 2). `--lit-room` now implies 0.8.
Honest limits: ~1 point of 255 gained; the factor was picked from frame 1's median and the frame 2 sweep agrees, so frame 2 is no longer fully held out for lighting. Still UNKNOWN: how the real game computes per-square light (lamp, door cone, vision cone), time of day, daytime ambient. Reproduce frame 2: camera centre = iso(8305.8252, 11691.6582) - (0, 56), `--player 8305.8252 11691.6582`.

## The 82 unresolved tile names — checked, cause still UNKNOWN

`zomboid-ctl resolve ..` lists them: `vegetation_trees_01_*` (13), `vegetation_groundcover_01_*` (about 28), `lighting_outdoor_01_off_*`, `rooftop_furniture_56/57`, `street_decoration_01_17`.
Raw check (`grep -a` over every `media/texturepacks/*.pack`): `rooftop_furniture_55` occurs in Tiles2x.pack, `rooftop_furniture_56` and `_57` occur in no pack, so those names are really absent (not a reader bug). `vegetation_trees_01_0` occurs in no pack and only as a comment in `media/newtiledefinitions.tiles.txt`; the jumbo tree packs name their pages `JumboTrees2x<N>`. A name that `resolve` does resolve, `vegetation_groundcover_01_9`, also gives no raw grep hit, so raw grep is not a reliable test for names stored in another form; do not read this as proof for groundcover.
Hypothesis (unverified): trees and some groundcover are drawn by the real game through other sprite names (jumbo trees). Needs an oracle frame with a tree on screen.

## IsoZombie constants (class-file facts) — measured from the jar, behaviour UNKNOWN

`node tools/classdump.mjs ../projectzomboid.jar zombie/characters/IsoZombie [filter]` parses the class's constant pool and lists fields (with ConstantValue) and method names; no decompiler needed and no code copied.
Read: vision radius 10 (min) to 20 (max) tiles with penalties fog 7, rain 2.5, darkness 5; unseen hearing offset 2 / 5 (heavy rain) / 10; lunge time 180; eat body distance 1, time 3600; crawler damage range 1.5; speed types sprinter/fast shambler/shambler/random; hearing types pinpoint/normal/poor.
Now in `sim::iso_zombie_constants` (not used by the simulation). The sim's `chase within 12 tiles, speed 1.2` stays PLACEHOLDER: the real combination of these values and the movement speeds (which live in animation data, not in these constants) are unmeasured. Next measurement would be bytecode of `IsoZombie.updateInternal` or an oracle capture of a zombie crossing known tiles.

### `classdump --code` : what the zombie walk/lunge methods touch (bytecode order, NOT decompiled)

`node tools/classdump.mjs ../projectzomboid.jar zombie/characters/IsoZombie x --code getZombieWalkTowardSpeed` lists constants, field accesses and calls in order; branches and stack are not interpreted.
`getZombieWalkTowardSpeed(float,float,Vector2)` loads 1.3 twice, `getSpeedMod()`, 0.006 then `Vector2.setLength`, reads `SandboxOptions.lore.speed`, `inactive`, `speedType`, loads 0.08, calls `setLength` again, writes `running`, and ends with `getLength()`/`setLength`. `getZombieLungeSpeed()` reads the same sandbox speed and `speedType` and writes `running` (no float constants).
Facts: the numbers 0.006 and 0.08 are the only per-tick vector lengths the method uses. UNKNOWN: which one applies to which speed type, how 1.3 and `getSpeedMod()` enter, and the tick-to-tile scale. The sim keeps its PLACEHOLDER speed until this is read with branch structure or measured on a real zombie.

### `getZombieWalkTowardSpeed` read by hand from the bytecode (`--hex`, `--cp`) — medium confidence

Branch offsets resolved by hand against the JVMS opcode table (109 bytes, pc 0..108). With `a1` = float arg 1, `a2` = float arg 2, `v` = the Vector2 arg:
```
f  = clamp(a2 / 24.0, 1.0, 1.3)
v.setLength((a1 * this.getSpeedMod() + 0.006) * f)
if ((SandboxOptions.lore.speed.getValue() == 1 && !this.inactive) || this.speedType == 1) {   // sprinter
    v.setLength(0.08); this.running = true;
}
if (v.getLength() > a2) v.setLength(a2)
```
Resolved: constants 24, 1.0, 1.3, 0.006, 0.08 and the branch conditions (pc 62 `if_icmpne`, pc 69 `ifeq`, pc 77 `if_icmpne`). UNKNOWN: what `a1` and `a2` are at the call sites, what `getSpeedMod()` returns, the length-per-tick to tiles-per-second scale, and whether `inactive` means what its name suggests. Not applied to the sim: without those the numbers cannot be turned into a speed in tiles/s, so `sim.rs` keeps its PLACEHOLDER.

### CORRECTION: `getZombieWalkTowardSpeed` is probably dead code

`node tools/jargrep.mjs ../projectzomboid.jar getZombieWalkTowardSpeed` finds the name only in `zombie/characters/IsoZombie` itself, and no method of IsoZombie calls it. No class in the jar references it (a Lua call would need a binding, none found by name). So the formula read above is most likely unused in 42.21.0 and must NOT be taken as the zombie movement speed. By contrast `getZombieLungeSpeed` is referenced by `zombie/ai/states/LungeState`. Real walking speed is probably driven by the animation/walk type (`walkType`, `speedType`); that path is not read yet.
Tools: `tools/jargrep.mjs <jar> <string>` lists classes whose constant pool contains the string.

## Zombie walk animation sets (media/AnimSets/zombie/walktoward) — measured file facts

Plain-text per-variant animation name and `m_SpeedScale` (blank = not set in the file). The distance moved per animation cycle (root motion) is stored in the model animation files, NOT read yet, so tiles/second stays UNKNOWN.

| variant | m_AnimName | m_SpeedScale |
|---|---|---|
| defaultWalktoward |  | 0.80 |
| inTrees | Zombie_WalkTrees | 0.80 |
| slow |  | 0.80 |
| sprint1 | Zombie_Sprint |  |
| sprint2 | Zombie_Sprint02 |  |
| sprint3 | Zombie_Sprint03 |  |
| sprint4 | Zombie_Sprint05 |  |
| sprint5 | Zombie_Sprint |  |
| sprintToward |  | 0.80 |
| sprintWalk1 | Zombie_Walk | 0.92 |
| sprintWalk2 | Zombie_Walk2 | 1.04 |
| sprintWalk3 | Zombie_Walk3 |  |
| sprintWalk4 | Zombie_Walk4 | 0.60 |
| sprintWalk5 | Zombie_Walk3 | 0.92 |
| walktoward1 | Zombie_Walk | 0.92 |
| walktoward2 | Zombie_Walk2 | 1.04 |
| walktoward3 | Zombie_Walk3 |  |
| walktoward4 | Zombie_Walk4 | 0.60 |
| walktoward5 | Zombie_Walk3 | 0.92 |
| zombieWalkSlow1 | Zombie_Walk5 |  |
| zombieWalkSlow2 | Zombie_Walk2 | 0.64 |
| zombieWalkSlow3 | Zombie_Walk5 | 0.72 |

## FMOD banks (audio) — first look, nothing decoded

`media/sound/banks/Desktop/`: ZomboidSound.bank (647 MB), ZomboidMusic.bank (667 MB), ZomboidSoundMP.bank (40 MB), radio.bank (9 MB), ZomboidSound.strings.bank (171 KB).
The strings bank is a RIFF container, form type `FEV `, with two chunks: `FMT ` (8 bytes) and one `LIST` (171192 bytes) that holds typed sub-lists (`PROJBNKI`, `IBSSLIST`, `GBSSLIST`, `EVTSLIST`, `VCASLIST`, `PRMSLIST`, ...). Path-like names such as `/Zombie`, `/ZombieClaw`, `/Reverb`, `/Locks` are present. A naive search for `event:/` finds nothing, so the event names are stored per sub-list, not as one string. No event table or audio stream is decoded; the audio area stays 0%.

## Third oracle frame (new game run, 1710x1074 client area, night, paused) — a failure, recorded

Run: real game 42.21.0 in `profile/`, Continue, closed the guide/info/inventory/loot windows (click points at this window size: Continue (260,516), start (855,500), guide Close (1422,851), info X (308,108), inventory X (293,8), loot X (863,8)). Frame: `screenshots/game_run2_clean.png`. The character stands near the south door of the spawn room; a tree and a lit road are visible outside.
Port frame with the same saved position, camera centre iso(8305.8252, 11691.6582) - (0, 56), zoom 1.0, 1710x1074, `--lit-room`: `screenshots/port_run2.png`. Whole-frame mean|luma diff| = 27.69 of 255.
What matches: the room geometry, posters, windows, furniture and the character marker land on the same pixels as in the game (camera model holds at a third frame size).
What does NOT match, measured by eye on the two frames: (1) the port draws the opaque south-west wall with the door and a full floor row there, the game shows that side cut away/black (the `retain_room` rule was fitted on frames 1 and 2 and does not carry over to this position); (2) the port has no outside squares, so the tree, grass and the lit road (visible in the game through the door) are missing; (3) lighting falloff as before.
Consequence: the visibility rule is NOT validated beyond two positions. UNKNOWN: whether the real rule depends on the player's distance to the wall, line of sight, or the room's facing. Next measurement: capture two more positions in the same room to separate those hypotheses.

## Run 3 (new real-game launch): determinism confirmed, second visibility failure

- Determinism: the start frame of run 2 (`game_run2_clean.png`) vs run 3 (`game_run3_p0.png`), same window size: mean|luma diff| 0.57 of 255 (imgdiff). The oracle is reproducible across launches.
- Position 1: from the start, click play, hold S 900 ms, click pause: `game_run3_p1.png`. Position read from players.db after Esc -> exit: (8306.65625, 11692.8720703125), z 0 (start was 8305.8252, 11691.6582). Port frame with `--lit-room` at that position and camera centre iso(pos) - (0, 56): `port_run3_p1.png`, diff 28.01 of 255.
- Camera model holds again (posters, windows, furniture on the same pixels). The visibility rule does not: in the real frame the south-west wall (the one with the door) is cut away and a SECOND room (a fridge, left) is visible beyond it; the port keeps only the player's room and draws the full wall. So "only the player's room is visible" is wrong in general; the real rule also reveals neighbouring squares through cut walls. UNKNOWN: its exact definition (distance, line of sight, room adjacency).
- Tool facts at 1710x1074: after the intro the main menu is interactive about 55 s after launch; clicks sent earlier are ignored. Esc menu: Back (229,516), Settings (251,574), Exit (229,632), Exit to desktop (312,690). The game process may exit on its own while the session is idle (observed).

## The real cutaway code (class-file facts) and a hypothesis that failed

Read with `tools/jargrep.mjs` and `tools/classdump.mjs --code` (names, constants, call order; branches not interpreted):
- `zombie.iso.IsoCell.SetCutawayRoomsForPlayer()` builds `tempPlayerCutawayRoomIds` (sorted list of room ids): the player's room id (or the roof-room id of its building), the `playerWindowPeekingRoomId`, and for the four squares at (x+-1.5, y+-1.5) around the player: if `getCanSee(player)` and the room id is not already in the list and `dot((sq.x+0.5-px, sq.y+0.5-py), player.getForwardDirection()) > 0` (the comparison constant is `fconst_0`, so > 0 or >= 0 is not resolved), the room id is added.
- `IsoCell.IsCutawaySquare`, `FBORenderCutaways.IsCutawaySquare/isCutawayDueToPeeking/checkOccludedRooms/shouldRenderBuildingSquare/CanBuildingSquareOccludePlayer` use that list, `CutawayWall.occludedRoomIds`, N/W/E/S adjacent squares and `BuildingDef` bounds; the real rule is a function of the facing direction and line of sight, not of the player's room alone.
- Rooms at the third/fourth position (from `zomboid-ctl rooms .. 32_45 lx ly`): the spawn `livingroom` has rects (109,168,6,4) and (113,172,2,4) (cell-local); the `kitchen` is (109,172,4,3); a `bedroom` is (109,175,4,4). Position 1 is cell-local (114.66, 172.87).
Test of the simplest reading (`zomboid-client --lit-room --probe-rooms`: add the rooms at the four +-1.5 probe squares, WITHOUT the can-see and facing tests the port lacks): position 0 got worse (27.17 -> 29.49 of 255), position 1 unchanged (28.01) because the probe squares at x 113 / 116 hit only the livingroom, not the kitchen (x 109..112). So this reading does not explain the fridge/kitchen visible in the real frame at position 1; the flag stays off by default. Not understood yet: which mechanism reveals the kitchen (cut wall plus line of sight through the opening, or `occludedRoomIds`). Needs the facing direction (players.db `data` blob, 4363 bytes, undecoded) and a fuller reading of `checkOccludedRooms`.

## players.db `data` blob: position and a probable facing field — partly decoded

First bytes (big-endian): `01 01`, 8 zero bytes, then float x at offset 10, float y at 14 (both match the `x`/`y` columns exactly: 8306.65625 / 11692.8720703125), float z at 18 (0.0), then `00 00 00 05` at 22..25. `05` read as `IsoDirections` is SE (N=0, NW=1, W=2, SW=3, S=4, SE=5, E=6, NE=7), which agrees with the walk that produced this save (+0.83, +1.21 in x/y, i.e. south-east). UNVERIFIED guess: one frame only; a second save with a different facing is needed to confirm that offset 25 is the facing.
Consequence for the visibility rule: the +-1.5 probe squares do not reveal the kitchen at position 1 even with facing SE (they land in the livingroom, see the previous section), so the facing field does not explain the real frame. Remaining hypothesis (NOT tested): the real game draws every square the player has line of sight to (a vision cone, blocked by walls/closed doors, widened by windows and open doors) and cuts away only walls that hide the player; the port's "player's room only" mask is a special case of that. Testing it needs a line-of-sight pass over the wall/door data (`Scene::collision` has wall edges) and 4+ real frames at known positions.

## Line-of-sight experiment (`zomboid-client --lit-room --los <radius>`) — mixed, opt-in only

`Collision::los` walks the segment from the player to a square centre in 1/8-square steps and refuses to cross a blocked square or a wall edge (doors and windows see-through, like `can_step`). `--los R` keeps instances on squares within R tiles that have line of sight, on top of the room mask.
Whole-frame mean|luma diff| of 255 (position 0 and 1 from run 3, 1710x1074; frame 1 from run 1, 1920x1050):

| radius | position 0 | position 1 | frame 1 |
|---|---|---|---|
| off | 27.17 | 28.01 | 15.16 |
| 3 | 26.09 | 27.80 | 15.20 |
| 5 | 24.35 | 29.26 | 14.79 |
| 7 | 22.64 | 30.72 | 14.71 |
| 10 | 18.97 | 31.39 | 12.19 |

It helps two frames (the real game does reveal squares beyond the room) and hurts position 1: at radius 10 the port also draws the bathroom/bedroom to the west, the opaque south exterior wall and a lot of grass, none of which the real frame shows (checked by eye in `tests/out/l1_10.png`). So line of sight alone is not the rule: missing pieces are the cutaway of camera-facing walls, a vision cone/light limit (night: only lit squares are drawn) and probably the facing direction. The flag stays OFF by default; do not quote 12.19 as a result, the three-frame mean (23.45 -> 20.85 at radius 10) hides a regression.

## Native lighting (Lighting64.dll): Ghidra headless works

- MEASURED: a complete Ghidra 12.1.3 exists in `E:\xplane10-web\toolchain\ghidra_12.1.3_PUBLIC`; its `analyzeHeadless` imported and analysed the DLL in 40 s (2158 functions, 53 of them JNI exports).
- MEASURED (export names, the DLL's interface): `addLight addRoomLight addTempLight setLightColor setLightActive removeLight updateTorch getSquareLighting getVertLight getLightInfo getDarkMulti getTargetDarkMulti getCanSee getCouldSee getSeen getVisibleRooms squareSet squareSetLightTransmission squareAddWindow squareAddDoor squareAddCurtain stateBeginUpdate stateEndUpdate stateEndFrame DoLightingUpdateNew playerSet scroll* teleport chunk*`. Largest: `stateEndUpdate` (3737 bytes, 0x180016370) and `getSquareLighting` (1668 bytes, 0x1800140d0).
- Takeaway: the DLL also owns can-see/could-see/seen and the visible-room list, so the visibility rule that fails on position 1 is probably computed here, not only in Java.
- UNKNOWN: falloff, colour and room-light maths. Nothing is decompiled yet; next targets are `stateEndUpdate` and `getSquareLighting`.

Earlier note, kept for history (the REA route):

- MEASURED: `zombie.iso.LightingJNI` loads `System.loadLibrary("Lighting")`; `Lighting64.dll` (x86_64 PE, sha256 1866188d8e253de521ba208f06f1db9b9019f333162ff3c31dd83506fafee773) sits in the game root.
- BLOCKED: REA `open_binary` succeeds, but every analysis call fails with `provider_unavailable`: Ghidra is configured at `E:\tools\ghidra_12.1.4_PUBLIC` and that folder has no `application.properties`/`analyzeHeadless.bat` (the install is missing or incomplete). Hopper, IDA and Kuna are not usable on this host.
- UNKNOWN: the light falloff, colour and room-light model. Nothing was read from the DLL.
- To unblock: install a full Ghidra release and point `GHIDRA_INSTALL_DIR` at it.

### getSquareLighting decompiled (Ghidra headless, read by hand)

- MEASURED: `getSquareLighting(playerIndex<4, x, y, z<64, int[] out)` only marshals precomputed per-square data into an int array of at least 43 entries (else `IllegalArgumentException: output array is too small`). It does no lighting maths. Per square it copies: a 3-bit flag word (bits 0-2 of one byte), 3 colour bytes, two floats scaled by a double constant, nine more 32-bit words, then a light list capped at 5 entries (7 floats each: id, position, colour packed as RGB bytes divided by a float constant 255-style, strength). When more than 5 lights reach a square, the weakest (by strength/radius ratio, via `FUN_180001f90`) is replaced.
- Consequence: the falloff and colour maths live in the update pass (`stateEndUpdate`, 0x180016370, 3737 bytes) and its helpers, not here. The constants `DAT_1800784ac/b0` and `_DAT_180078758` are not read yet. UNKNOWN: everything numeric.
- Local scripts: Ghidra headless scripts live outside the repo (scratchpad); they use the game's DLL and are not committed.

### stateEndUpdate decompiled (read by hand, not the maths)

- MEASURED: `stateEndUpdate` (0x180016370) swaps double-buffered state and drains the add/remove light queues (list/deque helpers `FUN_1800127f0`, `FUN_18000efa0`, `FUN_18000f230`, ...). It contains no falloff or colour formula.
- UNKNOWN: where the per-square light maths runs. Likely a worker thread started from `DoLightingUpdateNew` (0x180012aa0) or a thread entry in the DLL; not located yet. Next: follow `CreateThread`/thread-entry xrefs and read the float constants there.

### Per-light contribution in Lighting64.dll (FUN_180009600, read by hand)

Found by ranking all 2158 functions by float-instruction count and decompiling the ones with `sqrtss`. `DoLightingUpdateNew` (0x180012aa0) calls `FUN_180009c30` once per active player (4 slots), which scrolls the per-player grid and runs the propagation; `FUN_180009600` (0x180009600) turns one light plus one square into an RGB value. Stated as observations; the port will re-implement it, not copy it.

- MEASURED constants (read from the DLL's data section): 0.5 (tile centre offset), 1.0, 255.0, -1.0, 0.8, 1.15, 30.0, 1000.0.
- Light record fields as used (offsets): +0x04/+0x08 x/y, +0x10/+0x14/+0x18 colour r/g/b as floats, +0x1c/+0x20 facing vector, +0x24 radius, +0x28 intensity, +0x2c cone flag (char), +0x30 cone value, +0x34 cone kind (int). A global int (`DAT_18008f008`, initial value 1 in the file) multiplies the radius.
- Rule for a square (x, y) that has propagated path distance `d`: the light applies when `d < radius*G` and the straight-line distance from the square centre (x+0.5, y+0.5) to the light is under `radius*G`, the square is on the light's floor, and (cone lights only) the facing dot product passes the cone test; non-cone lights additionally need `dot < 0.8` or an equal-to-minus-one case (the exact meaning of that test is UNKNOWN).
- Falloff: `t = clamp(d / (radius*G), 0, 1)`; `att = (1 - t*t) * 1.15`; channel = `(int)(att * intensity * colour_c * 255 * coneFactor)` with `coneFactor = 1` for non-cone lights and `min(1, 2*(−dot − cone))` for cone lights. Integer result per channel, not clamped here.
- UNKNOWN: how channels from several lights are combined (the lookup keeps at most 5 lights per square), what feeds the night ambient (`getDarkMulti`), how light passes windows and doors (`squareAddWindow`, `squareSetLightTransmission`), and which lights exist in the oracle frames. No port code and no oracle diff yet, so the falloff above is NOT claimed to match.

### How lights combine on a square (FUN_180007a70, read by hand)

- MEASURED: for each (light, square) it runs the per-light function above; each channel is clamped to 255 (a byte), then stored per channel as the MAX of what the square already holds (red at +0xc, green +0xd, blue +0xe of the square's light-cache record). Lights are NOT summed here. It skips a light when the square was already visited by this light with a path distance not larger than the new one.
- MEASURED: it then pushes the light's id, position, the three clamped channels divided by 255 and the distance into a per-square light list (`FUN_18000c100`), which is the up-to-5-entry list `getSquareLighting` returns; Java sees both the max colour and that list.
- UNKNOWN: how Java turns the max colour and the list into vertex colours (`getVertLight`), the night ambient (`getDarkMulti`), and which lights exist in the oracle frames.

### Getters: getLightInfo, getVertLight, getDarkMulti (read by hand)

- MEASURED: all three only read precomputed per-square arrays after bounds checks (`playerIndex < 4`, else `IllegalArgumentException: invalid playerIndex`; null state gives `IllegalStateException: state is null`; out-of-grid squares return 0).
- `getLightInfo(player, x, y, z, 0|1|2)`: returns the stored max colour byte for red, green or blue divided by 255. This is the MAX-combined colour from the section above.
- `getVertLight(player, x, y, z, vertex 0..7)`: returns one packed 32-bit colour per square per vertex (8 vertices per square, matching the 8 corner lights Java asks for). Computed elsewhere in the update pass; not read yet.
- `getDarkMulti(player, x, y, z)`: returns a stored 32-bit value per square (the array `getSquareLighting` also exposes, scaled by a double). Where it is written is UNKNOWN.
- Consequence: the remaining numeric work is the writer of the vertex-light and dark-multiplier arrays inside `FUN_180009c30` (0x180009c30, 952 instructions) and the helpers it calls (`FUN_180003970`, `FUN_180004800` are the next float-heavy callees).
