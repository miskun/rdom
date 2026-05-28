//! End-to-end paint pipeline tests — cascade → layout → paint into
//! a `Buffer`. Large test bed; move to `tests/paint_pass.rs` when
//! the crate's integration-test story consolidates.

use super::*;
use crate::prelude::*;

// ── Helpers ──────────────────────────────────────────────────────

fn pipeline(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) -> Buffer {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

fn row(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in buf.area.x..buf.area.right() {
        if let Some(c) = buf.cell(x, y) {
            if c.is_spacer() {
                continue;
            }
            s.push_str(c.symbol());
        }
    }
    s
}

// ── Text painting ────────────────────────────────────────────────

#[test]
fn single_text_span_paints() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = dom.create_element("span");
    let t = dom.create_text_node("hello");
    dom.append_child(span, t).unwrap();
    dom.append_child(root, span).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "hello");
}

#[test]
fn cjk_text_paints_with_spacer() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = dom.create_element("span");
    let t = dom.create_text_node("中");
    dom.append_child(span, t).unwrap();
    dom.append_child(root, span).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 4, 1));
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "中");
    assert!(buf.cell(1, 0).unwrap().is_spacer());
}

#[test]
fn emoji_paints() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = dom.create_element("span");
    let t = dom.create_text_node("🦀 rust");
    dom.append_child(span, t).unwrap();
    dom.append_child(root, span).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 10, 1));
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "🦀");
    assert!(buf.cell(1, 0).unwrap().is_spacer());
    assert_eq!(buf.cell(2, 0).unwrap().symbol(), " ");
    assert_eq!(buf.cell(3, 0).unwrap().symbol(), "r");
}

// ── Background fill ──────────────────────────────────────────────

#[test]
fn bg_fills_outer_rect_css_way() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    dom.append_child(root, d).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(5))
            .height(Size::Fixed(3))
            .bg(Color::Rgb(255, 0, 0)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    for y in 0..3 {
        for x in 0..5 {
            assert_eq!(buf.cell(x, y).unwrap().bg, Color::Rgb(255, 0, 0));
        }
    }
    // Outside the element: untouched bg.
    assert_eq!(buf.cell(6, 0).unwrap().bg, Color::Reset);
}

#[test]
fn bg_covers_border_cells() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    dom.append_child(root, d).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(5))
            .height(Size::Fixed(3))
            .bg(Color::Rgb(255, 0, 0))
            .border(Border::single())
            .border_fg(Color::Rgb(255, 255, 255)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    // Top-left border char '┌' on a Red background with White fg.
    let tl = buf.cell(0, 0).unwrap();
    assert_eq!(tl.symbol(), "┌");
    assert_eq!(tl.fg, Color::Rgb(255, 255, 255));
    assert_eq!(tl.bg, Color::Rgb(255, 0, 0));
}

// ── Border drawing ───────────────────────────────────────────────

#[test]
fn border_single_all_four_sides() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    dom.append_child(root, d).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(4))
            .height(Size::Fixed(3))
            .border(Border::single()),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "┌");
    assert_eq!(buf.cell(3, 0).unwrap().symbol(), "┐");
    assert_eq!(buf.cell(0, 2).unwrap().symbol(), "└");
    assert_eq!(buf.cell(3, 2).unwrap().symbol(), "┘");
    assert_eq!(buf.cell(1, 0).unwrap().symbol(), "─");
    assert_eq!(buf.cell(0, 1).unwrap().symbol(), "│");
    assert_eq!(buf.cell(3, 1).unwrap().symbol(), "│");
}

#[test]
fn border_rounded_uses_curves() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    dom.append_child(root, d).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(4))
            .height(Size::Fixed(3))
            .border(Border::rounded()),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "╭");
    assert_eq!(buf.cell(3, 0).unwrap().symbol(), "╮");
    assert_eq!(buf.cell(0, 2).unwrap().symbol(), "╰");
    assert_eq!(buf.cell(3, 2).unwrap().symbol(), "╯");
}

#[test]
fn border_top_only() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    dom.append_child(root, d).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(4))
            .height(Size::Fixed(3))
            .border(Border::top()),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    // Top row: all horizontals, no corners (Top-only doesn't draw sides).
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "─");
    assert_eq!(buf.cell(3, 0).unwrap().symbol(), "─");
    // Middle row: blank
    assert_eq!(buf.cell(0, 1).unwrap().symbol(), " ");
    // Bottom row: blank
    assert_eq!(buf.cell(0, 2).unwrap().symbol(), " ");
}

#[test]
fn border_fg_applies() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    dom.append_child(root, d).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(3))
            .height(Size::Fixed(2))
            .border(Border::single())
            .border_fg(Color::Rgb(0, 255, 255)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Rgb(0, 255, 255));
}

/// Paint-layer invariant: an element with `border` set but no
/// `background-color` does NOT wipe the underlying cell bg. The
/// border ring writes `symbol + fg` only; `cell.bg` is owned by
/// whatever ran `fill_bg` (this element's, an ancestor's, or
/// nothing). Pre-fix `paint_border` unconditionally wrote
/// `Color::Reset` into ring cells, which on a real terminal renders
/// as the terminal's default bg — visible as a black ring poking
/// through a parent's `background-color` (regression surfaced by
/// `positioning_demo`).
#[test]
fn border_with_no_bg_preserves_parent_bg() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .bg(Color::Rgb(50, 50, 50)),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::rounded())
                .border_fg(Color::Rgb(200, 200, 200)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 12, 6));

    // Child has no bg of its own. Its border cells must preserve
    // the parent's bg, NOT reset to Color::Reset.
    let tl = buf.cell(0, 0).unwrap();
    assert_eq!(tl.symbol(), "╭");
    assert_eq!(tl.fg, Color::Rgb(200, 200, 200));
    assert_eq!(
        tl.bg,
        Color::Rgb(50, 50, 50),
        "border ring must preserve parent bg when child has no background-color"
    );

    // Spot-check another ring cell.
    let top_horizontal = buf.cell(3, 0).unwrap();
    assert_eq!(top_horizontal.symbol(), "─");
    assert_eq!(top_horizontal.bg, Color::Rgb(50, 50, 50));
}

// ── Text within bordered box ────────────────────────────────────

#[test]
fn text_inside_bordered_box_respects_inset() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("d");
    let t = dom.create_text_node("hi");
    dom.append_child(d, t).unwrap();
    dom.append_child(root, d).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "d",
        TuiStyle::new()
            .width(Size::Fixed(6))
            .height(Size::Fixed(3))
            .border(Border::single()),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    // Border outside, text inside at (1, 1).
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "┌");
    assert_eq!(buf.cell(1, 1).unwrap().symbol(), "h");
    assert_eq!(buf.cell(2, 1).unwrap().symbol(), "i");
}

// ── Pseudo-elements ─────────────────────────────────────────────

#[test]
fn before_content_paints_at_start() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("item");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "s::before",
        TuiStyle::new().content(Content::Str("▾ ".into())),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    assert_eq!(row(&buf, 0).trim_end(), "▾ item");
}

#[test]
fn after_content_paints_after_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("x");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "s::after",
        TuiStyle::new().content(Content::Str("!".into())),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "x!");
}

#[test]
fn before_and_after_both_paint() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("mid");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "s::before",
            TuiStyle::new().content(Content::Str("[".into())),
        )
        .rule_unchecked(
            "s::after",
            TuiStyle::new().content(Content::Str("]".into())),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "[mid]");
}

// ── Clipping ─────────────────────────────────────────────────────

#[test]
fn text_clips_at_buffer_right_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("hello world");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 5, 1));
    // Only "hello" fits; "world" clipped.
    assert_eq!(row(&buf, 0), "hello");
}

#[test]
fn negative_layout_rect_partially_visible() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let t = dom.create_text_node("AAA");
    dom.append_child(a, t).unwrap();
    let t = dom.create_text_node("BBB");
    dom.append_child(b, t).unwrap();
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(1)));

    dom.cascade(&sheet);
    // Scroll c so that a is off-screen at y=-1, b at y=0.
    dom.node_mut(c).ext_mut().unwrap().scroll_y = 1;
    dom.layout_dom(Rect::new(0, 0, 10, 2));

    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    dom.paint_dom(&mut buf, Rect::new(0, 0, 10, 2));

    // b should be visible at y=0 (a scrolled off).
    assert_eq!(row(&buf, 0).trim_end(), "BBB");
}

// ── Nested elements ────────────────────────────────────────────

#[test]
fn nested_elements_paint_recursively() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    let t = dom.create_text_node("nested");
    dom.append_child(inner, t).unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "outer",
        TuiStyle::new()
            .padding(Padding::all(1))
            .border(Border::single())
            .width(Size::Fixed(12))
            .height(Size::Fixed(4)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 5));

    // Border on outer.
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "┌");
    assert_eq!(buf.cell(11, 0).unwrap().symbol(), "┐");
    // inner's text at (2, 2): padding=1 + border=1 = inset of 2.
    assert_eq!(buf.cell(2, 2).unwrap().symbol(), "n");
    assert_eq!(buf.cell(7, 2).unwrap().symbol(), "d");
}

// ── Row layout paints side by side ───────────────────────────────

#[test]
fn row_children_paint_at_their_layout_positions() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let r = dom.create_element("r");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let t = dom.create_text_node("LEFT");
    dom.append_child(a, t).unwrap();
    let t = dom.create_text_node("RIGHT");
    dom.append_child(b, t).unwrap();
    dom.append_child(r, a).unwrap();
    dom.append_child(r, b).unwrap();
    dom.append_child(root, r).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "r",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .gap(1),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(4)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Fixed(5)));
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    assert_eq!(row(&buf, 0).trim_end(), "LEFT RIGHT");
}

// ── Styling ──────────────────────────────────────────────────────

#[test]
fn fg_applies_to_painted_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("red");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("s", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
    assert_eq!(buf.cell(2, 0).unwrap().fg, Color::Rgb(255, 0, 0));
}

#[test]
fn bold_modifier_applies() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("BOLD");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("s", TuiStyle::new().bold(true));
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert!(buf.cell(0, 0).unwrap().modifier.contains(Modifier::BOLD));
}

#[test]
fn pseudo_element_has_its_own_fg() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("body");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("s", TuiStyle::new().fg(Color::Rgb(0, 0, 255)))
        .rule_unchecked(
            "s::before",
            TuiStyle::new()
                .content(Content::Str("▾".into()))
                .fg(Color::Rgb(255, 0, 0)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "▾");
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
    assert_eq!(buf.cell(1, 0).unwrap().symbol(), "b");
    assert_eq!(buf.cell(1, 0).unwrap().fg, Color::Rgb(0, 0, 255));
}

// ── Overflow hidden ──────────────────────────────────────────────

#[test]
fn overflow_hidden_clips_child_text() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let box_ = dom.create_element("box");
    let span = dom.create_element("span");
    let t = dom.create_text_node("overflowing text");
    dom.append_child(span, t).unwrap();
    dom.append_child(box_, span).unwrap();
    dom.append_child(root, box_).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "box",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(1))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked("span", TuiStyle::new().width(Size::Fixed(100)));
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    // Only 6 cells visible even though span declared width 100 (no
    // text wrap). Buffer width is 20; expect 6 visible chars then
    // 14 untouched blanks.
    let visible = row(&buf, 0);
    assert_eq!(visible, "overfl              ");
}

