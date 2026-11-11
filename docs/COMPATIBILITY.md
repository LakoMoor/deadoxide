## Progress measurement (2026-11-10)
- Lua compile coverage: 1395/1395 installed files (100%), `cargo run -q --release -p zomboid-lua -- .. compile`. Compile and execution share a narrow numeric-suffix retry; it preserves strings and comments. This measures syntax coverage, not game compatibility; full Kahlua lexing remains UNKNOWN.
- Lua warning oracle: 22/22 recorded normalized lines (100%) as a sorted multiset, compared with `tests/golden/lua_real/client_lua_warnings.txt` using the equivalent PowerShell filter and sort. This covers only the logged warning categories in that fixture, not their order or all Lua behavior.
- Texture names resolved: 25981/26063 (99.69%) in `zomboid-ctl resolve ..`; 82 remain unresolved and their rendering behavior is UNKNOWN.
- Overall game parity percentage: UNKNOWN. Scope and weighting have not been measured; these percentages cannot be averaged into a 1:1 completion figure.
- Full Windows check passed on 2026-11-11: `pwsh -NoProfile -File tools/check.ps1` ran fmt, workspace Clippy, 38 workspace tests and the Lua warning diff: `require + map-objects + Mannequin warnings IDENTICAL (22 lines)` / `ALL CHECKS PASSED`. One real-content scene test was ignored in this run and passed when explicitly run separately. The PowerShell equivalent uses a local fxc wrapper because this host's Git Bash cannot initialize MSYS and fxc cannot resolve GPUI's include in the Cargo cache. The original `sh tools/check.sh` itself remains unavailable on this host.
- Current headless port screenshot: `screenshots/port_adjacent_2026-11-11.png`, Rosewood x=8303.5, y=11689.5, z=0, zoom 1, 1920x1050, experimental `--lit-room`. `cargo run -q --release -p zomboid-ctl -- imgdiff .. screenshots/game_clean2.png screenshots/port_adjacent_2026-11-11.png 650 300 1280 600` reports mean luma difference 12.55/255 at shift (0,0), on the specified crop and existing paused-night oracle. This is not whole-frame parity; player rendering, lighting and visibility remain incomplete, and the still does not test movement or windows.
- `node tools/gen_progress.mjs` refreshed the local site on 2026-11-11: 36 tests in four core crates, 22/22 selected warning lines. Its overall 20.5% and per-area percentages are explicitly subjective weighted estimates, not measured compatibility; overall actual parity remains UNKNOWN. Launcher tests are excluded from that site count.

## Kahlua numeric literals (2026-11-10)
The installed jar's Kahlua compiler and VM were invoked headlessly with `tests/original_probe/FloatProbe.java`, using the bundled JRE; the tested expressions and observed outputs are recorded in `docs/PORT.md`. The Rust tests check those ten values and three malformed cases. On those samples the results agree; this is not a complete lexer comparison. The fallback also preserves quoted and long strings and comments in a separate Rust regression test. It remains a partial adapter invoked after a malformed-number error; untested Kahlua forms and full script behavior remain UNKNOWN.

## Translator formatting (2026-11-10)
`TranslatorProbe` invokes the installed jar's `zombie.core.Translator.getText` with controlled entries in its UI map. The Rust test covers 18 cases observed from that oracle: string and positional substitutions, legacy `%1` forms, escaping, insufficient arguments, malformed conversions and two templates from the installed EN files. All 18 produce the recorded original output. The actual `Challenge_Challenge2_WaveStart` entry was also exercised through the Lua harness with arguments 4 and 9; it returned `Wave 4 in : 9 (press Enter to skip)`, matching the original probe on the same template and arguments. Other Java Formatter conversions, object stringification, warnings, language fallback and translation loading semantics remain UNKNOWN or incomplete; 18/18 is coverage of these selected cases only, not a percentage for the translator.

