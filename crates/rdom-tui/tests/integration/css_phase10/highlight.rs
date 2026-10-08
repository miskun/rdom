//! C10-HIGHLIGHT — CSS Custom Highlight API 1: the ranges of a registered
//! highlight (`Dom::highlights_mut`) painted in its `::highlight(name)`
//! style (§5), overlapping highlights ordered by priority then
//! registration (§5.2), `::selection` the topmost highlight layer (CSS
//! Pseudo-Elements 4 §3.5), all through one overlay path.

use rdom_tui::prelude::*;
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{Color, Highlight, Position, Range, Selection};

use super::text_el;

const YELLOW: Color = Color::Rgb(255, 255, 0);
const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// A `<p>` holding `text` under the root: the dom and its text node.
fn para(text: &str) -> (TuiDom, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = text_el(&mut dom, root, "p", "", text);
    let t = dom.node(p).first_child().unwrap().id();
    (dom, t)
}

fn bytes(t: NodeId, start: usize, end: usize) -> Range {
    Range::ordered_unchecked(Position::new(t, start), Position::new(t, end))
}

/// Cascade `css`, lay out and paint `dom` in `w` × 1.
fn paint(dom: &mut TuiDom, css: &str, w: u16) -> Buffer {
    super::lay_out(dom, css, w, 1);
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    dom.paint_dom(&mut buf, area);
    buf
}

fn bg(buf: &Buffer, x: u16) -> Color {
    buf.cell(x, 0).unwrap().bg
}

/// §5: the text in a registered highlight's ranges takes its
/// `::highlight(name)` style — here its background and color — and the
/// text outside them does not.
#[test]
fn a_registered_highlight_paints_its_ranges() {
    let (mut dom, t) = para("hello world");
    dom.highlights_mut()
        .set("hit", Highlight::new([bytes(t, 6, 11)]));
    let buf = paint(
        &mut dom,
        "::highlight(hit) { background-color: yellow; color: blue }",
        12,
    );
    assert_eq!(bg(&buf, 6), YELLOW);
    assert_eq!(bg(&buf, 10), YELLOW);
    assert_eq!(buf.cell(6, 0).unwrap().fg, BLUE);
    assert_ne!(bg(&buf, 5), YELLOW);
    assert_eq!(buf.cell(6, 0).unwrap().symbol(), "w", "the text stays");
}

/// §5.2: where highlights overlap, the higher `priority` paints above;
/// at equal priority, the one registered later.
#[test]
fn overlapping_highlights_paint_by_priority_then_registration() {
    let css = "::highlight(a) { background-color: red } ::highlight(b) { background-color: blue }";
    let (mut dom, t) = para("abcdef");
    dom.highlights_mut()
        .set("a", Highlight::new([bytes(t, 0, 4)]).with_priority(1));
    dom.highlights_mut()
        .set("b", Highlight::new([bytes(t, 2, 6)]));
    let buf = paint(&mut dom, css, 6);
    assert_eq!((bg(&buf, 1), bg(&buf, 3), bg(&buf, 5)), (RED, RED, BLUE));

    let (mut dom, t) = para("abcdef");
    dom.highlights_mut()
        .set("a", Highlight::new([bytes(t, 0, 4)]));
    dom.highlights_mut()
        .set("b", Highlight::new([bytes(t, 2, 6)]));
    let buf = paint(&mut dom, css, 6);
    assert_eq!(bg(&buf, 3), BLUE, "registered later, above");
}

/// The ranges are live (DOM §5.3): text inserted before a highlight moves
/// it, so the same characters stay highlighted.
#[test]
fn a_highlight_follows_its_text_through_an_edit() {
    let (mut dom, t) = para("hello world");
    dom.highlights_mut()
        .set("hit", Highlight::new([bytes(t, 6, 11)]));
    dom.node_mut(t).edit_text(0, 0, "say ").unwrap();
    let buf = paint(
        &mut dom,
        "::highlight(hit) { background-color: yellow }",
        16,
    );
    assert_ne!(bg(&buf, 6), YELLOW);
    assert_eq!(bg(&buf, 10), YELLOW, "the w of world, moved by 4");
    assert_eq!(bg(&buf, 14), YELLOW);
}

/// CSS Pseudo-Elements 4 §3.5: `::selection` is the topmost highlight
/// layer — painted over a custom highlight, through the same overlay.
/// A highlight's `text-decoration` draws (§3.2).
#[test]
fn the_selection_paints_over_a_highlight_and_decorations_draw() {
    let (mut dom, t) = para("abcd");
    dom.highlights_mut()
        .set("x", Highlight::new([bytes(t, 0, 4)]));
    dom.set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 2),
    )));
    let buf = paint(
        &mut dom,
        "::highlight(x) { background-color: yellow; text-decoration: underline } \
         ::selection { background-color: blue }",
        4,
    );
    assert_eq!((bg(&buf, 0), bg(&buf, 3)), (BLUE, YELLOW));
    let underlined = |x| {
        buf.cell(x, 0)
            .unwrap()
            .modifier
            .contains(rdom_tui::Modifier::UNDERLINED)
    };
    assert!(underlined(3), "the highlight's underline");
}
