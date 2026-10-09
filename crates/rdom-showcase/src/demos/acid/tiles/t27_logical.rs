//! Tile 27 — flow-relative properties (CSS Logical 1), the same boxes in
//! an `ltr` and an `rtl` column (ACID-COVERAGE).
//!
//! Flow-relative sizes and their `min-` / `max-` forms; padding, margins
//! and insets by their `-block` / `-inline` sides and shorthands; borders by
//! every flow-relative shorthand and longhand; the four flow-relative
//! corner radii; `overflow-block` / `overflow-inline`; and the scroll
//! paddings and margins, `overscroll-behavior` and `writing-mode` by their
//! flow-relative names — which draw nothing in a static frame (the stage-2
//! script scrolls; every box lays out `horizontal-tb`).

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "27",
    title: "Flow-relative properties",
    class: "acid-t27",
    page: 8,
    x: 1,
    y: 1,
    w: 58,
    h: 11,
    markup: r#"
<div class="band"><div class="col"><div class="sz">a</div><div class="bd">dd</div><div class="rd"></div><div class="mg">m</div><div class="ov"><span class="sm">efghij</span><br>k</div></div><div class="col rtl"><div class="sz">a</div><div class="bd">dd</div><div class="rd"></div><div class="mg">m</div><div class="ov"><span class="sm">efghij</span><br>k</div></div></div>
"#,
    css: r#"
.acid-t27 .band { display: flex; gap: 2; align-items: flex-start; }
.acid-t27 .col { inline-size: 28; writing-mode: horizontal-tb; }
.acid-t27 .rtl { direction: rtl; }
.acid-t27 .sz {
  box-sizing: border-box; inline-size: 5; block-size: 2;
  min-inline-size: 3; max-inline-size: 9; min-block-size: 1; max-block-size: 4;
  padding-inline-start: 1; padding-block-start: 1; background-color: rgb(0, 128, 128);
}
.acid-t27 .bd {
  box-sizing: border-box; inline-size: 6; block-size: 3;
  border-block: solid; border-block-color: currentcolor; border-block-style: solid; border-block-width: thin;
  border-block-start: solid;
  border-block-start-color: currentcolor; border-block-start-style: solid; border-block-start-width: thin;
  border-block-end: double;
  border-block-end-color: currentcolor; border-block-end-style: double; border-block-end-width: thin;
  border-inline: solid; border-inline-color: currentcolor; border-inline-style: solid; border-inline-width: thin;
  border-inline-start: solid rgb(255, 0, 0);
  border-inline-start-color: rgb(255, 0, 0); border-inline-start-style: solid; border-inline-start-width: thin;
  border-inline-end: dashed;
  border-inline-end-color: rgb(0, 0, 255); border-inline-end-style: dashed; border-inline-end-width: thick;
}
.acid-t27 .rd {
  box-sizing: border-box; inline-size: 4; block-size: 3; border: solid;
  border-start-start-radius: 1; border-start-end-radius: 0;
  border-end-start-radius: 0; border-end-end-radius: 1;
}
.acid-t27 .mg {
  box-sizing: border-box; inline-size: 2; background-color: rgb(128, 128, 0);
  margin-block: 0; margin-block-start: 1; margin-block-end: 0;
  margin-inline: 0; margin-inline-start: 1; margin-inline-end: 0;
  padding-block: 0; padding-block-start: 0; padding-block-end: 0;
  padding-inline: 0 1; padding-inline-start: 0; padding-inline-end: 1;
  position: relative;
  inset-block: 0; inset-block-start: 0; inset-block-end: auto;
  inset-inline: auto; inset-inline-start: 1; inset-inline-end: auto;
}
.acid-t27 .ov {
  inline-size: 4; block-size: 1; white-space: nowrap;
  overflow-inline: hidden; overflow-block: clip;
  overscroll-behavior-block: contain; overscroll-behavior-inline: none;
  scroll-padding-block: 0; scroll-padding-block-start: 1; scroll-padding-block-end: 0;
  scroll-padding-inline: 0; scroll-padding-inline-start: 1; scroll-padding-inline-end: 0;
}
.acid-t27 .sm {
  scroll-margin-block: 0; scroll-margin-block-start: 1; scroll-margin-block-end: 0;
  scroll-margin-inline: 0; scroll-margin-inline-start: 1; scroll-margin-inline-end: 0;
}
"#,
    late_css: "",
    setup: None,
    script: None,
};
