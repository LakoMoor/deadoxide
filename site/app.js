// DeadOxide site: renders progress.json, the comparison gallery and the mod registry. No build step, no dependencies.
const REPO = "LakoMoor/deadoxide";
const $ = (s, r = document) => r.querySelector(s);
const el = (tag, attrs = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) k === "class" ? (n.className = v) : k === "html" ? (n.innerHTML = v) : n.setAttribute(k, v);
  for (const c of kids) n.append(c);
  return n;
};
const fmt = (n) => Number(n).toLocaleString("ru-RU");

async function progress() {
  const p = await (await fetch("progress.json", { cache: "no-store" })).json();
  const C = 2 * Math.PI * 104;
  const fg = $("#ring-fg");
  fg.setAttribute("stroke-dasharray", C.toFixed(1));
  fg.setAttribute("stroke-dashoffset", C.toFixed(1));
  requestAnimationFrame(() => requestAnimationFrame(() => fg.setAttribute("stroke-dashoffset", (C * (1 - p.overall / 100)).toFixed(1))));
  $("#overall").textContent = p.overall.toFixed(1).replace(".", ",");
  $("#updated").textContent = `Обновлено ${p.generated} · ${p.game}`;
  const list = $("#areas");
  for (const a of p.areas) {
    const bar = el("div", { class: "bar" }, el("i"));
    const d = el("details", { class: "area" });
    d.append(
      el("summary", {},
        el("div", { class: "t" }, a.name, el("span", { class: `tag ${a.measured ? "m" : "e"}` }, a.measured ? "измерено" : "оценка автора"), el("span", { class: "tag" }, `вес ${a.weight}`)),
        el("div", { class: "p" }, a.percent.toFixed(a.percent % 1 ? 1 : 0).replace(".", ",") + "%"),
        bar),
      el("div", { class: "more" }, el("div", {}, a.how), el("ul", {}, ...a.facts.map((f) => el("li", {}, f)))));
    list.append(d);
    setTimeout(() => (bar.firstChild.style.width = a.percent + "%"), 120);
  }
  const n = p.numbers;
  const stats = [
    [`${fmt(n.luaOk)} / ${fmt(n.luaAll)}`, "Lua-файлов игры выполняются без ошибки"],
    [`${n.logKindsMatched} / ${n.goldenLogLines}`, "видов строк лога реального клиента воспроизведено"],
    [fmt(n.zones), "зон карты Muldraugh сверено с настоящим Lua"],
    [`${n.frameDiff1} → ${n.frameDiffBeforeMask}`, "разница кадра с игрой из 255 (сейчас ← без маски)"],
    [fmt(n.tilesUsed - n.tilesMissing) + " / " + fmt(n.tilesUsed), "имён тайлов находятся в пакетах текстур"],
    [fmt(n.testsPassed), "автотестов проходит"],
  ];
  $("#stats").append(...stats.map(([b, s]) => el("div", { class: "stat" }, el("b", {}, b), el("span", {}, s))));
}

function slider(c) {
  const box = el("div", { class: "slider" });
  const base = el("img", { src: c.port, alt: "порт", loading: "lazy" });
  const top = el("img", { src: c.game, alt: "оригинал", class: "top", loading: "lazy" });
  const h = el("div", { class: "handle" });
  box.append(base, top, h, el("span", { class: "lab l" }, "оригинал"), el("span", { class: "lab r" }, "порт"));
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
      el("h3", {}, c.title, el("span", { class: "d" }, `разница с игрой: ${String(c.diff).replace(".", ",")} из 255 (меньше — лучше)`)),
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
      el("a", { class: "btn", href: m.url }, "Скачать .zip"),
      el("p", {}, m.description)));
  } catch { host.append(el("p", {}, "Каталог модов пока недоступен.")); }
}

progress(); gallery(); mods();
