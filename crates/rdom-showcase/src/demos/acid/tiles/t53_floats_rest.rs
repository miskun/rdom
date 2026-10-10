//! Tile 53 — overflowing text over a float, and a cleared first child's
//! margin (CSS 2.1 §8.3.1, §9.5.2, Appendix E; ACID-STATIC-REST, tile 19's
//! left-out cases).
//!
//! A 6-wide paragraph whose unbreakable `nowrap` word overflows it, over a
//! float at the right of their 12-wide parent; under a spacer, a parent
//! whose float is followed by a first in-flow child with `clear: left` and
//! `margin-top: 2`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "53",
    title: "Floats: overflow over, cleared margin",
    class: "acid-t53",
    page: 13,
    x: 1,
    y: 10,
    w: 38,
    h: 6,
    markup: r#"
<div class="fo"><div class="fr"></div><p class="ov">overflowing</p></div><div class="gap"></div><div class="cf"><div class="fl">F</div><div class="cl">x</div></div>
"#,
    css: r#"
.acid-t53 .fo { width: 12; height: 1; }
.acid-t53 .fr { float: right; width: 4; height: 1; background-color: rgb(128, 0, 0); }
.acid-t53 .ov { width: 6; margin: 0; white-space: nowrap; }
.acid-t53 .gap { height: 1; }
.acid-t53 .fl { float: left; width: 2; height: 3; background-color: rgb(0, 128, 128); }
.acid-t53 .cl { clear: left; margin-top: 2; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
