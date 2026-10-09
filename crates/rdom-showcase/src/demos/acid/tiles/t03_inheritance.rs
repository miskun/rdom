//! Tile 3 — inheritance and the CSS-wide keywords (CSS Cascade 4 §3.1,
//! §7.3; CSS Variables 1 §2–§4).
//!
//! Inherited `color` and font properties; the non-inherited
//! `background-color` shown on a child that overflows its one-row parent
//! (only an inherited value paints that row); `initial`, `inherit` and
//! `unset` on both kinds; custom properties with `var()`, fallbacks and a
//! missing variable (invalid at computed-value time).

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "3",
    title: "Inheritance & keywords",
    class: "acid-t3",
    page: 1,
    x: 81,
    y: 1,
    w: 38,
    h: 9,
    markup: r#"
<div class="i-col">color <span>kid</span> <em class="i-init">initial</em> <span class="i-unset">unset</span> <span class="i-mid"><span class="i-inh">inherit</span></span></div>
<div class="i-font">bold <span>kid <a class="i-norm">normal</a></span></div>
<div class="i-bg">parent<div class="i-k1">no-inherit</div></div>
<div class="i-bg">parent<div class="i-k2">inherit</div></div>
<div class="i-bg">parent<div class="i-k3">unset</div></div>
<div class="i-var"><span class="v0">var</span> <span class="v1">fallback</span> <span class="v2">missing</span> <span class="v3">nested</span> <span class="v4"><a>deep</a></span></div>
"#,
    css: r#"
.acid-t3 .i-col { color: rgb(0, 160, 0); }
.acid-t3 .i-init { color: initial; }
.acid-t3 .i-unset { color: rgb(192, 0, 0); color: unset; }
.acid-t3 .i-mid { color: rgb(0, 0, 192); }
.acid-t3 .i-inh { color: rgb(192, 0, 0); color: inherit; }
.acid-t3 .i-font { font-weight: bold; font-style: italic; color: rgb(0, 160, 0); }
.acid-t3 .i-norm { font-weight: initial; font-style: normal; }
.acid-t3 .i-bg { background-color: rgb(0, 0, 128); height: 1; margin-bottom: 1; }
.acid-t3 .i-k2 { background-color: inherit; }
.acid-t3 .i-k3 { background-color: rgb(192, 0, 0); background-color: unset; }
.acid-t3 .i-var { --c: rgb(0, 160, 0); color: rgb(0, 0, 192); }
.acid-t3 .v0 { color: var(--c); }
.acid-t3 .v1 { color: var(--nope, rgb(0, 160, 0)); }
.acid-t3 .v2 { color: rgb(192, 0, 0); color: var(--nope); }
.acid-t3 .v3 { color: var(--nope, var(--c)); }
.acid-t3 .v4 a { color: var(--c); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