// ── Empty tree / no-op ──────────────────────────────────────────

#[test]
fn empty_dom_paints_empty_buffer() {
    let mut dom = TuiDom::new();
    let sheet = Stylesheet::bare();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 5, 2));
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));
    dom.paint_dom(&mut buf, Rect::new(0, 0, 5, 2));
    assert_eq!(row(&buf, 0), "     ");
    assert_eq!(row(&buf, 1), "     ");
}

// ── LayoutRect → Rect clipping ───────────────────────────────────

#[test]
fn layout_rect_to_grid_basic() {
    let layout = LayoutRect::new(2, 3, 10, 5);
    let clip = Rect::new(0, 0, 20, 10);
    let out = layout_rect_to_grid(layout, clip).unwrap();
    assert_eq!(out, Rect::new(2, 3, 10, 5));
}

#[test]
fn layout_rect_to_grid_partial_left() {
    let layout = LayoutRect::new(-3, 0, 10, 5); // x=-3..7
    let clip = Rect::new(0, 0, 20, 10);
    let out = layout_rect_to_grid(layout, clip).unwrap();
    assert_eq!(out, Rect::new(0, 0, 7, 5));
}

#[test]
fn layout_rect_to_grid_fully_negative() {
    let layout = LayoutRect::new(-10, 0, 5, 5);
    let clip = Rect::new(0, 0, 20, 10);
    assert!(layout_rect_to_grid(layout, clip).is_none());
}

#[test]
fn layout_rect_to_grid_beyond_clip() {
    let layout = LayoutRect::new(5, 5, 100, 100);
    let clip = Rect::new(0, 0, 20, 10);
    let out = layout_rect_to_grid(layout, clip).unwrap();
    assert_eq!(out, Rect::new(5, 5, 15, 5));
}

// ── Pseudo-element with var() ────────────────────────────────────

#[test]
fn pseudo_content_with_var_resolves() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    dom.append_child(root, s).unwrap();

    // Explicit height since pseudo-only content doesn't drive
    // intrinsic sizing in v1 (no inline layout yet).
    let sheet = Stylesheet::bare()
        .define_var("arrow", "▸")
        .rule_unchecked("s", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked(
            "s::before",
            TuiStyle::new().content(Content::Var("arrow".into())),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "▸");
}

// ── Bg + fg composition ─────────────────────────────────────────

#[test]
fn text_inherits_element_bg_via_paint_style() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("s");
    let t = dom.create_text_node("AB");
    dom.append_child(s, t).unwrap();
    dom.append_child(root, s).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "s",
        TuiStyle::new()
            .bg(Color::Rgb(0, 0, 0))
            .fg(Color::Rgb(255, 255, 255))
            .width(Size::Fixed(5)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    // Cell 0 = "A" with fg White on bg Black.
    let a = buf.cell(0, 0).unwrap();
    assert_eq!(a.symbol(), "A");
    assert_eq!(a.fg, Color::Rgb(255, 255, 255));
    assert_eq!(a.bg, Color::Rgb(0, 0, 0));
    // Cell 2 = trailing cell inside the element's outer rect; bg
    // filled (no symbol painted there).
    let c2 = buf.cell(2, 0).unwrap();
    assert_eq!(c2.bg, Color::Rgb(0, 0, 0));
}

// ── Selection overlay ───────────────────────────────────────────────

/// Build a paragraph "hello" inside `<p>…<span/></p>` so the IFC
/// check (requires one inline element child) is satisfied. Returns
/// (dom, text_node) so tests can set a selection on it.
fn ifc_paragraph(text: &str) -> (TuiDom, rdom_core::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();
    (dom, t)
}

fn ifc_sheet(width: u16) -> Stylesheet {
    Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(width)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline))
}

/// Same as `ifc_sheet`, plus the UA `*::selection` default that
/// `Stylesheet::new()` would inject (these tests run on the
/// `bare` baseline, so the UA selection rule is missing unless
/// we add it back explicitly). Mirrors
/// `*::selection { background-color: #394B7E; color: white }`.
fn ifc_sheet_with_ua_selection(width: u16) -> Stylesheet {
    ifc_sheet(width).rule_unchecked(
        "*::selection",
        TuiStyle::new()
            .bg(Color::Rgb(0x39, 0x4B, 0x7E))
            .fg(Color::Rgb(0xFF, 0xFF, 0xFF)),
    )
}

#[test]
fn selection_range_paints_ua_overlay_on_selected_cells_only() {
    use rdom_core::{Position, Selection};

    let (mut dom, t) = ifc_paragraph("hello");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 1),
        Position::new(t, 4),
    )));

    let buf = pipeline(
        &mut dom,
        &ifc_sheet_with_ua_selection(20),
        Rect::new(0, 0, 20, 2),
    );

    // UA `*::selection` rule paints selected cells with explicit
    // bg #394B7E + fg white. Cells outside the selection keep
    // their default (Reset) bg.
    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    let ua_sel_fg = Color::Rgb(0xFF, 0xFF, 0xFF);

    // Cells 0 ('h') and 4 ('o') — outside selection — not overlaid.
    assert_ne!(buf.cell(0, 0).unwrap().bg, ua_sel_bg);
    assert_ne!(buf.cell(4, 0).unwrap().bg, ua_sel_bg);
    // Cells 1 ('e'), 2 ('l'), 3 ('l') — inside selection — overlaid.
    for x in 1..=3 {
        let c = buf.cell(x, 0).unwrap();
        assert_eq!(c.bg, ua_sel_bg, "cell {x} bg");
        assert_eq!(c.fg, ua_sel_fg, "cell {x} fg");
    }
}

#[test]
fn selection_uses_author_styled_overlay_when_present() {
    use rdom_core::{Position, Selection};

    let (mut dom, t) = ifc_paragraph("hello");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 1),
        Position::new(t, 4),
    )));

    let sheet = ifc_sheet(20).rule_unchecked(
        "p::selection",
        TuiStyle::new()
            .bg(Color::Rgb(255, 255, 0))
            .fg(Color::Rgb(0, 0, 0)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 2));

    // Outside selection: untouched by the author overlay.
    assert_ne!(buf.cell(0, 0).unwrap().bg, Color::Rgb(255, 255, 0));

    // Inside selection: author colors win. The overlay paints
    // explicit fg/bg cells (no REVERSED modifier any more).
    for x in 1..=3 {
        let c = buf.cell(x, 0).unwrap();
        assert_eq!(c.bg, Color::Rgb(255, 255, 0), "cell {x} bg");
        assert_eq!(c.fg, Color::Rgb(0, 0, 0), "cell {x} fg");
    }
}

#[test]
fn selection_uses_ua_default_when_no_author_rule() {
    // Regression guard: when no author `::selection` rule cascades,
    // the UA default `*::selection { background-color: #394B7E;
    // color: white }` paints selected cells. The fallback to a bare
    // `Style::new()` only fires if the author explicitly removes the
    // UA rule, which we don't do here.
    use rdom_core::{Position, Selection};

    let (mut dom, t) = ifc_paragraph("hello");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 1),
        Position::new(t, 4),
    )));

    let buf = pipeline(
        &mut dom,
        &ifc_sheet_with_ua_selection(20),
        Rect::new(0, 0, 20, 2),
    );

    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    let ua_sel_fg = Color::Rgb(0xFF, 0xFF, 0xFF);
    for x in 1..=3 {
        let c = buf.cell(x, 0).unwrap();
        assert_eq!(c.bg, ua_sel_bg, "cell {x} bg should be UA selection bg");
        assert_eq!(c.fg, ua_sel_fg, "cell {x} fg should be UA selection fg");
    }
}

#[test]
fn collapsed_selection_paints_no_selection_overlay() {
    use rdom_core::{Position, Selection};

    let (mut dom, t) = ifc_paragraph("hello");
    // Caret at offset 2 — no range. No focused editable, so no
    // caret either; the buffer should remain at default bg even
    // though the UA `*::selection` rule is loaded.
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let buf = pipeline(
        &mut dom,
        &ifc_sheet_with_ua_selection(20),
        Rect::new(0, 0, 20, 2),
    );

    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    for x in 0..5 {
        assert_ne!(
            buf.cell(x, 0).unwrap().bg,
            ua_sel_bg,
            "cell {x} should not get the selection overlay for a collapsed caret"
        );
    }
}

#[test]
fn no_selection_paints_no_selection_overlay() {
    let (mut dom, _t) = ifc_paragraph("hello");
    // UA `*::selection` rule is loaded but no selection exists, so
    // overlay never fires.
    let buf = pipeline(
        &mut dom,
        &ifc_sheet_with_ua_selection(20),
        Rect::new(0, 0, 20, 2),
    );

    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    for x in 0..5 {
        assert_ne!(buf.cell(x, 0).unwrap().bg, ua_sel_bg);
    }
}

#[test]
fn cjk_selection_overlays_full_grapheme_cells() {
    use rdom_core::{Position, Selection};

    // "中文": byte 0..3 = 中 (2 cells), byte 3..6 = 文 (2 cells).
    // Selecting [0, 3) → the "中" grapheme — both of its cells
    // (0 and 1) should get the UA selection overlay.
    let (mut dom, t) = ifc_paragraph("中文");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 3),
    )));

    let buf = pipeline(
        &mut dom,
        &ifc_sheet_with_ua_selection(20),
        Rect::new(0, 0, 20, 2),
    );

    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    assert_eq!(buf.cell(0, 0).unwrap().bg, ua_sel_bg);
    assert_eq!(buf.cell(1, 0).unwrap().bg, ua_sel_bg);
    // Cells 2-3 are "文" — not selected.
    assert_ne!(buf.cell(2, 0).unwrap().bg, ua_sel_bg);
    assert_ne!(buf.cell(3, 0).unwrap().bg, ua_sel_bg);
}

#[test]
fn selection_across_inline_element_highlights_both_fragments() {
    use rdom_core::{Position, Selection};

    // <p>ab<code>XY</code>cd</p>: three fragments across two text
    // nodes. Selection from (t_ab, 1) to (t_cd, 1) should highlight
    // cells 1 ("b"), 2-3 ("XY" entire <code>), 4 ("c") — leaving
    // cells 0 ("a") and 5 ("d") untouched.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t_ab = dom.create_text_node("ab");
    dom.append_child(p, t_ab).unwrap();
    let code = dom.create_element("code");
    let t_xy = dom.create_text_node("XY");
    dom.append_child(code, t_xy).unwrap();
    dom.append_child(p, code).unwrap();
    let t_cd = dom.create_text_node("cd");
    dom.append_child(p, t_cd).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked("code", TuiStyle::new().display(Display::Inline))
        // UA `*::selection` default — see `ifc_sheet_with_ua_selection`.
        .rule_unchecked(
            "*::selection",
            TuiStyle::new()
                .bg(Color::Rgb(0x39, 0x4B, 0x7E))
                .fg(Color::Rgb(0xFF, 0xFF, 0xFF)),
        );

    dom.set_selection(Some(Selection::new(
        Position::new(t_ab, 1),
        Position::new(t_cd, 1),
    )));

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 2));

    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    assert_ne!(buf.cell(0, 0).unwrap().bg, ua_sel_bg); // 'a'
    assert_eq!(buf.cell(1, 0).unwrap().bg, ua_sel_bg); // 'b'
    assert_eq!(buf.cell(2, 0).unwrap().bg, ua_sel_bg); // 'X'
    assert_eq!(buf.cell(3, 0).unwrap().bg, ua_sel_bg); // 'Y'
    assert_eq!(buf.cell(4, 0).unwrap().bg, ua_sel_bg); // 'c'
    assert_ne!(buf.cell(5, 0).unwrap().bg, ua_sel_bg); // 'd'
}

