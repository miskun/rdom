//! Tile 14 — tables and border collapse (CSS 2.1 §17, CSS Tables 3
//! §11.5, Selectors 4 §16, HTML §15.3.8).
//!
//! A collapsed 2 × 2 table whose borders contest each other (`hidden`
//! wins, `double` beats `solid`, a wider border beats a narrower one, a
//! cell beats the table, `none` loses to the table); a separated table
//! spaced by `border-spacing` with a `rowspan` and a `colspan` cell and an
//! empty cell under `empty-cells: hide`; column rules (`col.x || td`,
//! `td:nth-col(3)`) on a spanning cell, a right-aligned `th` row and a
//! bottom caption; an anonymous table of `display: table-row` /
//! `table-cell` boxes; `vertical-align: top / middle / bottom / baseline`
//! cells in a three-row row, a centred `th` row and caption and a zebra
//! stripe; a bordered `width: 100%` table filling its box; a
//! `table-layout: fixed` table whose long cell is clipped.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "14",
    title: "Tables & border collapse",
    class: "acid-t14",
    page: 3,
    x: 1,
    y: 1,
    w: 58,
    h: 13,
    markup: r#"
<div class="band"><table class="cc"><tr><td>a</td><td class="d">b</td></tr><tr><td class="h">c</td><td class="w">d</td></tr></table><table class="sp"><tr><td rowspan="2">r</td><td>a</td><td></td></tr><tr><td colspan="2">wide</td></tr></table><table class="cs"><caption>cap</caption><colgroup><col><col class="x"><col></colgroup><tr><th>A</th><th>B</th><th>C</th></tr><tr><td>aa</td><td>bb</td><td>cc</td></tr><tr><td>dd</td><td colspan="2">ef</td></tr></table><div class="an"><div class="r"><span class="c">ab</span><span class="c">c</span></div><div class="r"><span class="c">d</span><span class="c">efg</span></div></div></div>
<div class="band"><table class="va"><caption>cap</caption><thead><tr><th>A</th><th>B</th><th>C</th><th>D</th><th>E</th></tr></thead><tbody><tr><td class="t">top</td><td class="m">mid</td><td class="b">bot</td><td class="l">bas</td><td>1<br>2<br>3</td></tr><tr><td>z</td><td>z</td><td>z</td><td>z</td><td>z</td></tr></tbody></table><div class="fw"><table class="f"><tr><td>a</td><td>b</td></tr></table></div><table class="fx"><tr><td class="w4">x</td><td>y</td></tr><tr><td>longer</td><td>z</td></tr></table></div>
"#,
    css: r#"
.acid-t14 .band { display: flex; gap: 2; align-items: flex-start; margin-bottom: 1; }
.acid-t14 .cc { border-collapse: collapse; border: solid rgb(255, 0, 0); }
.acid-t14 .cc td { border: solid; }
.acid-t14 .cc .d { border-style: double; border-top-style: none; }
.acid-t14 .cc .h { border-left-style: hidden; }
.acid-t14 .cc .w { border-bottom-width: thick; }
.acid-t14 .sp { border-spacing: 1 0; empty-cells: hide; }
.acid-t14 .sp td { border: solid; padding: 0; }
.acid-t14 .cs { text-align: right; caption-side: bottom; }
.acid-t14 .cs col.x || td { color: rgb(255, 0, 0); }
.acid-t14 .cs td:nth-col(3) { background-color: rgb(0, 128, 0); }
.acid-t14 .an .r { display: table-row; }
.acid-t14 .an .c { display: table-cell; padding: 0 1; }
.acid-t14 .va .t { vertical-align: top; }
.acid-t14 .va .m { vertical-align: middle; }
.acid-t14 .va .b { vertical-align: bottom; }
.acid-t14 .va .l { vertical-align: baseline; }
.acid-t14 .va tbody tr:nth-child(even) { background-color: rgb(0, 0, 128); }
.acid-t14 .fw { width: 14; }
.acid-t14 .f { width: 100%; border: solid; }
.acid-t14 .fx { table-layout: fixed; width: 12; }
.acid-t14 .fx td { padding: 0; overflow: hidden; }
.acid-t14 .fx .w4 { width: 4; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
