//! Substrate regression: a `<footer>` styled `display: flex` with two
//! `<div>` children (left has `flex: 1`, right has intrinsic content)
//! is the **web-canonical pattern** for "hints on the left, info on
//! the right" status bars. It works in every browser.
//!
//! The showcase status bar uses exactly this pattern. After three
//! attempts to make it work via cosmetic CSS changes, both bugs
//! persist:
//!
//! 1. Whitespace text nodes between inline-level children of the
//!    left div get dropped — `<span>↑↓</span> navigate` renders as
//!    `↑↓navigate`.
//! 2. The right-aligned div's text content doesn't render at all —
//!    `X: 42 Y: 7` is invisible.
//!
//! This file pins the contract end-to-end (DOM → cascade → layout →
//! PAINT) so we can see exactly which step drops the data, fix the
//! substrate, and prevent regression.

use rdom_tui::layout::{Direction, Flow, Size};
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, PaintExt, Stylesheet, TuiDom, TuiNodeExt, TuiStyle};

/// Build the showcase status bar shape:
///
/// ```text
/// <footer class="bar">                    ← display: flex, flex-direction: row
///   <div class="left">                    ← flex: 1
///     <span class="key">↑↓</span>
///     <text> </text>
///     <span class="label">navigate</span>
///   </div>
///   <div class="right">X: 42 Y: 7</div>   ← intrinsic-width
/// </footer>
/// ```
fn build() -> (TuiDom, Stylesheet) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let bar = dom.create_element("footer");
    dom.set_attribute(bar, "class", "bar").unwrap();
    dom.append_child(root, bar).unwrap();

    let left = dom.create_element("div");
    dom.set_attribute(left, "class", "left").unwrap();
    dom.append_child(bar, left).unwrap();

    let key = dom.create_element("span");
    dom.set_attribute(key, "class", "key").unwrap();
    let key_text = dom.create_text_node("↑↓");
    dom.append_child(key, key_text).unwrap();
    dom.append_child(left, key).unwrap();

    let space = dom.create_text_node(" ");
    dom.append_child(left, space).unwrap();

    let label = dom.create_element("span");
    dom.set_attribute(label, "class", "label").unwrap();
    let label_text = dom.create_text_node("navigate");
    dom.append_child(label, label_text).unwrap();
    dom.append_child(left, label).unwrap();

    let right = dom.create_element("div");
    dom.set_attribute(right, "class", "right").unwrap();
    let right_text = dom.create_text_node("X: 42 Y: 7");
    dom.append_child(right, right_text).unwrap();
    dom.append_child(bar, right).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            ".bar",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(40))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(".left", TuiStyle::new().width(Size::Flex(1)));

    (dom, sheet)
}

fn paint(dom: &mut TuiDom, sheet: &Stylesheet, w: u16, h: u16) -> Buffer {
    dom.cascade(sheet);
    dom.layout_dom(Rect::new(0, 0, w, h));
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    dom.paint_dom(&mut buf, Rect::new(0, 0, w, h));
    buf
}

fn row_text(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        if let Some(c) = buf.cell(x, y) {
            if c.is_spacer() {
                continue;
            }
            s.push_str(c.symbol());
        }
    }
    s
}

#[test]
fn both_slots_get_nonzero_layout_rects() {
    // Step 1 of the chain: after cascade + layout, both flex
    // children must have positive width. If this fails, the IFC
    // / flex routing decision is wrong and nothing downstream can
    // possibly work.
    let (mut dom, sheet) = build();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 1));

    let bar = dom.node(dom.root()).child_nodes().next().unwrap().id();
    let children: Vec<_> = dom.node(bar).child_nodes().map(|n| n.id()).collect();
    assert_eq!(children.len(), 2, "footer has 2 children");
    let left_rect = dom.node(children[0]).layout_rect().expect("left laid out");
    let right_rect = dom.node(children[1]).layout_rect().expect("right laid out");

    eprintln!("DBG left={left_rect:?} right={right_rect:?}");
    assert!(
        left_rect.width > 0,
        "left slot must have positive width (it has flex: 1); got {left_rect:?}"
    );
    assert!(
        right_rect.width > 0,
        "right slot must have positive width (it has intrinsic content 'X: 42 Y: 7'); \
         got {right_rect:?}"
    );
}

