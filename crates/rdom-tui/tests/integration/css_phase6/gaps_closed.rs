//! C6G-DOCS — the two Box Alignment gaps the Phase 6 coverage rows
//! over-claimed, closed: `align-content` on a block container whose
//! content is inline (CSS Box Alignment 3 §5.1), and `justify-self` /
//! `align-self` on absolutely positioned boxes (§6.1 / §6.2, in the
//! inset-modified containing block, CSS Position 3 §4).

use super::{el, lay_out, paint, rect, rows};
use rdom_tui::TuiDom;

fn text(dom: &mut TuiDom, parent: rdom_tui::NodeId, data: &str) {
    let t = dom.create_text_node(data);
    dom.append_child(parent, t).unwrap();
}

/// CSS Box Alignment 3 §5.1: `align-content` on a block container
/// aligns its content as a whole in its content box — lines of inline
/// content too, not only block-level children.
#[test]
fn align_content_moves_inline_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    text(&mut dom, b, "ab");
    let buf = paint(&mut dom, ".b { height: 5; align-content: end }", 4, 5);
    assert_eq!(rows(&buf, 4, 5), ["    ", "    ", "    ", "    ", "ab  "]);
}

/// The same in an inline formatting context (an inline element beside
/// the text) holding an atomic inline: the atom's box moves with its
/// line, and `min-height` gives an `auto` height the free space (CSS 2.1
/// §10.7, as for block content).
#[test]
fn align_content_moves_an_inline_formatting_context_with_its_atoms() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    text(&mut dom, b, "x ");
    let i = el(&mut dom, b, "i", "");
    text(&mut dom, i, "z");
    text(&mut dom, b, " ");
    let s = el(&mut dom, b, "span", "ib");
    text(&mut dom, s, "y");
    lay_out(
        &mut dom,
        ".b { min-height: 5; align-content: center } .ib { display: inline-block }",
        6,
        5,
    );
    assert_eq!(rect(&dom, s).y, 2);
    assert_eq!(rect(&dom, b).height, 5);
}

/// CSS Box Alignment 3 §6.1 / CSS Position 3 §4: `justify-self: end`
/// places an absolutely positioned box at the end of its inset-modified
/// containing block (between `left` and `right`) — CSS 2.1's
/// over-constrained rule (`right` ignored) is what `normal` does.
#[test]
fn justify_self_places_an_absolutely_positioned_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "div", "cb");
    let a = el(&mut dom, cb, "div", "a");
    let n = el(&mut dom, cb, "div", "n");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 20; height: 10 } \
         .a, .n { position: absolute; left: 2; right: 2; width: 4; height: 1 } \
         .a { justify-self: end }",
        20,
        10,
    );
    assert_eq!(rect(&dom, a).x, 14);
    assert_eq!(rect(&dom, n).x, 2);
}

/// §6.1: an aligned box with an `auto` size is sized `fit-content`, not
/// stretched between its insets, then centered.
#[test]
fn an_aligned_auto_width_is_fit_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "div", "cb");
    let a = el(&mut dom, cb, "div", "a");
    text(&mut dom, a, "ab");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 20; height: 10 } \
         .a { position: absolute; left: 0; right: 0; top: 0; justify-self: center }",
        20,
        10,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, a).width), (9, 2));
}

/// §6.2 `align-self` on the block axis: `end` between `top` and
/// `bottom`.
#[test]
fn align_self_places_an_absolutely_positioned_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "div", "cb");
    let a = el(&mut dom, cb, "div", "a");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 20; height: 10 } \
         .a { position: absolute; top: 0; bottom: 0; height: 2; width: 1; align-self: end }",
        20,
        10,
    );
    assert_eq!(rect(&dom, a).y, 8);
}

/// CSS Position 3 §4.1: with one inset `auto`, that inset is 0 for the
/// inset-modified containing block — `left: 4; justify-self: end` ends
/// the box at the containing block's right edge.
#[test]
fn one_auto_inset_is_zero_for_alignment() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "div", "cb");
    let a = el(&mut dom, cb, "div", "a");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 20; height: 10 } \
         .a { position: absolute; left: 4; top: 0; width: 3; height: 1; justify-self: end }",
        20,
        10,
    );
    assert_eq!(rect(&dom, a).x, 17);
}

/// CSS 2.1 §10.3.7: `auto` margins between two insets center the box,
/// whatever `justify-self` says (Box Alignment 3 §6.1: auto margins win).
#[test]
fn auto_margins_win_over_justify_self() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "div", "cb");
    let a = el(&mut dom, cb, "div", "a");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 20; height: 10 } \
         .a { position: absolute; left: 0; right: 0; width: 4; height: 1; margin: 0 auto; \
              justify-self: end }",
        20,
        10,
    );
    assert_eq!(rect(&dom, a).x, 8);
}
