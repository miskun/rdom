//! Selection serialization follows the rendered text
//! (`P7-CLIPBOARD-WS-1`, HTML §3.2.7 "rendered text collection steps"):
//! whitespace per `white-space`, block and `<br>` line breaks, table cells
//! and rows, no generated content.

use rdom_core::{NodeId, Position, Range};

use crate::TuiDom;
use crate::layout::{Margin, MarginValue};
use crate::render::{LayoutExt, Rect};
use crate::runtime::selection::clipboard::serialize_selection;
use crate::style::{CascadeExt, Content, Stylesheet, TuiStyle};

/// Append `<tag>text</tag>` to `parent`; returns (element, text node).
fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, text: &str) -> (NodeId, NodeId) {
    let e = dom.create_element(tag);
    let t = dom.create_text_node(text);
    dom.append_child(e, t).unwrap();
    dom.append_child(parent, e).unwrap();
    (e, t)
}

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) -> NodeId {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
    t
}

fn copy(
    dom: &mut TuiDom,
    sheet: &Stylesheet,
    from: (NodeId, usize),
    to: (NodeId, usize),
) -> String {
    dom.cascade(sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 20));
    let range = Range::ordered_unchecked(Position::new(from.0, from.1), Position::new(to.0, to.1));
    serialize_selection(dom, &range)
}

fn len(dom: &TuiDom, t: NodeId) -> usize {
    dom.node(t).node_value().unwrap().len()
}

#[test]
fn a_selection_across_two_paragraphs_has_one_line_break() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (_, a) = el(&mut dom, root, "p", "a");
    let (_, b) = el(&mut dom, root, "p", "b");
    assert_eq!(copy(&mut dom, &Stylesheet::new(), (a, 0), (b, 1)), "a\nb");
}

#[test]
fn a_paragraph_with_a_vertical_margin_is_set_off_by_a_blank_line() {
    // innerText's two required line breaks around `<p>` stand for the
    // paragraph margin; rdom's UA `<p>` has none, so they apply when the
    // author gives it one.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (_, a) = el(&mut dom, root, "p", "a");
    let (_, b) = el(&mut dom, root, "p", "b");
    let sheet = Stylesheet::new().rule_unchecked(
        "p",
        TuiStyle::new().margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Cells(0),
            MarginValue::Cells(1),
            MarginValue::Cells(0),
        )),
    );
    assert_eq!(copy(&mut dom, &sheet, (a, 0), (b, 1)), "a\n\nb");
}

#[test]
fn collapsible_whitespace_collapses_and_trims_at_line_edges() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (p, t1) = el(&mut dom, root, "p", "  one   two \n\t three ");
    let (_, t2) = el(&mut dom, p, "span", "  four  ");
    let _ = t2;
    let (_, t3) = el(&mut dom, root, "p", "\n five\n");
    let end = len(&dom, t3);
    assert_eq!(
        copy(&mut dom, &Stylesheet::new(), (t1, 0), (t3, end)),
        "one two three four\nfive"
    );
}

#[test]
fn whitespace_collapses_across_inline_boundaries() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (p, t1) = el(&mut dom, root, "p", "a ");
    el(&mut dom, p, "b", " b ");
    let t3 = text(&mut dom, p, " c");
    assert_eq!(
        copy(&mut dom, &Stylesheet::new(), (t1, 0), (t3, 2)),
        "a b c"
    );
}

#[test]
fn pre_keeps_its_whitespace() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (_, t) = el(&mut dom, root, "pre", "  a   b\n\n  c  ");
    let end = len(&dom, t);
    assert_eq!(
        copy(&mut dom, &Stylesheet::new(), (t, 0), (t, end)),
        "  a   b\n\n  c  "
    );
}

#[test]
fn pre_wrap_keeps_its_whitespace() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (_, t) = el(&mut dom, root, "div", " x  y ");
    let sheet = Stylesheet::new().rule_unchecked(
        "div",
        TuiStyle::new().white_space(crate::layout::WhiteSpace::PreWrap),
    );
    assert_eq!(copy(&mut dom, &sheet, (t, 0), (t, 6)), " x  y ");
}

