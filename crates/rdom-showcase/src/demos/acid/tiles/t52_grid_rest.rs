//! Tile 52 — a baseline row group and an `aspect-ratio` item (CSS Grid 2
//! §6.2, §10; CSS Box Alignment 3 §9; CSS Sizing 4 §5.1; ACID-STATIC-REST,
//! tile 18's left-out cases).
//!
//! A grid row whose items are baseline-aligned — one with two rows of
//! padding above its text — beside an item with `aspect-ratio: 3 / 1`
//! (`align-self: normal`, which behaves as `start` for it).

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "52",
    title: "Grid: baseline row, aspect-ratio",
    class: "acid-t52",
    page: 13,
    x: 81,
    y: 1,
    w: 38,
    h: 3,
    markup: r#"
<div class="g"><div class="b1">a</div><div class="b2">b</div><div class="ar"></div></div>
"#,
    css: r#"
.acid-t52 .g { display: grid; grid-template-columns: 4 4 6; column-gap: 1; align-items: baseline; }
.acid-t52 .b1 { padding-top: 2; background-color: rgb(0, 0, 128); }
.acid-t52 .b2 { background-color: rgb(0, 128, 128); }
.acid-t52 .ar { align-self: normal; aspect-ratio: 3 / 1; background-color: rgb(128, 0, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