#[test]
fn right_slot_is_positioned_at_right_end_of_row() {
    // The whole point of `flex: 1` on left + nothing on right is
    // "left grows, right hugs its content at the far end."
    let (mut dom, sheet) = build();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 1));

    let bar = dom.node(dom.root()).child_nodes().next().unwrap().id();
    let children: Vec<_> = dom.node(bar).child_nodes().map(|n| n.id()).collect();
    let right_rect = dom.node(children[1]).layout_rect().expect("right laid out");

    let right_edge = right_rect.x + right_rect.width as i32;
    assert_eq!(
        right_edge, 40,
        "right slot must reach the row's right edge (x=40); got right={right_rect:?}"
    );
}

#[test]
fn right_slot_text_is_painted() {
    // PAINT contract: the text "X: 42 Y: 7" must appear somewhere
    // in row 0. If this fails, layout looks fine but paint isn't
    // running through the right slot's inline content.
    let (mut dom, sheet) = build();
    let buf = paint(&mut dom, &sheet, 40, 1);
    let painted = row_text(&buf, 0);
    eprintln!("DBG painted row 0: {painted:?}");
    assert!(
        painted.contains("X: 42 Y: 7"),
        "right slot text 'X: 42 Y: 7' must appear in painted output; got {painted:?}"
    );
}

#[test]
fn two_inline_spans_in_plain_block_render_both() {
    // BASELINE: same `<div><span>A</span><span>B</span></div>` shape,
    // but NOT inside a flex parent. If this passes, the bug is the
    // flex+IFC interaction; if it fails too, the IFC walker itself
    // is broken for multi-span content.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    dom.set_attribute(outer, "class", "outer").unwrap();
    dom.append_child(root, outer).unwrap();
    let s1 = dom.create_element("span");
    let t1 = dom.create_text_node("A");
    dom.append_child(s1, t1).unwrap();
    dom.append_child(outer, s1).unwrap();
    let s2 = dom.create_element("span");
    let t2 = dom.create_text_node("B");
    dom.append_child(s2, t2).unwrap();
    dom.append_child(outer, s2).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        ".outer",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let buf = paint(&mut dom, &sheet, 20, 1);
    let painted = row_text(&buf, 0);
    eprintln!("DBG painted (plain block): {painted:?}");
    assert!(
        painted.contains("A") && painted.contains("B"),
        "BOTH spans must render even without a flex parent; got {painted:?}"
    );
}

#[test]
fn left_slot_renders_second_inline_span() {
    // Minimal reproduction: two `<span>`s directly inside the
    // left flex slot, no text node between them. Painted output
    // must contain BOTH "A" and "B" — anything less means the
    // IFC walker / packer / painter is dropping content after
    // the first inline element. Strictly more diagnostic than
    // the whitespace-between-spans test below.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let bar = dom.create_element("footer");
    dom.set_attribute(bar, "class", "bar").unwrap();
    dom.append_child(root, bar).unwrap();
    let left = dom.create_element("div");
    dom.set_attribute(left, "class", "left").unwrap();
    dom.append_child(bar, left).unwrap();
    let s1 = dom.create_element("span");
    let t1 = dom.create_text_node("A");
    dom.append_child(s1, t1).unwrap();
    dom.append_child(left, s1).unwrap();
    let s2 = dom.create_element("span");
    let t2 = dom.create_text_node("B");
    dom.append_child(s2, t2).unwrap();
    dom.append_child(left, s2).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            ".bar",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(".left", TuiStyle::new().width(Size::Flex(1)));

    let buf = paint(&mut dom, &sheet, 20, 1);
    let painted = row_text(&buf, 0);
    eprintln!("DBG painted row 0: {painted:?}");
    assert!(
        painted.contains("A") && painted.contains("B"),
        "BOTH spans must render — IFC packer dropping content after the first \
         inline element is a substrate bug; got {painted:?}"
    );
}

#[test]
fn left_slot_preserves_whitespace_between_inline_spans() {
    // PAINT contract: the left slot has `<span>↑↓</span> <span>navigate</span>`
    // — a single space text node BETWEEN the two spans. CSS inline
    // formatting must preserve it (per CSS Text §White Space
    // Processing: a single whitespace character at content boundaries
    // is NOT collapsed away unless the surrounding rules say
    // `white-space: nowrap` or trimming applies). The user-visible
    // bug: "↑↓navigate" instead of "↑↓ navigate".
    let (mut dom, sheet) = build();
    let buf = paint(&mut dom, &sheet, 40, 1);
    let painted = row_text(&buf, 0);
    eprintln!("DBG painted row 0: {painted:?}");
    assert!(
        painted.contains("↑↓ navigate"),
        "whitespace between inline spans must be preserved; got {painted:?}"
    );
}