## Russian translation loading (2026-11-11)
`TranslationLoadProbe` initializes the installed game's filesystem with `/tmp/pz-translation-probe` as cache, initializes `Languages`, selects RU in `Core`, and invokes the original `Translator.loadFiles` without opening a window. Two runs produced the same selected lines in `tests/golden/translator_load_ru.txt`: the stack is EN then RU, `Challenge_YouHaveOneDay_name` returns the Russian value, absent RU key `UI_servers_last_wipe` falls back to English, and a missing key returns its key / null via `getText` / `getTextOrNull`. The Rust test loads the actual installed RU and EN JSON and compares those three cases through the Lua functions. `ZOMBOID_LANG=RU CARGO_HOME=/tmp/pz-cargo cargo run -q --release -p zomboid-lua -- .. run 'return getText("Challenge_YouHaveOneDay_name")'` printed `У вас один день`. The runtime now accepts `ZOMBOID_LANG` and defaults to EN; non-EN languages currently use the same EN → selected-language assumption. Only RU was checked for the complete loading and lookup cases; ES_MX's stack was observed as EN → ES_MX, but its fallback lookup was not checked. Other languages, mods, duplicate keys, exact error handling and runtime language switching remain unverified. This is the Lua harness, not an in-game language selection UI.

## Recipe name translation (2026-11-11)
`RecipeNamesProbe` initializes the original translation loader in RU and captures `Translator.getRecipeName` and `getTextOrNull` for seven keys twice, with identical output in `tests/golden/recipe_names_ru.txt`. The samples cover Russian translations, an English fallback, unknown IDs and case sensitivity. The Rust harness loads `Recipes.json` into a separate map and binds `Translator.getRecipeName`; the test compares all seven original results and passes the first two `Base.SmithingMag1` recipe IDs through the real script-item list into the translator. This removes the previous incorrect exposure of these selected recipe keys via `getTextOrNull`. It does not establish complete category routing, mod overrides, language switching or the rendered literature UI; other Translator methods remain permissive stubs.

## Learned recipe lists (2026-11-11)
`ItemRecipesProbe` loads original scripts until the macOS Bullet native-library failure, then inspects `Item.getLearnedRecipes`. Two runs produced identical `tests/golden/item_recipes_partial.txt`: `Base.Axe` and `Base.BookCarpentry1` return null; `Base.SmithingMag1` and `Base.SmithingMag2` return ordered lists of 17 and 12 recipe IDs, including the `base:kitchentools` entry. The Rust Lua harness now returns nil or a Java-style list with zero-based `get`, and a real-content test compares all entries with the fixture using the loop shape from `ISLiteratureUI:setLists`. Empty/malformed values, mods, list mutation, recipe execution and the UI itself remain unverified. Original loading was partial, not a full game run.

## Script item properties (2026-11-11)
`ItemPropertiesProbe` loads the original scripts until the macOS Bullet native-library failure, then inspects `Base.Axe`, `Base.BookCarpentry1`, and `Base.F_Hair_Stubble`. Two captures match `tests/golden/item_properties_partial.txt`: their actual weights, obsolete and hidden booleans, skill names, recorded-media value and literature/weapon/clothing membership. `ItemTypeProbe` records all 16 built-in type names in `tests/golden/item_type_names.txt`. Rust now exposes these methods and type identifiers; a real-content test compares the selected items and names with the original fixture, and the actual `forageSystem.lua` iterator test exercises its non-obsolete/non-hidden filter. The type identifiers are Lua strings, not Java `ItemType` objects; mod-defined types, non-default obsolete/media cases, malformed properties, item list order, and the remaining getters are unverified. This is script-item data, not inventory instances or a completed UI.

