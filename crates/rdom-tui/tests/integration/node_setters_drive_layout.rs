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
use rdom_tui::{Direction, Display, Flow, Size, TuiStyle};

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
