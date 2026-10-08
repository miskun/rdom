//! C10-FIRST — CSS Pseudo-Elements 4 §2.2 `::first-line` and §2.3
//! `::first-letter`: the first formatted line of a block container and
//! its first typographic letter unit, styled through the packer's style
//! switch at the line / letter boundary — the DOM text is never split,
//! so selection, the caret, copy and hit-testing stay on the source.

use rdom_tui::layout::{Float, TextCase};
use rdom_tui::prelude::*;

use super::{el, lay_out, text_el};

/// §2.2.1 / §2.3.1: `::first-line` exists on a block container whose
/// rules match it, its computed style inheriting from the block; the
/// properties outside its subset do not reach it. `::first-letter`
/// inherits from `::first-line` (the fictional tag sequence, §2.3.1:
/// it sits inside the first line's pseudo-element). An inline element
/// is no block container: it has neither.
#[test]
fn first_line_and_first_letter_are_cascaded() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = text_el(&mut dom, root, "p", "", "hello ");
    let span = text_el(&mut dom, p, "span", "", "world");
    lay_out(
        &mut dom,
        "p { color: green; letter-spacing: 1 } \
         p::first-line, span::first-line { color: red; padding: 3 } \
         p::first-letter, span::first-letter { float: left; text-transform: uppercase }",
        20,
        3,
    );
    let ext = dom.node(p).ext().unwrap();
    let line = ext
        .computed_first_line()
        .map(|s| &**s)
        .expect("p has a ::first-line");
    assert_eq!(line.fg, rdom_tui::Color::Rgb(255, 0, 0));
    assert_eq!(
        line.text.letter_spacing.cells(),
        1,
        "inherited from the block"
    );
    assert_eq!(line.padding.top.resolve(20), 0, "padding does not apply");
    let letter = ext
        .computed_first_letter()
        .map(|s| &**s)
        .expect("p has a ::first-letter");
    assert_eq!(
        letter.fg,
        rdom_tui::Color::Rgb(255, 0, 0),
        "from ::first-line"
    );
    assert_eq!(letter.float, Float::Left);
    assert_eq!(letter.text.text_transform.case, TextCase::Uppercase);
    let span_ext = dom.node(span).ext().unwrap();
    assert!(span_ext.computed_first_line().is_none());
    assert!(span_ext.computed_first_letter().is_none());
    let _ = el;
}

const RED: rdom_tui::Color = rdom_tui::Color::Rgb(255, 0, 0);

/// Paint `css` over a `<p>` holding `text` in a `w` × `h` viewport: the
/// buffer and the `<p>`.
fn para(css: &str, text: &str, w: u16, h: u16) -> (rdom_tui::render::Buffer, TuiDom, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = text_el(&mut dom, root, "p", "", text);
    let buf = super::paint(&mut dom, css, w, h);
    (buf, dom, p)
}

fn fg(buf: &rdom_tui::render::Buffer, x: u16, y: u16) -> rdom_tui::Color {
    buf.cell(x, y).expect("in the buffer").fg
}

/// §2.2: `::first-line` styles the contents of the first formatted line
/// of its block container; the block's other lines keep its own style.
#[test]
fn first_line_colors_the_first_line_only() {
    let (buf, ..) = para(
        "p { width: 7 } p::first-line { color: red }",
        "aaa bbb ccc",
        8,
        2,
    );
    assert_eq!(super::rows(&buf, 8, 2), ["aaa bbb ", "ccc     "]);
    assert_eq!(fg(&buf, 0, 0), RED);
    assert_eq!(fg(&buf, 6, 0), RED);
    assert_ne!(fg(&buf, 0, 1), RED);
}

/// §2.2.1 (the fictional tag sequence): `::first-line` sits between the
/// block and its inline content, so an inline element on the first line
/// without its own value inherits the first line's — a `::before` there
/// too — while one with its own keeps it.
#[test]
fn first_line_inherits_into_inline_children() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    text_el(&mut dom, p, "span", "", "ab");
    let t = dom.create_text_node(" ");
    dom.append_child(p, t).unwrap();
    text_el(&mut dom, p, "span", "g", "cd");
    let buf = super::paint(
        &mut dom,
        "p::before { content: '*' } p::first-line { color: red } .g { color: green }",
        10,
        1,
    );
    assert_eq!(super::rows(&buf, 10, 1), ["*ab cd    "]);
    assert_eq!(fg(&buf, 0, 0), RED, "the ::before");
    assert_eq!(fg(&buf, 1, 0), RED, "an inline child");
    assert_eq!(
        fg(&buf, 4, 0),
        rdom_tui::Color::Rgb(0, 128, 0),
        "its own color"
    );
}

/// §2.2: when a block container's first in-flow content is a block, its
/// first formatted line is that block's first line — `div::first-line`
/// reaches it — and no later block's.
#[test]
fn first_line_reaches_into_the_first_child_block() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let div = el(&mut dom, root, "div", "");
    text_el(&mut dom, div, "p", "", "aaa");
    text_el(&mut dom, div, "p", "", "bbb");
    let buf = super::paint(&mut dom, "div::first-line { color: red }", 5, 2);
    assert_eq!(super::rows(&buf, 5, 2), ["aaa  ", "bbb  "]);
    assert_eq!(fg(&buf, 0, 0), RED);
    assert_ne!(fg(&buf, 0, 1), RED);
}

