// Generates site/progress.json: how much of Project Zomboid is reproduced, with the evidence for every number.
// Measured items run real commands against the player's own install. Items that cannot be measured are an AUTHOR ESTIMATE and say so.
//   node tools/gen_progress.mjs
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repo = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1")), "..");
const exe = (n) => path.join(repo, "target", "release", n + (process.platform === "win32" ? ".exe" : ""));
const run = (bin, args) => execFileSync(exe(bin), args, { cwd: repo, encoding: "utf8", maxBuffer: 64 << 20, stdio: ["ignore", "pipe", "pipe"] });
const pct = (a, b) => (b ? (100 * a) / b : 0);

// ---- measured ----
const lua = run("zomboid-lua", ["..", "run"]);
const ex = Object.fromEntries([...lua.matchAll(/^(shared|client|server)\s+executed ok=(\d+) errored=(\d+)/gm)].map((m) => [m[1], { ok: +m[2], bad: +m[3] }]));
const luaOk = Object.values(ex).reduce((s, v) => s + v.ok, 0);
const luaAll = Object.values(ex).reduce((s, v) => s + v.ok + v.bad, 0);
const globals = +(lua.match(/-- (\d+) distinct undefined globals touched/)?.[1] ?? 0);

const res = run("zomboid-ctl", ["resolve", ".."]);
const m = res.match(/distinct tile names used=(\d+) known pack textures=(\d+) unresolved=(\d+)/);
const tilesUsed = +m[1], tilesMissing = +m[3];

const rooms = run("zomboid-ctl", ["rooms", "..", "32_45"]).match(/parsed to byte (\d+) of (\d+)/);
const headerParsed = +rooms[1], headerTotal = +rooms[2];

const zones = run("zomboid-ctl", ["zones", ".."]).match(/: (\d+) zones/)[1];

const diff = (g, p) => +run("zomboid-ctl", ["imgdiff", "..", g, p]).match(/at shift 0,0 = ([\d.]+)/)[1];
const d1 = diff("screenshots/game_clean2.png", "screenshots/port_lit_room.png");
const d2 = diff("screenshots/game_s3_moved_S.png", "screenshots/port_s3.png");
const d0 = diff("screenshots/game_clean2.png", "screenshots/port_z0_aligned.png");

// Same filtered, sorted multiset as tests/lua_require_parity.sh, without requiring
// a Unix shell. A mismatch is an error, never silently reported as zero matches.
const warningLines = (text) => text.split(/\r?\n/).filter((s) => /require|can.t find map objects|Mannequin zone/.test(s)).map((s) => s.trimStart()).sort();
const expected = warningLines(fs.readFileSync(path.join(repo, "tests/golden/lua_real/client_lua_warnings.txt"), "utf8"));
const actual = warningLines(lua);
if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error("Lua warning oracle differs; progress not written");
const golden = expected.length, parityKinds = actual.length;

