//! Tile 50 — wrapped cells beside a row span (CSS 2.1 §17.5.3, §17.6.2;
//! ACID-STATIC-REST, tile 14's left-out case).
//!
//! A collapsed table whose first cell spans two rows, beside a cell whose
//! three words wrap in a two-cell column, and a one-line cell under it.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "50",
    title: "Tables: wraps beside a row span",
    class: "acid-t50",
    page: 13,
    x: 1,
    y: 1,
    w: 38,
    h: 7,
    markup: r#"
<table class="ws"><tr><td rowspan="2">R</td><td class="w">aa bb cc</td></tr><tr><td>x</td></tr></table>
"#,
    css: r#"
.acid-t50 .ws { border-collapse: collapse; }
.acid-t50 .ws td { border: solid; padding: 0; }
.acid-t50 .ws .w { width: 2; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
