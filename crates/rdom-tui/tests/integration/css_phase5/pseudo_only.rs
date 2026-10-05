//! C5G-PSEUDO-ONLY — an element whose only content is its `::before` /
//! `::after` shows it. CSS 2.1 §12.1 / CSS Pseudo-Elements 4 §2.4: the
//! pseudo-elements are the element's first / last child boxes, so an
//! otherwise empty element's generated content is its content — it gets
//! a line box (CSS 2.1 §9.2.1.1), the box is as tall as that line, and
//! the text paints.

use super::{el, paint, rows, size};
use rdom_tui::{NodeId, TuiDom};

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
}

const BEFORE: &str = ".b::before { content: \"<\" }";
const AFTER: &str = ".b::after { content: \">\" }";
const BOTH: &str = ".b::before { content: \"<\" } .b::after { content: \">\" }";

/// An empty block `<div class=b>` above a sibling line.
fn block_host(css: &str) -> (Vec<String>, (u16, u16)) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "div", "");
    let b = el(&mut dom, body, "div", "b");
    let next = el(&mut dom, body, "div", "");
    text(&mut dom, next, "next");
    let buf = paint(&mut dom, css, 6, 3);
    (rows(&buf, 6, 3), size(&dom, b))
}

#[test]
fn an_empty_block_shows_its_before() {
    let (rows, size) = block_host(BEFORE);
    assert_eq!(rows, vec!["<     ", "next  ", "      "]);
    assert_eq!(size, (6, 1));
}

#[test]
fn an_empty_block_shows_its_after() {
    let (rows, _) = block_host(AFTER);
    assert_eq!(rows, vec![">     ", "next  ", "      "]);
}

#[test]
fn an_empty_block_shows_both() {
    let (rows, _) = block_host(BOTH);
    assert_eq!(rows, vec!["<>    ", "next  ", "      "]);
}

/// An empty inline `<span class=b>` between two words of a line: its
/// generated content is inline content in the line.
fn inline_host(css: &str) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    text(&mut dom, p, "a");
    el(&mut dom, p, "span", "b");
    text(&mut dom, p, "c");
    let buf = paint(&mut dom, css, 6, 1);
    rows(&buf, 6, 1)
}

#[test]
fn an_empty_inline_shows_its_before() {
    assert_eq!(inline_host(BEFORE), vec!["a<c   "]);
}

#[test]
fn an_empty_inline_shows_its_after() {
    assert_eq!(inline_host(AFTER), vec!["a>c   "]);
}

#[test]
fn an_empty_inline_shows_both() {
    assert_eq!(inline_host(BOTH), vec!["a<>c  "]);
}

/// An empty inline as the only content of its block.
#[test]
fn an_empty_inline_alone_in_a_block_shows_its_pseudos() {
    for (css, want) in [(BEFORE, "<     "), (AFTER, ">     "), (BOTH, "<>    ")] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "p", "");
        el(&mut dom, p, "span", "b");
        let buf = paint(&mut dom, css, 6, 1);
        assert_eq!(rows(&buf, 6, 1), vec![want], "{css}");
    }
}

/// A flex item (the document root lays its children out as flex items)
/// and an inline block measure their generated content: a row tall, as
/// wide as the text (CSS Sizing 3 §5.2 — the pseudo-elements are the
/// content the intrinsic size measures).
#[test]
fn a_pseudo_only_flex_item_and_inline_block_measure_their_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let next = el(&mut dom, root, "div", "");
    text(&mut dom, next, "next");
    let buf = paint(&mut dom, BOTH, 6, 2);
    assert_eq!(rows(&buf, 6, 2), vec!["<>    ", "next  "]);
    assert_eq!(size(&dom, b).1, 1);

    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    text(&mut dom, p, "a");
    let ib = el(&mut dom, p, "span", "b ib");
    text(&mut dom, p, "c");
    let buf = paint(
        &mut dom,
        &format!("{BOTH} .ib {{ display: inline-block }}"),
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), vec!["a<>c  "]);
    assert_eq!(size(&dom, ib), (2, 1));
}

/// A block whose generated content holds a line is not empty: its
/// vertical margins do not collapse through it (CSS 2.1 §8.3.1).
#[test]
fn a_pseudo_only_block_keeps_its_margins_apart() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "div", "");
    el(&mut dom, body, "div", "b");
    let next = el(&mut dom, body, "div", "");
    text(&mut dom, next, "next");
    let buf = paint(&mut dom, &format!("{BEFORE} .b {{ margin: 1 0 }}"), 6, 4);
    assert_eq!(
        rows(&buf, 6, 4),
        vec!["      ", "<     ", "      ", "next  "]
    );
}