## Script item list API (2026-11-11)
`ScriptItemsProbe` initializes the original filesystem with a scratch cache and calls `ScriptManager.Load()` on the installed build. On this macOS host loading stops at the Windows-only Bullet native call; 5,092 items are available at that point, not a full original load. Two runs produced identical `tests/golden/script_items_partial.txt`: the returned list is nonempty, includes `Base.Axe`, uses zero-based `get`, rejects negative and end indices, and the Lua globals return the same manager and a list of the same size. The Rust Lua harness now exposes `getScriptManager`, `getAllItems`, and `size`, `isEmpty`, `get`, `contains` on its item list. A real-content test executes the actual `iterList` function extracted from the installed `forageSystem.lua` across the loaded Rust items and checks all entries and the axe. Ordering, completeness, live mutations and broader Java collection semantics remain unverified; the partially loaded original count must not be compared as a full inventory count. `ISLiteratureUI:setLists` still requires unimplemented item methods, item types and UI.

## Translation namespaces (2026-11-11)
The real RU files repeat `Base.CannedCarrotsOpen` in `ItemName.json` and `EvolvedRecipeName.json`; map-description files also repeat `title`. Flattening all files into one `getText` map made these unrelated values visible under the wrong API. `TranslationCategoriesProbe` initializes the original loader and invokes both `Translator.getText/getTextOrNull` and `LuaManager.GlobalObject.getItemNameFromFullType/getItemEvolvedRecipeName`; two RU runs yielded identical `tests/golden/translator_categories_ru.txt`. The Rust harness now keeps item names separately and exposes `getItemNameFromFullType`, while excluding map descriptions and evolved-recipe names from `getText`. Three item keys, including an unknown key, and `title` are checked through the real Lua functions against the original output. Other `getText` namespace routing, evolved-recipe lookup, mods, script-defined names, and error semantics remain incomplete; this does not implement the game inventory UI.

## Translator value conversions (2026-11-11)
`TranslatorValuesProbe` calls the installed jar's `Translator.getText` with typed Java arguments; two runs produced the same `tests/golden/translator_values.txt`. Rust now supports `%S`, `%b`, `%B` and reuse of the previous argument via `%<s` (also for the supported conversions). Thirteen selected cases covering strings, booleans, null, simple numbers, legacy substitutions and mixed explicit/ordinary indices are compared to this fixture. Null is normalized to an empty string by the original in these cases, so `%b` produces `true`. This is a subset: width/precision, numeric conversions, Java object conversions, locale-sensitive casing and Java floating-point string formatting are not fully implemented or verified. The probe shows e.g. `1e20` and `0.0001` use Java scientific notation; Rust's conversion currently differs.

## Static adjacent-square collision (2026-11-10)
`tests/original_probe/CollisionProbe.java` invokes the installed jar's `IsoGridSquare.CalculateCollide` on synthetic z=0 square pairs with a supplied lookup, all four boolean options false and no objects. Thirteen property combinations were checked in both east/west and south/north directions (26 pairs, 52 directed results). On these cases, `solid`/`solidtrans` block entry when tested alone, `collideW`/`collideN` block crossing their respective edges in both directions, and `WallW`/`WallN` alone do not block. Door/window flags by themselves also do not change these static checks; actual door/window objects were not tested. The Rust test `static_square_flags_match_original_adjacent_samples` agrees with all 26 pairs through the scene's flag mapping and `Collision::can_step`.

Corrected the previous assumption that `WallN`/`WallW` themselves block movement. The client now derives static edge flags from `collideN`/`collideW`; its experimental LOS also uses this collision grid, so LOS can change and is not independently verified. Real walk traces, continuous diagonal response, stairs, door/window states, collision near character edges and other CalculateCollide options remain UNKNOWN.

## Static diagonal-square collision (2026-11-10)
`tests/original_probe/DiagonalProbe.java` calls the installed jar's `IsoGridSquare.CalculateCollide` for four diagonal directions, with all four squares present, z=0, no objects and all four options false. It enumerates the eight combinations of `solid`, `collideN` and `collideW` independently on each of the four squares: 4 × 8⁴ = 16,384 cases. `tests/golden/collision_diagonal.txt` records the original's results; a repeated original run produced the same file, and `sim::tests::diagonal_static_flags_match_original_oracle` compares all 16,384 to Rust. For these inputs, the original blocks when the destination is solid or any of the four edges around the diagonal is blocked; it does not block solely because a side square is solid.

