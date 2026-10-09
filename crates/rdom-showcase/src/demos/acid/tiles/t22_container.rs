//! Tile 22 — container queries and containment (CSS Conditional 5 §6;
//! CSS Containment 2 §3–§4; CSS Will Change 1 §3).
//!
//! The same card in a 20-cell sidebar and a 36-cell pane (`@container
//! (width >= 30)` making it a row), with a `50cqw` bar under each; a
//! named outer container chosen past an unnamed inner one; a `size`
//! container answering `(height > 2)` where an `inline-size` one answers
//! unknown, and a `100cqmin` bar; a `style(--theme: dark)` query;
//! `contain: paint` clipping a wide child, `contain: layout` holding a
//! `position: fixed` child, `contain: style` scoping a counter,
//! `content-visibility: hidden` keeping a bordered box's border, and
//! `will-change: opacity` lifting a negative `z-index` child above its
//! box's background.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "22",
    title: "Container queries & containment",
    class: "acid-t22",
    page: 6,
    x: 1,
    y: 6,
    w: 58,
    h: 10,
    markup: r#"
<div class="band"><div class="cq side"><div class="card"><b>IMG</b><b>text</b></div><div class="bar"></div></div><div class="cq pane"><div class="card"><b>IMG</b><b>text</b></div><div class="bar"></div></div></div>
<div class="band"><div class="outer"><div class="inner"><b class="q1">named</b> <b class="q2">near</b></div></div><div class="sz"><b class="q3">tall</b><div class="bm"></div></div><div class="is"><b class="q4">unk</b></div><div class="th"><b class="q5">sty</b></div></div>
<div class="band"><div class="cp"><div class="wide">abcdefgh</div></div><div class="cl"><b class="fx">F</b></div><div class="cnt"><b class="ci">a</b> <span class="cs"><b class="ci">b</b></span> <b class="ci">c</b></div><div class="cv">zz</div><div class="wc"><b class="neg">N</b></div></div>
"#,
    css: r#"
.acid-t22 .band { display: flex; gap: 2; align-items: flex-start; margin-bottom: 1; }
.acid-t22 b { font-weight: normal; }
.acid-t22 .cq { container-type: inline-size; }
.acid-t22 .side { width: 20; }
.acid-t22 .card b { display: block; }
.acid-t22 .pane { width: 36; }
@container (width >= 30) { .acid-t22 .card { display: flex; gap: 1; } }
.acid-t22 .bar { width: 50cqw; height: 1; background-color: rgb(0, 128, 128); }
.acid-t22 .outer { container: box / inline-size; width: 24; }
.acid-t22 .inner { container-type: inline-size; width: 10; }
@container box (width > 20) { .acid-t22 .q1 { color: rgb(0, 160, 0); } }
@container (width > 20) { .acid-t22 .q2 { color: rgb(0, 160, 0); } }
.acid-t22 .sz { container-type: size; width: 10; height: 3; }
@container (height > 2) { .acid-t22 .q3, .acid-t22 .q4 { color: rgb(0, 160, 0); } }
.acid-t22 .bm { width: 100cqmin; height: 1; background-color: rgb(0, 0, 128); }
.acid-t22 .is { container-type: inline-size; width: 8; }
.acid-t22 .th { --theme: dark; width: 8; }
@container style(--theme: dark) { .acid-t22 .q5 { color: rgb(0, 160, 0); } }
.acid-t22 .cp { contain: paint; width: 4; height: 1; }
.acid-t22 .wide { width: 8; background-color: rgb(128, 0, 0); }
.acid-t22 .cl { contain: layout; width: 8; height: 2; background-color: rgb(0, 0, 128); }
.acid-t22 .fx { position: fixed; top: 1; left: 1; background-color: rgb(0, 128, 128); }
.acid-t22 .cnt { counter-reset: n; }
.acid-t22 .ci { counter-increment: n; }
.acid-t22 .ci::before { content: counter(n); }
.acid-t22 .cs { contain: style; }
.acid-t22 .cv { content-visibility: hidden; border: solid; width: 4; }
.acid-t22 .wc { position: relative; will-change: opacity; width: 4; height: 1; background-color: rgb(0, 0, 128); }
.acid-t22 .neg { position: absolute; z-index: -1; left: 0; top: 0; color: rgb(255, 255, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