// Launcher has a separate Windows shader build prerequisite; do not imply it
// was tested when only the four core crates have been checked.
const tests = execFileSync("cargo", ["test", "--release", "-p", "zomboid-lua", "-p", "zomboid-content", "-p", "zomboid-client", "-p", "zomboid-ctl"], { cwd: repo, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
const passed = [...tests.matchAll(/test result: ok\. (\d+) passed/g)].reduce((s, x) => s + +x[1], 0);

const FAMILIES = [
  ["maps: lotheader (with rooms)", true], ["maps: lotpack (chunks)", true], ["texture packs (.pack)", true], ["tile definitions", true],
  ["item scripts and blocks", true], ["translations", true], ["map zones (objects.lua)", true], ["save: players.db", true],
  ["3D models and animations", false], ["FMOD sound banks", false], ["fonts (.fnt)", false], ["recipe and vehicle scripts", false],
  ["other save files (map_*.bin, chunks)", false], ["cell header tail", false], ["UI layout and UI textures", false], ["mod loading", false],
];
// ---- areas: score 0..1, weight. Score formulas are written next to each area. ----
const areas = [
  { id: "formats", name: "Reading the game's files", weight: 15, measured: false,
    score: FAMILIES.filter((f) => f[1]).length / FAMILIES.length,
    how: "share of the game's format families we read (list below); coverage inside each family is also incomplete",
    facts: [`format families read: ${FAMILIES.filter((f) => f[1]).length} of ${FAMILIES.length}`, `cell header parsed: ${headerParsed} of ${headerTotal} bytes`, `tile names resolved: ${tilesUsed - tilesMissing} of ${tilesUsed}`, `Muldraugh map zones: ${zones}`, "not read: " + FAMILIES.filter((f) => !f[1]).map((f) => f[0]).join(", ")] },
  { id: "lua", name: "The game's Lua scripts run", weight: 20, measured: false,
    score: pct(luaOk, luaAll) / 100 * 0.2,
    how: "Lua files that execute without error, counted as 20% of the area weight (running is not the same as correct); the rest is the real Java API, which barely exists yet (author estimate: ~0)",
    facts: [`Lua files without error under stubs: ${luaOk} of ${luaAll}`, `undefined Java names the Lua touches: ${globals}`, `selected real-client log lines reproduced: ${parityKinds} of ${golden}`, "numeric literals, translation formatting, item APIs and Lua event callback mutation and error cases checked against selected original Java cases; world clock events reach explicitly loaded Lua handlers"] },
  { id: "render", name: "World rendering", weight: 20, measured: false,
    score: Math.max(0, 1 - d1 / 60) * 0.5,
    how: "frame similarity to the game, 1 - (diff/60), times 0.5: lighting, characters, weather and animation are not there yet",
    facts: [`frame 1 difference from the game: ${d1} of 255`, `frame 2 difference: ${d2} of 255 (lighting factor was also checked on this frame)`, `before the visibility mask: ${d0} of 255`] },
  { id: "sim", name: "Gameplay and zombies", weight: 25, measured: false, score: 0.03,
    how: "author estimate: a 30 Hz step, movement and collision exist in simplified form; zombies are placeholders with invented parameters",
    facts: ["26 cardinal, 32,768 diagonal, 256 water, 256 water-window, 4,096 window-flag, 8,192 mixed-solid and 9,216 adjacent-window static cases checked against the original", "a real water square at (1, 4803, 0) checked through loaded scene collision; no original walk trace", "continuous player movement, sliding, doors, window objects and stairs: incomplete", "zombies: placeholder", "inventory, crafting, skills, health: none"] },
  { id: "saves", name: "Saves", weight: 5, measured: false, score: 0.1,
    how: "author estimate: player position and selected time fields are read; most save state and writing are missing",
    facts: ["player position and floor drive window and offscreen camera; a synthetic live-WAL test verifies committed player coordinates", "saved time and day length initialize a partial World clock; controlled clock state and event-signal traces match selected original cases; explicitly loaded Lua handlers receive clock events; game APIs and save writing remain incomplete"] },
  { id: "ui", name: "Menus, interface, settings", weight: 5, measured: false, score: 0.05, how: "author estimate: a world window and a launcher exist, the game's UI does not", facts: ["DeadOxide launcher", "HUD, inventory, windows: none"] },
  { id: "audio", name: "Audio", weight: 5, measured: false, score: 0, how: "author estimate", facts: ["not started"] },
  { id: "net", name: "Multiplayer and networking", weight: 5, measured: false, score: 0, how: "author estimate", facts: ["not started"] },
];
const totalW = areas.reduce((s, a) => s + a.weight, 0);
for (const a of areas) { a.percent = +(a.score * 100).toFixed(1); }
const overall = +(areas.reduce((s, a) => s + a.score * a.weight, 0) / totalW * 100).toFixed(1);

const out = {
  generated: new Date().toISOString().slice(0, 10),
  game: "Project Zomboid 42.21.0 (Steam build 25485521)",
  overall,
  note: "Overall and area percentages are subjective weighted estimates, not measured game parity. Facts list the limited measured cases. Full compatibility is unknown.",
  areas,
  numbers: { luaOk, luaAll, globals, tilesUsed, tilesMissing, headerParsed, headerTotal, zones: +zones, frameDiff1: d1, frameDiff2: d2, frameDiffBeforeMask: d0, logKindsMatched: parityKinds, goldenLogLines: golden, testsPassed: passed },
};
fs.mkdirSync(path.join(repo, "site"), { recursive: true });
fs.writeFileSync(path.join(repo, "site", "progress.json"), JSON.stringify(out, null, 2));
console.log(JSON.stringify({ overall, areas: areas.map((a) => [a.id, a.percent]), numbers: out.numbers }, null, 1));
