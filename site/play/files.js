// Which files of the player's install the browser build needs, and how to feed them to the wasm module.
// Shared by the page (files picked by the player) and by tools/web_smoke.mjs (files read from disk in Node).

const MAP = "Muldraugh, KY";

/** Path relative to the game root ("media/...") for a picked file, or null if it is not inside a media folder. */
export function gamePath(relativePath) {
  const p = relativePath.replaceAll("\\", "/");
  const i = p.indexOf("media/");
  return i < 0 || (i > 0 && p[i - 1] !== "/") ? null : p.slice(i);
}

/** Does the scene of cell (cx, cy) need this file? Texture packs: only the 2x tile packs (the largest is ~320 MB). */
export function needed(path, cx, cy) {
  const lower = path.toLowerCase();
  if (/^media\/[^/]+\.tiles$/.test(lower)) return true;
  if (lower.startsWith("media/scripts/") && lower.endsWith(".txt") && !lower.includes("tempnotworking")) return true;
  if (/^media\/maps\/[^/]+\/map\.info$/.test(lower)) return true;
  if (path === "media/lua/shared/Sandbox/Apocalypse.lua") return true;
  const map = `media/maps/${MAP}/`;
  if (path.startsWith(map)) {
    const name = path.slice(map.length);
    if (name.endsWith(".lotheader") || name === "objects.lua" || name === "basements.lua") return true;
    if (name === `world_${cx}_${cy}.lotpack`) return true;
  }
  if (lower.startsWith("media/binmap/") && lower.endsWith(".pzby")) return true;
  if (lower.startsWith("media/basement_access/") && lower.endsWith(".pzby")) return true;
  if (/^media\/texturepacks\/[^/]+\.pack$/.test(lower)) {
    const n = path.split("/").pop();
    return n.includes("2x") && !n.startsWith("UI");
  }
  return false;
}

/** World square -> cell. */
export function cellOf(x, y) {
  return [Math.floor(x / 256), Math.floor(y / 256)];
}

/** Known starting points (from the game's own spawnpoints.lua; only coordinates, no game data). */
export const SPAWNS = [{ name: "Rosewood house", x: 8303.5, y: 11689.5, z: 0 }];
