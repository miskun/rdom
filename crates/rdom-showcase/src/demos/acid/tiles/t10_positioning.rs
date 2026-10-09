//! Tile 10 — positioning (CSS 2.1 §9.3–§9.6, §10.3.7, §10.6.4; CSS
//! Position 3).
//!
//! A relatively offset span keeping its place in the line; inside a
//! bordered `position: relative` box, absolutely positioned boxes by
//! `top` / `left`, by `right` / `bottom`, by `inset` with an `auto` size
//! between the offsets, and with every offset `auto` at its static
//! position; a `position: fixed` box placed against the viewport — the
//! page — whatever its ancestors.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "10",
    title: "Positioning",
    class: "acid-t10",
    page: 2,
    x: 60,
    y: 5,
    w: 38,
    h: 10,
    markup: r#"
<p>ab <span class="rel">rel</span> cd</p>
<div class="pc">x<span class="a5">S</span>y<b class="a1">A1</b><b class="a2">A2</b><b class="a4">A4</b><b class="fx">FX</b></div>
"#,
    css: r#"
.acid-t10 b { font-weight: normal; }
.acid-t10 .rel { position: relative; top: 1; left: 2; }
.acid-t10 .pc { position: relative; width: 30; height: 6; margin-top: 1; border: solid; }
.acid-t10 .a5 { position: absolute; }
.acid-t10 .a1 { position: absolute; top: 2; left: 3; }
.acid-t10 .a2 { position: absolute; right: 1; bottom: 0; }
.acid-t10 .a4 { position: absolute; inset: 1 2 2 20; background-color: rgb(0, 128, 128); }
.acid-t10 .fx { position: fixed; top: 13; left: 70; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