#[test]
fn selection_on_wrapped_second_line_paints_that_row() {
    use rdom_core::{Position, Selection};

    // Width 6 → "hello" on row 0, "world" on row 1. Select "world".
    let (mut dom, t) = ifc_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    let buf = pipeline(
        &mut dom,
        &ifc_sheet_with_ua_selection(6),
        Rect::new(0, 0, 10, 3),
    );

    let ua_sel_bg = Color::Rgb(0x39, 0x4B, 0x7E);
    // Row 0 ("hello") — untouched.
    for x in 0..5 {
        assert_ne!(
            buf.cell(x, 0).unwrap().bg,
            ua_sel_bg,
            "row 0 cell {x} should not have the selection overlay"
        );
    }
    // Row 1 ("world") — all 5 cells get the UA selection overlay.
    for x in 0..5 {
        assert_eq!(
            buf.cell(x, 1).unwrap().bg,
            ua_sel_bg,
            "row 1 cell {x} should have the selection overlay"
        );
    }
}

// ── Scrollbar paint ─────────────────────────────────────────────────

fn paint_with_scroll_content(
    overflow_x: Option<Overflow>,
    overflow_y: Option<Overflow>,
    content_w: usize,
    content_h: usize,
) -> Buffer {
    use crate::style::TuiStyle;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    let inner = dom.create_element("inner");
    dom.append_child(root, c).unwrap();
    dom.append_child(c, inner).unwrap();

    let mut rule = TuiStyle::new()
        .width(Size::Fixed(10))
        .height(Size::Fixed(6));
    if let Some(o) = overflow_y {
        rule = rule.overflow_y(o);
    }
    if let Some(o) = overflow_x {
        rule = rule.overflow_x(o);
    }
    // Real child sized to the requested content extent — drives the
    // layout pass to record the matching `scroll_content_*` AND lets
    // the `Overflow::Auto` two-pass detect overflow naturally
    // (synthetic post-layout injection bypasses the pass-2 trigger).
    let inner_rule = TuiStyle::new()
        .width(Size::Fixed(content_w as u16))
        .height(Size::Fixed(content_h as u16));
    let sheet = Stylesheet::bare()
        .rule_unchecked("c", rule)
        .rule_unchecked("inner", inner_rule);

    pipeline(&mut dom, &sheet, Rect::new(0, 0, 12, 8))
}

#[test]
fn overflow_scroll_y_always_paints_vertical_scrollbar_even_when_content_fits() {
    // Container 10x6 with overflow-y: scroll. Content = 5 lines
    // (smaller than viewport of 5 rows after gutter). `scroll`
    // means always paint. Thumb fills the track.
    // Scrollbar paints as `::scrollbar` (track) + `::scrollbar-thumb`
    // (thumb). UA defaults give track content `" "` (colored
    // gutter) and thumb content `"┃"`. Track-vs-thumb is now
    // distinguished by glyph, with bg DarkGray under both.
    let buf = paint_with_scroll_content(None, Some(Overflow::Scroll), 5, 5);
    let col: Vec<(String, crate::style::Color)> = (0..6)
        .map(|y| {
            let c = buf.cell(9, y).unwrap();
            (c.symbol().to_string(), c.bg)
        })
        .collect();
    for (y, (sym, bg)) in col.iter().enumerate() {
        assert!(
            sym == " " || sym == "┃",
            "column 9 row {y} should be track ` ` or thumb `┃`, got {sym:?}"
        );
        assert_eq!(
            *bg,
            crate::style::Color::Rgb(169, 169, 169),
            "row {y} should have track bg (DarkGray)"
        );
    }
}

#[test]
fn overflow_auto_y_hides_scrollbar_when_content_fits() {
    // Auto case: content = 3, viewport = 6 → no scrollbar.
    let buf = paint_with_scroll_content(None, Some(Overflow::Auto), 5, 3);
    // Column 9 should not have the scrollbar bg.
    for y in 0..6 {
        let bg = buf.cell(9, y).unwrap().bg;
        assert_ne!(
            bg,
            crate::style::Color::Rgb(169, 169, 169),
            "auto + fits should not paint scrollbar bg at row {y}"
        );
    }
}

#[test]
fn overflow_auto_y_shows_scrollbar_when_content_overflows() {
    let buf = paint_with_scroll_content(None, Some(Overflow::Auto), 5, 100);
    // Column 9 rows 0..6 should have the scrollbar glyph + bg.
    let mut has_scrollbar = false;
    for y in 0..6 {
        let c = buf.cell(9, y).unwrap();
        if c.bg == crate::style::Color::Rgb(169, 169, 169)
            && (c.symbol() == " " || c.symbol() == "┃")
        {
            has_scrollbar = true;
        }
    }
    assert!(has_scrollbar, "auto + overflow should paint scrollbar");
}

#[test]
fn overflow_scroll_x_paints_horizontal_scrollbar_at_bottom_row() {
    // Container 10x6, overflow-x: scroll. Gutter reserves bottom
    // row 5. Track paints on row 5, columns 0..10. Horizontal
    // thumb uses `━` (heavy horizontal); track is `" "`.
    let buf = paint_with_scroll_content(Some(Overflow::Scroll), None, 5, 5);
    for x in 0..10 {
        let c = buf.cell(x, 5).unwrap();
        let sym = c.symbol();
        assert!(
            sym == " " || sym == "━",
            "row 5 col {x} should be track ` ` or horizontal thumb `━`, got {sym:?}"
        );
        assert_eq!(
            c.bg,
            crate::style::Color::Rgb(169, 169, 169),
            "row 5 col {x} should have scrollbar bg"
        );
    }
}

#[test]
fn both_axes_scroll_leaves_corner_unpainted() {
    // Both overflow-x and overflow-y scroll. Vertical track
    // stops one row short (row 5 unclaimed), horizontal track
    // stops one col short (col 9 unclaimed). The bottom-right
    // corner (9, 5) is the shared corner — no scrollbar.
    let buf = paint_with_scroll_content(Some(Overflow::Scroll), Some(Overflow::Scroll), 50, 50);
    let corner_bg = buf.cell(9, 5).unwrap().bg;
    assert_ne!(
        corner_bg,
        crate::style::Color::Rgb(169, 169, 169),
        "corner should not have scrollbar bg"
    );
}

#[test]
fn overflow_x_hidden_does_not_reserve_gutter() {
    // overflow-x: hidden should NOT reserve a row. Content area
    // stays at full height 6.
    use crate::style::TuiStyle;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    dom.append_child(root, c).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(6))
            .overflow_x(Overflow::Hidden)
            .overflow_y(Overflow::Hidden),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 12, 8));

    // No scrollbars anywhere. Use bg as the witness: scrollbar
    // cells carry the UA DarkGray bg.
    for x in 0..12 {
        for y in 0..8 {
            let bg = buf.cell(x, y).unwrap().bg;
            assert_ne!(
                bg,
                crate::style::Color::Rgb(169, 169, 169),
                "hidden overflow should paint no scrollbar at ({x},{y})"
            );
        }
    }
}

/// Vertical scrollbar thumb defaults to `┃` U+2503 HEAVY VERTICAL.
/// UA `*::scrollbar-thumb` rule supplies bg + fg but leaves
/// `content` unset; paint picks the axis-appropriate fallback
/// glyph (heavy-vertical for vertical scrollbars). lazygit /
/// gum / lipgloss convention for vertical-handle.
#[test]
fn vertical_scrollbar_thumb_paints_heavy_vertical_by_default() {
    let buf = paint_with_scroll_content(None, Some(Overflow::Auto), 5, 100);
    // Find a thumb cell — `┃` glyph at col 9. With viewport=6
    // and content=100, the thumb is tiny (1 cell); the rest of
    // the column is track ` `.
    let thumb_found = (0..6).any(|y| buf.cell(9, y).unwrap().symbol() == "┃");
    assert!(
        thumb_found,
        "vertical thumb should paint with heavy-vertical glyph `┃` by default"
    );
    // None of the cells should paint the horizontal glyph.
    for y in 0..6 {
        assert_ne!(
            buf.cell(9, y).unwrap().symbol(),
            "━",
            "vertical scrollbar must NOT use horizontal glyph at row {y}"
        );
    }
}

/// Horizontal scrollbar thumb defaults to `━` U+2501 HEAVY
/// HORIZONTAL. Mirror of the vertical case — same UA rule,
/// paint chooses axis-appropriate fallback.
#[test]
fn horizontal_scrollbar_thumb_paints_heavy_horizontal_by_default() {
    let buf = paint_with_scroll_content(Some(Overflow::Auto), None, 100, 5);
    // Bottom track row is row 5. Find a thumb cell.
    let thumb_found = (0..10).any(|x| buf.cell(x, 5).unwrap().symbol() == "━");
    assert!(
        thumb_found,
        "horizontal thumb should paint with heavy-horizontal glyph `━` by default"
    );
    // None of the cells should paint the vertical glyph.
    for x in 0..10 {
        assert_ne!(
            buf.cell(x, 5).unwrap().symbol(),
            "┃",
            "horizontal scrollbar must NOT use vertical glyph at col {x}"
        );
    }
}

/// Author override of `::scrollbar-thumb { content: ... }`
/// applies to BOTH axes (documented limitation; per-axis
/// targeting tracked as `UA-SB-1` in TECH_DEBT). The author
/// picks a glyph that reads both ways — block characters work,
/// directional glyphs don't.
#[test]
fn author_content_override_on_scrollbar_thumb_applies_to_both_axes() {
    use crate::style::TuiStyle;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    dom.append_child(root, c).unwrap();
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(6))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "c::scrollbar-thumb",
            TuiStyle::new().content(Content::Str("█".into())),
        );
    let _ = pipeline(&mut dom, &sheet, Rect::new(0, 0, 12, 8));
    if let Some(ext) = dom.node_mut(c).ext_mut() {
        ext.scroll_content_width = 100;
        ext.scroll_content_height = 100;
    }
    let mut buf = Buffer::empty(Rect::new(0, 0, 12, 8));
    dom.paint_dom(&mut buf, Rect::new(0, 0, 12, 8));

    // Vertical scrollbar at col 9, somewhere in rows 0..5 is the
    // thumb. Author content `█` overrides the per-axis fallback.
    let v_thumb = (0..5).any(|y| buf.cell(9, y).unwrap().symbol() == "█");
    assert!(v_thumb, "author content `█` applies to vertical thumb");
    // Horizontal scrollbar at row 5; somewhere in cols 0..9 is the
    // thumb. Same author content applies.
    let h_thumb = (0..9).any(|x| buf.cell(x, 5).unwrap().symbol() == "█");
    assert!(h_thumb, "author content `█` applies to horizontal thumb");
}

