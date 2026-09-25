//! B.1 tests — caret detection + paint.
//!
//! Covers:
//! - `is_editable()` recognizes `contenteditable="true"` / `""`.
//! - `nearest_editable_ancestor` walks up through the tree.
//! - `cell_of_position` maps positions to cells for ASCII, CJK,
//!   mid-line, end-of-line, wrapped paragraphs.
//! - The paint pass paints explicit fg/bg on the caret cell when a
//!   focused editable has a collapsed selection, and leaves the
//!   buffer untouched otherwise.

use rdom_core::{Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::node::{TuiNodeExt, nearest_editable_ancestor};
use crate::render::{Buffer, LayoutExt, PaintExt, Rect};
use crate::runtime::editing::caret::cell_of_position;
use crate::style::{CascadeExt, Color, Stylesheet, TuiStyle};

// ── is_editable ─────────────────────────────────────────────────────

#[test]
fn is_editable_true_attr_matches() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.set_attribute(el, "contenteditable", "true").unwrap();
    dom.append_child(root, el).unwrap();
    assert!(dom.node(el).is_editable());
}

#[test]
fn is_editable_empty_string_matches_html_boolean_attr() {
    // HTML lets `contenteditable` (no value) mean true. Parser emits
    // it as an empty-string attr value.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.set_attribute(el, "contenteditable", "").unwrap();
    dom.append_child(root, el).unwrap();
    assert!(dom.node(el).is_editable());
}

#[test]
fn is_editable_false_does_not_match() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.set_attribute(el, "contenteditable", "false").unwrap();
    dom.append_child(root, el).unwrap();
    assert!(!dom.node(el).is_editable());
}

#[test]
fn is_editable_absent_returns_false() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    assert!(!dom.node(el).is_editable());
}

// ── C.4a: tag-based editability ────────────────────────────────────

#[test]
fn input_default_type_is_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("input");
    dom.append_child(root, el).unwrap();
    assert!(dom.node(el).is_editable());
}

#[test]
fn input_text_family_types_are_editable() {
    for ty in ["text", "password", "email", "url", "tel", "search"] {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let el = dom.create_element("input");
        dom.set_attribute(el, "type", ty).unwrap();
        dom.append_child(root, el).unwrap();
        assert!(dom.node(el).is_editable(), "type={} should be editable", ty);
    }
}

#[test]
fn input_non_text_types_are_not_editable() {
    // C.4b will give checkbox / radio their own behavior — they
    // route through `<button>`-style activation, not text editing.
    for ty in ["checkbox", "radio", "submit", "reset", "button", "hidden"] {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let el = dom.create_element("input");
        dom.set_attribute(el, "type", ty).unwrap();
        dom.append_child(root, el).unwrap();
        assert!(
            !dom.node(el).is_editable(),
            "type={} should not be text-editable",
            ty
        );
    }
}

#[test]
fn textarea_is_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("textarea");
    dom.append_child(root, el).unwrap();
    assert!(dom.node(el).is_editable());
}

#[test]
fn disabled_input_is_not_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("input");
    dom.set_attribute(el, "disabled", "").unwrap();
    dom.append_child(root, el).unwrap();
    assert!(!dom.node(el).is_editable());
}

#[test]
fn disabled_textarea_is_not_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("textarea");
    dom.set_attribute(el, "disabled", "").unwrap();
    dom.append_child(root, el).unwrap();
    assert!(!dom.node(el).is_editable());
}

#[test]
fn readonly_input_is_still_editable_for_focus_routing() {
    // `readonly` does NOT make the element non-editable in the
    // `is_editable` sense — it stays focusable, selectable, and
    // routes through `nearest_editable_ancestor` like any input.
    // `perform_edit` is what blocks the actual mutation.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("input");
    dom.set_attribute(el, "readonly", "").unwrap();
    dom.append_child(root, el).unwrap();
    assert!(dom.node(el).is_editable());
}

// ── nearest_editable_ancestor ──────────────────────────────────────

#[test]
fn nearest_editable_ancestor_returns_self_when_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let editable = dom.create_element("div");
    dom.set_attribute(editable, "contenteditable", "true")
        .unwrap();
    dom.append_child(root, editable).unwrap();
    assert_eq!(nearest_editable_ancestor(&dom, editable), Some(editable));
}

