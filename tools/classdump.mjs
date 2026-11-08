// Dump fields (with ConstantValue) and method names of a class inside projectzomboid.jar, without decompiling.
//   node tools/classdump.mjs ../projectzomboid.jar zombie/characters/IsoZombie [filter]
// Output is a HINT about the original (names and compile-time constants), never copied code.
import fs from "node:fs";
import zlib from "node:zlib";

const [jar, cls, filter] = process.argv.slice(2);
const buf = fs.readFileSync(jar);
// end of central directory
let e = buf.length - 22;
while (buf.readUInt32LE(e) !== 0x06054b50) e--;
const n = buf.readUInt16LE(e + 10);
let p = buf.readUInt32LE(e + 16);
let data = null;
for (let i = 0; i < n; i++) {
  const method = buf.readUInt16LE(p + 10), csize = buf.readUInt32LE(p + 20), nl = buf.readUInt16LE(p + 28), xl = buf.readUInt16LE(p + 30), cl = buf.readUInt16LE(p + 32), off = buf.readUInt32LE(p + 42);
  const name = buf.toString("utf8", p + 46, p + 46 + nl);
  if (name === cls + ".class") {
    const lnl = buf.readUInt16LE(off + 26), lxl = buf.readUInt16LE(off + 28), s = off + 30 + lnl + lxl;
    const raw = buf.subarray(s, s + csize);
    data = method === 0 ? raw : zlib.inflateRawSync(raw);
    break;
  }
  p += 46 + nl + xl + cl;
}
if (!data) { console.error("class not found:", cls); process.exit(1); }

let o = 8;
const u1 = () => data[o++], u2 = () => { const v = data.readUInt16BE(o); o += 2; return v; }, u4 = () => { const v = data.readUInt32BE(o); o += 4; return v; };
const cpn = u2();
const cp = [null];
for (let i = 1; i < cpn; i++) {
  const t = u1();
  if (t === 1) { const l = u2(); cp[i] = { t, v: data.toString("utf8", o, o + l) }; o += l; }
  else if (t === 3) cp[i] = { t, v: data.readInt32BE((o += 4) - 4) };
  else if (t === 4) cp[i] = { t, v: data.readFloatBE((o += 4) - 4) };
  else if (t === 5) { cp[i] = { t, v: data.readBigInt64BE(o) }; o += 8; i++; }
  else if (t === 6) { cp[i] = { t, v: data.readDoubleBE(o) }; o += 8; i++; }
  else if (t === 7 || t === 8 || t === 16 || t === 19 || t === 20) cp[i] = { t, v: u2() };
  else if (t === 15) { cp[i] = { t, v: [u1(), u2()] }; }
  else cp[i] = { t, v: [u2(), u2()] };
}
const utf = (i) => cp[i]?.v;
o += 6; // access, this, super
const ic = u2(); o += ic * 2;
const code = [];
const readAttrs = (cb) => { const c = u2(); for (let k = 0; k < c; k++) { const nm = utf(u2()), l = u4(), at = o; cb(nm, at, l); o = at + l; } };
const out = [];
const members = (kind) => {
  const c = u2();
  for (let k = 0; k < c; k++) {
    const acc = u2(), name = utf(u2()), desc = utf(u2());
    let cv;
    readAttrs((an, at, len) => { if (an === "Code") code.push({ name, desc, at: at + 8, len: data.readUInt32BE(at + 4) }); if (an === "ConstantValue") { const idx = data.readUInt16BE(at); const e2 = cp[idx]; cv = e2.t === 8 ? JSON.stringify(utf(e2.v)) : String(e2.v); } });
    out.push(`${kind} ${acc & 8 ? "static " : ""}${acc & 16 ? "final " : ""}${name} ${desc}${cv !== undefined ? " = " + cv : ""}`);
  }
};
members("field");
members("method");
for (const l of out) if (!filter || l.toLowerCase().includes(filter.toLowerCase())) console.log(l);

// ---- optional: --code <method> lists the constants, fields and calls a method touches, in bytecode order (no decompilation) ----
const ci = process.argv.indexOf("--code");
if (ci > 0) {
  const want = process.argv[ci + 1];
  const ref = (i) => { const r = cp[i]; if (!r) return "?"; if (r.t === 9 || r.t === 10 || r.t === 11) { const c = cp[r.v[0]].v, nt = cp[r.v[1]].v; return `${utf(c)}.${utf(nt[0])}${r.t === 9 ? "" : utf(nt[1])}`; } return String(r.v); };
  const cst = (i) => { const r = cp[i]; return r.t === 8 ? JSON.stringify(utf(r.v)) : String(r.v); };
  // operand byte counts per opcode (JVMS table); -1 = special
  const len = new Array(256).fill(0);
  for (const op of [0x10, 0x12, 0x15, 0x16, 0x17, 0x18, 0x19, 0x36, 0x37, 0x38, 0x39, 0x3a, 0xa9, 0xbc]) len[op] = 1;
  for (const op of [0x11, 0x13, 0x14, 0x84, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xbb, 0xbd, 0xc0, 0xc1, 0xc6, 0xc7, 0xa7, 0xa8]) len[op] = 2;
  for (let op = 0x99; op <= 0xa6; op++) len[op] = 2;
  len[0xc5] = 3; for (const op of [0xb9, 0xba, 0xc8, 0xc9]) len[op] = 4;
  for (const m of code.filter((c) => c.name === want)) {
    console.log(`\n== ${m.name}${m.desc} (${m.len} bytes) ==`);
    let pc = 0; const base = m.at, end = m.at + m.len;
    while (base + pc < end) {
      const op = data[base + pc]; let ln = 1 + len[op]; const a = base + pc + 1;
      if (op === 0x12) console.log(`  ldc ${cst(data[a])}`);
      else if (op === 0x13 || op === 0x14) console.log(`  ldc ${cst(data.readUInt16BE(a))}`);
      else if (op >= 0xb2 && op <= 0xb8 || op === 0xb9) console.log(`  ${["getstatic","putstatic","getfield","putfield","invokevirtual","invokespecial","invokestatic","invokeinterface"][op - 0xb2]} ${ref(data.readUInt16BE(a))}`);
      else if (op === 0xaa) { const p4 = (base + pc + 4) & ~3; const lo = data.readInt32BE(p4 + 4), hi = data.readInt32BE(p4 + 8); ln = p4 + 12 + (hi - lo + 1) * 4 - (base + pc); }
      else if (op === 0xab) { const p4 = (base + pc + 4) & ~3; ln = p4 + 8 + data.readInt32BE(p4 + 4) * 8 - (base + pc); }
      else if (op === 0xc4) ln = data[a] === 0x84 ? 6 : 4;
      else if (op >= 0x0b && op <= 0x0f || op === 0x03 || op === 0x04 || op === 0x05 || op === 0x06 || op === 0x07 || op === 0x08) console.log(`  const op=0x${op.toString(16)}`);
      pc += ln;
    }
  }
}