#[test]
fn cross_axis_independence_v1() {
    // v1 deviates from CSS Overflow L3's cross-axis rule: each
    // axis is resolved independently. `overflow-y: scroll` with
    // default `overflow-x: visible` does NOT bump x to auto.
    // Rationale documented in style/cascade/apply.rs.
    use crate::style::{CascadeExt, TuiStyle};
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    dom.append_child(root, c).unwrap();
    let sheet =
        Stylesheet::bare().rule_unchecked("c", TuiStyle::new().overflow_y(Overflow::Scroll));
    dom.cascade(&sheet);
    let c_computed = dom.node(c).computed().cloned().unwrap();
    assert_eq!(c_computed.overflow_y, Overflow::Scroll);
    assert_eq!(c_computed.overflow_x, Overflow::Visible);
}

// ── C.4a: <input type="password"> masks at paint ───────────────────

#[test]
fn input_type_text_paints_value_verbatim() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "hello").unwrap();
    dom.append_child(root, input).unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    // `.trim()` handles the leading cell of UA padding on
    // text-family input chrome (`padding: 0 1`).
    assert_eq!(row(&buf, 0).trim(), "hello");
}

#[test]
fn input_type_password_masks_value_with_bullets() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "password").unwrap();
    dom.set_attribute(input, "value", "secret").unwrap();
    dom.append_child(root, input).unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    // Six characters → six bullets. Value attribute stays unmasked.
    // `.trim()` handles the leading cell of UA padding on
    // text-family input chrome (`padding: 0 1`).
    assert_eq!(
        row(&buf, 0).trim(),
        "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
    );
    assert_eq!(dom.node(input).get_attribute("value"), Some("secret"));
}

#[test]
fn input_type_email_does_not_mask() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "email").unwrap();
    dom.set_attribute(input, "value", "a@b.c").unwrap();
    dom.append_child(root, input).unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    // `.trim()` handles the leading cell of UA padding on
    // text-family input chrome (`padding: 0 1`).
    assert_eq!(row(&buf, 0).trim(), "a@b.c");
}

// ── <input type=checkbox|radio> glyph rendering ─────────────────────

#[test]
fn unchecked_checkbox_renders_empty_box_glyph_via_ua_before_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.append_child(root, cb).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "[ ]");
}

#[test]
fn checked_checkbox_renders_x_glyph_via_ua_before_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.set_attribute(cb, "checked", "").unwrap();
    dom.append_child(root, cb).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "[x]");
}

#[test]
fn unchecked_radio_renders_empty_circle_glyph() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let r = dom.create_element("input");
    dom.set_attribute(r, "type", "radio").unwrap();
    dom.append_child(root, r).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "( )");
}

#[test]
fn checked_radio_renders_filled_circle_glyph() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let r = dom.create_element("input");
    dom.set_attribute(r, "type", "radio").unwrap();
    dom.set_attribute(r, "checked", "").unwrap();
    dom.append_child(root, r).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    assert_eq!(row(&buf, 0).trim_end(), "(\u{2022})");
}

// ── Stage B: positioned `::before` / `::after` pseudo-elements ──

#[test]
fn absolute_right_after_pseudo_pins_to_host_right_edge() {
    // The flagship Stage B case: `::after { position: absolute; right: 0 }`
    // on a host with `position: relative` pins the closing bracket
    // to the right edge of the host's box. This enables the "wide
    // button with floating brackets" pattern even when inline-block
    // isn't appropriate.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    let label = dom.create_text_node("Hello");
    dom.append_child(host, label).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "host",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "host::after",
            TuiStyle::new()
                .content(Content::Str("]".into()))
                .position(crate::layout::Position::Absolute)
                .right(crate::layout::Length::Cells(0)),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    // Host text "Hello" paints at cols 0-4. The `]` ::after pseudo
    // pins to the right edge of the host (col 19).
    let row0 = row(&buf, 0);
    assert!(
        row0.starts_with("Hello"),
        "host text at start, got {row0:?}"
    );
    assert_eq!(
        buf.cell(19, 0).unwrap().symbol(),
        "]",
        "absolute position right=0 must pin to host's right edge"
    );
}

#[test]
fn absolute_left_before_pseudo_pins_to_host_left_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    let label = dom.create_text_node("text");
    dom.append_child(host, label).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "host",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(10))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "host::before",
            TuiStyle::new()
                .content(Content::Str("[".into()))
                .position(crate::layout::Position::Absolute)
                .left(crate::layout::Length::Cells(0)),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    assert_eq!(
        buf.cell(0, 0).unwrap().symbol(),
        "[",
        "absolute position left=0 pins to host's left edge"
    );
}

#[test]
fn static_pseudo_still_paints_inline() {
    // Regression guard: default static-position pseudos paint
    // inline via the existing inline-append path (unchanged).
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    let label = dom.create_text_node("hi");
    dom.append_child(host, label).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked("host", TuiStyle::new().display(Display::InlineBlock))
        .rule_unchecked(
            "host::before",
            TuiStyle::new().content(Content::Str("[".into())),
        )
        .rule_unchecked(
            "host::after",
            TuiStyle::new().content(Content::Str("]".into())),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    // Static pseudos (no position set) render inline: `[hi]`.
    assert_eq!(row(&buf, 0).trim_end(), "[hi]");
}

#[test]
fn display_none_host_suppresses_positioned_pseudo() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked("host", TuiStyle::new().display(Display::None))
        .rule_unchecked(
            "host::before",
            TuiStyle::new()
                .content(Content::Str("[".into()))
                .position(crate::layout::Position::Absolute)
                .left(crate::layout::Length::Cells(0)),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    // `display: none` host suppresses generated content entirely.
    assert_eq!(row(&buf, 0).trim_end(), "");
}

/// D-M5N-5: a positioned pseudo whose CB-relative offset puts it
/// partly off-viewport must clip cleanly at the viewport edge.
/// `paint_pass::positioned_pseudos::layout_rect_to_grid` is the
/// chokepoint; this guards against regressions where a pseudo
/// extends past `clip.right()` and writes outside the buffer or
/// drops the visible prefix instead of clipping it.
#[test]
fn positioned_pseudo_clips_at_right_viewport_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    dom.append_child(root, host).unwrap();

    // Viewport is 10 wide. Host occupies cols 0..5. The `::after`
    // pseudo is placed at left=3 with 5-char content "ABCDE". Its
    // rect spans cols 3..8 inside the host CB; with the host pinned
    // to col 0, that's cols 3..8 in viewport — fully visible.
    // Then narrow the viewport to 6 so cols 6..8 clip off.
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "host",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(5))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "host::after",
            TuiStyle::new()
                .content(Content::Str("ABCDE".into()))
                .position(crate::layout::Position::Absolute)
                .left(crate::layout::Length::Cells(3)),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 6, 1));
    // Visible cells: cols 3 = 'A', 4 = 'B', 5 = 'C'. Cols 6..8 of
    // the pseudo are clipped — buffer width is 6.
    assert_eq!(buf.cell(3, 0).unwrap().symbol(), "A");
    assert_eq!(buf.cell(4, 0).unwrap().symbol(), "B");
    assert_eq!(buf.cell(5, 0).unwrap().symbol(), "C");
    // Buffer only has cols 0..6; out-of-range writes would have
    // panicked / been dropped. Reaching this assertion means
    // clipping worked.
}

/// D-M5N-5 companion: a positioned pseudo placed entirely off the
/// LEFT edge of the viewport (its whole rect at x < 0) must produce
/// no buffer writes. The `layout_rect_to_grid` returns `None` for a
/// rect with no positive-area intersection, and the iteration loop
/// short-circuits before touching `buf`.
#[test]
fn positioned_pseudo_entirely_off_left_edge_paints_nothing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    let label = dom.create_text_node("host");
    dom.append_child(host, label).unwrap();
    dom.append_child(root, host).unwrap();

    // Host width 5 with a `::after` whose `right: 50` would place
    // its 3-char box at col (5 - 50 - 3) = -48 .. -45 — entirely
    // off the left edge of any reasonable viewport.
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "host",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(5))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "host::after",
            TuiStyle::new()
                .content(Content::Str("XYZ".into()))
                .position(crate::layout::Position::Absolute)
                .right(crate::layout::Length::Cells(50)),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    // Pseudo rect is fully off-viewport-left → no pseudo cells
    // anywhere. Host text "host" still paints normally at cols 0..3.
    assert_eq!(row(&buf, 0).trim_end(), "host");
}

/// D-M5N-5: a positioned pseudo with empty `content: ""` produces
/// a zero-width rect. The paint pass MUST treat width=0 (or
/// height=0) as a no-op — no buffer writes, no panics. The
/// existing `if rect.width == 0 || rect.height == 0 { continue; }`
/// guard is what's under test.
#[test]
fn zero_width_positioned_pseudo_paints_nothing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    let label = dom.create_text_node("host");
    dom.append_child(host, label).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "host",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(10))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "host::before",
            TuiStyle::new()
                .content(Content::Str("".into()))
                .position(crate::layout::Position::Absolute)
                .left(crate::layout::Length::Cells(0)),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    // Host text "host" paints at cols 0..3; the empty ::before
    // pseudo paints nothing on top of it. Row reads "host".
    assert_eq!(row(&buf, 0).trim_end(), "host");
}

// ── Pseudo-element generated content participates in intrinsic width ──

#[test]
fn auto_width_element_with_pseudo_chrome_sizes_to_include_pseudo_content() {
    // Regression test: `::before` / `::after` content was painted but
    // did not contribute to the element's intrinsic-width layout box,
    // so an auto-width element with bracketed pseudo chrome around a
    // text-content child (e.g. `<button>` rendered as `[ Submit ]`)
    // clipped at paint time.
    //
    // This test puts a `<row>` with a `<custom>` element that has text
    // content and `::before` / `::after` brackets. The container is
    // wider than the content needs; the custom element has no explicit
    // width, so its width must come from intrinsic sizing. If pseudo
    // content is included, the full `[ Submit ]` renders unclipped.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("custom");
    let t = dom.create_text_node("Submit");
    dom.append_child(el, t).unwrap();
    dom.append_child(root, el).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked("custom", TuiStyle::new().display(Display::Block))
        .rule_unchecked(
            "custom::before",
            TuiStyle::new().content(Content::Str("[ ".into())),
        )
        .rule_unchecked(
            "custom::after",
            TuiStyle::new().content(Content::Str(" ]".into())),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    assert_eq!(row(&buf, 0).trim_end(), "[ Submit ]");
}

// ── UA disclosure triangle on <summary> ─────────────────────────

/// Closed `<details>` (no `open` attribute) renders the right-
/// pointing triangle before the summary text. Naked rdom — UA
/// stylesheet only, no author CSS.
#[test]
fn ua_summary_renders_right_triangle_when_closed() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let details = dom.create_element("details");
    let summary = dom.create_element("summary");
    let label = dom.create_text_node("Advanced");
    dom.append_child(summary, label).unwrap();
    dom.append_child(details, summary).unwrap();
    dom.append_child(root, details).unwrap();

    // No author CSS; rely on the UA `summary::before` rule.
    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    assert_eq!(row(&buf, 0).trim_end(), "▸ Advanced");
}

