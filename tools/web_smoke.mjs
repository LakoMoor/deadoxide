// Smoke test of the browser build in Node: mounts the same files the page would read from a picked folder, builds the
// Rosewood cell through the wasm module and checks the result against the desktop client's numbers for that cell.
//   node tools/web_smoke.mjs <game root>
import fs from "node:fs";
import path from "node:path";
import { initSync, mount, WebScene } from "../site/play/pkg/zomboid_web.js";
import { needed, cellOf, SPAWNS } from "../site/play/files.js";

const repo = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const root = path.resolve(repo, process.argv[2] ?? "..");
initSync({ module: fs.readFileSync(path.join(repo, "site/play/pkg/zomboid_web_bg.wasm")) });
const spawn = SPAWNS[0];
const [cx, cy] = cellOf(spawn.x, spawn.y);
let files = 0, bytes = 0;
const walk = (dir, rel) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = rel + "/" + e.name;
    if (e.isDirectory()) walk(path.join(dir, e.name), p);
    else if (needed(p, cx, cy)) { const d = fs.readFileSync(path.join(dir, e.name)); mount(p, d); files++; bytes += d.length; }
  }
};
walk(path.join(root, "media"), "media");
const t0 = Date.now();
const scene = new WebScene(cx, cy, spawn.x, spawn.y, spawn.z);
const inst = scene.instances();
const ranges = Array.from(scene.level_ranges());
let h = 0xcbf29ce484222325n;
for (const b of new Uint8Array(inst.buffer, inst.byteOffset, inst.byteLength)) h = ((h ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn;
const before = [scene.player_x(), scene.player_y()];
// walk "screen up" for one second: the spawn room's wall must stop the player inside the cell
for (let i = 0; i < 30; i++) scene.step(0, -1, 1 / 30);
const out = { files, megabytes: +(bytes / 1048576).toFixed(0), seconds: (Date.now() - t0) / 1000, sprites: inst.length / 9, unresolved: scene.unresolved(), atlas: [scene.atlas_width(), scene.atlas_height(), scene.atlas_layers()], levels: ranges.length - 1, instanceHash: h.toString(16), before, after: [scene.player_x(), scene.player_y()] };
console.log(JSON.stringify(out));
if (process.argv[3]) {
  const want = JSON.parse(process.argv[3]);
  for (const k of Object.keys(want)) if (JSON.stringify(out[k]) !== JSON.stringify(want[k])) { console.error(`MISMATCH ${k}: web ${JSON.stringify(out[k])} native ${JSON.stringify(want[k])}`); process.exit(1); }
  console.log("web build matches the native client on: " + Object.keys(want).join(", "));
}
