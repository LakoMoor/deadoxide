import init, { mount, WebScene } from "./pkg/zomboid_web.js";
import { gamePath, needed, cellOf, SPAWNS } from "./files.js";
import { Renderer } from "./render.js";

const $ = (id) => document.getElementById(id);
const status = (t) => ($("status").textContent = t);
const keys = new Set();
let scene = null, renderer = null, zoom = 1, levelTop = 1, last = 0;

async function mountFiles(entries, cx, cy) {
  // entries: [{ path, read: () => Promise<ArrayBuffer>, size }]
  const wanted = entries.filter((e) => needed(e.path, cx, cy));
  const total = wanted.reduce((s, e) => s + e.size, 0);
  let done = 0;
  for (const e of wanted) {
    mount(e.path, new Uint8Array(await e.read()));
    done += e.size;
    status(`Reading your game files: ${(done / 1048576).toFixed(0)} of ${(total / 1048576).toFixed(0)} MB`);
  }
  return wanted.length;
}

async function start(entries) {
  const spawn = SPAWNS[0];
  const [cx, cy] = cellOf(spawn.x, spawn.y);
  $("pick").hidden = true;
  status("Loading the engine");
  await init();
  const n = await mountFiles(entries, cx, cy);
  if (!entries.some((e) => e.path === "media/newtiledefinitions.tiles")) {
    $("pick").hidden = false;
    return status(`That folder does not look like a Project Zomboid install (no media/newtiledefinitions.tiles among ${entries.length} files). Pick the folder that contains "media".`);
  }
  status(`Building cell ${cx}_${cy} from ${n} files (tile definitions, 4,065 cell headers, basements, objects, collision)`);
  await new Promise((r) => setTimeout(r, 30));
  const t0 = performance.now();
  scene = new WebScene(cx, cy, spawn.x, spawn.y, spawn.z);
  renderer = new Renderer($("view"));
  renderer.setAtlas(scene.atlas_width(), scene.atlas_height(), scene.atlas_layers(), (i) => scene.atlas_layer(i));
  scene.drop_atlas_pixels();
  const inst = scene.instances();
  const all = new Float32Array(inst.length + 9);
  all.set(inst);
  renderer.setInstances(all);
  levelTop = spawn.z - scene.min_level() + 1;
  status(`Cell ${cx}_${cy}: ${inst.length / 9} sprites, ${scene.unresolved()} without texture, built in ${((performance.now() - t0) / 1000).toFixed(1)} s. WASD or arrows: walk. Wheel: zoom. [ and ]: floors.`);
  $("view").hidden = false;
  $("view").focus();
  requestAnimationFrame(frame);
}

function frame(now) {
  const dt = Math.min((now - last) / 1000, 0.25);
  last = now;
  let sx = 0, sy = 0;
  if (keys.has("KeyW") || keys.has("ArrowUp")) sy -= 1;
  if (keys.has("KeyS") || keys.has("ArrowDown")) sy += 1;
  if (keys.has("KeyA") || keys.has("ArrowLeft")) sx -= 1;
  if (keys.has("KeyD") || keys.has("ArrowRight")) sx += 1;
  if (sx || sy) scene.step(sx, sy, dt);
  const c = $("view");
  if (c.width !== c.clientWidth || c.height !== c.clientHeight) { c.width = c.clientWidth; c.height = c.clientHeight; }
  renderer.setMarker(scene.player_marker());
  const ranges = scene.level_ranges();
  renderer.draw(scene.player_px(), zoom, ranges[Math.max(1, Math.min(levelTop, ranges.length - 1))]);
  $("pos").textContent = `x ${scene.player_x().toFixed(1)}  y ${scene.player_y().toFixed(1)}  floors up to ${levelTop + scene.min_level() - 1}`;
  requestAnimationFrame(frame);
}

addEventListener("keydown", (e) => {
  keys.add(e.code);
  if (!scene) return;
  if (e.code === "BracketLeft") levelTop = Math.max(1, levelTop - 1);
  if (e.code === "BracketRight") levelTop = Math.min(scene.level_ranges().length - 1, levelTop + 1);
  if (e.code.startsWith("Arrow")) e.preventDefault();
});
addEventListener("keyup", (e) => keys.delete(e.code));
$("view").addEventListener("wheel", (e) => { e.preventDefault(); zoom = Math.max(0.1, Math.min(4, zoom * 1.1 ** (-e.deltaY / 100))); }, { passive: false });

$("folder").addEventListener("change", (ev) => {
  const entries = [];
  for (const f of ev.target.files) {
    const path = gamePath(f.webkitRelativePath);
    if (path) entries.push({ path, size: f.size, read: () => f.arrayBuffer() });
  }
  start(entries).catch((e) => { console.error(e); $("pick").hidden = false; status("Failed: " + (e.message || e)); });
});

// Development only: ?dev=<base url of a local server that serves the game folder>&list=<url of a JSON file list>.
const q = new URLSearchParams(location.search);
if (q.get("dev") && ["localhost", "127.0.0.1"].includes(location.hostname)) {
  const base = q.get("dev");
  fetch(q.get("list")).then((r) => r.json()).then((list) =>
    start(list.map(([path, size]) => ({ path, size, read: () => fetch(base + encodeURI(path)).then((r) => r.arrayBuffer()) })))
  ).catch((e) => status("Failed: " + e.message));
}
