//! Tile 25 — multi-column layout (CSS Multi-column 1, CSS Fragmentation
//! 3).
//!
//! A bordered three-column article whose `column-rule` meets its border
//! in junctions, with a red paragraph split across its first two columns;
//! the same flow with `widows: 3` moving the paragraph's break up; a
//! `column-span: all` element splitting the columns into two balanced
//! sets, the second with a `break-before: column`; a `column-width` box
//! with a fixed `height` under `column-fill: auto`, filling column by
//! column into an overflow column.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "25",
    title: "Multi-column layout",
    class: "acid-t25",
    page: 7,
    x: 1,
    y: 1,
    w: 90,
    h: 8,
    markup: r#"
<div class="row"><div class="mc a1"><div>d1</div><div>d2</div><p class="sp">r1<br>r2</p><div>d3</div><div>d4</div><div>d5</div><div>d6</div></div><div class="mc a2"><div>d1</div><div>d2</div><p class="sp w3">r1<br>r2<br>r3</p><div>d3</div><div>d4</div><div>d5</div><div>d6</div></div><div class="mc a3"><div>s1</div><div>s2</div><div>s3</div><div class="spn">SPAN</div><div>t1</div><div>t2</div><div class="bb">t3</div><div>t4</div><div>t5</div><div>t6</div></div></div>
<div class="band"><div class="a4"><div>u1</div><div>u2</div><div>u3</div><div>u4</div><div>u5</div><div>u6</div></div></div>
"#,
    css: r#"
.acid-t25 .band { display: flex; gap: 2; align-items: flex-start; margin-bottom: 1; }
.acid-t25 .row { margin-bottom: 1; }
.acid-t25 .mc { display: inline-block; vertical-align: top; margin-right: 2; columns: 3; width: 26; }
.acid-t25 .a1 { column-rule: solid; border: solid; }
.acid-t25 .a3 { column-rule: solid; }
.acid-t25 .sp { background-color: rgb(200, 0, 0); orphans: 1; widows: 1; }
.acid-t25 .w3 { widows: 3; }
.acid-t25 .spn { column-span: all; background-color: rgb(0, 0, 128); }
.acid-t25 .bb { break-before: column; }
.acid-t25 .a4 { column-width: 12; column-fill: auto; height: 2; width: 26; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
