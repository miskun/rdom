//! Tile 2 — specificity (Selectors 4 §17).
//!
//! Each word is one contest between a green rule written first and a red
//! rule written later: order alone would pick red, so green shows only
//! when the specificity rdom computes puts it ahead.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "2",
    title: "Specificity",
    class: "acid-t2",
    page: 1,
    x: 41,
    y: 1,
    w: 38,
    h: 3,
    markup: r#"
<div><data class="k1">type&lt;class</data> <a id="k2" class="k2">id&gt;class</a> <time id="k3" class="k3">where:0</time></div>
<div><output id="k4" class="k4">is:max</output> <a class="k5">not:arg</a> <a class="k6">list:branch</a></div>
<div><a class="k7" data-k7="">attr=class</a> <a id="k8" class="k8">nth-of</a></div>
"#,
    css: r#"
.acid-t2 .k1 { color: rgb(0, 160, 0); }
.acid-t2 data { color: rgb(192, 0, 0); }
.acid-t2 #k2 { color: rgb(0, 160, 0); }
.acid-t2 .k2.k2.k2.k2 { color: rgb(192, 0, 0); }
.acid-t2 time { color: rgb(0, 160, 0); }
.acid-t2 :where(#k3) { color: rgb(192, 0, 0); }
.acid-t2 :is(#k4, output) { color: rgb(0, 160, 0); }
.acid-t2 output.k4.k4 { color: rgb(192, 0, 0); }
.acid-t2 .k5:not(#nope) { color: rgb(0, 160, 0); }
.acid-t2 a.k5.k5 { color: rgb(192, 0, 0); }
.acid-t2 a.k6 { color: rgb(0, 160, 0); }
.acid-t2 #nope, .acid-t2 .k6 { color: rgb(192, 0, 0); }
.acid-t2 [data-k7] { color: rgb(192, 0, 0); }
.acid-t2 .k7 { color: rgb(0, 160, 0); }
.acid-t2 :nth-child(1 of #k8) { color: rgb(0, 160, 0); }
.acid-t2 a.k8.k8 { color: rgb(192, 0, 0); }
"#,
    late_css: "",
};
