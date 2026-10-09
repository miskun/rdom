//! Tile 8 — inline formatting (CSS Text 3, CSS Text Decoration 4, CSS
//! Inline 3, CSS 2.1 §10.8, CSS Fonts 4).
//!
//! Five bands of side-by-side blocks (flex rows, `align-items:
//! flex-start`): the `white-space` modes, an inline block in a line,
//! `sub` / `super`; a styled span across a wrap and the breaking
//! controls, tabs, `text-transform`; `text-indent`, `text-align`,
//! justification, `text-wrap: balance`; `line-height` and `lh`, text
//! decorations propagating, font weights and the `font` shorthand;
//! `vertical-align` on inline blocks and the decoration styles.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "8",
    title: "Inline formatting",
    class: "acid-t8",
    page: 1,
    x: 61,
    y: 24,
    w: 58,
    h: 20,
    markup: "
<div class=\"band\"><div class=\"ws normal\">aa  bb
cc dd</div><div class=\"ws nowrap\">aa  bb
cc dd</div><div class=\"ws pre\">aa  bb
cc dd</div><div class=\"ws pre-wrap\">aa  bb
cc dd</div><div class=\"ws pre-line\">aa  bb
cc dd</div><div class=\"ws break-spaces\">aa  bb
cc dd</div><div class=\"atom\">x<span class=\"ib\">ib</span>y</div><div class=\"va\">a<sub>2</sub>b<sup>3</sup>c</div></div>
<div class=\"band\"><p class=\"sp\">ab <span class=\"hl\">cd ef gh</span> ij</p><div class=\"brk anywhere\">abcdefghij</div><div class=\"brk all\">ab cdefgh</div><div class=\"brk keep\">日本語の文</div><div class=\"brk shy\">ab&shy;cdef</div><div class=\"tab\">a&#9;b
ab&#9;c</div><div class=\"tt\"><span class=\"up\">ab</span> <span class=\"cap\">cd ef</span> <span class=\"lo\">GH</span></div></div>
<div class=\"band\"><div class=\"ind\">aa bb cc dd</div><div class=\"ind hang\">aa bb cc dd</div><div class=\"al center\">ab</div><div class=\"al right\">ab</div><div class=\"just\">aa bb cc dd</div><div class=\"bal\">aaa bbb ccc ddd</div></div>
<div class=\"band\"><div class=\"lh3\">x</div><div class=\"lhu\">y</div><div class=\"lhp\">z</div><div class=\"dec\"><span class=\"u\">ab<span class=\"o\">cd</span><span class=\"ib2\">ef</span>gh</span></div><div class=\"fonts\"><div><span class=\"w6\">600</span> <span class=\"bd\">bolder</span> <b><span class=\"lt\">lighter</span></b> <span class=\"ob\">obl</span></div><div class=\"fs\">x</div></div></div>
<div class=\"band\"><div class=\"vb\">a<span class=\"ibm\">1<br>2<br>3</span>b<span class=\"ibt\">4<br>5</span>c<span class=\"ibb\">6<br>7</span>d</div><div class=\"dec2\"><span class=\"dd\">dbl</span> <span class=\"ds\">dsh</span> <span class=\"dt\">dot</span> <s>str</s> <span class=\"ov\">ovl</span></div></div>
",
    css: r#"
.acid-t8 .band { display: flex; gap: 1; align-items: flex-start; margin-bottom: 1; }
.acid-t8 .ws { width: 5; overflow: clip; }
.acid-t8 .normal { white-space: normal; }
.acid-t8 .nowrap { white-space: nowrap; }
.acid-t8 .pre { white-space: pre; }
.acid-t8 .pre-wrap { white-space: pre-wrap; }
.acid-t8 .pre-line { white-space: pre-line; }
.acid-t8 .break-spaces { white-space: break-spaces; }
.acid-t8 .atom { width: 9; }
.acid-t8 .ib { display: inline-block; padding: 0 1; border: solid; }
.acid-t8 .va { width: 7; }
.acid-t8 .sp { width: 10; }
.acid-t8 .hl {
  background-color: rgb(0, 0, 128); font-weight: bold; font-style: italic;
  text-decoration: underline;
}
.acid-t8 .brk { width: 4; }
.acid-t8 .anywhere { overflow-wrap: anywhere; }
.acid-t8 .all { word-break: break-all; }
.acid-t8 .keep { word-break: keep-all; overflow: clip; }
.acid-t8 .tab { width: 8; white-space: pre; tab-size: 4; }
.acid-t8 .tt { width: 11; }
.acid-t8 .up { text-transform: uppercase; }
.acid-t8 .cap { text-transform: capitalize; }
.acid-t8 .lo { text-transform: lowercase; }
.acid-t8 .ind { width: 8; text-indent: 2; }
.acid-t8 .hang { text-indent: 2 hanging; }
.acid-t8 .al { width: 7; }
.acid-t8 .center { text-align: center; }
.acid-t8 .right { width: 5; text-align: right; }
.acid-t8 .just { width: 9; text-align: justify; text-align-last: right; }
.acid-t8 .bal { width: 12; text-wrap: balance; }
.acid-t8 .lh3 { width: 5; line-height: 3; background-color: rgb(0, 0, 128); }
.acid-t8 .lhu { width: 3; line-height: 2; height: 2lh; background-color: rgb(0, 128, 128); }
.acid-t8 .lhp { width: 3; line-height: 200%; background-color: rgb(128, 0, 128); }
.acid-t8 .dec { width: 20; }
.acid-t8 .u { text-decoration: underline wavy rgb(192, 0, 0); }
.acid-t8 .o { text-decoration: overline; }
.acid-t8 .ib2 { display: inline-block; }
.acid-t8 .fonts { width: 23; }
.acid-t8 .w6 { font-weight: 600; }
.acid-t8 .bd { font-weight: bolder; }
.acid-t8 .lt { font-weight: lighter; }
.acid-t8 .ob { font-style: oblique 10deg; }
.acid-t8 .fs { line-height: 3; font: bold 16px serif; background-color: rgb(0, 0, 128); }
.acid-t8 .vb { width: 7; }
.acid-t8 .ibm, .acid-t8 .ibt, .acid-t8 .ibb { display: inline-block; }
.acid-t8 .ibm { vertical-align: middle; background-color: rgb(0, 0, 128); }
.acid-t8 .ibt { vertical-align: top; background-color: rgb(0, 128, 128); }
.acid-t8 .ibb { vertical-align: bottom; background-color: rgb(128, 0, 128); }
.acid-t8 .dec2 { width: 20; }
.acid-t8 .dd { text-decoration: underline double; }
.acid-t8 .ds { text-decoration: underline dashed; }
.acid-t8 .dt { text-decoration: underline dotted; }
.acid-t8 .ov { text-decoration: overline; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