/// Closed `<details>` hides its non-summary children. UA rule
/// `details:not([open]) > *:not(summary) { display: none }` matches
/// real-browser disclosure widget semantics: the body must collapse
/// out of layout when closed, not just visually disappear. Without
/// this rule a closed details would show its "hidden" content with
/// a right-pointing triangle pointing at it — incoherent UX.
#[test]
fn ua_closed_details_hides_non_summary_children() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let details = dom.create_element("details");
    // No `open` attribute — closed.
    let summary = dom.create_element("summary");
    let s_t = dom.create_text_node("Click to expand");
    dom.append_child(summary, s_t).unwrap();
    dom.append_child(details, summary).unwrap();
    let body = dom.create_element("p");
    let b_t = dom.create_text_node("SECRET");
    dom.append_child(body, b_t).unwrap();
    dom.append_child(details, body).unwrap();
    dom.append_child(root, details).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 30, 3));
    assert_eq!(row(&buf, 0).trim_end(), "▸ Click to expand");
    // The `<p>SECRET</p>` body must NOT paint anywhere when the
    // parent `<details>` is closed.
    for y in 0..3 {
        assert!(
            !row(&buf, y).contains("SECRET"),
            "row {y} leaked closed-details content: {:?}",
            row(&buf, y)
        );
    }
}

/// Open `<details open>` renders the down-pointing triangle. The
/// `details:open > summary::before` rule overrides the base
/// `summary::before { content: "▸ " }` via higher specificity.
/// And the body now actually paints (the closed-details
/// suppression rule doesn't match when `open` is present).
#[test]
fn ua_summary_renders_down_triangle_when_open() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let details = dom.create_element("details");
    dom.set_attribute(details, "open", "").unwrap();
    let summary = dom.create_element("summary");
    let label = dom.create_text_node("Advanced");
    dom.append_child(summary, label).unwrap();
    dom.append_child(details, summary).unwrap();
    dom.append_child(root, details).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    assert_eq!(row(&buf, 0).trim_end(), "▾ Advanced");
}

// ── UA bullets on <ul> ──────────────────────────────────────────

/// `<ul>` renders each direct `<li>` child with a UA bullet marker
/// (`• ` from the `ul > li::before` rule). Naked rdom — no author CSS.
#[test]
fn ua_ul_renders_bullet_before_each_li() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = dom.create_element("ul");
    for label in ["first", "second"] {
        let li = dom.create_element("li");
        let t = dom.create_text_node(label);
        dom.append_child(li, t).unwrap();
        dom.append_child(ul, li).unwrap();
    }
    dom.append_child(root, ul).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 3));
    // `ul` has padding-left 2 from the existing UA rule, so the
    // bullet+text start at viewport col 2.
    assert_eq!(row(&buf, 0).trim_end(), "  • first");
    assert_eq!(row(&buf, 1).trim_end(), "  • second");
}

/// `<ol>` gets the same `• ` bullet marker as `<ul>` in 0.1.0
/// (honest fallback until CSS counters ship; a static `"1. "` on
/// every item would lie about ordering). Tracked as `UA-OL-1`.
#[test]
fn ua_ol_renders_bullet_marker() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ol = dom.create_element("ol");
    let li = dom.create_element("li");
    let t = dom.create_text_node("first");
    dom.append_child(li, t).unwrap();
    dom.append_child(ol, li).unwrap();
    dom.append_child(root, ol).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    // Padding-left 2 from the `ol` rule + `• ` marker from the
    // new `ol > li::before` rule.
    assert_eq!(row(&buf, 0).trim_end(), "  • first");
}

// ── C.6: <progress> + <meter> gauge rendering ────────────────────

#[test]
fn progress_renders_block_bar_at_value_max_ratio() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "0.5").unwrap();
    dom.set_attribute(p, "max", "1").unwrap();
    dom.append_child(root, p).unwrap();
    // Force a 10-wide track so the half-fill is unambiguous.
    let sheet =
        Stylesheet::new().rule_unchecked("progress", TuiStyle::new().width(Size::Fixed(10)));

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    assert_eq!(
        row(&buf, 0).trim_end(),
        "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}"
    );
}

#[test]
fn progress_without_value_renders_indeterminate_track() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("progress");
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::new().rule_unchecked("progress", TuiStyle::new().width(Size::Fixed(5)));

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert_eq!(
        row(&buf, 0).trim_end(),
        "\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}"
    );
}

// ── Polish #9: <a href> OSC 8 hyperlinks ─────────────────────────

#[test]
fn anchor_with_href_tags_painted_cells_with_link() {
    // Wrap in `<p>` so the IFC paint path fires — inline elements
    // directly under the Fragment root aren't painted (they need
    // a block parent to own the IFC).
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "https://example.com").unwrap();
    let t = dom.create_text_node("click");
    dom.append_child(a, t).unwrap();
    dom.append_child(p, a).unwrap();
    dom.append_child(root, p).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    // Each cell under the anchor text should carry the href.
    for x in 0..5 {
        assert_eq!(
            buf.cell(x, 0).unwrap().link(),
            Some("https://example.com"),
            "cell {x} missing link"
        );
    }
}

#[test]
fn anchor_without_href_does_not_tag_cells() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let a = dom.create_element("a");
    // No href attribute.
    let t = dom.create_text_node("plain");
    dom.append_child(a, t).unwrap();
    dom.append_child(p, a).unwrap();
    dom.append_child(root, p).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    assert!(buf.cell(0, 0).unwrap().link().is_none());
}

#[test]
fn nested_inline_inside_anchor_inherits_the_href_link() {
    // `<p><a href><b>bold</b></a></p>` — `<b>` fragment's cells
    // should still be tagged with the enclosing anchor's href.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "https://nested.test").unwrap();
    let b = dom.create_element("b");
    let t = dom.create_text_node("bold");
    dom.append_child(b, t).unwrap();
    dom.append_child(a, b).unwrap();
    dom.append_child(p, a).unwrap();
    dom.append_child(root, p).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 10, 1));
    for x in 0..4 {
        assert_eq!(
            buf.cell(x, 0).unwrap().link(),
            Some("https://nested.test"),
            "cell {x} should carry the anchor's link"
        );
    }
}

// ── Polish #8: <dialog>::backdrop ──────────────────────────────

#[test]
fn modal_dialog_backdrop_fills_viewport_with_bg() {
    use crate::style::Color;
    let mut dom = TuiDom::new();
    let root = dom.root();
    // An unrelated background element so we can verify the
    // backdrop overlays it.
    let bg_el = dom.create_element("p");
    let bg_text = dom.create_text_node("under");
    dom.append_child(bg_el, bg_text).unwrap();
    dom.append_child(root, bg_el).unwrap();
    // Modal dialog.
    let dlg = dom.create_element("dialog");
    dom.set_attribute(dlg, "open", "").unwrap();
    dom.set_attribute(dlg, "data-rdom-modal", "").unwrap();
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "dialog::backdrop",
        TuiStyle::new().bg(Color::Rgb(169, 169, 169)),
    );
    // Viewport big enough that the dialog (empty + UA chrome:
    // 1-cell border + padding 1 2 → 4 tall outer; width auto
    // stretches to the parent's width per flex cross-stretch)
    // leaves rows below it for the backdrop assertion. Layout:
    //   row 0      → `<p>under</p>` text
    //   rows 1..4  → dialog outer rect (top border, padding,
    //                  empty content, bottom border)
    //   rows 5..9  → uncovered viewport → backdrop bg.
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 30, 10));

    // Cells below the dialog carry the backdrop bg. Cells covered
    // by the dialog (its border + content area) are excluded —
    // the dialog repaints over the backdrop without a bg of its
    // own, so those cells either stay backdrop-tinted (content)
    // or get reset by border drawing.
    for y in 5..10 {
        for x in 0..30 {
            assert_eq!(
                buf.cell(x, y).unwrap().bg,
                Color::Rgb(169, 169, 169),
                "cell ({x},{y}) should have backdrop bg"
            );
        }
    }
    // And a sanity check that the dialog has a visible top-left
    // border corner at row 1, proving UA chrome painted. UA dialog
    // border is `Rounded`.
    assert_eq!(buf.cell(0, 1).unwrap().symbol(), "╭");
}

#[test]
fn non_modal_dialog_has_no_backdrop() {
    use crate::style::Color;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.set_attribute(dlg, "open", "").unwrap();
    // NOT modal — no data-rdom-modal marker.
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "dialog::backdrop",
        TuiStyle::new().bg(Color::Rgb(255, 0, 0)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    // No backdrop fill → cell bg remains default (Reset).
    assert_eq!(buf.cell(0, 0).unwrap().bg, Color::Reset);
}

#[test]
fn closed_dialog_has_no_backdrop() {
    // `data-rdom-modal` without `open` shouldn't paint a backdrop
    // (dialog is closed — the backdrop only exists while shown).
    use crate::style::Color;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.set_attribute(dlg, "data-rdom-modal", "").unwrap();
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "dialog::backdrop",
        TuiStyle::new().bg(Color::Rgb(255, 0, 0)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert_eq!(buf.cell(0, 0).unwrap().bg, Color::Reset);
}

#[test]
fn modal_dialog_repaints_over_backdrop() {
    // The dialog subtree re-paints on top of the backdrop — so
    // the dialog's own content area is tinted with its bg, not
    // the backdrop's.
    use crate::style::Color;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    dom.set_attribute(dlg, "open", "").unwrap();
    dom.set_attribute(dlg, "data-rdom-modal", "").unwrap();
    let t = dom.create_text_node("hi");
    dom.append_child(dlg, t).unwrap();
    dom.append_child(root, dlg).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "dialog::backdrop",
            TuiStyle::new().bg(Color::Rgb(169, 169, 169)),
        )
        .rule_unchecked("dialog", TuiStyle::new().bg(Color::Rgb(0, 0, 255)));
    // Viewport size accounts for UA dialog chrome (border 1 +
    // padding 1 2): 2-char text content → 6 wide × 4 tall outer.
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Dialog's content cell at the inner-content origin
    // (col = border + padding-left = 1 + 2 = 3,
    //  row = border + padding-top  = 1 + 1 = 2). Author bg Blue
    // paints across the outer rect, so the content cell carries
    // Blue — proving the dialog overpainted the backdrop.
    assert_eq!(buf.cell(3, 2).unwrap().bg, Color::Rgb(0, 0, 255));
    // A cell well outside the dialog's outer rect carries the
    // backdrop's bg.
    assert_eq!(buf.cell(15, 9).unwrap().bg, Color::Rgb(169, 169, 169));
}

// ── C.8a: <table> family rendering ──────────────────────────────

#[test]
fn table_renders_rows_stacked_vertically_cells_horizontally() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tbody = dom.create_element("tbody");
    for (name, age) in [("Alice", "30"), ("Bob", "25")] {
        let tr = dom.create_element("tr");
        let td1 = dom.create_element("td");
        let t1 = dom.create_text_node(name);
        dom.append_child(td1, t1).unwrap();
        let td2 = dom.create_element("td");
        let t2 = dom.create_text_node(age);
        dom.append_child(td2, t2).unwrap();
        dom.append_child(tr, td1).unwrap();
        dom.append_child(tr, td2).unwrap();
        dom.append_child(tbody, tr).unwrap();
    }
    dom.append_child(table, tbody).unwrap();
    dom.append_child(root, table).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 3));
    let r0 = row(&buf, 0);
    let r1 = row(&buf, 1);
    // Cells use Auto width + padding 0 1 0 1 so each cell has a
    // leading and trailing 1-cell padding. Without column sync
    // (C.8b) cells don't align across rows — we just assert both
    // values appear on their respective rows.
    assert!(r0.contains("Alice") && r0.contains("30"), "row 0: {r0:?}");
    assert!(r1.contains("Bob") && r1.contains("25"), "row 1: {r1:?}");
}