The player's axis-separated movement now checks that static diagonal rule when both axes cross a square boundary in one step. Its choice to slide along one axis remains PLACEHOLDER: the oracle above checks square collision, not the original moving character's continuous response. Sliding order, character radius, missing squares, doors, window objects, stairs, other z levels and options remain UNKNOWN. The experimental LOS algorithm is separate and is not validated by this movement oracle.

## Transparent squares and window flags (2026-11-11)
`TransparentDiagonalProbe` repeats the 16,384 diagonal static configurations above using `solidtrans` instead of `solid`; its original output is byte-identical to the prior fixture, and Rust independently tests those cases. `WindowSolidProbe` enumerates 512 combinations of `solidtrans`, four window flags, two collision edges and two transparency flags on the destination in eight directions, with otherwise empty present squares, z=0, no objects and all four options false. The original's 4096 results are recorded in `tests/golden/collision_window_solid.txt`, repeated identically, and compared through the actual Rust tile-property mapping. On these samples `solidtrans` does not by itself block entry when a destination window flag is present; relevant `collideN`/`collideW` still block. This corrects the earlier unconditional treatment of `solidtrans`. Actual window objects, mixed `solid` and `solidtrans`, doors, other options and continuous movement remain UNKNOWN. LOS also shares the collision grid and has not been separately revalidated.

## Adjacent windows and hoppable flags (2026-11-11)
`AdjacentWindowProbe` tests 1,152 combinations: a `solidtrans` destination, seven flags varied on one of the nine surrounding squares, movement from west to east, z=0, all squares present, no objects and all options false. The installed jar's `CalculateCollide` output is recorded in `tests/golden/collision_adjacent_window.txt`; a second run was byte-identical. The Rust scene mapping and collision grid agree on all cases. An appropriate north-window flag on the square south of the destination, a west-window flag on the square east of it, or either hoppable flag on either square can exempt destination `solidtrans` from blocking, as can window/hoppable flags on the destination. The previous port overlooked these neighboring cases; it now retains directional window flags and hoppable properties. This is a limited synthetic square-level comparison, not a walk, window-object or jumping test. Other directions, combined flags on multiple surrounding squares, missing squares, object interaction and full movement response remain unverified.

## Adjacent window flags in eight directions (2026-11-11, macOS)
`AdjacentWindowDirectionsProbe` runs the installed 42.21.0 jar on the local Java 27 runtime. It varies seven flags on each of nine cells around a `solidtrans` destination for all eight adjacent source directions: 9,216 object-free z=0 cases with all options false. Two independent runs produced byte-identical `tests/golden/collision_adjacent_window_directions.txt`; the Rust scene mapping and `Collision::can_step` agree with all cases. This expands the earlier west-to-east test, without establishing behavior for flags on multiple cells, unloaded squares, dynamic window objects, or continuous movement.

## Mixed solid and window flags (2026-11-11)
`MixedSolidWindowProbe` varies ten destination flags, including `solid` and `solidtrans`, over all eight adjacent directions, with object-free present squares, z=0 and all options false. Two original runs produced identical `tests/golden/collision_mixed_solid_window.txt`; the Rust tile mapping and collision check agree in all 8,192 cases. This does not cover actual window objects or continuous player movement.

## Water and static collision (2026-11-10)
`tests/original_probe/WaterProbe.java` calls the installed `IsoGridSquare.CalculateCollide` on present, object-free z=0 square pairs in four cardinal directions, with all four options false. It enumerates `water`, `solidtrans` and `solidfloor` on both squares (4 × 8² = 256 cases). The original blocks movement when the destination has `solidtrans`, and also when the source has both `water` and `solidtrans`; `water` or `solidfloor` alone do not change the result in these cases. Rust aggregates flags from all tiles on a square and agrees with the stored original output in all 256 cases. This is not evidence of real swimming, object state, or how every real square's final flags are built. This rule also affects the experimental LOS because it uses the movement grid.