#[test]
fn nearest_editable_ancestor_walks_up_to_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let editable = dom.create_element("div");
    dom.set_attribute(editable, "contenteditable", "true")
        .unwrap();
    let inner = dom.create_element("span");
    dom.append_child(editable, inner).unwrap();
    dom.append_child(root, editable).unwrap();
    assert_eq!(nearest_editable_ancestor(&dom, inner), Some(editable));
}

#[test]
fn nearest_editable_ancestor_none_when_nothing_editable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let plain = dom.create_element("div");
    dom.append_child(root, plain).unwrap();
    assert_eq!(nearest_editable_ancestor(&dom, plain), None);
}

// ── cell_of_position ───────────────────────────────────────────────

/// Build a `<p contenteditable>hello<span/></p>` fixture. The span
/// makes the <p> qualify as an IFC. Returns (dom, p, text_node).
fn editable_paragraph(text: &str) -> (TuiDom, rdom_core::NodeId, rdom_core::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 10));
    (dom, p, t)
}

#[test]
fn cell_of_position_at_start_is_origin() {
    let (dom, _p, t) = editable_paragraph("hello");
    let (x, y) = cell_of_position(&dom, Position::new(t, 0)).unwrap();
    assert_eq!((x, y), (0, 0));
}

#[test]
fn cell_of_position_advances_with_offset() {
    let (dom, _p, t) = editable_paragraph("hello");
    for byte_off in 0..=5 {
        let (x, _) = cell_of_position(&dom, Position::new(t, byte_off)).unwrap();
        assert_eq!(x, byte_off as u16);
    }
}

#[test]
fn cell_of_position_cjk_wide_glyph_advances_two_cells() {
    let (dom, _p, t) = editable_paragraph("中文");
    let (x0, _) = cell_of_position(&dom, Position::new(t, 0)).unwrap();
    let (x_mid, _) = cell_of_position(&dom, Position::new(t, 3)).unwrap(); // after '中'
    let (x_end, _) = cell_of_position(&dom, Position::new(t, 6)).unwrap(); // after '文'
    assert_eq!(x0, 0);
    assert_eq!(x_mid, 2);
    assert_eq!(x_end, 4);
}

#[test]
fn cell_of_position_wrapped_line_returns_correct_row() {
    // Narrow width forces wrapping. Position after "hello " (byte 6)
    // should be on row 1, cell 0 — the wrap drop.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node("hello world");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(6)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 10, 5));

    // Byte 0 = 'h' at row 0 col 0.
    let (x0, y0) = cell_of_position(&dom, Position::new(t, 0)).unwrap();
    assert_eq!((x0, y0), (0, 0));
    // Byte 6 = 'w' at row 1 col 0 (after wrap).
    let (x6, y6) = cell_of_position(&dom, Position::new(t, 6)).unwrap();
    assert_eq!((x6, y6), (0, 1));
}

// ── paint_caret_if_editable (integration through paint pass) ───────

fn pipeline(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) -> Buffer {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

fn editable_sheet() -> Stylesheet {
    Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline))
}

#[test]
fn caret_paints_when_editable_is_focused_with_collapsed_selection() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    let buf = pipeline(&mut dom, &editable_sheet(), Rect::new(0, 0, 30, 5));

    // Cells 0-2 ("hel") aren't selected → plain (bg == Reset). Cell 3
    // ("l" index 3, byte 3 is after the first 'l' and on the second 'l')
    // has the caret painted → bg is the high-contrast white fallback
    // (no cascaded color on <p>, so Auto caret-color → white).
    // Cell 4 plain again.
    assert_ne!(
        buf.cell(3, 0).unwrap().bg,
        Color::Reset,
        "caret cell should have an explicit bg painted"
    );
    for x in [0u16, 1, 2, 4] {
        assert_eq!(
            buf.cell(x, 0).unwrap().bg,
            Color::Reset,
            "cell {x} should not have the caret bg painted"
        );
    }
}

#[test]
fn caret_does_not_paint_when_no_focus() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));
    // focused is None
    let buf = pipeline(&mut dom, &editable_sheet(), Rect::new(0, 0, 30, 5));
    for x in 0..5 {
        assert_eq!(buf.cell(x, 0).unwrap().bg, Color::Reset);
    }
}

#[test]
fn caret_does_not_paint_on_non_editable_even_with_selection() {
    // Same paragraph but WITHOUT contenteditable. Clicking in it
    // places a selection (runtime); but we shouldn't paint a caret
    // for a non-editable.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let buf = pipeline(&mut dom, &editable_sheet(), Rect::new(0, 0, 30, 5));
    for x in 0..5 {
        assert_eq!(buf.cell(x, 0).unwrap().bg, Color::Reset);
    }
}