/// §2.2.1 with CSS Text 3 §2.1 / §9: `text-transform` and
/// `letter-spacing` on `::first-line` reshape the first line only. The
/// style switches at the line boundary: the word that no longer fits is
/// laid out on the next line in the block's own style (`ccc`, `cd`).
#[test]
fn first_line_transform_and_spacing_switch_at_the_line_end() {
    let (buf, ..) = para(
        "p { width: 7 } p::first-line { text-transform: uppercase }",
        "aaa bbb ccc",
        7,
        2,
    );
    assert_eq!(super::rows(&buf, 7, 2), ["AAA BBB", "ccc    "]);
    let (buf, ..) = para(
        "p { width: 7 } p::first-line { letter-spacing: 1 }",
        "ab cd ef",
        7,
        2,
    );
    assert_eq!(super::rows(&buf, 7, 2), ["a b    ", "cd ef  "]);
}

/// The DOM text is never split: a cell of the reshaped first line maps
/// back to its source offset (CSS Text 3 §9: a unit and its spacing are
/// one for hit-testing), so the caret and selection work in the source.
#[test]
fn the_first_lines_cells_map_to_the_source_text() {
    let (_, dom, p) = para("p::first-line { letter-spacing: 1 }", "abc", 8, 1);
    let text = dom.node(p).first_child().unwrap().id();
    let at = dom.position_at(2, 0).expect("a text position");
    assert_eq!((at.node, at.offset), (text, 1));
}

/// §2.2.1: the background properties apply — behind the first line's
/// text, not the next line's.
#[test]
fn first_line_background_paints_the_first_lines_text() {
    let (buf, ..) = para(
        "p { width: 3 } p::first-line { background-color: blue }",
        "aa bb",
        3,
        2,
    );
    let blue = rdom_tui::Color::Rgb(0, 0, 255);
    assert_eq!(buf.cell(0, 0).unwrap().bg, blue);
    assert_ne!(buf.cell(0, 1).unwrap().bg, blue);
}

/// CSS Overflow 4 §4 with §2.2: a line-clamped block's first line is
/// still its first formatted line; the clamp counts the reshaped lines.
#[test]
fn first_line_in_a_line_clamped_block() {
    let (buf, ..) = para(
        "p { width: 3; line-clamp: 2 } p::first-line { text-transform: uppercase }",
        "aa bb cc",
        3,
        3,
    );
    let rows = super::rows(&buf, 3, 3);
    assert_eq!(rows[0], "AA ");
    assert!(rows[1].starts_with("bb"), "{rows:?}");
    assert_eq!(rows[2], "   ");
}

// ── ::first-letter (part 3) ─────────────────────────────────────────

const BLUE: rdom_tui::Color = rdom_tui::Color::Rgb(0, 0, 255);

/// §2.3: `::first-letter` styles the first typographic letter unit of
/// its block's first formatted line, and nothing after it.
#[test]
fn first_letter_styles_the_first_letter_only() {
    let (buf, ..) = para("p::first-letter { color: blue }", "hello", 8, 1);
    assert_eq!(super::rows(&buf, 8, 1), ["hello   "]);
    assert_eq!(fg(&buf, 0, 0), BLUE);
    assert_ne!(fg(&buf, 1, 0), BLUE);
}

/// §2.3.2: punctuation preceding and following the first letter belongs
/// to it; leading white space does not; the letter can sit in an inline
/// element.
#[test]
fn first_letter_takes_its_punctuation() {
    let (buf, ..) = para("p::first-letter { color: blue }", "  (a). b", 8, 1);
    assert_eq!(super::rows(&buf, 8, 1), ["(a). b  "]);
    for x in 0..4 {
        assert_eq!(fg(&buf, x, 0), BLUE, "cell {x}");
    }
    assert_ne!(fg(&buf, 5, 0), BLUE);
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    text_el(&mut dom, p, "b", "", "xy");
    let buf = super::paint(&mut dom, "p::first-letter { color: blue }", 4, 1);
    assert_eq!(fg(&buf, 0, 0), BLUE);
    assert_ne!(fg(&buf, 1, 0), BLUE);
}

/// §2.3: the first letter is the first of the line's content — a
/// `::before`'s generated text included.
#[test]
fn first_letter_reaches_into_before_content() {
    let (buf, ..) = para(
        "p::before { content: 'Xy ' } p::first-letter { color: blue }",
        "ab",
        8,
        1,
    );
    assert_eq!(super::rows(&buf, 8, 1), ["Xy ab   "]);
    assert_eq!(fg(&buf, 0, 0), BLUE);
    assert_ne!(fg(&buf, 1, 0), BLUE);
    assert_ne!(fg(&buf, 3, 0), BLUE);
}

/// §2.3.1 with CSS Text 3 §2.1: `text-transform` on `::first-letter`
/// reshapes the letter alone; its cell still maps to the source.
#[test]
fn first_letter_text_transform() {
    let (buf, dom, p) = para(
        "p::first-letter { text-transform: uppercase }",
        "hello",
        8,
        1,
    );
    assert_eq!(super::rows(&buf, 8, 1), ["Hello   "]);
    let text = dom.node(p).first_child().unwrap().id();
    let at = dom.position_at(1, 0).expect("a text position");
    assert_eq!((at.node, at.offset), (text, 1));
}

/// §2.3.1 (a drop cap): a floated `::first-letter` is a float, its box
/// as tall as its `line-height` makes it plus its padding — three rows
/// here, the letter on the middle one (half the leading above it, CSS
/// Inline 3 §5.1) — and the rest of the text wraps beside it, then
/// below. A click on it targets its block.
#[test]
fn a_floated_first_letter_is_a_drop_cap() {
    let (buf, dom, p) = para(
        "p { width: 8 } p::first-letter { float: left; line-height: 3; padding-right: 1 }",
        "Lorem ipsum dolor sit",
        8,
        4,
    );
    assert_eq!(
        super::rows(&buf, 8, 4),
        ["  orem  ", "L ipsum ", "  dolor ", "sit     "]
    );
    assert_eq!(dom.hit_test(0, 1), Some(p));
}
