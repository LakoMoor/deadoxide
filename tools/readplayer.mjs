import { DatabaseSync } from "node:sqlite";
const db = new DatabaseSync(process.argv[2], { readOnly: true });
console.log("tables:", db.prepare("select name from sqlite_master where type='table'").all().map((r) => r.name).join(","));
const cols = db.prepare("pragma table_info(localPlayers)").all().map((c) => c.name);
console.log("columns:", cols.join(","));
for (const r of db.prepare("select * from localPlayers").all()) {
  const o = {};
  for (const k of Object.keys(r)) o[k] = r[k] instanceof Uint8Array ? `blob(${r[k].length})` : r[k];
  console.log(JSON.stringify(o));
}
