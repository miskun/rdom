//! Tile 46 — scroll-driven animations (Scroll-driven Animations 1;
//! ACID-INTERACTIVE I18).
//!
//! A scroll container (14 × 4, sixteen rows) holding a sticky teal bar
//! whose `width` follows `scroll()`, a two-row navy item fading in over
//! its `view()` timeline's `entry` range, and a magenta item whose
//! `width` peaks at a `contain 50%` keyframe; below it an `rtl` scroller
//! naming an inline `scroll-timeline` that an olive bar outside it reads
//! through `timeline-scope`. Step I18 scrolls them and finally removes the
//! container's scrolling (`overflow: clip`).

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "46",
    title: "Scroll-driven animations",
    class: "acid-t46",
    page: 12,
    x: 61,
    y: 6,
    w: 38,
    h: 7,
    markup: r#"
<div class="sc"><div class="bar"></div><p>r1</p><p>r2</p><p>r3</p><p>r4</p><p>r5</p><div class="v1">v1</div><p>r8</p><p>r9</p><div class="v2"></div><p>r11</p><p>r12</p><p>r13</p><p>r14</p><p>r15</p></div>
<div class="ts"><div class="hs"><div class="wide"></div></div><div class="hbar"></div></div>
"#,
    css: r#"
@keyframes acid-g12 { from { width: 0; } to { width: 12; } }
@keyframes acid-op { from { opacity: 0; } to { opacity: 1; } }
@keyframes acid-v2 { contain 50% { width: 8; } }
.acid-t46 .sc { width: 14; height: 4; overflow: auto; scrollbar-width: none; }
.acid-t46 .sc.flat { overflow: clip; }
.acid-t46 p { margin: 0; }
.acid-t46 .bar {
  position: sticky; top: 0; width: 1; height: 1; background-color: rgb(0, 128, 128);
  animation: acid-g12 linear; animation-timeline: scroll();
}
.acid-t46 .v1 {
  height: 2; background-color: rgb(0, 0, 128); color: rgb(200, 200, 200);
  animation: acid-op linear both; animation-timeline: view(); animation-range: entry;
}
.acid-t46 .v2 {
  width: 2; height: 1; background-color: rgb(128, 0, 128);
  animation: acid-v2 linear; animation-timeline: view();
}
.acid-t46 .ts { timeline-scope: --acid-h; margin-top: 1; }
.acid-t46 .hs {
  direction: rtl; width: 12; height: 1; overflow-x: auto; overflow-y: hidden;
  scrollbar-width: none; scroll-timeline: --acid-h inline;
}
.acid-t46 .wide { width: 24; height: 1; }
.acid-t46 .hbar {
  width: 1; height: 1; background-color: rgb(128, 128, 0);
  animation: acid-g12 linear; animation-timeline: --acid-h;
}
"#,
    late_css: "",
    setup: None,
    script: None,
};
