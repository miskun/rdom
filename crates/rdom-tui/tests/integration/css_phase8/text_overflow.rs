//! C8-TEXT-OVERFLOW — `text-overflow` (CSS Overflow 4 §3): inline content
//! overflowing a line box edge of a block container that clips is cut
//! at a whole character and marked with `…` or a string, line by line.

use super::{el, paint, rows};
use rdom_tui::prelude::*;
use rdom_tui::runtime::selection::clipboard::serialize_selection;

/// A `.b` block (`width: 6`, `nowrap`, `overflow: hidden` unless `decl`
/// says otherwise) holding `text`, painted in 10 × 2: the first row.
fn line(decl: &str, text: &str) -> String {
    rows_of(decl, text).remove(0)
}

fn rows_of(decl: &str, text: &str) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node(text);
    dom.append_child(b, t).unwrap();
    let css =
        format!(".b {{ width: 6; white-space: nowrap; overflow: hidden; height: 2; {decl} }}");
    let buf = paint(&mut dom, &css, 10, 2);
    rows(&buf, 10, 2)
}

/// §3: `ellipsis` hides characters at the end edge "as necessary to fit"
/// U+2026; a `<string>` the same for the string; `clip` (initial) cuts at
/// the edge; content that fits is untouched.
#[test]
fn the_end_edge_takes_the_marker() {
    assert_eq!(line("text-overflow: ellipsis", "abcdefghij"), "abcde…    ");
    assert_eq!(line("text-overflow: '->'", "abcdefghij"), "abcd->    ");
    assert_eq!(line("", "abcdefghij"), "abcdef    ");
    assert_eq!(line("text-overflow: ellipsis", "abcdef"), "abcdef    ");
}

/// §3: the property applies where the block "has overflow other than
/// visible"; a `visible` block's line paints on uncut.
#[test]
fn a_visible_block_is_not_ellipsed() {
    assert_eq!(
        line("overflow: visible; text-overflow: ellipsis", "abcdefghij"),
        "abcdefghij"
    );
}

/// §3: characters are hidden whole — a wide character that does not fit
/// beside the marker goes, leaving its cells blank (the marker's width
/// counted in cells); and "the first character ... on a line must be
/// clipped rather than ellipsed".
#[test]
fn whole_characters_are_hidden_and_the_first_is_clipped() {
    assert_eq!(line("text-overflow: ellipsis", "ab中文字"), "ab中…     ");
    assert_eq!(
        line("width: 1; text-overflow: ellipsis", "abc"),
        "a         "
    );
}

/// §3 applies per line box: each overflowing line is marked, a short one
/// not.
#[test]
fn each_line_box_is_marked_on_its_own() {
    assert_eq!(
        rows_of("white-space: pre; text-overflow: ellipsis", "abcdefgh\nxy"),
        ["abcde…    ", "xy        "]
    );
}

/// §3: one value applies "only to the end line box edge" — under `rtl`
/// the left one, which an overflowing `rtl` line overflows (it starts at
/// the right edge, C8-RTL-LINE-OVERFLOW); the two-value form names the
/// line-left then the line-right edge, and the right one hides nothing.
#[test]
fn rtl_marks_the_named_line_edge() {
    assert_eq!(
        line("direction: rtl; text-overflow: ellipsis", "abcdefghij"),
        "…fghij    "
    );
    assert_eq!(
        line("direction: rtl; text-overflow: clip ellipsis", "abcdefghij"),
        "efghij    "
    );
}

/// The marker sits at the scrollport's edge: scrolled, content hidden
/// at the line-left edge takes the first value too.
#[test]
fn a_scrolled_line_is_marked_at_both_edges() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("abcdefghij");
    dom.append_child(b, t).unwrap();
    let css = ".b { width: 6; height: 1; white-space: nowrap; overflow: hidden; \
               text-overflow: ellipsis ellipsis }";
    super::lay_out(&mut dom, css, 10, 1);
    dom.node_mut(b).set_scroll_left(2).unwrap();
    let buf = paint(&mut dom, css, 10, 1);
    assert_eq!(rows(&buf, 10, 1), ["…defg…    "]);
}

/// The hidden characters are paint only: copying the line copies all of
/// it, as browsers do.
#[test]
fn copying_an_ellipsed_line_copies_the_whole_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let t = dom.create_text_node("abcdefghij");
    dom.append_child(b, t).unwrap();
    paint(
        &mut dom,
        ".b { width: 6; white-space: nowrap; overflow: hidden; text-overflow: ellipsis }",
        10,
        1,
    );
    let range = rdom_tui::Range::ordered_unchecked(
        rdom_tui::Position::new(t, 0),
        rdom_tui::Position::new(t, 10),
    );
    assert_eq!(serialize_selection(&dom, &range), "abcdefghij");
}