#[test]
fn table_header_cells_are_rendered_bold_via_ua_style() {
    // We don't inspect style bytes directly — instead verify that
    // the `th` cascade yields bold=true by checking the computed
    // style at the node.
    use crate::node::TuiNodeExt;
    use crate::style::CascadeExt;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let thead = dom.create_element("thead");
    let tr = dom.create_element("tr");
    let th = dom.create_element("th");
    let t = dom.create_text_node("Name");
    dom.append_child(th, t).unwrap();
    dom.append_child(tr, th).unwrap();
    dom.append_child(thead, tr).unwrap();
    dom.append_child(table, thead).unwrap();
    dom.append_child(root, table).unwrap();

    dom.cascade(&Stylesheet::new());

    let computed = dom.node(th).computed().cloned().unwrap();
    assert!(computed.modifiers.contains(crate::style::Modifier::BOLD));
}

#[test]
fn table_caption_uses_italic_dim_style() {
    use crate::node::TuiNodeExt;
    use crate::style::CascadeExt;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let caption = dom.create_element("caption");
    let t = dom.create_text_node("My Table");
    dom.append_child(caption, t).unwrap();
    dom.append_child(table, caption).unwrap();
    dom.append_child(root, table).unwrap();
    dom.cascade(&Stylesheet::new());

    let computed = dom.node(caption).computed().cloned().unwrap();
    assert!(computed.modifiers.contains(crate::style::Modifier::ITALIC));
    // Caption is muted via `fg: TEXT_MUTED` (#7F868B).
    assert_eq!(computed.fg, crate::style::Color::Rgb(127, 134, 139));
}

#[test]
fn bare_tr_without_tbody_still_renders() {
    // Per HTML, <table> may contain <tr> directly without a <tbody>
    // wrapper. Our flex layout doesn't care — we test the result
    // renders correctly.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tr = dom.create_element("tr");
    let td = dom.create_element("td");
    let t = dom.create_text_node("Direct");
    dom.append_child(td, t).unwrap();
    dom.append_child(tr, td).unwrap();
    dom.append_child(table, tr).unwrap();
    dom.append_child(root, table).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 1));
    assert!(row(&buf, 0).contains("Direct"));
}

#[test]
fn table_column_sync_makes_cells_align_across_rows() {
    // Verifies the C.8b pre-pass: cell widths are equalized per
    // column, so the boundary between column 0 and column 1
    // lands on the same x coordinate in every row.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let tbody = dom.create_element("tbody");
    for (a, b) in [("Alice", "30"), ("Bo", "251")] {
        let tr = dom.create_element("tr");
        for text in [a, b] {
            let td = dom.create_element("td");
            let t = dom.create_text_node(text);
            dom.append_child(td, t).unwrap();
            dom.append_child(tr, td).unwrap();
        }
        dom.append_child(tbody, tr).unwrap();
    }
    dom.append_child(table, tbody).unwrap();
    dom.append_child(root, table).unwrap();

    // Run the sync pre-pass before the cascade/layout. App::build
    // does this automatically, but pipeline() operates on a bare
    // dom — sync manually first.
    crate::runtime::builtins::table::size_all_tables(&mut dom);

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 30, 2));
    let r0 = row(&buf, 0);
    let r1 = row(&buf, 1);
    // Column 0 max is "Alice"(5)+pad 2 = 7. Column 1 value starts
    // at x=7 in both rows — "30" and "251" should land at the
    // same offset. Check by locating both at matching x.
    let i30 = r0.find("30").expect("row 0 has '30'");
    let i251 = r1.find("251").expect("row 1 has '251'");
    assert_eq!(i30, i251, "r0={r0:?} r1={r1:?}");
}

#[test]
fn colgroup_and_col_do_not_render_visible_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let colgroup = dom.create_element("colgroup");
    let col1 = dom.create_element("col");
    let col2 = dom.create_element("col");
    dom.append_child(colgroup, col1).unwrap();
    dom.append_child(colgroup, col2).unwrap();
    dom.append_child(table, colgroup).unwrap();
    let tr = dom.create_element("tr");
    let td = dom.create_element("td");
    let t = dom.create_text_node("X");
    dom.append_child(td, t).unwrap();
    dom.append_child(tr, td).unwrap();
    dom.append_child(table, tr).unwrap();
    dom.append_child(root, table).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 2));
    // Data row should be the first visible row — colgroup/col
    // don't take vertical space.
    assert!(
        row(&buf, 0).contains("X"),
        "row 0 should contain X, got {:?}",
        row(&buf, 0)
    );
}

// ── C.7b: <select> dropdown chrome rendering ────────────────────

#[test]
fn closed_dropdown_renders_arrow_plus_selected_label() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    let o1 = dom.create_element("option");
    dom.set_attribute(o1, "value", "dog").unwrap();
    dom.set_attribute(o1, "selected", "").unwrap();
    let t1 = dom.create_text_node("Dog");
    dom.append_child(o1, t1).unwrap();
    let o2 = dom.create_element("option");
    let t2 = dom.create_text_node("Cat");
    dom.append_child(o2, t2).unwrap();
    dom.append_child(sel, o1).unwrap();
    dom.append_child(sel, o2).unwrap();
    dom.append_child(root, sel).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 3));
    // Row 0 paints `<select>`'s 20-wide chrome row: 1-cell left
    // padding, selected option label "Dog", spaces filling the
    // gap, then the UA `::after` `▾` chevron pinned at right: 1.
    // Options are clipped by height:1/overflow:hidden.
    let r0 = row(&buf, 0);
    assert!(
        r0.contains("Dog"),
        "selected option label echoed; got {r0:?}"
    );
    assert!(
        r0.contains('\u{25BE}'),
        "UA `::after` chevron should appear; got {r0:?}"
    );
    assert_eq!(row(&buf, 1).trim_end(), "");
}

#[test]
fn open_dropdown_renders_options_inline_without_chrome() {
    // Open dropdown expands inline — options replace the chrome
    // row. Selection stays visible via `option[selected]` bg.
    // (Overlay popups are out of scope for v1; no top-layer.)
    let mut dom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    crate::runtime::builtins::select::open(&mut dom, sel);
    let o1 = dom.create_element("option");
    dom.set_attribute(o1, "value", "dog").unwrap();
    dom.set_attribute(o1, "selected", "").unwrap();
    let t1 = dom.create_text_node("Dog");
    dom.append_child(o1, t1).unwrap();
    let o2 = dom.create_element("option");
    let t2 = dom.create_text_node("Cat");
    dom.append_child(o2, t2).unwrap();
    dom.append_child(sel, o1).unwrap();
    dom.append_child(sel, o2).unwrap();
    dom.append_child(root, sel).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 4));
    let r0 = row(&buf, 0);
    let r1 = row(&buf, 1);
    assert!(
        r0.contains("Dog"),
        "row 0 should include Dog option, got {r0:?}"
    );
    assert!(
        r1.contains("Cat"),
        "row 1 should include Cat option, got {r1:?}"
    );
}

#[test]
fn listbox_renders_all_options_without_chrome() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    dom.set_attribute(sel, "multiple", "").unwrap();
    let o1 = dom.create_element("option");
    let t1 = dom.create_text_node("Dog");
    dom.append_child(o1, t1).unwrap();
    let o2 = dom.create_element("option");
    let t2 = dom.create_text_node("Cat");
    dom.append_child(o2, t2).unwrap();
    dom.append_child(sel, o1).unwrap();
    dom.append_child(sel, o2).unwrap();
    dom.append_child(root, sel).unwrap();

    let buf = pipeline(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 20, 4));
    // Listbox: no chrome row; options start at row 0.
    assert!(row(&buf, 0).contains("Dog"));
    assert!(row(&buf, 1).contains("Cat"));
}

#[test]
fn meter_renders_block_bar_at_value_in_min_max_range() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "min", "0").unwrap();
    dom.set_attribute(m, "max", "100").unwrap();
    dom.set_attribute(m, "value", "30").unwrap();
    dom.append_child(root, m).unwrap();
    let sheet = Stylesheet::new().rule_unchecked("meter", TuiStyle::new().width(Size::Fixed(10)));

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));
    assert_eq!(
        row(&buf, 0).trim_end(),
        "\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}"
    );
}

// ── Paint z-list / stacking (M2 §12.7-12.8) ───────────────────────

fn bg_at(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).map(|c| c.bg).unwrap_or(Color::Reset)
}

