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
const readAttrs = (cb) => { const c = u2(); for (let k = 0; k < c; k++) { const nm = utf(u2()), l = u4(), at = o; cb(nm, at, l); o = at + l; } };
const out = [];
const members = (kind) => {
  const c = u2();
  for (let k = 0; k < c; k++) {
    const acc = u2(), name = utf(u2()), desc = utf(u2());
    let cv;
    readAttrs((an, at) => { if (an === "ConstantValue") { const idx = data.readUInt16BE(at); const e2 = cp[idx]; cv = e2.t === 8 ? JSON.stringify(utf(e2.v)) : String(e2.v); } });
    out.push(`${kind} ${acc & 8 ? "static " : ""}${acc & 16 ? "final " : ""}${name} ${desc}${cv !== undefined ? " = " + cv : ""}`);
  }
};
members("field");
members("method");
for (const l of out) if (!filter || l.toLowerCase().includes(filter.toLowerCase())) console.log(l);
