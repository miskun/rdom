//! Tile 28 — box longhands (CSS Backgrounds 3, CSS Box 4, CSS Masking 1,
//! CSS Containment 2, CSS Conditional 5, CSS Color Adjust 1;
//! ACID-COVERAGE).
//!
//! The `background` shorthand and its longhands, a background clipped to
//! the content box; a `box-shadow`; every physical border side's
//! shorthand and longhand and the four corner radii; `padding-bottom`,
//! `min-height`, `margin-trim`; the legacy `clip`; the `mask` properties,
//! `rotate`, `scale`, `transform-origin`, `box-decoration-break`,
//! `interpolate-size` and `background-blend-mode`, which draw nothing here;
//! `contain-intrinsic-size` sizing a `content-visibility: hidden` box;
//! `container-name`; `color-scheme` choosing `light-dark()`'s arm.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "28",
    title: "Box longhands",
    class: "acid-t28",
    page: 8,
    x: 61,
    y: 1,
    w: 58,
    h: 4,
    markup: r#"
<div class="band"><div class="bg">bg</div><div class="sh"></div><div class="b4"></div><div class="b5"></div><div class="mh"></div><div class="tr"><p class="tc">t</p></div><div class="cw"><div class="cx">wxyz</div></div><div class="mk">mk</div><div class="cv">zz</div><div class="cq"><b class="cqq">cq</b></div><div class="cs">ls</div></div>
"#,
    css: r#"
.acid-t28 .band { display: flex; gap: 2; align-items: flex-start; }
.acid-t28 b { font-weight: normal; }
.acid-t28 .bg {
  box-sizing: border-box; width: 4; height: 3; padding: 1;
  background: rgb(0, 128, 128) url(x.png) no-repeat 1 1 / 2 2 fixed padding-box content-box;
  background-image: none; background-position: 0 0; background-size: auto;
  background-repeat: repeat; background-attachment: scroll; background-origin: padding-box;
  background-clip: content-box; background-blend-mode: multiply;
}
.acid-t28 .sh { width: 3; height: 1; background-color: rgb(200, 0, 0); box-shadow: 1 1 rgb(0, 0, 255); }
.acid-t28 .b4 {
  box-sizing: border-box; width: 6; height: 4;
  border-width: thin;
  border-top: solid; border-right: double rgb(0, 0, 255); border-bottom: solid; border-left: solid;
  border-right-style: double; border-bottom-style: solid; border-left-style: solid;
  border-top-color: rgb(255, 0, 0); border-right-color: rgb(0, 0, 255);
  border-bottom-color: currentcolor; border-left-color: currentcolor;
  border-top-width: thin; border-right-width: thin; border-bottom-width: thin; border-left-width: thin;
}
.acid-t28 .b5 {
  box-sizing: border-box; width: 4; border: solid; padding-bottom: 1;
  border-top-left-radius: 1; border-top-right-radius: 0;
  border-bottom-right-radius: 1; border-bottom-left-radius: 0;
}
.acid-t28 .mh { width: 2; min-height: 3; background-color: rgb(128, 128, 0); }
.acid-t28 .tr { width: 3; margin-trim: block; background-color: rgb(128, 0, 0); }
.acid-t28 .tc { margin-top: 1; }
.acid-t28 .cw { position: relative; width: 4; height: 1; }
.acid-t28 .cx { position: absolute; left: 0; top: 0; clip: rect(0, 2, 1, 0); }
.acid-t28 .mk {
  background-color: rgb(0, 128, 128);
  mask: url(m.svg); mask-image: url(m.svg); mask-mode: alpha; mask-repeat: no-repeat;
  mask-position: center; mask-clip: border-box; mask-origin: border-box; mask-size: contain;
  mask-composite: add; mask-type: luminance;
  mask-border: url(b.svg) 30 fill; mask-border-source: url(b.svg); mask-border-slice: 30 fill;
  mask-border-width: 1; mask-border-outset: 0; mask-border-repeat: stretch; mask-border-mode: alpha;
  rotate: 0deg; scale: 1; transform-origin: left top;
  box-decoration-break: clone; interpolate-size: allow-keywords;
}
.acid-t28 .cv {
  content-visibility: hidden; background-color: rgb(0, 0, 128);
  contain-intrinsic-size: 9 9;
  contain-intrinsic-width: 4; contain-intrinsic-height: 2;
  contain-intrinsic-inline-size: 4; contain-intrinsic-block-size: 2;
}
.acid-t28 .cq { width: 3; container-name: nm; container-type: inline-size; }
@container nm (width > 0) { .acid-t28 .cqq { color: rgb(0, 160, 0); } }
.acid-t28 .cs { color-scheme: light; color: light-dark(rgb(0, 160, 0), rgb(160, 0, 0)); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