#[test]
fn higher_z_index_paints_on_top() {
    // Three absolutely-positioned siblings at the same rect with
    // different z-index. The highest z paints last (= on top).
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.append_child(root, c).unwrap();

    let base = || {
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .left(crate::layout::Length::Cells(0))
            .width(Size::Fixed(3))
            .height(Size::Fixed(1))
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            base()
                .bg(Color::Rgb(255, 0, 0))
                .z_index(crate::layout::ZIndex::Value(1)),
        )
        .rule_unchecked(
            "b",
            base()
                .bg(Color::Rgb(0, 0, 255))
                .z_index(crate::layout::ZIndex::Value(5)),
        )
        .rule_unchecked(
            "c",
            base()
                .bg(Color::Rgb(0, 128, 0))
                .z_index(crate::layout::ZIndex::Value(3)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(
        bg_at(&buf, 0, 0),
        Color::Rgb(0, 0, 255),
        "z=5 should be on top"
    );
}

#[test]
fn z_index_auto_paints_in_document_order() {
    // Two absolutes with z-index: auto (= 0). Document order
    // tiebreak — `b` is later in the document, paints last, sits
    // on top.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let base = || {
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .left(crate::layout::Length::Cells(0))
            .width(Size::Fixed(3))
            .height(Size::Fixed(1))
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked("a", base().bg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("b", base().bg(Color::Rgb(0, 0, 255)));
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(bg_at(&buf, 0, 0), Color::Rgb(0, 0, 255));
}

#[test]
fn positioned_paints_above_in_flow_content() {
    // In-flow `bg-bar` paints first; absolute `tooltip` lays on
    // top regardless of document order.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let bar = dom.create_element("bar");
    let tip = dom.create_element("tip");
    dom.append_child(root, tip).unwrap(); // earlier in document
    dom.append_child(root, bar).unwrap(); // later in document

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "bar",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(255, 0, 0)),
        )
        .rule_unchecked(
            "tip",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(3))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(0, 0, 255)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    // Even though tip comes BEFORE bar in document order,
    // positioned elements always paint last → tip is on top.
    assert_eq!(bg_at(&buf, 0, 0), Color::Rgb(0, 0, 255));
    // Cell past tooltip (x=4) shows the in-flow bar's bg.
    assert_eq!(bg_at(&buf, 4, 0), Color::Rgb(255, 0, 0));
}

#[test]
fn negative_z_index_paints_before_zero() {
    // Flat-sort model paints lowest z first. A z=-1 absolute paints
    // BEFORE a z=0 absolute, so the z=0 element ends up on top in
    // the overlap.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let lo = dom.create_element("lo");
    let hi = dom.create_element("hi");
    dom.append_child(root, lo).unwrap();
    dom.append_child(root, hi).unwrap();

    let base = || {
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .left(crate::layout::Length::Cells(0))
            .width(Size::Fixed(3))
            .height(Size::Fixed(1))
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "lo",
            base()
                .bg(Color::Rgb(255, 0, 0))
                .z_index(crate::layout::ZIndex::Value(-1)),
        )
        .rule_unchecked(
            "hi",
            base()
                .bg(Color::Rgb(0, 0, 255))
                .z_index(crate::layout::ZIndex::Value(0)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    // z=0 paints after z=-1 → z=0 (Blue) on top.
    assert_eq!(bg_at(&buf, 0, 0), Color::Rgb(0, 0, 255));
}

#[test]
fn higher_z_paints_on_top_of_lower_z() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let lo = dom.create_element("lo");
    let hi = dom.create_element("hi");
    dom.append_child(root, lo).unwrap();
    dom.append_child(root, hi).unwrap();

    let base = || {
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .left(crate::layout::Length::Cells(0))
            .width(Size::Fixed(3))
            .height(Size::Fixed(1))
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "lo",
            base()
                .bg(Color::Rgb(255, 0, 0))
                .z_index(crate::layout::ZIndex::Value(10)),
        )
        .rule_unchecked(
            "hi",
            base()
                .bg(Color::Rgb(0, 0, 255))
                .z_index(crate::layout::ZIndex::Value(2)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    // lo has higher z (10 > 2) — paints on top.
    assert_eq!(bg_at(&buf, 0, 0), Color::Rgb(255, 0, 0));
}

// ── opacity alpha-blend (T4) ─────────────────────────────────────

/// Element with `opacity: 0.5` and `fg: #FF0000` (red) on a parent
/// with `bg: #000000` (black) should blend to `#7F0000` — half-red.
/// 0.5 * 255 + 0.5 * 0 = 127.5 → 128 (round-to-nearest).
#[test]
fn opacity_half_blends_fg_against_black_parent_bg() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    let t = dom.create_text_node("x");
    dom.append_child(child, t).unwrap();
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(0, 0, 0)),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .width(Size::Fixed(3))
                .height(Size::Fixed(1))
                .fg(Color::Rgb(255, 0, 0))
                .opacity(0.5),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    // The `x` glyph at child's content origin gets the blended fg.
    let cell = buf.cell(0, 0).unwrap();
    assert_eq!(cell.symbol(), "x");
    assert_eq!(cell.fg, Color::Rgb(128, 0, 0));
}

/// `opacity: 1.0` is a no-op — colors pass through unblended.
#[test]
fn opacity_one_does_not_blend() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    let t = dom.create_text_node("x");
    dom.append_child(c, t).unwrap();
    dom.append_child(root, c).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .width(Size::Fixed(3))
            .height(Size::Fixed(1))
            .fg(Color::Rgb(255, 0, 0))
            .opacity(1.0),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
}

// ── Opacity composition over existing content ─────────────────────
//
// These tests pin the three opacity regimes when an element with bg
// paints **over cells that already hold a glyph**. Surfaced by
// `positioning_demo`'s absolute-positioned cyan badge sitting on
// top of relbox's rounded border ring: at full opacity the badge
// must occlude the border glyphs in its rect (CSS opaque box); at
// partial opacity the underlying glyphs must show through (the
// authoring intent of `opacity < 1`); at zero opacity the painter
// is invisible.

/// `opacity: 1.0` over an existing glyph: the opaque painter fully
/// occludes — `cell.symbol` is cleared (to SPACE) at cells the
/// painter doesn't itself write a glyph to. Without this, an
/// `<absolute>` strip with `bg: cyan` sitting over a border ring
/// renders cyan with the ring's `─` / `╮` glyphs poking through in
/// the padding cells.
#[test]
fn opacity_one_overlay_occludes_underlying_glyphs() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let bg_box = dom.create_element("b");
    let overlay = dom.create_element("o");
    dom.append_child(root, bg_box).unwrap();
    dom.append_child(root, overlay).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::rounded())
                .border_fg(Color::Rgb(200, 200, 200)),
        )
        .rule_unchecked(
            "o",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(6))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(61, 144, 206)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    // Overlay covers row 0. Without symbol-occlusion, the box's
    // border row 0 (╭, ─, ─, ─, ─, ╮) would still be visible
    // through the overlay's bg. With occlusion: row 0 is solid cyan
    // with cleared symbols.
    for x in 0..6 {
        let cell = buf.cell(x, 0).unwrap();
        assert_eq!(
            cell.bg,
            Color::Rgb(61, 144, 206),
            "row 0 col {x}: overlay bg"
        );
        assert_eq!(
            cell.symbol(),
            " ",
            "row 0 col {x}: overlay must clear underlying border glyph"
        );
    }
}

/// `opacity: 0.5` over an existing glyph: the painter blends bg
/// against the underlying area (via the cascade-time `alpha_blend`
/// pre-bake against `parent_bg`), and the painter does NOT clear
/// underlying symbols — translucent overlays let what's beneath
/// bleed through.
#[test]
fn opacity_half_overlay_blends_bg_and_preserves_underlying_glyphs() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let bg_box = dom.create_element("b");
    let overlay = dom.create_element("o");
    dom.append_child(root, bg_box).unwrap();
    dom.append_child(root, overlay).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::rounded())
                .border_fg(Color::Rgb(200, 200, 200)),
        )
        .rule_unchecked(
            "o",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(6))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(200, 100, 0))
                .opacity(0.5),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    // bg pre-baked: alpha_blend((200,100,0), 0.5, parent_bg=Reset
    // → black fallback) = (100, 50, 0). All cells in the overlay's
    // rect get the blended bg.
    for x in 0..6 {
        let cell = buf.cell(x, 0).unwrap();
        assert_eq!(
            cell.bg,
            Color::Rgb(100, 50, 0),
            "row 0 col {x}: blended bg at 50% opacity"
        );
    }
    // Border glyphs must STILL be visible at row 0 — translucent
    // overlay doesn't occlude.
    assert_eq!(
        buf.cell(0, 0).unwrap().symbol(),
        "╭",
        "left border bleeds through translucent overlay"
    );
    assert_eq!(
        buf.cell(5, 0).unwrap().symbol(),
        "╮",
        "right border bleeds through translucent overlay"
    );
    assert_eq!(
        buf.cell(2, 0).unwrap().symbol(),
        "─",
        "top horizontal border bleeds through translucent overlay"
    );
}

/// `opacity: 0.0` over an existing glyph: the painter is invisible.
/// `alpha_blend` collapses the painter's bg to exactly `parent_bg`,
/// which gets written to the cell; the cell's existing symbol stays
/// (translucent regime — no occlusion).
#[test]
fn opacity_zero_overlay_is_invisible_keeps_symbols() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let bg_box = dom.create_element("b");
    let overlay = dom.create_element("o");
    dom.append_child(root, bg_box).unwrap();
    dom.append_child(root, overlay).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::rounded())
                .border_fg(Color::Rgb(200, 200, 200)),
        )
        .rule_unchecked(
            "o",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(6))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(200, 100, 0))
                .opacity(0.0),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 5));

    // At opacity 0, bg collapses to parent_bg (Reset → black fallback
    // in the alpha_blend implementation). And — critically — the
    // border glyphs underneath remain.
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "╭");
    assert_eq!(buf.cell(2, 0).unwrap().symbol(), "─");
    assert_eq!(buf.cell(5, 0).unwrap().symbol(), "╮");
}

/// Paint-layer invariant follow-up: a translucent element with
/// its own bg and own text must produce the **same** `cell.bg` in
/// text cells as in non-text cells. Pre-fix, `style_from_computed`
/// (used by `paint_inline_content`'s own-text path) included
/// `bg = computed.bg` in the glyph style. The compose pipeline
/// then alpha-blended that raw bg against the cell's existing bg
/// (which already held the result of this element's `fill_bg`) —
/// a double-blend, visible under any non-trivial opacity. At
/// opacity 1.0 the second application is idempotent and the bug
/// is invisible; at opacity 0.6 the text cells render a brighter
/// version of the bg color than surrounding cells.
///
/// Fix: own-text glyph paints route through
/// `glyph_style_from_computed` which omits bg (the bg is owned by
/// `fill_bg`). Cells written by glyph paint leave `cell.bg`
/// untouched, preserving the single-blended bg from `fill_bg`.
#[test]
fn translucent_card_own_text_does_not_double_blend_bg() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let card = dom.create_element("c");
    let txt = dom.create_text_node("hi");
    dom.append_child(card, txt).unwrap();
    dom.append_child(root, card).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .width(Size::Fixed(8))
            .height(Size::Fixed(1))
            .bg(Color::Rgb(200, 0, 0))
            .opacity(0.5),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 1));

    // Expected single-blend bg: blend((200,0,0), 0.5, parent_bg
    // → fallback #000) = (100, 0, 0). Every cell of the card —
    // including the cells under the "hi" glyphs — must share this
    // bg. Pre-fix the text cells were (150, 0, 0) (double-blend).
    let bg_under_text = buf.cell(0, 0).unwrap().bg;
    let bg_no_text = buf.cell(2, 0).unwrap().bg;
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(buf.cell(2, 0).unwrap().symbol(), " ");
    assert_eq!(
        bg_under_text, bg_no_text,
        "text cells must share the bg of non-text cells (no double-blend)"
    );
    assert_eq!(
        bg_under_text,
        Color::Rgb(100, 0, 0),
        "expected single-blend (100,0,0); got {bg_under_text:?}"
    );
}

