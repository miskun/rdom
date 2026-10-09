//! Tile 18 — grid layout (CSS Grid 2).
//!
//! `grid-template` with named areas, row sizes and line names; `fr`,
//! `minmax()`, `fit-content()` and fixed tracks sharing a width;
//! `repeat(auto-fill)` beside `repeat(auto-fit)` collapsing its empty
//! track; `dense` auto-placement into implicit rows beside a line-placed
//! `1 / -1` item; a padded `subgrid` sizing its parent's columns; an
//! absolutely positioned child in a named area of a bordered grid; a long
//! word in a `1fr` column beside the same in `minmax(0, 1fr)`; two items
//! in one cell ordered by `z-index`; an `inline-grid` in a line of text;
//! an `rtl` grid; `justify-content: space-between` widening a spanned
//! area, an item self-aligned to the end, one centred by `auto` margins.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "18",
    title: "Grid layout",
    class: "acid-t18",
    page: 4,
    x: 1,
    y: 37,
    w: 58,
    h: 12,
    markup: r#"
<div class="band"><div class="g1"><div class="hd">H</div><div class="sd">S</div><div class="mn">M</div></div><div class="g2"><div class="a">a</div><div class="b">b</div><div class="c">cc</div><div class="d">dd dd</div></div><div class="g4"><div class="a2">a</div><div class="b2">b</div><div class="c1">c</div><div class="f">f</div></div><div class="g5"><div class="sub"><div>aaa</div><div>b</div></div><div>x</div><div>y</div></div><div class="g8"><div class="z1">A</div><div class="z2">B</div></div></div>
<div class="band"><div class="g3 af"><div>1</div><div>2</div></div><div class="g3 at"><div>1</div><div>2</div></div><div class="g6"><div>p</div><div class="ab">Q</div></div><div class="g7 fr"><div>abcdefg</div><div>x</div></div><div class="g7 mm"><div>abcdefg</div><div>x</div></div></div>
<div class="band"><div>ab<span class="ig"><b>X</b><b>Y</b></span>cd</div><div class="g10"><div>1</div><div>2</div></div><div class="g11"><div class="sa">A</div><div class="sb">b</div><div class="sc">C</div></div></div>
"#,
    css: r#"
.acid-t18 .band { display: flex; gap: 2; align-items: flex-start; margin-bottom: 1; }
.acid-t18 b { font-weight: normal; }
.acid-t18 .g1 {
  display: grid;
  grid-template: [top] "h h" 1 [mid] "s m" 2 [bot] / [a] 4 [b] 8 [c];
}
.acid-t18 .hd { grid-area: h; background-color: rgb(0, 128, 128); }
.acid-t18 .sd { grid-area: s; background-color: rgb(128, 128, 0); }
.acid-t18 .mn { grid-column: b / c; grid-row: mid / bot; background-color: rgb(0, 0, 128); }
.acid-t18 .g2 { display: grid; width: 20; grid-template-columns: 2 1fr minmax(2, 3) fit-content(4); }
.acid-t18 .g2 .a, .acid-t18 .g4 .a2 { background-color: rgb(0, 128, 128); }
.acid-t18 .g2 .b, .acid-t18 .g4 .b2 { background-color: rgb(0, 0, 128); }
.acid-t18 .g2 .c, .acid-t18 .g4 .c1 { background-color: rgb(128, 128, 0); }
.acid-t18 .g2 .d, .acid-t18 .g4 .f { background-color: rgb(128, 0, 0); }
.acid-t18 .g4 { display: grid; grid-template-columns: repeat(3, 2); grid-auto-rows: 1; grid-auto-flow: row dense; }
.acid-t18 .a2, .acid-t18 .b2 { grid-column: span 2; }
.acid-t18 .f { grid-row: 3; grid-column: 1 / -1; }
.acid-t18 .g5 { display: grid; grid-template-columns: auto auto; }
.acid-t18 .sub {
  grid-column: 1 / 3; display: grid; grid-template-columns: subgrid;
  padding: 0 1; background-color: rgb(0, 0, 128);
}
.acid-t18 .g8 { display: grid; grid-template-columns: 4; }
.acid-t18 .z1 { grid-area: 1 / 1; z-index: 2; background-color: rgb(192, 0, 0); }
.acid-t18 .z2 { grid-area: 1 / 1; z-index: 1; background-color: rgb(0, 160, 0); }
.acid-t18 .g3 { display: grid; width: 11; justify-content: end; }
.acid-t18 .g3 > div { background-color: rgb(0, 128, 128); }
.acid-t18 .af { grid-template-columns: repeat(auto-fill, 3); }
.acid-t18 .at { grid-template-columns: repeat(auto-fit, 3); }
.acid-t18 .g6 {
  display: grid; position: relative; border: solid;
  grid-template-areas: "p q"; grid-template-columns: 3 3; grid-template-rows: 2;
}
.acid-t18 .ab { position: absolute; grid-area: q; inset: 0; background-color: rgb(0, 128, 128); }
.acid-t18 .g7 { display: grid; width: 8; }
.acid-t18 .fr { grid-template-columns: 1fr 1fr; }
.acid-t18 .mm { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
.acid-t18 .ig { display: inline-grid; grid-template-columns: 1 1; column-gap: 1; }
.acid-t18 .g10 { display: grid; direction: rtl; width: 4; grid-template-columns: 2 2; }
.acid-t18 .g11 {
  display: grid; width: 10; grid-template-columns: 2 2 2; grid-template-rows: 2 1;
  justify-content: space-between;
}
.acid-t18 .sa { grid-column: 1 / 3; grid-row: 1; background-color: rgb(0, 128, 128); }
.acid-t18 .sb { grid-column: 3; grid-row: 1; justify-self: end; align-self: end; background-color: rgb(0, 0, 128); }
.acid-t18 .sc { grid-column: 1 / -1; grid-row: 2; width: 2; margin: 0 auto; background-color: rgb(128, 128, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
