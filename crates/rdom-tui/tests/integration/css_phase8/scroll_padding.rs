//! C8-SCROLL-PADDING — `scroll-padding` and `scroll-margin` (CSS Scroll
//! Snap 1 §4): the scroll container's optimal viewing region and the
//! target's scroll snap area, which `scrollIntoView` (CSSOM View §5.1:
//! "the scroll-into-view position") and keyboard focus scrolling align.

use super::{el, lay_out};
use rdom_tui::accessors::{ScrollIntoViewOptions, ScrollLogicalPosition};
use rdom_tui::prelude::*;

/// A 10 × 4 `.port` (`overflow-y: auto`, styled `port`) of ten one-row
/// `.row`s, the sixth (row 5) styled `target`: the dom, the port and the
/// target.
fn port(port: &str, target: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "port");
    let mut t = p;
    for i in 0..10 {
        let row = el(&mut dom, p, "div", if i == 5 { "row t" } else { "row" });
        if i == 5 {
            t = row;
        }
    }
    lay_out(
        &mut dom,
        &format!(
            ".port {{ width: 10; height: 4; overflow-y: auto; {port} }} \
             .row {{ height: 1 }} .t {{ {target} }}"
        ),
        12,
        6,
    );
    (dom, p, t)
}

fn into_view(dom: &mut TuiDom, id: NodeId, block: ScrollLogicalPosition) {
    dom.node_mut(id)
        .scroll_into_view_with(ScrollIntoViewOptions::new().block(block))
        .unwrap();
}

/// CSSOM View §5.1 with Scroll Snap 1 §4.1: `scroll-padding` insets the
/// scrollport's edges the element is aligned to — `start` puts row 5 one
/// row below the top under `scroll-padding-top: 1`; a percentage is of
/// the scrollport's height (50% of 4).
#[test]
fn scroll_padding_insets_the_alignment_edge() {
    let (mut dom, p, t) = port("", "");
    into_view(&mut dom, t, ScrollLogicalPosition::Start);
    assert_eq!(dom.node(p).scroll_top(), Some(5));
    let (mut dom, p, t) = port("scroll-padding-top: 1", "");
    into_view(&mut dom, t, ScrollLogicalPosition::Start);
    assert_eq!(dom.node(p).scroll_top(), Some(4));
    let (mut dom, p, t) = port("scroll-padding: 50% 0 0", "");
    into_view(&mut dom, t, ScrollLogicalPosition::Start);
    assert_eq!(dom.node(p).scroll_top(), Some(3));
}

/// §4.1 at the end edge: `end` aligns row 5's bottom (6) one row above
/// the scrollport's bottom under `scroll-padding-bottom: 1` — `scrollTop`
/// 6 − (4 − 1) = 3.
#[test]
fn scroll_padding_insets_the_end_edge() {
    let (mut dom, p, t) = port("scroll-padding-bottom: 1", "");
    into_view(&mut dom, t, ScrollLogicalPosition::End);
    assert_eq!(dom.node(p).scroll_top(), Some(3));
}

/// Scroll Snap 1 §4.2: `scroll-margin` outsets the element's box — the
/// area aligned — so `start` stops 2 rows above it.
#[test]
fn scroll_margin_outsets_the_target() {
    let (mut dom, p, t) = port("", "scroll-margin-top: 2");
    into_view(&mut dom, t, ScrollLogicalPosition::Start);
    assert_eq!(dom.node(p).scroll_top(), Some(3));
    let (mut dom, p, t) = port("scroll-padding-top: 1", "scroll-margin-block-start: 1");
    into_view(&mut dom, t, ScrollLogicalPosition::Start);
    assert_eq!(dom.node(p).scroll_top(), Some(3));
}

/// HTML's focusing steps scroll the element into view (`nearest`): Tab
/// to a button below the scrollport brings it in, its bottom at the
/// optimal viewing region's — the scrollport less `scroll-padding-bottom`.
#[test]
fn keyboard_focus_scrolls_into_view_within_the_padding() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "port");
    let mut buttons = Vec::new();
    for _ in 0..8 {
        let b = el(&mut dom, p, "button", "b");
        let t = dom.create_text_node("go");
        dom.append_child(b, t).unwrap();
        buttons.push(b);
    }
    lay_out(
        &mut dom,
        ".port { width: 10; height: 4; overflow-y: auto; scroll-padding-bottom: 1 } \
         .b { display: block; height: 1 }",
        12,
        6,
    );
    rdom_tui::runtime::focus::focus_node(&mut dom, Some(buttons[2]));
    rdom_tui::runtime::focus::tabindex::focus_next(&mut dom);
    assert_eq!(dom.focused(), Some(buttons[3]));
    // Row 3's bottom (4) at the region's bottom (4 − 1): scrollTop 1.
    assert_eq!(dom.node(p).scroll_top(), Some(1));
}
