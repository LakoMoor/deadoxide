// DeadOxide site: renders progress.json, the comparison gallery and the mod registry. No build step, no dependencies.
const REPO = "LakoMoor/deadoxide";
const $ = (s, r = document) => r.querySelector(s);
const el = (tag, attrs = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) k === "class" ? (n.className = v) : k === "html" ? (n.innerHTML = v) : n.setAttribute(k, v);
  for (const c of kids) n.append(c);
  return n;
};
const fmt = (n) => Number(n).toLocaleString("en-US");

async function progress() {
  const p = await (await fetch("progress.json", { cache: "no-store" })).json();
  const C = 2 * Math.PI * 104;
  const fg = $("#ring-fg");
  fg.setAttribute("stroke-dasharray", C.toFixed(1));
  fg.setAttribute("stroke-dashoffset", C.toFixed(1));
  requestAnimationFrame(() => requestAnimationFrame(() => fg.setAttribute("stroke-dashoffset", (C * (1 - p.overall / 100)).toFixed(1))));
  $("#overall").textContent = p.overall.toFixed(1);
  $("#updated").textContent = `Updated ${p.generated} · ${p.game}`;
  const list = $("#areas");
  for (const a of p.areas) {
    const bar = el("div", { class: "bar" }, el("i"));
    const d = el("details", { class: "area" });
    d.append(
      el("summary", {},
        el("div", { class: "t" }, a.name, el("span", { class: `tag ${a.measured ? "m" : "e"}` }, a.measured ? "measured" : "author estimate"), el("span", { class: "tag" }, `weight ${a.weight}`)),
        el("div", { class: "p" }, a.percent.toFixed(a.percent % 1 ? 1 : 0) + "%"),
        bar),
      el("div", { class: "more" }, el("div", {}, a.how), el("ul", {}, ...a.facts.map((f) => el("li", {}, f)))));
    list.append(d);
    setTimeout(() => (bar.firstChild.style.width = a.percent + "%"), 120);
  }
  const n = p.numbers;
  const stats = [
    [`${fmt(n.luaOk)} / ${fmt(n.luaAll)}`, "of the game's Lua files run without error"],
    [`${n.logKindsMatched} / ${n.goldenLogLines}`, "real-client log line kinds reproduced"],
    [fmt(n.zones), "Muldraugh map zones checked against the real Lua"],
    [`${n.frameDiff1} → ${n.frameDiffBeforeMask}`, "mean frame difference from the game out of 255: now → before the visibility mask"],
    [fmt(n.tilesUsed - n.tilesMissing) + " / " + fmt(n.tilesUsed), "tile names found in texture packs"],
    [fmt(n.testsPassed), "automated tests pass"],
  ];
  $("#stats").append(...stats.map(([b, s]) => el("div", { class: "stat" }, el("b", {}, b), el("span", {}, s))));
}

function slider(c) {
  const box = el("div", { class: "slider" });
  const base = el("img", { src: c.port, alt: "port", loading: "lazy" });
  const top = el("img", { src: c.game, alt: "original", class: "top", loading: "lazy" });
  const h = el("div", { class: "handle" });
  box.append(base, top, h, el("span", { class: "lab l" }, "original"), el("span", { class: "lab r" }, "port"));
  let down = false;
  const set = (x) => {
    const r = box.getBoundingClientRect();
    const t = Math.max(0, Math.min(1, (x - r.left) / r.width));
    top.style.clipPath = `inset(0 ${100 - t * 100}% 0 0)`;
    h.style.left = t * 100 + "%";
  };
  top.style.clipPath = "inset(0 50% 0 0)";
  box.addEventListener("pointerdown", (e) => { down = true; box.setPointerCapture(e.pointerId); set(e.clientX); });
  box.addEventListener("pointermove", (e) => down && set(e.clientX));
  box.addEventListener("pointerup", () => (down = false));
  return box;
}

async function gallery() {
  const g = await (await fetch("gallery.json")).json();
  const host = $("#compare-list");
  for (const c of g.compare) {
    host.append(el("div", { class: "cmp" },
      el("h3", {}, c.title, el("span", { class: "d" }, `difference from the game: ${c.diff} of 255 (lower is better)`)),
      slider(c), el("p", {}, c.caption)));
  }
  const po = $("#portonly");
  for (const s of g.portOnly) po.append(el("div", { class: "shot" }, el("img", { src: s.src, alt: s.title, loading: "lazy" }), el("div", {}, el("b", {}, s.title), el("span", {}, s.caption))));
}

async function mods() {
  const host = $("#mods-list");
  try {
    const r = await (await fetch("registry.json", { cache: "no-store" })).json();
    if (!r.mods.length) throw new Error("empty");
    for (const m of r.mods) host.append(el("div", { class: "mod" },
      el("div", {}, el("b", {}, m.name), " ", el("span", { class: "meta" }, `v${m.version} · ${m.author}`)),
      el("a", { class: "btn", href: m.url }, "Download .zip"),
      el("p", {}, m.description)));
  } catch { host.append(el("p", {}, "The mod catalog is unavailable right now.")); }
}

progress(); gallery(); mods();

async function notes() {
  const host = $("#notes-list");
  try {
    const c = await (await fetch("changelog.json", { cache: "no-store" })).json();
    for (const r of c.releases) {
      const box = el("div", { class: "rel" }, el("h3", {}, `v${r.version}`, el("span", {}, r.date)));
      if (r.new?.length) box.append(el("h4", {}, "New"), el("ul", {}, ...r.new.map((s) => el("li", {}, s))));
      if (r.fixed?.length) box.append(el("h4", {}, "Fixed"), el("ul", {}, ...r.fixed.map((s) => el("li", {}, s))));
      host.append(box);
    }
  } catch { host.append(el("p", {}, "Patch notes are unavailable right now.")); }
}
notes();
