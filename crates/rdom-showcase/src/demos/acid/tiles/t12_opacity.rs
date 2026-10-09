//! Tile 12 — group opacity (CSS Color 4 §3.2, Compositing 1 §5;
//! DIVERGENCES §2 "`opacity` composites per cell").
//!
//! Translucent boxes over a line of grey text and a grey border: a
//! `0.5` red box, a `0.5` green box inside a `0.5` group (0.25 overall),
//! an `opacity: 0` box, a `0.5` box with text of its own (its glyphs win
//! the cells), a `0.4` one (the backdrop's glyphs stay), and a `0.5` box
//! over the border.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "12",
    title: "Group opacity",
    class: "acid-t12",
    page: 2,
    x: 60,
    y: 23,
    w: 38,
    h: 5,
    markup: r#"
<div class="op"><p class="bt">abcdefghijklmnopqrstuvwxyz0123456789</p><div class="bb"></div><div class="t1"></div><div class="g1"><div class="g2"></div></div><div class="t0">XX</div><div class="t5">ZZ</div><div class="t4">YY</div><div class="t6"></div></div>
"#,
    css: r#"
.acid-t12 .op { position: relative; height: 5; color: rgb(100, 100, 100); }
.acid-t12 .bb { width: 10; height: 1; margin-top: 1; border: solid; }
.acid-t12 .t1, .acid-t12 .g1, .acid-t12 .t0, .acid-t12 .t5, .acid-t12 .t4, .acid-t12 .t6 {
  position: absolute; top: 0; height: 1;
}
.acid-t12 .t1 { left: 0; width: 4; opacity: 0.5; background-color: rgb(200, 0, 0); }
.acid-t12 .g1 { left: 6; width: 4; opacity: 0.5; }
.acid-t12 .g2 { height: 1; opacity: 0.5; background-color: rgb(0, 200, 0); }
.acid-t12 .t0 { left: 12; width: 4; opacity: 0; background-color: rgb(200, 0, 0); }
.acid-t12 .t5 {
  left: 18; width: 4; opacity: 0.5; background-color: rgb(200, 0, 0); color: rgb(0, 0, 200);
}
.acid-t12 .t4 {
  left: 24; width: 2; opacity: 0.4; background-color: rgb(200, 0, 0); color: rgb(0, 0, 200);
}
.acid-t12 .t6 { left: 0; top: 2; width: 6; height: 2; opacity: 0.5; background-color: rgb(200, 0, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
