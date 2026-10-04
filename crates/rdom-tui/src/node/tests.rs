//! The node extension traits: builder chains and getters.

use super::*;
use crate::TuiDom;
use crate::layout::{Border, Direction, Overflow, Padding, Size};
use crate::style::{Color, TuiStyle};

#[test]
fn builder_chain_sets_fields() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_width(Size::Fixed(40))
        .set_height(Size::Flex(1.0))
        .set_padding(Padding::symmetric(2, 1))
        .set_border(Border::single())
        .set_gap(1)
        .set_overflow(Overflow::Hidden)
        .set_direction(Direction::Row);

    let n = dom.node(div);
    assert_eq!(n.width(), Some(Size::Fixed(40)));
    assert_eq!(n.height(), Some(Size::Flex(1.0)));
    assert_eq!(n.padding(), Some(Padding::symmetric(2, 1)));
    assert_eq!(n.border(), Some(Border::single()));
    assert_eq!(n.gap(), Some(1));
    assert_eq!(n.overflow(), Some(Overflow::Hidden));
    assert_eq!(n.direction(), Some(Direction::Row));
}

/// C2G-LAYOUT-SAFETY — CSS Flexbox §7.1: a flex weight is `<number
/// [0,∞]>`; the setters keep a Rust-built one in range.
#[test]
fn size_setters_keep_flex_weights_in_range() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_width(Size::Flex(-3.0))
        .set_height(Size::Flex(f32::NAN));
    let n = dom.node(div);
    assert_eq!(n.width(), Some(Size::Auto));
    assert_eq!(n.height(), Some(Size::Auto));
}

#[test]
fn min_max_constraints() {
    use rdom_style::layout::{MaxSize, MinSize};
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_min_width(10u16)
        .set_max_width(100u16)
        .set_min_height(MinSize::Cells(5))
        .set_max_height(MaxSize::Cells(50));
    let e = dom.node(div).tui_ext().unwrap();
    use crate::style::Value;
    assert_eq!(
        e.inline_style_or_empty().min_width,
        Some(Value::Specified(MinSize::Cells(10)))
    );
    assert_eq!(
        e.inline_style_or_empty().max_width,
        Some(Value::Specified(MaxSize::Cells(100)))
    );
    assert_eq!(
        e.inline_style_or_empty().min_height,
        Some(Value::Specified(MinSize::Cells(5)))
    );
    assert_eq!(
        e.inline_style_or_empty().max_height,
        Some(Value::Specified(MaxSize::Cells(50)))
    );
    // `MaxSize::None` declares `none` (C3G-API).
    dom.node_mut(div).set_max_width(MaxSize::None);
    assert_eq!(
        dom.node(div)
            .tui_ext()
            .unwrap()
            .inline_style_or_empty()
            .max_width,
        Some(Value::Specified(MaxSize::None))
    );
}

#[test]
fn inline_style_settable() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)).bold(true));

    let n = dom.node(div);
    let s = n.inline_style().unwrap();
    use crate::style::{TuiColor, Value};
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(s.bold, Some(Value::Specified(true)));
}

#[test]
fn before_after_content_setters() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_before_content("▾ ")
        .set_after_content(" ←");
    let e = dom.node(div).tui_ext().unwrap();
    assert_eq!(e.before_content.as_deref(), Some("▾ "));
    assert_eq!(e.after_content.as_deref(), Some(" ←"));

    dom.node_mut(div).clear_before_content();
    assert!(dom.node(div).ext().unwrap().before_content.is_none());
}

#[test]
fn setters_noop_on_non_element() {
    let mut dom: TuiDom = TuiDom::new();
    let t = dom.create_text_node("hi");
    // No panic, no-op:
    dom.node_mut(t)
        .set_width(Size::Fixed(5))
        .set_border(Border::single());
    assert!(dom.node(t).tui_ext().is_none());
}

#[test]
fn scroll_setter() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div).set_scroll(12, 34);
    let e = dom.node(div).tui_ext().unwrap();
    assert_eq!(e.scroll_x, 12);
    assert_eq!(e.scroll_y, 34);
}
