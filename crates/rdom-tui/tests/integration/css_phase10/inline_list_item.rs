//! C10G-INLINE-LIST-ITEM — CSS Display 3 §2.3 / CSS Lists 3 §3.1: every
//! list item, block-level or inline, generates a `::marker`. An inline
//! list item's marker is its first inline box: CSS Lists 3 §3.5 makes
//! `outside` "equivalent to `inside`" when the list item is an inline box,
//! so it sits in the line, not beside it. The same holds for an inline
//! list-item `::before` / `::after` (CSS Pseudo-Elements 4 §4).

use rdom_tui::ext::PseudoSlot;
use rdom_tui::prelude::*;
use rdom_tui::{Color, HitTestExt};

use super::{paint, paint_tree, rows, text_el};

const RED: Color = Color::Rgb(255, 0, 0);

/// CSS Lists 3 §3.1 / §4.6: two `display: inline list-item` spans each
/// carry a marker numbered by the `list-item` counter they increment —
/// "1. " and "2. " — as the first inline box of each span, in the line.
#[test]
fn inline_list_items_carry_their_markers_in_the_line() {
    let rows = paint_tree(
        ".l { counter-reset: list-item } \
         .i { display: inline list-item; list-style-type: decimal }",
        10,
        1,
        |dom, root| {
            let l = super::el(dom, root, "div", "l");
            text_el(dom, l, "span", "i", "a");
            text_el(dom, l, "span", "i", "b");
        },
    );
    assert_eq!(rows, ["1. a2. b  "]);
}

/// CSS Lists 3 §3.5: "If the list item is an inline box, this value
/// [`outside`] is equivalent to `inside`" — the initial position puts the
/// marker in the line, not in the parent's padding.
#[test]
fn an_inline_list_items_outside_marker_is_inside() {
    let rows = paint_tree(
        ".l { padding-left: 3 } .i { display: inline list-item; list-style: square outside }",
        8,
        1,
        |dom, root| {
            let l = super::el(dom, root, "div", "l");
            text_el(dom, l, "span", "i", "a");
        },
    );
    assert_eq!(rows, ["   ▪ a  "]);
}

/// `::marker` rules style an inline list item's marker (CSS Lists 3
/// §3.2), and the pseudo hit test names it.
#[test]
fn an_inline_list_items_marker_takes_marker_rules() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = super::el(&mut dom, root, "div", "p");
    let i = text_el(&mut dom, p, "span", "i", "a");
    let buf = paint(
        &mut dom,
        ".i { display: inline list-item } .i::marker { content: '→ '; color: red }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["→ a   "]);
    assert_eq!(buf.cell(0, 0).unwrap().fg, RED);
    assert_ne!(buf.cell(2, 0).unwrap().fg, RED, "the item's own text");
    assert_eq!(dom.hit_test_pseudo(0, 0), Some((i, PseudoSlot::Marker)));
}

/// CSS Pseudo-Elements 4 §4: an inline list-item `::before` carries
/// `::before::marker` as its first inline box, ahead of its own content.
#[test]
fn an_inline_list_item_before_has_its_marker_first() {
    let rows = paint_tree(
        ".h::before { display: inline list-item; content: 'x'; list-style-type: '- ' }",
        6,
        1,
        |dom, root| {
            text_el(dom, root, "div", "h", "a");
        },
    );
    assert_eq!(rows, ["- xa  "]);
}

/// The marker is inline content the intrinsic sizes measure: a
/// shrink-to-fit box holding an inline list item is as wide as marker
/// and text (CSS Sizing 3 §5.1).
#[test]
fn an_inline_markers_width_is_measured() {
    let rows = paint_tree(
        ".b { display: inline-block; background: red } \
         .i { display: inline list-item; list-style-type: decimal }",
        8,
        1,
        |dom, root| {
            let w = super::el(dom, root, "div", "w");
            let b = super::el(dom, w, "div", "b");
            text_el(dom, b, "span", "i", "a");
            let t = dom.create_text_node("z");
            dom.append_child(w, t).unwrap();
        },
    );
    assert_eq!(rows, ["1. az   "]);
}
