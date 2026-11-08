// List classes in a jar whose constant pool mentions a string.   node tools/jargrep.mjs ../projectzomboid.jar getZombieWalkTowardSpeed
import fs from "node:fs";
import zlib from "node:zlib";
const [jar, needle] = process.argv.slice(2);
const buf = fs.readFileSync(jar);
let e = buf.length - 22;
while (buf.readUInt32LE(e) !== 0x06054b50) e--;
const n = buf.readUInt16LE(e + 10);
let p = buf.readUInt32LE(e + 16);
for (let i = 0; i < n; i++) {
  const method = buf.readUInt16LE(p + 10), csize = buf.readUInt32LE(p + 20), nl = buf.readUInt16LE(p + 28), xl = buf.readUInt16LE(p + 30), cl = buf.readUInt16LE(p + 32), off = buf.readUInt32LE(p + 42);
  const name = buf.toString("utf8", p + 46, p + 46 + nl);
  if (name.endsWith(".class")) {
    const s = off + 30 + buf.readUInt16LE(off + 26) + buf.readUInt16LE(off + 28);
    const raw = buf.subarray(s, s + csize);
    const d = method === 0 ? raw : zlib.inflateRawSync(raw);
    if (d.includes(needle)) console.log(name.slice(0, -6));
  }
  p += 46 + nl + xl + cl;
}
