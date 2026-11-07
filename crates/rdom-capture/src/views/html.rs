//! The `html` view (`specs/CAPTURE.md` §Views): a frame as one static,
//! self-contained HTML file for a person — the image the rasterizer draws,
//! with an Elements-style inspector over it. Hover a cell: what it shows,
//! the element whose paint it is, that element's boxes outlined, and how
//! its styles came to be (each property's winning declaration, selector
//! and `file:line`). Hover or click an element in the list to do the same.
//! No server and no network: the data is embedded as JSON, and the same
//! frame always gives the same bytes.

use serde::Serialize;

/// What the page shows. Every field is serialized into the page as is.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Report<'a> {
    pub title: String,
    pub meta: &'a crate::capture::Meta,
    pub profile: &'a str,
    pub scale: u32,
    /// One cell's size in the image, in pixels.
    pub cell: [u32; 2],
    /// The PNG, base64.
    pub png: String,
    pub width: u32,
    pub height: u32,
    /// The glyphs the font lacks.
    pub missing: Vec<String>,
    pub grid: &'a crate::capture::Grid,
    pub owners: &'a crate::capture::Owners,
    /// The elements in document order, as `[key, info]` pairs: node keys
    /// are numbers, which a JSON object would reorder.
    pub nodes: Vec<(&'a str, &'a crate::capture::NodeInfo)>,
    pub boxes: Vec<(&'a str, &'a crate::capture::Boxes)>,
    /// Each element's explanation, by node key, in document order.
    pub explain: Vec<(String, serde_json::Value)>,
}

/// The page for `report`.
pub(crate) fn page(report: &Report<'_>) -> String {
    let data = serde_json::to_string(report).expect("a report serializes");
    // Inside <script>, `</` would end the element early: JSON lets `/`
    // be escaped.
    let data = data.replace("</", "<\\/");
    let (head, tail) = TEMPLATE
        .split_once("{{DATA}}")
        .expect("the template has a data slot");
    let mut out = head.replace("{{TITLE}}", &escape(&report.title));
    out.push_str(&data);
    out.push_str(tail);
    out
}

