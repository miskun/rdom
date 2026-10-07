//! C10-PSEUDO-UNIFY — a positioned `::before` / `::after` is a generated
//! box like every other pseudo-element (CSS Pseudo 4 §2: it is rendered
//! "as if it were a real element", its `position` included), and the
//! positioning layer acts on it as it acts on an element: an absolutely
//! positioned one is sized and placed against its containing block like a
//! positioned element (CSS 2.1 §10.3.7 / §10.6.4), stacked in its host's
//! stacking context (Appendix E), hit-tested and counted in its scroll
//! container's overflow; a relatively positioned or sticky one is laid
//! out in flow and then shifted (§9.4.3, CSS Position 3 §3.4).

use rdom_tui::prelude::*;
use rdom_tui::render::Rect;

use super::{el, lay_out, paint_tree, text_el};

/// CSS Sizing 3 §3.1 / §5.1: an absolutely positioned box's
/// `min-content` width is its content's min-content contribution — its
/// longest unbreakable run, not its whole text — and its `auto` height
/// the rows its content wraps to at that width.
#[test]
fn an_absolute_pseudos_min_content_width_wraps_its_text() {
    let rows = paint_tree(
        ".h { position: relative; height: 3 } \
         .h::after { position: absolute; left: 0; top: 0; width: min-content; \
                     content: 'ab cd' }",
        10,
        3,
        |dom, root| {
            el(dom, root, "div", "h");
        },
    );
    assert_eq!(rows, ["ab        ", "cd        ", "          "]);
}

/// CSS 2.1 Appendix E with CSS Pseudo 4 §2 (a `::after` is its host's
/// last child): an absolutely positioned `::after` with `z-index: -1` is
/// a child stacking context of its host's, painted at step 2 — under the
/// host's in-flow text (step 7), showing where the text leaves cells.
#[test]
fn a_negative_z_index_pseudo_paints_under_its_hosts_text() {
    let rows = paint_tree(
        ".h { position: relative; z-index: 0 } \
         .h::after { position: absolute; left: 0; top: 0; z-index: -1; content: 'abcd' }",
        6,
        1,
        |dom, root| {
            text_el(dom, root, "div", "h", "XY");
        },
    );
    assert_eq!(rows, ["XYcd  "]);
}

/// CSS 2.1 §9.9.1: a box's `z-index` orders it within its stacking
/// context only. The `::after` of `.a` (a context at `z-index: 1`)
/// stays inside `.a`'s context, so `.b` (a context at `z-index: 2`)
/// paints over it, whatever the pseudo-element's own `z-index`.
#[test]
fn a_pseudo_is_stacked_inside_its_hosts_context() {
    let rows = paint_tree(
        ".a { position: relative; z-index: 1; height: 1 } \
         .a::after { position: absolute; left: 0; top: 1; z-index: 9; content: 'ppp' } \
         .b { position: relative; z-index: 2; height: 1 }",
        4,
        2,
        |dom, root| {
            el(dom, root, "div", "a");
            text_el(dom, root, "div", "b", "BB");
        },
    );
    assert_eq!(rows, ["    ", "BBp "]);
}

/// CSS Pseudo 4 §2: a pseudo-element is part of its host — its box is
/// hit like an element's, and an event on it targets the host (it has no
/// node of its own). A click on an absolutely positioned `::after` lying
/// outside its host's box targets the host.
#[test]
fn a_click_on_an_absolute_pseudo_targets_its_host() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = el(&mut dom, root, "div", "h");
    lay_out(
        &mut dom,
        ".h { position: relative; width: 4; height: 1 } \
         .h::after { position: absolute; left: 1; top: 2; content: 'xx' }",
        10,
        4,
    );
    assert_eq!(dom.hit_test(1, 2), Some(h));
    assert_ne!(dom.hit_test(5, 2), Some(h));
}

/// CSS Overflow 3 §2.2: a scroll container's scrollable overflow covers
/// "the border boxes of all boxes for which it is the containing block"
/// — an absolutely positioned `::after` included.
#[test]
fn an_absolute_pseudo_counts_in_its_scroll_containers_overflow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    lay_out(
        &mut dom,
        ".p { position: relative; width: 6; height: 3; overflow: auto } \
         .p::after { position: absolute; left: 0; top: 6; content: 'x' }",
        10,
        5,
    );
    assert_eq!(dom.node(p).scroll_height(), Some(7));
}

/// CSS 2.1 §10.3.7 / §10.6.4: an axis whose insets are both `auto`
/// starts at the static position — where the box would have been in
/// flow. An inline `::after` follows its host's last line.
#[test]
fn an_absolute_after_with_auto_insets_sits_at_its_static_position() {
    let rows = paint_tree(
        ".h { position: relative } .h::after { position: absolute; content: 'Z' }",
        6,
        2,
        |dom, root| {
            text_el(dom, root, "div", "h", "ab");
        },
    );
    assert_eq!(rows, ["abZ   ", "      "]);
}

/// CSS 2.1 §9.4.3: a relatively positioned box is laid out in flow and
/// then moved, "without affecting the layout of surrounding boxes": the
/// `::before`'s text keeps its cells in the line, the host's text after
/// them, and the pseudo-element moves down a row.
#[test]
fn a_relative_pseudo_shifts_from_its_place_in_the_line() {
    let rows = paint_tree(
        ".h::before { position: relative; top: 1; content: 'ab' }",
        6,
        2,
        |dom, root| {
            text_el(dom, root, "div", "h", "cd");
        },
    );
    assert_eq!(rows, ["  cd  ", "ab    "]);
}

/// CSS Position 3 §3.4: a sticky box stays in flow and, once scrolling
/// would carry it past its inset, is held at that inset inside its
/// scrollport. The scroller's block-level `::before` stays on its first
/// row while the content scrolls by two.
#[test]
fn a_sticky_pseudo_sticks_in_its_scrollport() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    for t in ["1", "2", "3", "4"] {
        text_el(&mut dom, p, "div", "", t);
    }
    let css = ".p { width: 3; height: 2; overflow: hidden } \
               .p::before { position: sticky; top: 0; display: block; content: 'S' }";
    lay_out(&mut dom, css, 3, 2);
    dom.node_mut(p).ext_mut().expect("styled").scroll_y = 2;
    dom.layout_dom(Rect::new(0, 0, 3, 2));
    let area = Rect::new(0, 0, 3, 2);
    let mut buf = rdom_tui::render::Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    let rows = super::rows(&buf, 3, 2);
    assert_eq!(rows, ["S  ", "3  "]);
}
