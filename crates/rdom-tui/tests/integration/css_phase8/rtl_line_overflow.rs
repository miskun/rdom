//! C8-RTL-LINE-OVERFLOW — a line wider than its `rtl` block starts at
//! the block's right (inline-start) edge and overflows the left (end)
//! edge (CSS Text 3 §7.1: `text-align: start` aligns a line box's
//! content to its start edge; CSS Writing Modes 4 §2.1: the inline-start
//! edge of an `rtl` box is its right one), as browsers lay it out. The
//! text keeps its logical order (rdom does not reorder bidirectional
//! text, DIVERGENCES).

use super::{el, paint, rows};
use rdom_tui::prelude::*;
use rdom_tui::render::inline::cell_of_position;

const BOX: &str = "width: 4; direction: rtl; white-space: nowrap";

/// A `.b` block styled `BOX` and `decl` holding `abcdefgh`, in a 10 × 1
/// viewport: the dom, the block and its text.
fn block(decl: &str) -> (TuiDom, NodeId, NodeId, String) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("abcdefgh");
    dom.append_child(b, t).unwrap();
    let css = format!(".b {{ {BOX}; {decl} }}");
    super::lay_out(&mut dom, &css, 10, 1);
    (dom, b, t, css)
}

fn painted(decl: &str) -> String {
    let (mut dom, _, _, css) = block(decl);
    let buf = paint(&mut dom, &css, 10, 1);
    rows(&buf, 10, 1).remove(0)
}

/// §7.1 with Writing Modes §2.1: the line's end is flush with the
/// box's right edge, so a clipping box shows the line's last cells.
#[test]
fn an_overflowing_rtl_line_shows_its_start_side() {
    assert_eq!(painted("overflow: hidden"), "efgh      ");
}

/// A box that does not clip paints the overflow past its left edge —
/// here the box sits 4 cells in, so the line paints from column 0.
#[test]
fn an_overflowing_rtl_line_paints_past_the_left_edge() {
    assert_eq!(painted("margin-left: 4"), "abcdefgh  ");
}

/// CSS Overflow 4 §3: one `text-overflow` value is the end line box
/// edge — the left one in an `rtl` block — where the line now overflows:
/// the marker replaces the last character that fits there. The two-value
/// form names line-left then line-right.
#[test]
fn one_value_text_overflow_marks_the_left_edge() {
    assert_eq!(
        painted("overflow: hidden; text-overflow: ellipsis"),
        "…fgh      "
    );
    assert_eq!(
        painted("overflow: hidden; text-overflow: ellipsis clip"),
        "…fgh      "
    );
    assert_eq!(
        painted("overflow: hidden; text-overflow: clip ellipsis"),
        "efgh      "
    );
}

/// CSSOM View §4: an `rtl` scroll container's scrolling area runs left
/// of its origin — the overflowing start of the line is reached with a
/// negative `scrollLeft` (C5G-RTL-SCROLL), and is the whole extent.
#[test]
fn the_overflow_is_reached_with_a_negative_scroll_left() {
    let (mut dom, b, _, css) = block("overflow: hidden");
    assert_eq!(dom.node(b).scroll_width(), Some(8));
    dom.node_mut(b).set_scroll_left(-4).unwrap();
    assert_eq!(dom.node(b).scroll_left(), Some(-4));
    let buf = paint(&mut dom, &css, 10, 1);
    assert_eq!(rows(&buf, 10, 1), ["abcd      "]);
}

/// The caret and hit-testing read the same geometry: the box's first
/// column holds `e` (byte 4), and the caret before `f` is in column 1.
#[test]
fn caret_and_hit_test_follow_the_shifted_line() {
    let (dom, _, t, _) = block("overflow: hidden");
    let hit = dom.caret_position_from_point(0, 0).expect("a text hit");
    assert_eq!((hit.node, hit.offset), (t, 4));
    assert_eq!(cell_of_position(&dom, Position::new(t, 5)), Some((1, 0)));
}

/// The selection highlight follows the shifted line too: in a box 2
/// cells in, `def` (bytes 3–6) sits at columns 1–3, and the box clips
/// column 1, so only `e` and `f` are highlighted.
#[test]
fn the_selection_highlight_follows_the_shifted_line() {
    let (mut dom, _, t, css) = block("overflow: hidden; margin-left: 2");
    dom.set_selection(Some(Selection {
        anchor: Position::new(t, 3),
        focus: Position::new(t, 6),
    }));
    let buf = paint(&mut dom, &css, 10, 1);
    let lit: Vec<bool> = (0..8)
        .map(|x| buf.cell(x, 0).unwrap().bg != rdom_tui::render::Color::Reset)
        .collect();
    assert_eq!(
        lit,
        [false, false, true, true, false, false, false, false],
        "row {:?}",
        rows(&buf, 10, 1)
    );
}

/// A point left of the box, on the overflowing part of the line, hits
/// the character painted there: in a box 2 cells in that does not clip,
/// column 0 shows `c` (byte 2).
#[test]
fn a_hit_left_of_the_box_finds_the_overflowing_text() {
    let (dom, _, t, _) = block("margin-left: 2");
    let hit = dom.caret_position_from_point(0, 0).expect("a text hit");
    assert_eq!((hit.node, hit.offset), (t, 2));
}