#[test]
fn br_is_a_line_break() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.append_child(root, p).unwrap();
    let a = text(&mut dom, p, "a");
    let br = dom.create_element("br");
    dom.append_child(p, br).unwrap();
    let b = text(&mut dom, p, "b");
    let _ = a;
    assert_eq!(copy(&mut dom, &Stylesheet::new(), (a, 0), (b, 1)), "a\nb");
}

#[test]
fn table_cells_are_tab_separated_and_rows_newline_separated() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    dom.append_child(root, table).unwrap();
    let tr1 = dom.create_element("tr");
    dom.append_child(table, tr1).unwrap();
    let (_, a) = el(&mut dom, tr1, "td", "a");
    el(&mut dom, tr1, "td", "b");
    let tr2 = dom.create_element("tr");
    dom.append_child(table, tr2).unwrap();
    el(&mut dom, tr2, "td", "c");
    let (_, d) = el(&mut dom, tr2, "th", "d");
    assert_eq!(
        copy(&mut dom, &Stylesheet::new(), (a, 0), (d, 1)),
        "a\tb\nc\td"
    );
}

#[test]
fn generated_content_is_not_copied() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (_, a) = el(&mut dom, root, "p", "a");
    let (_, b) = el(&mut dom, root, "p", "b");
    let sheet = Stylesheet::new().rule_unchecked(
        "p::before",
        TuiStyle::new().content(Content::Str("> ".into())),
    );
    assert_eq!(copy(&mut dom, &sheet, (a, 0), (b, 1)), "a\nb");
}

#[test]
fn list_markers_are_not_copied() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = dom.create_element("ul");
    dom.append_child(root, ul).unwrap();
    let (_, a) = el(&mut dom, ul, "li", "one");
    let (_, b) = el(&mut dom, ul, "li", "two");
    assert_eq!(
        copy(&mut dom, &Stylesheet::new(), (a, 0), (b, 3)),
        "one\ntwo"
    );
}

#[test]
fn display_none_content_is_not_copied() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (p, a) = el(&mut dom, root, "p", "a");
    el(&mut dom, p, "span", "HIDDEN");
    let c = text(&mut dom, p, "c");
    let sheet = Stylesheet::new().rule_unchecked(
        "span",
        TuiStyle::new().display(crate::layout::Display::None),
    );
    assert_eq!(copy(&mut dom, &sheet, (a, 0), (c, 1)), "ac");
}

#[test]
fn a_partial_selection_inside_one_text_node_collapses_too() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (_, t) = el(&mut dom, root, "p", "a   b   c");
    // "   b   " — the run collapses; the edges are mid-line, so a
    // collapsed space at either end stays.
    assert_eq!(copy(&mut dom, &Stylesheet::new(), (t, 1), (t, 8)), " b ");
}

/// HTML §3.2.7 rendered text reads the *used* `visibility` — paint's
/// answer (`render::visibility::visibility_of`): mid-way through a
/// transition from `hidden` the box presents `visible` (CSS Display 3
/// §4), its text is drawn, so it is copied.
#[test]
fn copy_reads_the_presented_visibility_mid_transition() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (p, a) = el(&mut dom, root, "p", "a");
    let (span, _) = el(&mut dom, p, "span", "B");
    let c = text(&mut dom, p, "c");
    let sheet = Stylesheet::new().rule_unchecked(
        "span",
        TuiStyle::new().visibility(crate::layout::Visibility::Hidden),
    );
    assert_eq!(copy(&mut dom, &sheet, (a, 0), (c, 1)), "ac");
    dom.node_mut(span)
        .ext_mut()
        .expect("cascaded")
        .presentation_for_mut(crate::ext::StyleSlot::Host)
        .visibility = Some(crate::layout::Visibility::Visible);
    let range = Range::ordered_unchecked(Position::new(a, 0), Position::new(c, 1));
    assert_eq!(serialize_selection(&dom, &range), "aBc");
}
