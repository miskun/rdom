//! Tile 55 — `position-try-order: most-height`, `anchor-center`, and a
//! `popovertarget` popover flipped by `flip-block` (CSS Anchor Positioning
//! 1 §3.1, §3.4, §4; HTML §6.12; ACID-STATIC-REST, tile 26's left-out
//! cases).
//!
//! On the page's last ten rows: a box below an anchor by `position-area:
//! bottom` whose `flip-block` fallback is tried first under
//! `position-try-order: most-height`; a box `justify-self: anchor-center`
//! under another anchor; a `popovertarget` button on the page's
//! second-to-last row whose popover (`position-area: bottom span-right`,
//! `position-try-fallbacks: flip-block`) the load script shows.

use rdom_tui::runtime::builtins::popover;
use rdom_tui::{NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "55",
    title: "Anchors: most-height, center, flip",
    class: "acid-t55",
    page: 13,
    x: 1,
    y: 40,
    w: 58,
    h: 10,
    markup: r#"
<div class="a1">A</div><div class="mh">MH</div><div class="a2">B</div><div class="ac">C</div><button popovertarget="pp55" class="pb">pp</button><div popover id="pp55" class="pp">alpha</div>
"#,
    css: r#"
.acid-t55 .a1 { position: absolute; left: 2; top: 6; width: 4; height: 1; anchor-name: --acid-a1; background-color: rgb(0, 0, 128); }
.acid-t55 .mh {
  position: absolute; position-anchor: --acid-a1; position-area: bottom;
  position-try-fallbacks: flip-block; position-try-order: most-height;
  width: 4; height: 2; background-color: rgb(0, 128, 128);
}
.acid-t55 .a2 { position: absolute; left: 20; top: 1; width: 3; height: 1; anchor-name: --acid-a2; background-color: rgb(0, 0, 128); }
.acid-t55 .ac {
  position: absolute; position-anchor: --acid-a2; top: anchor(bottom);
  justify-self: anchor-center; width: 5; height: 1; background-color: rgb(128, 0, 0);
}
.acid-t55 .pb { position: absolute; left: 30; top: 8; anchor-name: --acid-pp; }
.acid-t55 .pp {
  inset: auto; margin: 0; padding: 0;
  position-anchor: --acid-pp; position-area: bottom span-right;
  position-try-fallbacks: flip-block;
}
"#,
    late_css: "",
    setup: None,
    script: Some(show),
};

/// The load handler: `popover.showPopover()`.
fn show(dom: &mut TuiDom, tile: NodeId) {
    let pp = dom
        .query_selector_in(tile, ".pp")
        .expect("a valid selector")
        .expect("the tile holds it");
    popover::show_popover(dom, pp).expect("a popover");
}
