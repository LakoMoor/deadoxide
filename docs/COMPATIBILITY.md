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
