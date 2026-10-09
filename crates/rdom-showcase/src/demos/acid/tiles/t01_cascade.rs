//! Tile 1 — cascade order (CSS Cascade 4 §6, CSS Cascade 5 §6.4).
//!
//! Every word is one contest; the winning declaration paints it green,
//! every losing one red (the UA's own `mark` colours for the first).

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "1",
    title: "Cascade order",
    class: "acid-t1",
    page: 1,
    x: 1,
    y: 1,
    w: 38,
    h: 5,
    markup: r#"
<style>
.acid-t1 .c-style { color: rgb(192, 0, 0); }
.acid-t1 span.c-style2 { color: rgb(0, 160, 0); }
</style>
<div><mark class="c-ua">ua</mark> <span class="c-late">late-sheet</span> <span class="c-style">style-el</span> <span class="c-style2">style-spec</span></div>
<div><span class="c-inline" style="color: rgb(0, 160, 0)">inline</span> <span class="c-imp" style="color: rgb(192, 0, 0)">imp-sheet</span> <span class="c-impin" style="color: rgb(0, 160, 0) !important">imp-inline</span></div>
<div><span class="c-order">order</span> <span class="c-implate">imp-main</span> <span class="c-spec">spec-main</span> <span class="c-invalid">invalid</span></div>
<div><span class="c-layer">unlayered</span> <span class="c-layimp">imp-layer</span> <span id="c-layid" class="c-layid">layer-id</span></div>
"#,
    css: r#"
.acid-t1 { color: rgb(192, 0, 0); }
.acid-t1 .c-ua { color: rgb(0, 160, 0); background-color: transparent; }
.acid-t1 .c-late { color: rgb(192, 0, 0); }
.acid-t1 .c-style { color: rgb(0, 160, 0); }
.acid-t1 .c-style2 { color: rgb(192, 0, 0); }
.acid-t1 .c-inline { color: rgb(192, 0, 0); }
.acid-t1 .c-inline.c-inline.c-inline { color: rgb(192, 0, 0); }
.acid-t1 .c-imp { color: rgb(0, 160, 0) !important; }
.acid-t1 .c-impin { color: rgb(192, 0, 0) !important; }
.acid-t1 .c-order { color: rgb(192, 0, 0); }
.acid-t1 .c-order { color: rgb(0, 160, 0); }
.acid-t1 .c-implate { color: rgb(0, 160, 0) !important; }
.acid-t1 span.c-spec { color: rgb(0, 160, 0); }
.acid-t1 .c-invalid { color: rgb(0, 160, 0); color: not-a-color; }
@layer base {
  .acid-t1 span.c-layer.c-layer { color: rgb(192, 0, 0); }
  .acid-t1 .c-layimp { color: rgb(0, 160, 0) !important; }
  .acid-t1 #c-layid { color: rgb(192, 0, 0); }
}
.acid-t1 .c-layer { color: rgb(0, 160, 0); }
.acid-t1 .c-layimp { color: rgb(192, 0, 0) !important; }
.acid-t1 .c-layid { color: rgb(0, 160, 0); }
"#,
    late_css: r#"
.acid-t1 .c-late { color: rgb(0, 160, 0); }
.acid-t1 .c-implate { color: rgb(192, 0, 0); }
.acid-t1 .c-spec { color: rgb(192, 0, 0); }
"#,
    setup: None,
    script: None,
};
