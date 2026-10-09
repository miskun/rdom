//! Tile 30 — layout longhands (CSS Flexbox 1, CSS Grid 2, CSS Box
//! Alignment 3, CSS Multi-column 1, CSS Fragmentation 3, CSS UI 4 §5, CSS
//! Anchor Positioning 1 §4, CSS Position 4; ACID-COVERAGE).
//!
//! `flex-flow` and the `flex-grow` / `-shrink` / `-basis` longhands with
//! `align-content`; the `grid` shorthand, `grid-auto-columns`, the four
//! line longhands, `justify-items`, `place-content`, `place-self` and
//! `place-items`; `column-count` and the `column-rule` longhands with the
//! break properties and their legacy `page-break-*` aliases; the
//! `outline` longhands; `position-try`, `position-try-order` and
//! `overlay`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "30",
    title: "Layout longhands",
    class: "acid-t30",
    page: 8,
    x: 1,
    y: 14,
    w: 58,
    h: 3,
    markup: r#"
<div class="band"><div class="fl"><b class="g1">a</b><b class="g2">b</b></div><div class="gr"><b class="p">p</b><b class="q">q</b><b class="r">r</b></div><div class="pi"><b>c</b></div><div class="cc"><div>d1</div><div>d2</div><div>d3</div><div>d4</div></div><div class="ow"><b class="ol">o</b></div><div class="pw"><b class="pa">an</b><b class="pt">pt</b></div></div>
"#,
    css: r#"
.acid-t30 .band { display: flex; gap: 2; align-items: flex-start; }
.acid-t30 b { font-weight: normal; }
.acid-t30 .fl { display: flex; flex-flow: row wrap; width: 10; height: 3; align-content: flex-end; }
.acid-t30 .g1 { flex-grow: 1; flex-shrink: 0; flex-basis: 2; background-color: rgb(0, 128, 128); }
.acid-t30 .g2 { flex-grow: 0; flex-shrink: 1; flex-basis: 3; background-color: rgb(0, 0, 128); }
.acid-t30 .gr {
  display: grid; grid: 1 1 / 2 2 2; grid-auto-columns: 3;
  justify-items: end; place-content: start;
}
.acid-t30 .p {
  grid-row-start: 2; grid-row-end: 3; grid-column-start: 2; grid-column-end: 4;
  background-color: rgb(128, 128, 0);
}
.acid-t30 .q { grid-column-start: 4; background-color: rgb(128, 0, 0); }
.acid-t30 .r { place-self: start; background-color: rgb(0, 128, 128); }
.acid-t30 .pi { display: grid; grid-template-columns: 3; grid-template-rows: 2; place-items: end center; }
.acid-t30 .cc {
  width: 7; column-count: 2;
  column-rule-style: solid; column-rule-width: thin; column-rule-color: rgb(255, 0, 0);
}
.acid-t30 .cc > div {
  break-after: auto; break-inside: avoid;
  page-break-before: auto; page-break-after: auto; page-break-inside: auto;
}
.acid-t30 .ow { padding: 1; }
.acid-t30 .ol { display: block; outline-style: solid; outline-width: thick; outline-color: rgb(0, 0, 255); }
.acid-t30 .pw { position: relative; width: 6; height: 3; }
.acid-t30 .pa { anchor-name: --pa; }
.acid-t30 .pt {
  position: absolute; position-anchor: --pa; top: anchor(bottom); left: anchor(left);
  position-try: normal flip-block; position-try-order: normal; overlay: none;
}
"#,
    late_css: "",
    setup: None,
    script: None,
};