#[test]
fn caret_does_not_paint_when_selection_is_a_range_not_a_caret() {
    // Non-collapsed selection paints the selection overlay, not a
    // caret. Verified indirectly: no standalone caret-style cell
    // outside the range (cells 0 and 4 keep bg == Reset; cells 1-3
    // get the UA `*::selection` bg, not the caret white fallback).
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::new(
        Position::new(t, 1),
        Position::new(t, 4),
    )));

    let buf = pipeline(&mut dom, &editable_sheet(), Rect::new(0, 0, 30, 5));
    // Cells 0 and 4 are outside the selection — no caret, no overlay.
    assert_eq!(buf.cell(0, 0).unwrap().bg, Color::Reset);
    assert_eq!(buf.cell(4, 0).unwrap().bg, Color::Reset);
}

// ── P6G-PSEUDO-SHIFT-1: the caret shares the packer's geometry ──────

/// A 10-wide editable pure-text block with `::before { content: "> " }`.
fn editable_with_before(text: &str) -> (TuiDom, rdom_core::NodeId, rdom_core::NodeId) {
    use crate::style::Content;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().width(Size::Fixed(10)))
        .rule_unchecked(
            "p::before",
            TuiStyle::new().content(Content::Str("> ".into())),
        );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 5));
    (dom, p, t)
}

#[test]
fn caret_at_offset_zero_sits_after_the_before_pseudo() {
    let (dom, _p, t) = editable_with_before("abcdefg hi");
    assert_eq!(cell_of_position(&dom, Position::new(t, 0)), Some((2, 0)));
    // Past "abcdefg " the caret is on the wrapped line, at its start.
    assert_eq!(cell_of_position(&dom, Position::new(t, 8)), Some((0, 1)));
    assert_eq!(cell_of_position(&dom, Position::new(t, 10)), Some((2, 1)));
}

/// An empty editable has no text fragment: the caret sits at the start
/// of the block, before the `::before` — what browsers do for the
/// `[contenteditable]:empty::before` placeholder idiom.
#[test]
fn caret_in_empty_editable_sits_at_the_block_start() {
    let (dom, _p, t) = editable_with_before("");
    assert_eq!(cell_of_position(&dom, Position::new(t, 0)), Some((0, 0)));
}

/// A text control showing its placeholder (`:placeholder-shown::before`,
/// rdom's stand-in for `::placeholder`): the caret stays at the start of
/// the field, as browsers draw it.
#[test]
fn caret_in_empty_input_sits_at_the_start_of_its_placeholder() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "placeholder", "Name").unwrap();
    let t = dom.create_text_node("");
    dom.append_child(input, t).unwrap();
    dom.append_child(root, input).unwrap();
    dom.cascade(&Stylesheet::new());
    dom.layout_dom(Rect::new(0, 0, 20, 5));
    let content = dom.node(input).content_layout_rect().unwrap();
    assert_eq!(
        cell_of_position(&dom, Position::new(t, 0)),
        Some((content.x as u16, content.y as u16))
    );
}

// ── P6G-INLINE-PSEUDO-1: the caret around an inline element's pseudos ─

/// `<p contenteditable>a <b>bold</b> c</p>` with `b::before { "[" }` /
/// `b::after { "]" }` paints "a [bold] c": the caret at the end of "a "
/// sits before the `[`, inside `<b>` after it.
#[test]
fn caret_around_an_inline_elements_pseudos_follows_the_packed_text() {
    use crate::style::Content;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let a = dom.create_text_node("a ");
    let b = dom.create_element("b");
    let bold = dom.create_text_node("bold");
    let c = dom.create_text_node(" c");
    dom.append_child(b, bold).unwrap();
    dom.append_child(p, a).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, c).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "b::before",
            TuiStyle::new().content(Content::Str("[".into())),
        )
        .rule_unchecked(
            "b::after",
            TuiStyle::new().content(Content::Str("]".into())),
        );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 2));
    assert_eq!(cell_of_position(&dom, Position::new(bold, 0)), Some((3, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(bold, 4)), Some((7, 0)));
    assert_eq!(cell_of_position(&dom, Position::new(c, 1)), Some((9, 0)));
}
