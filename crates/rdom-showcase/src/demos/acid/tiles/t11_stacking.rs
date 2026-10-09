//! Tile 11 — stacking contexts (CSS 2.1 Appendix E, §9.9; CSS Position 3).
//!
//! Overlapping positioned boxes with negative, zero and positive
//! `z-index` over in-flow text; a `z-index: 99` box inside a `z-index: 1`
//! context that cannot rise above a `z-index: 2` sibling.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "11",
    title: "Stacking contexts",
    class: "acid-t11",
    page: 2,
    x: 60,
    y: 17,
    w: 38,
    h: 4,
    markup: r#"
<div class="sc"><p>INFLOW TEXT</p><div class="n">N</div><div class="z0">Z</div><div class="p2">P2</div><div class="ctx">C<div class="inner">I</div></div></div>
"#,
    css: r#"
.acid-t11 .sc { position: relative; height: 4; width: 30; }
.acid-t11 .n {
  position: absolute; z-index: -1; left: 0; top: 0; width: 6; height: 2;
  background-color: rgb(192, 0, 0);
}
.acid-t11 .z0 {
  position: absolute; z-index: 0; left: 3; top: 0; width: 4; height: 2;
  background-color: rgb(0, 160, 0);
}
.acid-t11 .p2 {
  position: absolute; z-index: 2; left: 8; top: 0; width: 6; height: 2;
  background-color: rgb(0, 0, 192);
}
.acid-t11 .ctx {
  position: absolute; z-index: 1; left: 12; top: 1; width: 8; height: 3;
  background-color: rgb(128, 0, 128);
}
.acid-t11 .inner {
  position: absolute; z-index: 99; left: -2; top: 0; width: 4; height: 1;
  background-color: rgb(255, 255, 0); color: rgb(0, 0, 0);
}
"#,
    late_css: "",
    setup: None,
    script: None,
};