/// Cell-level RMW: a translucent overlay blends against the
/// **actual cell bg** at paint time, not the painter's resolved
/// `parent_bg`. Scenario: bottom z=1 element with `bg: red` paints
/// red across its rect. Top z=2 element with `bg: blue; opacity:
/// 0.5` paints over it. The overlap cells should have
/// `bg = blend(blue, 0.5, red) = (128, 0, 128)` — magenta — not
/// `blend(blue, 0.5, parent_bg)` which would be a different color
/// when the bottom has a non-parent bg of its own.
#[test]
fn translucent_overlay_blends_against_actual_cell_bg_not_parent_bg() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("h");
    let lower = dom.create_element("lo");
    let upper = dom.create_element("up");
    dom.append_child(host, lower).unwrap();
    dom.append_child(host, upper).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "h",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .position(crate::layout::Position::Relative),
        )
        .rule_unchecked(
            "lo",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(8))
                .height(Size::Fixed(3))
                .bg(Color::Rgb(255, 0, 0))
                .z_index(crate::layout::ZIndex::Value(1)),
        )
        .rule_unchecked(
            "up",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(8))
                .height(Size::Fixed(3))
                .bg(Color::Rgb(0, 0, 255))
                .opacity(0.5)
                .z_index(crate::layout::ZIndex::Value(2)),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 12, 4));

    // Both elements cover cols 0..8 rows 0..3. Lower paints red
    // first (z=1), upper paints blue at opacity 0.5 over it (z=2).
    // Per-cell blend against the actual `cell.bg = red`.
    // Expected: alpha_blend((0,0,255), 0.5, (255,0,0))
    //   = (0.5*0 + 0.5*255, 0.5*0 + 0.5*0, 0.5*255 + 0.5*0)
    //   = (127.5, 0, 127.5) → (128, 0, 128) after round-to-nearest.
    for x in 0..8 {
        for y in 0..3 {
            let cell = buf.cell(x, y).unwrap();
            assert_eq!(
                cell.bg,
                Color::Rgb(128, 0, 128),
                "cell ({x},{y}): expected magenta from blend against cell.bg=red, got {:?}",
                cell.bg
            );
        }
    }
}

/// `opacity: 0.0` blends fully toward the parent bg → exactly the
/// parent's bg color for fg.
#[test]
fn opacity_zero_collapses_fg_to_parent_bg() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    let t = dom.create_text_node("x");
    dom.append_child(child, t).unwrap();
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(1))
                .bg(Color::Rgb(50, 100, 150)),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .width(Size::Fixed(3))
                .height(Size::Fixed(1))
                .fg(Color::Rgb(255, 0, 0))
                .opacity(0.0),
        );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    // fg blends with 0% src + 100% parent bg → parent bg.
    assert_eq!(buf.cell(0, 0).unwrap().fg, Color::Rgb(50, 100, 150));
}

// ── Border collapse paint joiner (M5.5c) ─────────────────────────

#[test]
fn collapse_two_bordered_siblings_render_t_junctions() {
    // Container with border + two side-by-side bordered children.
    // The shared edge between the children should render as
    // junction glyphs: top = ┬, bottom = ┴.
    use rdom_style::layout::{Border, BorderCollapse};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(11))
                .height(Size::Fixed(3))
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single())
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::single()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::single()),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 11, 3));
    // Layout: outer is 0..11, a is 0..6, b is 5..11. The shared
    // vertical edge is at x=5.
    // Top row: ┌─────┬────┐
    // Middle:  │     │    │
    // Bottom:  └─────┴────┘
    assert_eq!(buf.cell(0, 0).unwrap().symbol(), "┌");
    assert_eq!(buf.cell(5, 0).unwrap().symbol(), "┬", "top junction");
    assert_eq!(buf.cell(10, 0).unwrap().symbol(), "┐");
    assert_eq!(buf.cell(0, 2).unwrap().symbol(), "└");
    assert_eq!(buf.cell(5, 2).unwrap().symbol(), "┴", "bottom junction");
    assert_eq!(buf.cell(10, 2).unwrap().symbol(), "┘");
    // The shared middle column is a vertical line.
    assert_eq!(buf.cell(5, 1).unwrap().symbol(), "│");
}

#[test]
fn collapse_joiner_is_noop_when_no_element_has_collapse() {
    // Same shape as the previous test but border-collapse: separate
    // (default). No junctions should appear — each child paints its
    // own border ring independently.
    use rdom_style::layout::Border;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(14))
                .height(Size::Fixed(3))
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single()),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::single()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .height(Size::Fixed(3))
                .border(Border::single()),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 14, 3));
    // Without collapse, children sit INSIDE the parent's border:
    // outer.content starts at (1, 1). Child A's top-left is at
    // (1, 1), child B's top-left is at (7, 1). Outer's top row
    // (y=0) is the OUTER border with `┌`, then `─` between, then
    // `┐` — and crucially no junction in the middle since children
    // don't reach up to it.
    assert_eq!(buf.cell(1, 0).unwrap().symbol(), "─");
    assert_eq!(buf.cell(7, 0).unwrap().symbol(), "─");
    // Child A's own top-left at (1, 1).
    assert_eq!(buf.cell(1, 1).unwrap().symbol(), "┌");
    // Outer's left edge passes through (0, 1) — should be `│`,
    // unchanged by the joiner.
    assert_eq!(buf.cell(0, 1).unwrap().symbol(), "│");
}

#[test]
fn collapse_three_sibling_nested_grid_renders_correct_junctions() {
    // The user's ASCII art shape: outer rounded-rect (single-border
    // works as Square here for the test), three columns of children,
    // middle column split into two rows. Goal: every junction picks
    // the right glyph automatically from neighbor connectivity.
    //
    // ┌─────────────────────┐
    // ├─────┬─────────┬─────┤
    // │     │         │     │
    // │     ├─────────┤     │
    // │     │         │     │
    // ├─────┴─────────┴─────┤
    // └─────────────────────┘
    //
    // Simplified version for the test: just verify junction glyphs
    // exist where they should, not the full ASCII art. The detailed
    // app-shell demo (M5.6) will pin the full picture.
    use rdom_style::layout::{Border, BorderCollapse};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let row = dom.create_element("row");
    let left = dom.create_element("left");
    let mid = dom.create_element("mid");
    let right = dom.create_element("right");
    dom.append_child(row, left).unwrap();
    dom.append_child(row, mid).unwrap();
    dom.append_child(row, right).unwrap();
    dom.append_child(outer, row).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(15))
                .height(Size::Fixed(3))
                .border(Border::single())
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked(
            "row",
            // BORDER-MODEL-1: collapse is non-inheriting AND only
            // affects direct children. For `row`'s cell children
            // (left/mid/right) to share borders with `outer`'s ring
            // — the table-like pattern this test pins — `row`
            // declares its own border-collapse AND its own border
            // ring. Then row shares with outer (via outer's
            // collapse on its direct children), and cells share
            // with row (via row's own collapse). No more recursion
            // through transparent intermediates.
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single())
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked(
            "left",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(3))
                .border(Border::single()),
        )
        .rule_unchecked(
            "mid",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(3))
                .border(Border::single()),
        )
        .rule_unchecked(
            "right",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .height(Size::Fixed(3))
                .border(Border::single()),
        );

    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 15, 3));

    // Outer corners pre-collapse-stack: ┌ ─ … ─ ┐ on top row.
    // With three siblings overlapping at x=4 and x=8, those columns
    // get vertical bars in the middle row and become junctions on
    // the outer top/bottom borders.
    //
    // Layout: outer 0..15, row 0..15, left 0..5, mid 4..9, right 8..13.
    // Wait, with collapse on outer (border) and inner row (no border):
    // outer's content area is the OUTER rect (0..15). Children of row
    // (which has no border) sit normally. Sibling overlap kicks in
    // between children of the same flex container that BOTH have
    // borders — left/mid/right all have borders. left+mid: overlap at
    // x=4. mid+right: overlap at x=8.
    //
    // But wait — `row` itself has no border, so it doesn't engage the
    // parent-content-area-expanded rule with outer. `row` sits inside
    // outer's content (the OUTER rect, since outer has collapse +
    // border). So row's rect = (0,0,15,3). left starts at 0.
    //
    // Top row positions:
    // - outer paints ┌ at (0,0) and ┐ at (14,0) — its corners.
    // - left paints its border at x=0..4, y=0..2. Its top-left
    //   conflicts with outer's top-left at (0,0). Both write the same
    //   ┌ symbol.
    // - left's top-right at (4,0) sits ABOVE mid's top-left at (4,0).
    //   Same cell. Painted by both with ┌ (the later wins).
    //   The joiner sees (4,0) has W (from left's horizontal line at 3,0
    //   pointing east into 4,0), E (mid's horizontal at 5,0 pointing
    //   west into 4,0), S (vertical line at 4,1 pointing N into 4,0).
    //   That's 3 directions → ┬. Plus N if outer's top row also points
    //   into it. Outer's top row IS continuous along y=0, so the cell
    //   at (4,0) has — neighbors. N → look at (4,-1) → out of bounds.
    //   So mask = W+E+S = bits 8+2+4 = 14 → JUNCTION_TABLE[14] = ┬. ✓
    assert_eq!(
        buf.cell(4, 0).unwrap().symbol(),
        "┬",
        "first sibling boundary at top"
    );
    assert_eq!(
        buf.cell(8, 0).unwrap().symbol(),
        "┬",
        "second sibling boundary at top"
    );
    assert_eq!(
        buf.cell(4, 2).unwrap().symbol(),
        "┴",
        "first sibling boundary at bottom"
    );
    assert_eq!(
        buf.cell(8, 2).unwrap().symbol(),
        "┴",
        "second sibling boundary at bottom"
    );
}

// ── text-decoration: line-through → CROSSED_OUT reaches cells ───

#[test]
fn text_decoration_line_through_paints_crossed_out_modifier_on_cells() {
    // CSS 2.1 + tui SGR pipeline: `text-decoration: line-through`
    // cascades to `Modifier::CROSSED_OUT` on the computed style.
    // The paint pass must propagate that modifier to every text
    // cell so the backend emits SGR-9 (CROSSED_OUT, ECMA-48 §8.3.117).
    //
    // Regression for `D-M1-3`: `style_from_computed` /
    // `glyph_style_from_computed` previously masked the modifier
    // bitset with `BOLD | ITALIC | UNDERLINED` only, silently
    // dropping `CROSSED_OUT` between cascade and paint. Parser +
    // cascade worked end-to-end; the paint mask was the leak.
    use crate::layout::TextDecoration;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    let txt = dom.create_text_node("xy");
    dom.append_child(host, txt).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "host",
        TuiStyle::new()
            .display(Display::Block)
            .text_decoration(TextDecoration::LineThrough),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 4, 1));

    for x in 0..2 {
        let cell = buf.cell(x, 0).expect("cell present");
        assert!(
            cell.modifier.contains(Modifier::CROSSED_OUT),
            "cell ({x}, 0) {:?} missing CROSSED_OUT — paint dropped the modifier",
            cell.modifier
        );
    }
}
