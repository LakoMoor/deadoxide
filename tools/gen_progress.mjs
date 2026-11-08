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

const golden = fs.readFileSync(path.join(repo, "tests/golden/lua_real/client_lua_warnings.txt"), "utf8").split("\n").filter(Boolean).length;
let parityKinds = 0;
try {
  const out = execFileSync("sh", ["tests/lua_require_parity.sh"], { cwd: repo, encoding: "utf8" });
  parityKinds = +(out.match(/IDENTICAL \((\d+) kinds\)/)?.[1] ?? 0);
} catch {}

const tests = execFileSync("cargo", ["test", "--release", "--workspace"], { cwd: repo, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
const passed = [...tests.matchAll(/test result: ok\. (\d+) passed/g)].reduce((s, x) => s + +x[1], 0);

const FAMILIES = [
  ["карты: lotheader (с комнатами)", true], ["карты: lotpack (чанки)", true], ["пакеты текстур (.pack)", true], ["определения тайлов", true],
  ["скрипты предметов и блоки", true], ["переводы", true], ["зоны карты (objects.lua)", true], ["сохранение: players.db", true],
  ["3D-модели и анимации", false], ["звуковые банки FMOD", false], ["шрифты (.fnt)", false], ["скрипты рецептов и транспорта", false],
  ["остальные файлы сохранения (map_*.bin, чанки)", false], ["хвост заголовка ячейки", false], ["раскладка интерфейса и текстуры UI", false], ["загрузка модов", false],
];
// ---- areas: score 0..1, weight. Score formulas are written next to each area. ----
const areas = [
  { id: "formats", name: "Читаем файлы игры", weight: 15, measured: true,
    score: FAMILIES.filter((f) => f[1]).length / FAMILIES.length,
    how: "доля семейств форматов игры, которые мы читаем (список ниже); внутри каждого семейства покрытие тоже неполное",
    facts: [`семейств форматов прочитано: ${FAMILIES.filter((f) => f[1]).length} из ${FAMILIES.length}`, `заголовок ячейки разобран: ${headerParsed} из ${headerTotal} байт`, `имена тайлов разрешены: ${tilesUsed - tilesMissing} из ${tilesUsed}`, `зон карты Muldraugh: ${zones}`, "не читаем: " + FAMILIES.filter((f) => !f[1]).map((f) => f[0]).join(", ")] },
  { id: "lua", name: "Lua-скрипты игры работают", weight: 20, measured: true,
    score: pct(luaOk, luaAll) / 100 * 0.2,
    how: "файлов Lua выполняется без ошибки (20% веса области: запуск ≠ корректность), остальное — реальное Java API, которого пока почти нет (оценка автора: ~0)",
    facts: [`Lua-файлов без ошибки: ${luaOk} из ${luaAll}`, `неопределённых Java-имён, которые трогает Lua: ${globals}`, `строк лога реального клиента воспроизведено: ${parityKinds} видов`] },
  { id: "render", name: "Картинка мира", weight: 20, measured: true,
    score: Math.max(0, 1 - d1 / 60) * 0.5,
    how: "похожесть кадра на игру 1 − (разница/60), умноженная на 0,5: освещения, персонажей, погоды и анимаций пока нет",
    facts: [`разница кадра 1 с игрой: ${d1} из 255`, `разница кадра 2 (контрольный): ${d2} из 255`, `без маски видимости было: ${d0} из 255`] },
  { id: "sim", name: "Геймплей и зомби", weight: 25, measured: false, score: 0.03,
    how: "оценка автора: есть шаг 30 Гц, движение и столкновения упрощённо; зомби — заглушки с выдуманными параметрами",
    facts: ["движение игрока и столкновения (упрощённо)", "зомби: заглушка", "инвентарь, крафт, навыки, здоровье: нет"] },
  { id: "saves", name: "Сохранения", weight: 5, measured: false, score: 0.1,
    how: "оценка автора: читаем позицию игрока из players.db; остальное нет",
    facts: ["позиция игрока читается из настоящего сохранения", "время суток из map_t.bin не расшифровано"] },
  { id: "ui", name: "Меню, интерфейс, настройки", weight: 5, measured: false, score: 0.05, how: "оценка автора: окно мира и лаунчер есть, интерфейса игры нет", facts: ["лаунчер DeadOxide", "HUD, инвентарь, окна: нет"] },
  { id: "audio", name: "Звук", weight: 5, measured: false, score: 0, how: "оценка автора", facts: ["не начато"] },
  { id: "net", name: "Мультиплеер и транспорт", weight: 5, measured: false, score: 0, how: "оценка автора", facts: ["не начато"] },
];
const totalW = areas.reduce((s, a) => s + a.weight, 0);
for (const a of areas) { a.percent = +(a.score * 100).toFixed(1); }
const overall = +(areas.reduce((s, a) => s + a.score * a.weight, 0) / totalW * 100).toFixed(1);

const out = {
  generated: new Date().toISOString().slice(0, 10),
  game: "Project Zomboid 42.21.0 (Steam build 25485521)",
  overall,
  note: "Общий процент — взвешенная оценка. Измеренные области считаются командами (см. how/facts), остальные — оценка автора и помечены measured:false.",
  areas,
  numbers: { luaOk, luaAll, globals, tilesUsed, tilesMissing, headerParsed, headerTotal, zones: +zones, frameDiff1: d1, frameDiff2: d2, frameDiffBeforeMask: d0, logKindsMatched: parityKinds, goldenLogLines: golden, testsPassed: passed },
};
fs.mkdirSync(path.join(repo, "site"), { recursive: true });
fs.writeFileSync(path.join(repo, "site", "progress.json"), JSON.stringify(out, null, 2));
console.log(JSON.stringify({ overall, areas: areas.map((a) => [a.id, a.percent]), numbers: out.numbers }, null, 1));