/// `s` safe as HTML text.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const TEMPLATE: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{TITLE}}</title>
<style>
:root { --bg:#16181d; --panel:#1e2128; --line:#2e323c; --fg:#d8dce4; --dim:#8a92a3; --accent:#e8b04a; --pad:#5fbf7f; --content:#5f9fdf; }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg); font:13px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
header { padding:10px 16px; border-bottom:1px solid var(--line); }
header h1 { margin:0 0 2px; font-size:15px; }
header .meta { color:var(--dim); }
header .warn { color:var(--accent); }
main { display:flex; gap:16px; padding:16px; align-items:flex-start; flex-wrap:wrap; }
#frame { position:relative; flex:none; line-height:0; border:1px solid var(--line); cursor:crosshair; }
#frame img { image-rendering:pixelated; display:block; }
.box { position:absolute; pointer-events:none; }
#b-border { outline:2px solid var(--accent); }
#b-padding { background:rgba(95,191,127,.18); }
#b-content { background:rgba(95,159,223,.22); }
#b-cell { outline:1px solid #fff; mix-blend-mode:difference; }
aside { flex:1 1 420px; min-width:320px; display:flex; flex-direction:column; gap:12px; }
section { background:var(--panel); border:1px solid var(--line); border-radius:6px; padding:10px 12px; }
section h2 { margin:0 0 6px; font-size:12px; text-transform:uppercase; letter-spacing:.06em; color:var(--dim); }
.kv { display:grid; grid-template-columns:max-content 1fr; gap:2px 12px; }
.kv b { color:var(--dim); font-weight:normal; }
table { border-collapse:collapse; width:100%; }
td { padding:2px 8px 2px 0; vertical-align:top; border-top:1px solid var(--line); }
td.p { color:#9fc3ff; white-space:nowrap; }
td.s { color:var(--dim); word-break:break-all; }
.chip { display:inline-block; width:10px; height:10px; border:1px solid var(--line); vertical-align:-1px; margin-right:4px; }
#elements { max-height:360px; overflow:auto; }
#elements div { white-space:nowrap; cursor:pointer; padding:0 4px; border-radius:3px; }
#elements div:hover, #elements div.on { background:#2b3140; }
.tag { color:#e88f8f; } .id { color:var(--accent); } .cls { color:var(--pad); }
.muted { color:var(--dim); }
</style>
</head>
<body>
<header><h1 id="title"></h1><div class="meta" id="meta"></div><div class="warn" id="warn"></div></header>
<main>
  <div id="frame"><img id="img" alt="the frame">
    <div class="box" id="b-border"></div><div class="box" id="b-padding"></div><div class="box" id="b-content"></div><div class="box" id="b-cell"></div>
  </div>
  <aside>
    <section><h2>Cell</h2><div id="cell" class="muted">Hover the frame. Click to pin, Esc to unpin.</div></section>
    <section><h2>Element</h2><div id="element" class="muted">—</div></section>
    <section><h2>Elements</h2><div id="elements"></div></section>
  </aside>
</main>
<script type="application/json" id="data">{{DATA}}</script>
<script>
"use strict";
const D = JSON.parse(document.getElementById("data").textContent);
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"})[c]);
const [CW, CH] = D.cell;
const nodes = new Map(D.nodes);
const boxes = new Map(D.boxes);
const explain = new Map(D.explain);
let pinned = null;

$("title").textContent = D.title;
const m = D.meta;
$("meta").textContent = `${m.env.viewport.width}×${m.env.viewport.height} cells · t=${m.time_ms}ms · ${m.grid_hash} · profile ${D.profile} ×${D.scale} · rdom-capture ${m.rdom_capture} · rdom-tui ${m.rdom_tui}`;
const warns = (m.warnings || []).slice();
if (D.missing.length) warns.push(`glyphs the font lacks (drawn as boxes): ${D.missing.join(" ")}`);
$("warn").textContent = warns.join(" · ");
const img = $("img");
img.src = "data:image/png;base64," + D.png;
img.width = D.width; img.height = D.height;

function runAt(rows, x, y) {
  for (const r of rows[y] || []) if (x >= r.x && x < r.x + r.w) return r;
  return null;
}
function color(c) {
  if (c === "default") return `<span class="muted">default</span>`;
  const css = /^#[0-9a-f]{6}/.test(c) ? c.slice(0, 7) : null;
  return (css ? `<span class="chip" style="background:${css}"></span>` : "") + esc(c);
}
function label(key) {
  const n = nodes.get(key);
  if (!n) return esc(key);
  return `<span class="tag">${esc(n.tag)}</span>` + (n.id ? `<span class="id">#${esc(n.id)}</span>` : "") +
    (n.classes || []).map((c) => `<span class="cls">.${esc(c)}</span>`).join("") + ` <span class="muted">${esc(key)}</span>`;
}
function place(el, r) {
  if (!r) { el.style.display = "none"; return; }
  Object.assign(el.style, { display: "block", left: r[0] * CW + "px", top: r[1] * CH + "px", width: r[2] * CW + "px", height: r[3] * CH + "px" });
}
function showBoxes(key) {
  const b = key ? boxes.get(key) : null;
  place($("b-border"), b && b.border); place($("b-padding"), b && b.padding); place($("b-content"), b && b.content);
}
function showElement(key, part) {
  showBoxes(key);
  document.querySelectorAll("#elements div").forEach((d) => d.classList.toggle("on", d.dataset.key === key));
  if (!key) { $("element").innerHTML = `<span class="muted">nothing painted here</span>`; return; }
  const n = nodes.get(key), e = explain.get(key), b = boxes.get(key);
  let h = `<div class="kv"><b>element</b><span>${label(key)}</span>`;
  if (n) h += `<b>path</b><span>${esc(n.path)}</span>`;
  if (part) h += `<b>part</b><span>${esc(part)}</span>`;
  if (b) h += `<b>border box</b><span>${b.border.join(", ")}</span>` + (b.content ? `<b>content</b><span>${b.content.join(", ")}</span>` : "");
  if (e && e.animations && e.animations.length) h += `<b>animating</b><span>${esc(e.animations.join(", "))}</span>`;
  h += `</div>`;
  if (e && e.style && e.style.length) {
    h += `<table>`;
    for (const p of e.style) {
      const d = p.declaration;
      const where = d ? [d.selector, d.source].filter(Boolean).join("  ") : (p.ancestor ? `inherited from ${p.ancestor}` : p.from);
      const over = (p.overridden || []).map((o) => `${o.value} (${o.selector || o.origin})`).join("; ");
      h += `<tr title="${esc(over ? "overrode: " + over : "")}"><td class="p">${esc(p.property)}</td><td>${esc(p.value)}${d && d.important ? " !important" : ""}</td><td class="s">${esc(where)}</td></tr>`;
    }
    h += `</table>`;
  }
  $("element").innerHTML = h;
}
function showCell(x, y) {
  place($("b-cell"), [x, y, 1, 1]);
  const g = runAt(D.grid.rows, x, y), o = runAt(D.owners.rows, x, y);
  let h = `<div class="kv"><b>cell</b><span>${x}, ${y}</span>`;
  if (g) {
    h += `<b>run</b><span>${g.x}–${g.x + g.w - 1}: “${esc(g.text)}”</span><b>fg</b><span>${color(g.fg_shown || g.fg)}</span><b>bg</b><span>${color(g.bg_shown || g.bg)}</span>`;
    if (g.mods && g.mods.length) h += `<b>attrs</b><span>${esc(g.mods.join(" "))}</span>`;
  }
  if (o && o.joined && o.joined.length) h += `<b>joins</b><span>${o.joined.map(label).join(", ")}</span>`;
  if (o && o.layer) h += `<b>via layer</b><span>${label(o.layer)}</span>`;
  $("cell").innerHTML = h + `</div>`;
  showElement(o ? o.node : null, o ? o.part : null);
}
function cellOf(ev) {
  const r = img.getBoundingClientRect();
  return [Math.floor((ev.clientX - r.left) / CW), Math.floor((ev.clientY - r.top) / CH)];
}
$("frame").addEventListener("mousemove", (ev) => { if (!pinned) showCell(...cellOf(ev)); });
$("frame").addEventListener("click", (ev) => { pinned = cellOf(ev); showCell(...pinned); });
document.addEventListener("keydown", (ev) => { if (ev.key === "Escape") pinned = null; });

const depth = new Map();
let list = "";
for (const [key, n] of nodes) {
  const d = n.parent && depth.has(n.parent) ? depth.get(n.parent) + 1 : 0;
  depth.set(key, d);
  list += `<div data-key="${esc(key)}" style="padding-left:${4 + d * 14}px">${label(key)}</div>`;
}
$("elements").innerHTML = list;
$("elements").addEventListener("mouseover", (ev) => { const k = ev.target.closest("div[data-key]"); if (k && !pinned) showElement(k.dataset.key); });
$("elements").addEventListener("click", (ev) => { const k = ev.target.closest("div[data-key]"); if (k) { pinned = true; showElement(k.dataset.key); } });
</script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn text_is_escaped_for_html() {
        assert_eq!(
            escape(r#"<a href="x">&</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }
}
