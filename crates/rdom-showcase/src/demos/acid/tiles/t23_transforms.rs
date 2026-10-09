//! Tile 23 — transforms (CSS Transforms 1 / 2).
//!
//! A panel slid half off the tile's left edge by `translate: -50% 0`
//! (cut by the tile's clip) and another slid over an in-flow sibling it
//! overlaps without pushing; `transform: translateX(2) rotate(45deg)
//! translateY(1)` (the rotation inert, the translations summed);
//! `translate: 2.5` and `translate: 1.5` (both two cells, ties to even); a
//! `transform-box: content-box` percentage; a `transform: scale(1)` box
//! whose `z-index: -1` child paints above its background; a `position:
//! fixed` badge in a transformed card; an `overflow: auto` box whose
//! translated child gives it a scrollbar.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "23",
    title: "Transforms",
    class: "acid-t23",
    page: 6,
    x: 61,
    y: 5,
    w: 58,
    h: 5,
    markup: r#"
<div class="band"><div class="edge">0123456789</div><div class="sib">abcdefgh</div><div class="pnl">0123456789</div></div>
<div class="band b2"><div class="tf">T</div><div class="t25">A</div><div class="t15">B</div><div class="tb">C</div><div class="s1"><b class="zn">Z</b></div><div class="card"><b class="badge">!</b></div><div class="sc"><div class="ch">m</div></div></div>
"#,
    css: r#"
.acid-t23 .band { display: flex; gap: 2; align-items: flex-start; margin-bottom: 1; }
.acid-t23 .b2 { gap: 3; }
.acid-t23 b { font-weight: normal; }
.acid-t23 .edge { width: 10; translate: -50% 0; background-color: rgb(0, 128, 128); }
.acid-t23 .pnl { width: 10; translate: -50% 0; background-color: rgb(0, 0, 128); }
.acid-t23 .tf { transform: translateX(2) rotate(45deg) translateY(1); }
.acid-t23 .t25 { translate: 2.5; }
.acid-t23 .t15 { translate: 1.5; }
.acid-t23 .tb {
  width: 4; padding: 0 2; transform-box: content-box; translate: 50%;
  background-color: rgb(128, 128, 0);
}
.acid-t23 .s1 { transform: scale(1); width: 4; height: 1; background-color: rgb(128, 0, 0); }
.acid-t23 .zn { position: absolute; z-index: -1; left: 0; top: 0; color: rgb(255, 255, 0); }
.acid-t23 .card { transform: translateX(0); width: 8; height: 2; background-color: rgb(0, 0, 128); }
.acid-t23 .badge { position: fixed; right: 0; top: 0; background-color: rgb(0, 128, 128); }
.acid-t23 .sc {
  overflow: auto; width: 6; height: 3;
  scrollbar-width: thin; scrollbar-color: rgb(255, 255, 0) rgb(0, 0, 128);
}
.acid-t23 .ch { translate: 0 5; height: 1; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
