//! `EXT-LAYOUT-SETTERS-1`: the geometry node setters
//! (`set_width`/`set_direction`/…) must drive layout, not silently
//! write dead `ext.*` fields the cascade/layout never read.
//!
//! Before the fix: setters wrote raw `ext` fields; flex layout read
//! only `ComputedStyle`, so a container/children configured purely via
//! node setters laid out as if nothing was set (default column, auto
//! sizes). This test pins that the setters now flow through
//! `inline_style` → cascade → computed → layout.

use crate::common::render;
use rdom_tui::prelude::*;
use rdom_tui::render::Rect;
use rdom_tui::{Direction, Display, Flow, Size, Stylesheet, TuiStyle};

#[test]
fn node_setters_drive_flex_layout() {
    let mut dom = TuiDom::new();
    let root = dom.root();

    // Container is a flex row (display:flex via inline style; the
    // *direction* is set through the node setter under test).
    let container = dom.create_element("div");
    dom.node_mut(container)
        .set_inline_style(TuiStyle::new().display(Display::Block).flow(Flow::Flex))
        .set_direction(Direction::Row)
        .set_width(Size::Fixed(100))
        .set_height(Size::Fixed(10));
    dom.append_child(root, container).unwrap();

    // Two children sized purely via the node setters under test.
    let a = dom.create_element("div");
    dom.node_mut(a)
        .set_width(Size::Fixed(10))
        .set_height(Size::Fixed(3));
    dom.append_child(container, a).unwrap();

    let b = dom.create_element("div");
    dom.node_mut(b)
        .set_width(Size::Fixed(20))
        .set_height(Size::Fixed(3));
    dom.append_child(container, b).unwrap();

    let _ = render(&mut dom, &Stylesheet::new(), Rect::new(0, 0, 100, 10));

    let ra = dom.node(a).layout_rect().unwrap();
    let rb = dom.node(b).layout_rect().unwrap();

    // set_direction(Row) ⇒ children sit side by side.
    assert!(
        rb.x > ra.x,
        "set_direction(Row) should place children horizontally: a={ra:?} b={rb:?}"
    );
    assert_eq!(ra.y, rb.y, "row children share the top edge");
    // set_width(..) ⇒ honored by layout.
    assert_eq!(ra.width, 10, "set_width(10) must drive the laid-out width");
    assert_eq!(rb.width, 20, "set_width(20) must drive the laid-out width");
}

/// C6G-SIDE-SETTERS: the Phase 6 node setters — `visibility`, `order`,
/// `flex-wrap`, `flex-direction`, the six alignment properties — and
/// the spacing ones (`set_margin`, `set_padding` taking a plain count)
/// drive the cascade and layout, through the prelude's types.
#[test]
fn phase6_node_setters_drive_layout() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = dom.create_element("div");
    dom.node_mut(f)
        .set_inline_style(TuiStyle::new().flex())
        .set_flex_direction(FlexDirection::RowReverse)
        .set_flex_wrap(FlexWrap::Wrap)
        .set_justify_content(Align::Center)
        .set_align_items(Align::FlexEnd)
        .set_align_content(Align::Stretch)
        .set_justify_items(Align::Center)
        .set_width(Size::Fixed(10))
        .set_height(Size::Fixed(4))
        .set_padding(1u16);
    dom.append_child(root, f).unwrap();
    let a = dom.create_element("div");
    dom.node_mut(a)
        .set_width(Size::Fixed(2))
        .set_height(Size::Fixed(1))
        .set_order(1)
        .set_margin(Margin::all_cells(0))
        .set_align_self(Alignment::AUTO)
        .set_justify_self(Align::Start)
        .set_visibility(Visibility::Hidden);
    dom.append_child(f, a).unwrap();
    let b = dom.create_element("div");
    dom.node_mut(b)
        .set_width(Size::Fixed(2))
        .set_height(Size::Fixed(1));
    dom.append_child(f, b).unwrap();
    let _ = render(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 20, 6));

    let computed = dom.node(f).computed().unwrap().clone();
    assert_eq!(computed.flex_direction(), FlexDirection::RowReverse);
    assert_eq!(computed.flex_wrap, FlexWrap::Wrap);
    assert_eq!(computed.justify_items, Alignment::new(Align::Center));
    assert_eq!(computed.padding, Padding::all(1));
    let ca = dom.node(a).computed().unwrap();
    assert_eq!((ca.order, ca.visibility), (1, Visibility::Hidden));
    // The content box is 10 × 4 at (1, 1) (`width` / `height` measure
    // it, the padding lies outside). Row-reverse and centered, `a`
    // ordered after `b`: the 4 cells of items sit at x 4..8, `b` at the
    // main-start (right) end.
    let (ra, rb) = (
        dom.node(a).layout_rect().unwrap(),
        dom.node(b).layout_rect().unwrap(),
    );
    assert_eq!((rb.x, ra.x), (6, 4));
    // `align-content: stretch` gives the one line the 4 rows;
    // `align-items: flex-end` puts `b` on the last.
    assert_eq!(rb.y, 4);
    let _: GapValue = GapValue::Cells(1);
}
