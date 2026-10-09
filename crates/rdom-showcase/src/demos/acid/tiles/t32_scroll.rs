//! Tile 32 — scrolling at rest (CSS Scroll Snap 1, CSS Overflow 3, CSS
//! Overscroll Behavior 1, CSSOM View; ACID-COVERAGE).
//!
//! A `y mandatory` snap container with `scroll-padding` and items with
//! `scroll-snap-align`, `scroll-snap-stop` and `scroll-margin`, scrolled
//! by its load script to a position between two snap positions; its
//! `scroll-behavior` and `overscroll-behavior`, which act on other
//! scrolls; a container with both scrollbars styled by
//! `::scrollbar-thumb:vertical` and `::scrollbar-thumb:horizontal`.

use rdom_tui::{NodeId, TuiAccessorsMut, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "32",
    title: "Scrolling at rest",
    class: "acid-t32",
    page: 9,
    x: 1,
    y: 1,
    w: 58,
    h: 4,
    markup: r#"
<div class="band"><div class="sn"><div class="it">i0</div><div class="it">i1</div><div class="it">i2</div><div class="it">i3</div><div class="it">i4</div></div><div class="bars">0123456789
0123456789
0123456789
0123456789
0123456789
0123456789</div></div>
"#,
    css: r#"
.acid-t32 .band { display: flex; gap: 2; align-items: flex-start; }
.acid-t32 .sn {
  width: 8; height: 4; overflow: auto; scrollbar-width: none;
  scroll-snap-type: y mandatory; scroll-behavior: auto;
  scroll-padding: 1 0 0 0; scroll-padding-top: 1; scroll-padding-right: 0;
  scroll-padding-bottom: 0; scroll-padding-left: 0;
  overscroll-behavior: contain; overscroll-behavior-x: contain; overscroll-behavior-y: contain;
}
.acid-t32 .it {
  height: 3; scroll-snap-align: start; scroll-snap-stop: normal;
  scroll-margin: 0; scroll-margin-top: 0; scroll-margin-right: 0;
  scroll-margin-bottom: 0; scroll-margin-left: 0;
}
.acid-t32 .it:nth-child(odd) { background-color: rgb(0, 128, 128); }
.acid-t32 .it:nth-child(even) { background-color: rgb(0, 0, 128); }
.acid-t32 .bars { width: 6; height: 3; overflow: auto; white-space: pre; }
.acid-t32 .bars::scrollbar { content: "░"; color: rgb(0, 160, 0); }
.acid-t32 .bars::scrollbar-thumb:vertical { content: "█"; color: rgb(255, 255, 0); }
.acid-t32 .bars::scrollbar-thumb:horizontal { content: "▬"; color: rgb(255, 255, 0); }
"#,
    late_css: "",
    setup: None,
    script: Some(scroll_between),
};

/// The load handler: `scrollTop = 4` on the snap container.
fn scroll_between(dom: &mut TuiDom, tile: NodeId) {
    let sn = dom
        .query_selector_in(tile, ".sn")
        .expect("a valid selector")
        .expect("the tile holds .sn");
    dom.node_mut(sn)
        .set_scroll_top(4)
        .expect("a scroll container");
}
