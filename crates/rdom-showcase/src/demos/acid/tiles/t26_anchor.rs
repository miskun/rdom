//! Tile 26 — anchor positioning (CSS Anchor Positioning 1).
//!
//! A box placed by `top: anchor(--a bottom); left: anchor(--a right)`
//! with `width: anchor-size(--a width)`; a tooltip `position-area: top`
//! centred above its anchor; a box whose base position overflows the
//! tile, placed by its `@position-try --left` fallback; `anchor-scope`
//! keeping each list item's `--item` name to the item; an anchor
//! scrolled out of its scroller, its `position-visibility:
//! anchors-visible` box hidden and its `always` box shown.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "26",
    title: "Anchor positioning",
    class: "acid-t26",
    page: 7,
    x: 1,
    y: 11,
    w: 58,
    h: 10,
    markup: r#"
<div class="a">A</div><div class="p1">P1</div><div class="b">button</div><div class="tip">tip</div><div class="d">D</div><div class="p4">P4</div><div class="li l1"><b class="an">x</b><b class="tp">1</b></div><div class="li l2"><b class="an">y</b><b class="tp">2</b></div><div class="scr"><div class="pad"></div><b class="sa">S</b></div><b class="v1">v</b><b class="v2">V</b>
"#,
    css: r#"
.acid-t26 b { font-weight: normal; }
.acid-t26 .a { position: absolute; left: 2; top: 1; width: 4; height: 1; anchor-name: --a; background-color: rgb(0, 0, 128); }
.acid-t26 .p1 {
  position: absolute; top: anchor(--a bottom); left: anchor(--a right);
  width: anchor-size(--a width); height: 1; background-color: rgb(0, 128, 128);
}
.acid-t26 .b { position: absolute; left: 14; top: 3; anchor-name: --b; }
.acid-t26 .tip { position: absolute; position-anchor: --b; position-area: top; background-color: rgb(128, 128, 0); }
.acid-t26 .d { position: absolute; left: 50; top: 1; width: 4; anchor-name: --d; }
.acid-t26 .p4 {
  position: absolute; position-anchor: --d; top: anchor(top); left: anchor(right);
  width: 6; background-color: rgb(128, 0, 0); position-try-fallbacks: --left;
}
@position-try --left { left: auto; right: anchor(left); }
.acid-t26 .li { position: absolute; top: 5; anchor-scope: --item; }
.acid-t26 .l1 { left: 2; }
.acid-t26 .l2 { left: 8; }
.acid-t26 .an { anchor-name: --item; }
.acid-t26 .tp { position: absolute; position-anchor: --item; left: anchor(right); top: anchor(top); color: rgb(0, 160, 0); }
.acid-t26 .scr { position: absolute; left: 20; top: 5; width: 6; height: 2; overflow: auto; scrollbar-width: none; }
.acid-t26 .pad { height: 4; }
.acid-t26 .sa { display: block; anchor-name: --s; }
.acid-t26 .v1 { position: absolute; position-anchor: --s; right: anchor(left); top: anchor(top); position-visibility: anchors-visible; }
.acid-t26 .v2 { position: absolute; position-anchor: --s; left: anchor(right); top: anchor(top); position-visibility: always; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