`WaterWindowProbe` additionally recorded 256 combinations of water, solidtrans and windowN in four directions (`tests/golden/collision_water_window.txt`). All 256 now compare equal through the Rust scene mapping. Destination solidtrans+windowN is passable in these samples, but adding water makes it blocked; the destination-water check was corrected. The previously suspected northward discrepancy was wrong: the existing adjacent-window rule accounts for the source square's windowN. Window objects, other flags and real movement are not covered.

Real-content bridge: `cargo run -q --release -p zomboid-ctl -- tiledefs ..` reports 33,568 definitions and four definitions with both `water` and `solidtrans`. The original text file explicitly gives `blends_natural_02_0` the properties `water`, `solidtrans`, `solidfloor`; `real_water_tile_uses_original_measured_collision_flags` loads the installed definitions and verifies blocked entry and exit against the corresponding `WaterProbe` square cases. This does not establish that the original character follows the same movement response. Individual definition counts do not describe properties merged across a square.

Real-scene follow-up: `cargo test -q --release -p zomboid-client real_water_square_is_wired_into_loaded_scene -- --ignored --nocapture` passed on the installed build, locating `blends_natural_02_0` at `(1,4803,0)` in cell `0_18` and loading that cell through `scene::load_region`; the resulting collision grid blocks exit in all four cardinal directions. `screenshots/port_real_water_2026-11-11.png` shows the port at this location (zoom 1, z=0, 1920x1050, placeholder player, edge of loaded cell). No original screenshot exists for comparison. This is a Rust wiring check with real data, not an original movement trace or proof that other square properties have been fully reconstructed. The integration test is ignored in ordinary runs because it loads a real cell and atlas; its explicit invocation is required.

