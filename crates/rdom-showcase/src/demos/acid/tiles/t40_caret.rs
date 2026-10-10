//! Tile 40 — the caret and the pointer (CSS UI 4 §4.1, §6.2; HTML
//! §6.6; ACID-INTERACTIVE I9).
//!
//! Three fields: the default caret in a set `caret-color` and the rdom
//! `caret-text-color`; `caret-shape: underscore`; `caret-shape: bar`
//! under `caret-animation: manual`. Under them a link and a `cursor:
//! grab` box, for the terminal pointer's shape.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "40",
    title: "Caret and pointer",
    class: "acid-t40",
    page: 10,
    x: 61,
    y: 16,
    w: 38,
    h: 2,
    markup: r##"
<div class="cr"><input class="f1" value="ab"><input class="f2" value="ab"><input class="f3" value="ab"></div>
<div class="pt"><a href="#x">link</a> <span class="gr">grab</span></div>
"##,
    css: r#"
.acid-t40 .cr { display: flex; gap: 1; }
.acid-t40 input { width: 4; color: rgb(255, 255, 255); }
.acid-t40 .f1 { caret-color: rgb(255, 255, 0); caret-text-color: rgb(0, 0, 0); }
.acid-t40 .f2 { caret-shape: underscore; caret-color: rgb(255, 0, 0); }
.acid-t40 .f3 { caret-shape: bar; caret-color: rgb(0, 160, 0); caret-animation: manual; }
.acid-t40 .gr { cursor: grab; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
