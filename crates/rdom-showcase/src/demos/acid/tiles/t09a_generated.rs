//! Tile 9a — counters and generated content (CSS Generated Content 3,
//! CSS Lists 3 §4, CSS Counter Styles 3, CSS Pseudo-Elements 4 §2–§4,
//! HTML §15.5.20).
//!
//! Five bands: `content` strings, `attr()`, alt text and the legacy
//! single-colon `:before`; nested and sibling counters with `counters()`
//! and `counter-set`, predefined counter styles across systems, an
//! author `@counter-style` with `pad`, `negative`, `range` and a looping
//! `fallback`, and `symbols()`; quotes by depth and language, a block
//! `::before`, pseudos in a wrapped line; positioned pseudo-elements in the
//! stacking order; a closed and an open `<details>` with
//! `::details-content`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "9a",
    title: "Counters & generated content",
    class: "acid-t9a",
    page: 2,
    x: 1,
    y: 1,
    w: 58,
    h: 17,
    markup: r#"
<div class="band"><p class="g1" data-x="ATTR" tabindex="0">mid</p><p class="alt">alt</p><p class="sc">one</p></div>
<div class="band"><div class="ca"><div class="it">A<div class="lv"><div class="it">B</div><div class="it">C<div class="lv"><div class="it">D</div></div></div></div></div></div><div class="cb"><div class="sl"><div class="l">x</div><div class="l">y</div><div class="l cs">w</div><div class="l">v</div></div><div class="sl"><div class="l">z</div></div></div><p class="ps"></p><p class="cs1"></p></div>
<div class="band"><p class="qq"><span class="q1">a<span class="q2">b</span></span> <q lang="de">x</q> <q lang="fr">y</q> <q lang="ja">z</q> <span class="cq">w</span></p><div class="bf">text</div><p class="wr">aa <span class="ip">bb cc</span> dd</p></div>
<div class="band"><p class="host">HOST</p><div class="ctx"><p class="h2">AAAA</p><p class="sib">SS</p></div><p class="rb">xyz</p></div>
<div class="band"><details class="d1"><summary>closed</summary>hidden<p>para</p></details><details class="d2" open><summary>open</summary>shown<p>para</p></details></div>
"#,
    css: r#"
@counter-style acid-x {
  system: extends decimal; pad: 3 "0"; negative: "(" ")"; range: -5 50; fallback: acid-y;
}
@counter-style acid-y { system: cyclic; symbols: "*"; range: 1 5; fallback: acid-x; }
.acid-t9a .band { display: flex; gap: 1; align-items: flex-start; margin-bottom: 1; }
.acid-t9a .g1::before { content: "<" attr(data-x) ":"; }
.acid-t9a .g1::after { content: ">"; }
.acid-t9a .g1::before:hover { color: rgb(192, 0, 0); }
.acid-t9a .g1::after:active { color: rgb(0, 160, 0); }
.acid-t9a .g1::before:focus { color: rgb(0, 0, 192); }
.acid-t9a .alt::before { content: "★" / "star"; }
.acid-t9a .sc:before { content: "S:"; }
.acid-t9a .ca { width: 9; }
.acid-t9a .ca, .acid-t9a .lv { counter-reset: n; }
.acid-t9a .it { counter-increment: n; }
.acid-t9a .it::before { content: counters(n, ".") " "; }
.acid-t9a .cb { width: 5; }
.acid-t9a .sl { counter-reset: m; }
.acid-t9a .l { counter-increment: m; }
.acid-t9a .cs { counter-set: m 7; }
.acid-t9a .l::before { content: counters(m, ".") " "; }
.acid-t9a .ps { width: 15; counter-reset: p 12; }
.acid-t9a .ps::before {
  content: counter(p, upper-roman) " " counter(p, lower-greek) " " counter(p, hebrew) " "
    counter(p, cjk-decimal) " " counter(p, disc);
}
.acid-t9a .cs1 { width: 14; counter-reset: a 7 b -3 c 99 d 3; }
.acid-t9a .cs1::before {
  content: counter(a, acid-x) " " counter(b, acid-x) " " counter(c, acid-x) " "
    counter(d, symbols(cyclic "a" "b"));
}
.acid-t9a .qq { width: 22; quotes: "«" "»" "‹" "›"; }
.acid-t9a q { quotes: auto; }
.acid-t9a .q1::before, .acid-t9a .q2::before { content: open-quote; }
.acid-t9a .q1::after, .acid-t9a .q2::after { content: close-quote; }
.acid-t9a .cq::after { content: close-quote; }
.acid-t9a .bf { width: 6; }
.acid-t9a .bf::before { content: "BLOCK"; display: block; }
.acid-t9a .wr { width: 9; }
.acid-t9a .ip::before { content: "["; }
.acid-t9a .ip::after { content: "]"; }
.acid-t9a .host { width: 8; position: relative; }
.acid-t9a .host::after {
  content: "____"; position: absolute; left: 0; top: 0; z-index: -1;
  background-color: rgb(0, 0, 128); color: rgb(192, 0, 0);
}
.acid-t9a .ctx { width: 10; }
.acid-t9a .h2 { position: relative; z-index: 1; }
.acid-t9a .h2::after {
  content: "aaaa"; position: absolute; left: 2; top: 0; z-index: 5;
  background-color: rgb(0, 128, 128);
}
.acid-t9a .sib {
  width: 2; position: relative; z-index: 2; top: -1; left: 4;
  background-color: rgb(128, 0, 128);
}
.acid-t9a .rb { width: 5; }
.acid-t9a .rb::before { content: "R"; position: relative; left: 2; }
.acid-t9a details { width: 20; }
.acid-t9a details::details-content { color: rgb(0, 160, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
