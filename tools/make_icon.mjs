// DeadOxide icon: a rusted gear (Rust) with a tombstone cut out of it (the dead). One geometry -> SVG, PNG sizes and a Windows .ico.
//   node tools/make_icon.mjs        writes site/assets/{favicon.svg,icon-512.png,icon-256.png,icon.ico} and assets/icon-256.png
import fs from "node:fs";
import zlib from "node:zlib";
import path from "node:path";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1")), "..");
const TEETH = 12;
const gearR = (t) => {
  // t in [0,1): smooth-edged square wave of 12 teeth between 0.31 and 0.395
  const k = (t * TEETH) % 1;
  const on = k < 0.5 ? Math.min(1, k / 0.06, (0.5 - k) / 0.06) : 0;
  return 0.31 + 0.085 * Math.max(0, on);
};
const inRoundRect = (x, y, r) => {
  const ax = Math.abs(x) - (0.5 - r), ay = Math.abs(y) - (0.5 - r);
  return ax <= 0 || ay <= 0 ? Math.abs(x) <= 0.5 && Math.abs(y) <= 0.5 : Math.hypot(ax, ay) <= r;
};
const inStone = (x, y) => (Math.abs(x) <= 0.115 && y >= -0.02 && y <= 0.17) || Math.hypot(x, y + 0.02) <= 0.115;
const inCross = (x, y) => (Math.abs(x) <= 0.014 && y >= -0.1 && y <= 0.09) || (Math.abs(y + 0.04) <= 0.014 && Math.abs(x) <= 0.055);
const mix = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));

function px(x, y) {
  // returns [r,g,b,a] for a point in [-0.5,0.5]^2
  if (!inRoundRect(x, y, 0.2)) return [0, 0, 0, 0];
  const bg = mix([34, 26, 22], [14, 11, 10], (y + 0.5));
  const r = Math.hypot(x, y);
  const th = ((Math.atan2(y, x) / (2 * Math.PI)) + 1) % 1;
  if (r <= gearR(th)) {
    if (inCross(x, y)) return [226, 98, 42, 255];
    if (inStone(x, y)) return [23, 17, 14, 255];
    const rust = mix([232, 110, 46], [150, 56, 22], Math.min(1, (y + 0.4) / 0.8));
    const grain = ((Math.sin(x * 90) * Math.sin(y * 70)) * 6) | 0;
    return [rust[0] + grain, rust[1] + (grain >> 1), rust[2], 255];
  }
  return [...bg, 255];
}

function render(size, ss = 3) {
  const out = Buffer.alloc(size * size * 4);
  for (let j = 0; j < size; j++) for (let i = 0; i < size; i++) {
    let r = 0, g = 0, b = 0, a = 0;
    for (let sy = 0; sy < ss; sy++) for (let sx = 0; sx < ss; sx++) {
      const p = px((i + (sx + 0.5) / ss) / size - 0.5, (j + (sy + 0.5) / ss) / size - 0.5);
      r += p[0] * p[3]; g += p[1] * p[3]; b += p[2] * p[3]; a += p[3];
    }
    const o = (j * size + i) * 4, n = ss * ss;
    out[o] = a ? r / a : 0; out[o + 1] = a ? g / a : 0; out[o + 2] = a ? b / a : 0; out[o + 3] = a / n;
  }
  return out;
}

const crcTable = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc32 = (buf) => { let c = 0xffffffff; for (const b of buf) c = crcTable[(c ^ b) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
function png(size, rgba) {
  const chunk = (t, d) => { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(crc32(td)); return Buffer.concat([l, td, c]); };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4); ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y++) { raw[y * (size * 4 + 1)] = 0; rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4); }
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", zlib.deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0))]);
}

// ---- SVG with the same geometry ----
function svg() {
  const pts = [];
  const N = 480;
  for (let i = 0; i < N; i++) { const t = i / N; const r = gearR(t), a = t * 2 * Math.PI; pts.push([(0.5 + r * Math.cos(a)) * 512, (0.5 + r * Math.sin(a)) * 512]); }
  const gear = "M" + pts.map((p) => p[0].toFixed(1) + " " + p[1].toFixed(1)).join("L") + "Z";
  const S = 512, c = S / 2;
  const stone = `M${c - 0.115 * S} ${c + 0.17 * S}V${c - 0.02 * S}A${0.115 * S} ${0.115 * S} 0 0 1 ${c + 0.115 * S} ${c - 0.02 * S}V${c + 0.17 * S}Z`;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" role="img" aria-label="DeadOxide">
<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#221a16"/><stop offset="1" stop-color="#0e0b0a"/></linearGradient>
<linearGradient id="rust" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#e86e2e"/><stop offset="1" stop-color="#963816"/></linearGradient></defs>
<rect width="512" height="512" rx="102" fill="url(#bg)"/>
<path d="${gear}" fill="url(#rust)"/>
<path d="${stone}" fill="#17110e"/>
<path d="M${c - 0.014 * S} ${c - 0.1 * S}h${0.028 * S}v${0.19 * S}h${-0.028 * S}zM${c - 0.055 * S} ${c - 0.054 * S}h${0.11 * S}v${0.028 * S}h${-0.11 * S}z" fill="#e2622a"/>
</svg>
`;
}

const outDir = path.join(root, "site", "assets");
fs.mkdirSync(outDir, { recursive: true });
fs.mkdirSync(path.join(root, "assets"), { recursive: true });
fs.writeFileSync(path.join(outDir, "favicon.svg"), svg());
const sizes = { 512: png(512, render(512)), 256: png(256, render(256)), 64: png(64, render(64, 4)), 32: png(32, render(32, 4)) };
fs.writeFileSync(path.join(outDir, "icon-512.png"), sizes[512]);
fs.writeFileSync(path.join(outDir, "icon-256.png"), sizes[256]);
fs.writeFileSync(path.join(root, "assets", "icon-256.png"), sizes[256]);
// .ico with embedded PNGs (Windows Vista+)
const entries = [256, 64, 32].map((s) => sizes[s]);
const head = Buffer.alloc(6); head.writeUInt16LE(1, 2); head.writeUInt16LE(entries.length, 4);
let off = 6 + 16 * entries.length;
const dir = entries.map((buf, i) => { const s = [256, 64, 32][i]; const e = Buffer.alloc(16); e[0] = s === 256 ? 0 : s; e[1] = s === 256 ? 0 : s; e.writeUInt16LE(1, 4); e.writeUInt16LE(32, 6); e.writeUInt32LE(buf.length, 8); e.writeUInt32LE(off, 12); off += buf.length; return e; });
fs.writeFileSync(path.join(outDir, "icon.ico"), Buffer.concat([head, ...dir, ...entries]));
fs.copyFileSync(path.join(outDir, "icon.ico"), path.join(root, "assets", "icon.ico"));
console.log("icon written:", fs.readdirSync(outDir).join(", "));
