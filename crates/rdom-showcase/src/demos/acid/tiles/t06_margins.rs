//! Tile 6 — margin collapsing (CSS 2.1 §8.3.1).
//!
//! Nine lanes, each a flex item (its own block formatting context, so no
//! lane's margins reach the tile): adjacent siblings; a parent and its
//! first child; an empty block collapsing through; a negative margin; the
//! collapse blocked by padding, by a border and by a line box; flex items,
//! whose margins never collapse; a parent and its last child.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "6",
    title: "Margin collapsing",
    class: "acid-t6",
    page: 1,
    x: 1,
    y: 17,
    w: 38,
    h: 5,
    markup: r#"
<div class="ln"><p class="mb2">A</p><p class="mt1">B</p></div>
<div class="ln"><div class="par"><p class="mt2">C</p></div><p>D</p></div>
<div class="ln"><p class="mb1">E</p><div class="empty"></div><p class="mt1">F</p></div>
<div class="ln"><p class="mb3">E</p><p class="mtn1">F</p></div>
<div class="ln"><div class="par padt"><p class="mt1">C</p></div></div>
<div class="ln"><div class="bordt"><p class="mt1">C</p></div></div>
<div class="ln"><div>x<p class="mt1">C</p></div></div>
<div class="ln fcol"><p class="mb1">G</p><p class="mt1">H</p></div>
<div class="ln"><div class="par"><p class="mb2">J</p></div><p>K</p></div>
"#,
    css: r#"
.acid-t6 { display: flex; gap: 1; align-items: flex-start; }
.acid-t6 .ln { width: 3; }
.acid-t6 p { background-color: rgb(0, 0, 128); }
.acid-t6 .par { background-color: rgb(0, 128, 128); }
.acid-t6 .mt1 { margin-top: 1; }
.acid-t6 .mt2 { margin-top: 2; }
.acid-t6 .mtn1 { margin-top: -1; }
.acid-t6 .mb1 { margin-bottom: 1; }
.acid-t6 .mb2 { margin-bottom: 2; }
.acid-t6 .mb3 { margin-bottom: 3; }
.acid-t6 .empty { margin-top: 3; margin-bottom: 1; }
.acid-t6 .padt { padding-top: 1; }
.acid-t6 .bordt { border-top: solid; }
.acid-t6 .fcol { display: flex; flex-direction: column; }
"#,
    late_css: "",
};
