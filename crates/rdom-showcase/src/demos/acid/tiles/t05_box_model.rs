//! Tile 5 — box model (CSS Box 3, CSS Sizing 3 / 4, CSS Backgrounds 3
//! §4–§5).
//!
//! Two flex rows of boxes (`align-items: flex-start`, so each keeps the
//! height it computes): fixed, percentage and `calc()` widths, `min-*` /
//! `max-*` clamping, `aspect-ratio`, `box-sizing`, per-side padding, per-
//! side border styles and colours, a rounded and a `hidden` border.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "5",
    title: "Box model",
    class: "acid-t5",
    page: 1,
    x: 61,
    y: 12,
    w: 58,
    h: 10,
    markup: r#"
<div class="bx-row"><div class="b1">w6</div><div class="b2">25%</div><div class="b3">calc</div><div class="b4">min</div><div class="b5">max</div></div>
<div class="bx-row bx-2"><div class="b6">ar2</div><div class="b7">pad</div><div class="b8">sides</div><div class="b9">hid</div></div>
"#,
    css: r#"
.acid-t5 .bx-row { display: flex; gap: 1; align-items: flex-start; }
.acid-t5 .bx-2 { margin-top: 1; }
.acid-t5 .b1 { width: 6; height: 1; padding: 0 1; border: solid; }
.acid-t5 .b2 { box-sizing: border-box; width: 25%; height: 3; border: double; }
.acid-t5 .b3 { width: calc(50% - 20); height: 1; border: solid; border-radius: 1; }
.acid-t5 .b4 { width: 2; min-width: 5; height: 1; border: dashed; }
.acid-t5 .b5 {
  width: 30; max-width: 4; height: 1; border: solid;
  border-color: rgb(192, 0, 0) rgb(0, 160, 0) rgb(0, 0, 192) rgb(160, 160, 0);
}
.acid-t5 .b6 { width: 8; aspect-ratio: 2; border: solid; }
.acid-t5 .b7 { width: 5; height: 1; padding: 1 2 0 3; border: solid; }
.acid-t5 .b8 {
  width: 6; height: 2;
  border-style: solid double hidden dashed;
  border-color: rgb(0, 160, 0) rgb(0, 0, 192) rgb(192, 0, 0) rgb(160, 160, 0);
}
.acid-t5 .b9 { width: 3; height: 5; max-height: 1; border: hidden; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
