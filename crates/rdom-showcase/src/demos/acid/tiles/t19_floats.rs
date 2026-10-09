//! Tile 19 — floats (CSS 2.1 §9.5, §10.6.7, Appendix E; CSS Logical 1
//! §2.2).
//!
//! A left and a right float beside a paragraph that wraps around both,
//! and a third float too wide beside them going below; a float met
//! mid-line joining its line; a word too wide beside a float moving below
//! it; a `flow-root` box containing its float; an `overflow: hidden` box
//! beside a float, and a block's background under the float's lower
//! part; `clear: left`; `float: inline-start` under `rtl`; a clearfix
//! (`::after { content: ""; display: block; clear: both }`); a `.media`
//! float with `margin-right`; a float inside `display: contents`; a
//! floated `::before`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "19",
    title: "Floats",
    class: "acid-t19",
    page: 4,
    x: 61,
    y: 37,
    w: 58,
    h: 8,
    markup: r#"
<div class="band"><div class="ca"><p><b class="L">L</b><b class="R">R</b><b class="F3">F</b>one two three four five six seven</p><p>aa <b class="fm">M</b>bb cc</p><p><b class="fw">W</b>abcdefghijklmnopqr</p></div><div class="cb"><div class="fr"><b class="fb">B</b>x</div><div><b class="fo">O</b><div class="bx">hid</div></div><div class="pb">ppp</div><div><b class="fq">Q</b><p class="cq">z</p></div></div><div class="cc"><div class="rt"><b class="fi">I</b>ab cd</div><div class="cf"><b class="fc">C</b></div><div>after</div><div class="md"><b class="mi">M</b>txt</div><div class="pz">ab</div><div><div class="dc"><b class="fd">D</b></div>tail</div></div></div>
"#,
    css: r#"
.acid-t19 .band { display: flex; gap: 2; align-items: flex-start; }
.acid-t19 b { font-weight: normal; }
.acid-t19 .ca { width: 20; }
.acid-t19 .L { float: left; width: 3; height: 2; background-color: rgb(0, 128, 128); }
.acid-t19 .R { float: right; width: 3; height: 1; background-color: rgb(0, 0, 128); }
.acid-t19 .F3 { float: left; width: 16; height: 1; background-color: rgb(128, 128, 0); }
.acid-t19 .fm { float: left; width: 2; height: 1; background-color: rgb(128, 0, 0); }
.acid-t19 .fw { float: left; width: 3; height: 1; background-color: rgb(0, 128, 128); }
.acid-t19 .cb { width: 16; }
.acid-t19 .fr { display: flow-root; background-color: rgb(0, 0, 128); }
.acid-t19 .fb { float: left; width: 2; height: 3; background-color: rgb(0, 128, 128); }
.acid-t19 .fo { float: left; width: 3; height: 2; background-color: rgb(128, 128, 0); }
.acid-t19 .bx { overflow: hidden; background-color: rgb(128, 0, 0); }
.acid-t19 .pb { background-color: rgb(0, 128, 0); }
.acid-t19 .fq { float: left; width: 2; height: 2; background-color: rgb(0, 128, 128); }
.acid-t19 .cq { clear: left; }
.acid-t19 .cc { width: 14; }
.acid-t19 .rt { direction: rtl; }
.acid-t19 .fi { float: inline-start; width: 2; height: 1; background-color: rgb(0, 128, 128); }
.acid-t19 .cf { background-color: rgb(128, 0, 0); }
.acid-t19 .cf::after { content: ""; display: block; clear: both; }
.acid-t19 .fc { float: left; width: 2; height: 2; background-color: rgb(0, 0, 128); }
.acid-t19 .mi { float: left; width: 2; height: 1; margin-right: 1; background-color: rgb(128, 128, 0); }
.acid-t19 .pz::before { content: "F"; float: right; background-color: rgb(0, 128, 0); }
.acid-t19 .dc { display: contents; }
.acid-t19 .fd { float: left; width: 1; height: 1; background-color: rgb(0, 0, 128); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
