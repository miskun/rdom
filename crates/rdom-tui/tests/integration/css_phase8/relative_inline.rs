//! ACID-FIX-5 (found by acid tile 10) — a relatively positioned inline
//! element moves.
//!
//! CSS 2.1 §9.4.3 / CSS Position 3 §3.4: once a box is laid out in flow,
//! `position: relative` offsets it by its insets "without affecting the
//! layout of surrounding boxes" — an inline box included: its line keeps
//! the cells it took, and its text, background and decorations paint (and
//! are hit) at the moved place. Nested relative inline boxes add up.

use rdom_tui::prelude::*;
use rdom_tui::{HitTestExt, NodeId};

use super::{el, paint, rows};

/// `<p>ab <span class=rel>rel</span> cd</p>` in an 12 × 3 viewport.
fn line(dom: &mut TuiDom) -> NodeId {
    let root = dom.root();
    let p = el(dom, root, "p", "");
    let t = dom.create_text_node("ab ");
    dom.append_child(p, t).unwrap();
    let span = el(dom, p, "span", "rel");
    let t = dom.create_text_node("rel");
    dom.append_child(span, t).unwrap();
    let t = dom.create_text_node(" cd");
    dom.append_child(p, t).unwrap();
    span
}

#[test]
fn a_relative_inline_box_moves_off_its_cells() {
    let mut dom = TuiDom::new();
    let span = line(&mut dom);
    let buf = paint(
        &mut dom,
        "p { height: 2 } .rel { position: relative; top: 1; left: 2; background-color: rgb(0, 0, 128) }",
        12,
        3,
    );
    assert_eq!(
        rows(&buf, 12, 3),
        ["ab     cd   ", "     rel    ", "            "]
    );
    assert_eq!(
        buf.cell(5, 1).unwrap().bg,
        Color::Rgb(0, 0, 128),
        "its background moves"
    );
    assert_eq!(
        buf.cell(3, 0).unwrap().bg,
        Color::Reset,
        "nothing stays behind"
    );
    // Hit where it is drawn (inside its block's box, `height: 2`), and
    // not where its line left room for it.
    assert_eq!(dom.hit_test(6, 1), Some(span), "hit where it is drawn");
    assert_ne!(dom.hit_test(3, 0), Some(span), "not at its in-flow cells");
}

/// The insets of nested relative inline boxes add up; `right` / `bottom`
/// move the other way, and a negative inset moves up.
#[test]
fn nested_relative_inline_boxes_add_their_moves() {
    let mut dom = TuiDom::new();
    let span = line(&mut dom);
    let inner = el(&mut dom, span, "b", "in");
    let t = dom.create_text_node("X");
    dom.append_child(inner, t).unwrap();
    let buf = paint(
        &mut dom,
        "p { margin-top: 1 } .rel { position: relative; left: 1 }
         .in { position: relative; bottom: 1; right: 2 }",
        12,
        3,
    );
    // In flow: `ab relX cd`; the span moves right 1, its `X` then left 2
    // and up 1.
    assert_eq!(
        rows(&buf, 12, 3),
        ["     X      ", "ab  rel cd  ", "            "]
    );
}
