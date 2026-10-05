//! C8G-PSEUDO-BOXES — `::before` / `::after` honour their computed
//! `display` (CSS Pseudo 4 §2: they "are rendered as boxes … as if they
//! were real elements"; CSS 2.1 §12.1: their `display` makes them block-
//! or inline-level). A block-level one is a block box of its host's flow
//! — `content: ""` an empty one — that `clear` moves below floats: the
//! clearfix.

use super::{el, paint, rect, rows};
use rdom_tui::prelude::*;

/// `<body><div class=c>…</div><p>next</p></body>` with `.c` holding a
/// 3-row left float, styled by `css`: the painted 10 × 6 rows, the dom,
/// `.c` and the `<p>`.
fn clearfix(css: &str) -> (Vec<String>, TuiDom, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let c = el(&mut dom, body, "div", "c");
    let f = el(&mut dom, c, "div", "f");
    let t = dom.create_text_node("A");
    dom.append_child(f, t).unwrap();
    let p = el(&mut dom, body, "p", "");
    let t = dom.create_text_node("next");
    dom.append_child(p, t).unwrap();
    let buf = paint(
        &mut dom,
        &format!(".f {{ float: left; width: 3; height: 3 }} {css}"),
        10,
        6,
    );
    (rows(&buf, 10, 6), dom, c, p)
}

/// The clearfix: `.c::after { content: ""; display: block; clear: both }`
/// is an empty block box below the float (§9.5.2), so `.c` is 3 rows
/// tall and "next" goes below it, not beside the float.
#[test]
fn the_clearfix_contains_the_float() {
    let (painted, dom, c, p) = clearfix(".c::after { content: \"\"; display: block; clear: both }");
    assert_eq!(rect(&dom, c).height, 3, "{painted:?}");
    assert_eq!(rect(&dom, p).y, 3);
    assert_eq!(&painted[3][..4], "next");
}

/// Without it the float pokes out of `.c` (0 rows) and "next" wraps
/// beside it — the case the clearfix exists for.
#[test]
fn without_the_clearfix_the_text_wraps_beside_the_float() {
    let (painted, dom, c, _) = clearfix("");
    assert_eq!(rect(&dom, c).height, 0);
    assert_eq!(&painted[0][..7], "A  next");
}

/// A block host whose `::before` / `::after` is block-level, its content
/// and children: the painted 10 × 4 rows.
fn host(css: &str) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let h = el(&mut dom, body, "div", "h");
    let t = dom.create_text_node("body");
    dom.append_child(h, t).unwrap();
    let buf = paint(&mut dom, css, 10, 4);
    rows(&buf, 10, 4)
}

/// `display: block` puts the generated text on a line of its own, above
/// (`::before`) or below (`::after`) the host's text.
#[test]
fn a_block_pseudo_is_a_line_of_its_own() {
    assert_eq!(
        host(".h::before { content: \"Title\"; display: block }")[..2],
        ["Title     ", "body      "]
    );
    assert_eq!(
        host(".h::after { content: \"End\"; display: block }")[..2],
        ["body      ", "End       "]
    );
}

/// An empty block box keeps its size: `height: 2` pushes the text two
/// rows down; its margin separates it like any block's.
#[test]
fn an_empty_block_pseudo_takes_its_height_and_margins() {
    assert_eq!(
        host(".h::before { content: \"\"; display: block; height: 2 }")[2],
        "body      "
    );
    assert_eq!(
        host(".h::before { content: \"T\"; display: block; margin-bottom: 1 }")[..3],
        ["T         ", "          ", "body      "]
    );
}

/// `display: flex` / `grid` / `flow-root` make block-level boxes too.
#[test]
fn flex_grid_and_flow_root_pseudos_are_block_level() {
    for d in ["flex", "grid", "flow-root"] {
        assert_eq!(
            host(&format!(".h::before {{ content: \"T\"; display: {d} }}"))[..2],
            ["T         ", "body      "],
            "{d}"
        );
    }
}

/// `display: none` generates nothing; the default `inline` stays in the
/// host's line.
#[test]
fn none_and_inline_pseudos_are_unchanged() {
    assert_eq!(
        host(".h::before { content: \"T\"; display: none }")[0],
        "body      "
    );
    assert_eq!(host(".h::before { content: \"T\" }")[0], "Tbody     ");
}

/// The box paints like an element's: its background fills its border
/// box, a full row in its host.
#[test]
fn a_block_pseudo_paints_its_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let body = el(&mut dom, root, "body", "");
    let h = el(&mut dom, body, "div", "h");
    let t = dom.create_text_node("body");
    dom.append_child(h, t).unwrap();
    let buf = paint(
        &mut dom,
        ".h::before { content: \"T\"; display: block; background-color: rgb(200, 0, 0) }",
        10,
        3,
    );
    for x in 0..10 {
        assert_eq!(
            buf.cell(x, 0).unwrap().bg,
            rdom_tui::style::Color::Rgb(200, 0, 0),
            "column {x}"
        );
    }
    assert_ne!(
        buf.cell(0, 1).unwrap().bg,
        rdom_tui::style::Color::Rgb(200, 0, 0)
    );
}

/// Intrinsic sizes count it: a flex item hosting a block `::before` is
/// two rows tall (its line, then the host's text) and as wide as the
/// wider of the two.
#[test]
fn a_block_pseudo_counts_in_its_hosts_intrinsic_size() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let h = el(&mut dom, row, "div", "h");
    let t = dom.create_text_node("ab");
    dom.append_child(h, t).unwrap();
    super::lay_out(
        &mut dom,
        ".row { display: flex; align-items: flex-start } \
         .h::before { content: \"Title\"; display: block }",
        20,
        6,
    );
    let r = rect(&dom, h);
    assert_eq!((r.width, r.height), (5, 2));
}