For absent squares, `MissingSquareProbe` observes that `CalculateCollide` with these options returns false for a null destination, while a diagonal with a destination and both side squares missing returns true. This alone does not establish how the game's moving-object path treats unloaded chunks, so the client's loading-boundary behavior was not changed; it remains UNKNOWN.

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
| Static collision from solid/solidtrans/water/collide* | original CalculateCollide on synthetic adjacent squares | zomboid-client::scene + sim | partial | window WASD | real tile properties loaded; no real movement trace | 26 cardinal pairs, 16,384 diagonal and 256 water cases, z=0, no objects | unloaded cells, doors, windows, continuous movement, stairs, original player walk trace |
| Lua compile of media/lua | game's Lua files, stock Lua 5.1 with limited numeric-suffix fallback | zomboid-lua compile | partial | cargo run -p zomboid-lua -- .. compile | 1395/1395 | numeric samples below | implement and verify remaining Kahlua lexical semantics |
| Lua execution with stubbed Java API | – | zomboid-lua run | harness only | – | 1349/1395 run w/o error under stubs (not meaningful) | none | real Events/ScriptManager/etc.
| Script file grammar (media/scripts) | file inspection + grep counts | zomboid-content::script | yes (2 tests) | zomboid-ctl scripts | 1004/1004 parse; counts match grep for craftRecipe/vehicle | none | property typing, item/recipe interpretation |
| ScriptManager items + Events registry (Lua) | media/scripts content only; Events semantics assumed (Add/Remove/Trigger) | zomboid-lua::scripts | yes | zomboid-lua run | spot check: Axe weight, event dispatch, 5105 items | none | method surface (FindItem...), recipes, B42 item fields; capture real Lua log
| getText / getTextOrNull (EN) | Translate/EN/*.json; Translator.getText oracle on 18 templates | zomboid-lua::text | partial | zomboid-lua run | EN lookup and Challenge template | 18 selected formatting cases | full Java Formatter, object conversion, language selection/fallback and warnings |
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

## Saved-player render wiring (2026-11-11)
The client `--save` now initializes the camera and rendered level from the saved position in both window and `--shot` modes; `--shot` uses the saved player by default. Explicit `--player` still overrides it, with optional `--player-z`. The scene preserves its actual minimum level when rebuilding ranges after `--lit-room` filtering. The installed isolated Rosewood save has `(8306.65625, 11692.8720703125, 0)`; `--save profile/Saves/Sandbox/2026-11-07_22-34-13 --shot screenshots/port_save_auto_2026-11-11.png --lit-room` produced a PNG byte-identical to `32_45 --shot <out> -216717.8125 639928.875 0.5 1280 720 --player 8306.65625 11692.8720703125 --level 2 --lit-room` (SHA-256 `36b67dc7b91f5b5d58ae58c8585d406352dcc9e3fbc103600d6ac414cca056d4`). A scratch copy of the same DB with only the z column set to 1 produced `screenshots/port_save_z1_auto_2026-11-11.png`, byte-identical to explicit `--player-z 1 --level 3` and camera `-216717.8125 639736.875` (SHA-256 `849149ee259aee4a046846bf22001d5f677598c342569dae56f3765af08fbcb6`). These comparisons test port wiring, not an original screenshot or authentic upstairs save; the character remains a red marker and lighting is experimental. Save blobs, changed world chunks, time, inventory and writing remain unimplemented.

## Saves (first slice): the saved character position
`zomboid-content::save::load_player` reads `localPlayers` of a save's `players.db` (SQLite, read-only; rusqlite bundled): id, name, x, y, z, isDead.
`zomboid-client --save <save folder|players.db>` (or `.\play.cmd save`) reads it from a temp copy, picks the map cell from the position and puts the character there
(printed: `save: Тейлор Столлворт at (8303.5, 11689.5) z0`). Test (independent oracle): the saved square lies in a `livingroom` of the cell header at the saved z — matches the real screenshot.
NOT done: everything else in the save (inventory, health, skills, world modifications, `map/` chunk data, applying time of day, zombies); writing saves; loading any save other than the first local player; z > 0 starting levels untested.

## Save time/date prefix (2026-11-11)
The installed 42.21.0 jar is SHA-256 `e1a69eb743ede60b213a0fe7f8b83d4fcab773036d256cc4543a336f3b058a33` (REA archive identification). `SaveTimeProbe` invokes the original `GameTime.load(DataInputStream)` on a copy of the isolated profile's `map_t.bin` (SHA-256 `d3f557bf76844c1fd304f9718c8471010d7eded05d88809aa00808c81646db1d`) and on individually patched copies; it consumed the whole file and the selected outputs were repeated identically in `tests/golden/save_time_probe.txt`. In version 249, tested changes identify the current f32 time at offset 24 and day/month/year integers at 28/32/36; raw day and month are zero-based as used by the game's Lua. The sampled file yields 11:25, year 1993, month index 6, day index 8. Prior notes treated a different time as an unverified guess; the original loader now establishes these fields for this copied file. It does not establish the time of the older screenshots or explain their lighting.

`zomboid-content::save::parse_game_time_prefix` reads only these fields and rejects other versions, short/invalid headers and times outside [0,24). Its tests compare all 11 original probe cases, including original f32 minute truncation (6.1 becomes 06:05). `zomboid-ctl save-time <save-dir|map_t.bin>` inspects a file; the client reports the value when `--save` points to a folder with `map_t.bin`. On the copied save, the normal client `--save /tmp/pz-save-time-copy --shot <out> --lit-room` reported 11:25 and produced a PNG byte-identical to `screenshots/port_save_auto_2026-11-11.png`; this confirms the value is reported without changing the image, not original visual parity. Time is not yet applied to simulation, weather or lighting, and the remaining file fields, other versions, and consistent snapshots of actively written saves are unverified.

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
Still one scene and one room; the time represented in those older screenshots remains unverified. The time/date fields of a later copied save have since been measured as described above.

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

### Per-square colour from the light list (FUN_180003970, read by hand)

- MEASURED: for one square it walks the square's light list (the up-to-5-entry list above). For each entry it computes `k = 1 - dist / radiusByte` (a second, linear falloff on top of the (1-t^2) one used while propagating) and takes, per channel, the MAX over entries of `channel_byte * k` (integer truncation), starting from the square's existing colour.
- MEASURED: when a flag bit (`square+0x1d & 4`, probably "outdoors/sees sky") is set, the starting colour is raised to at least the global ambient colour held at `state+0x38` (three bytes). So outdoor squares never go below the ambient colour; indoor squares can go to 0.
- MEASURED: it also averages a float field (`dark`) of the square with those of up to four neighbours that are darker and visible (flag byte bits 0-3 pick the four directions), dividing by the number of terms used; negative global colour floats (`DAT_18008fdf8/fc/fe00`) are clamped to 0.
- UNKNOWN: who writes the ambient colour at `state+0x38` each frame (it must come from Java's time-of-day, via `stateBeginUpdate` or `playerSet`), and the vertex (8-corner) interpolation that follows. No port code and no oracle diff for this step.

### Where the DLL's ambient comes from (LightingJNI.update bytecode + stateEndFrame, read by hand)

- MEASURED (bytecode order of `zombie.iso.LightingJNI.update`): per player it calls `stateBeginUpdate(player, xMin, yMin, gridW, gridH)`, `updatePlayer`, then `stateEndFrame(F rmod, F gmod, F bmod, F ambient, F night, F viewDistance, F viewDistMax, Z cachecleared, F lightSourceUpdate, I skyLightLevel)`. The first five floats are `RenderSettings.PlayerRenderSettings.getRmod/getGmod/getBmod/getAmbient/getNight`; the sixth/seventh are `getViewDistance` and `GameTime.getViewDistMax`; the int is `GameTime.getSkyLightLevel()`.
- MEASURED (Lighting64.dll `stateEndFrame`): it stores these twelve values into the pending "frame" command of the state (type 8: fields rmod, gmod, bmod, ambient, night, view distance and so on) and ORs the cache-cleared flag; the update thread later applies them. So the ambient floor for outdoor squares is driven by `RenderSettings` (time of day and weather), not by anything inside the DLL.
- UNKNOWN: the `RenderSettings` values for the oracle frames (game paused at night). Next: read `RenderSettings.getPlayerSettings` / `updateRenderSettings` constants for the night case, or capture `ambient`/`night` from the real game's log at the oracle moment.

### RenderSettings.PlayerRenderSettings.updateRenderSettings (bytecode constants and fields, read by hand)

- MEASURED (class constants): `RenderSettings.AMBIENT_MIN_SHADER = 0.4`, `AMBIENT_MAX_SHADER = 1.0` (and the same for the legacy path).
- MEASURED (field flow): `night = ClimateManager.getNightStrength()`, `darkness = getDayLightStrength()`, `ambient = getAmbient()`, `viewDistance = getViewDistance()`, `desaturation = getDesaturation()`, `fogMod` is derived from `getFogIntensity()` with a 0.5 factor, `isExterior` comes from `getCurrentSquare().isInARoom()` (inverted), and the blend colour is the climate global light's exterior or interior colour (`ClimateColorInfo.getExterior()/getInterior()`); `blendIntensity` is that colour's alpha.
- MEASURED (constants seen in order, meaning not yet attributed to a variable): 0.07, 0.15, 0.25, 0.15, 0.075 (a table indexed by `SandboxOptions.nightDarkness`, with `ClimateMoon.getMoonFloat()` and `night` in the same expression), 0.925, 0.075, 0.25 (a desaturation floor), 0.2 (ambient with night-vision), 0.65 (lastStand ambient), 0.25 (lastStand darkness/night), 0.7 (a `GameTime.Lerp(.., 0.7, darkness)` that sets `rmod`).
- UNKNOWN: the exact formula that turns these into `rmod/gmod/bmod`, and the climate values at the oracle moment (night, game paused). The 0.8 flat factor used by `--lit-room` was fitted on two frames and is NOT derived from this chain.
- Next: read the climate side (`ClimateManager.getAmbient/getNightStrength` at the paused night time) or capture `rmod/gmod/bmod/ambient` from the real game at the oracle moment.
