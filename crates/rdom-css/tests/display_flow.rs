//! BFC-1 Phase 1: `display` keyword writes both outer (`Display`)
//! and inner (`Flow`) per the CSS3 Display Module mapping.

use rdom_css::parse_inline;
use rdom_style::Value;
use rdom_style::layout::{Display, Flow};

fn parse(decl: &str) -> rdom_style::TuiStyle {
    parse_inline(decl).style
}

#[test]
fn display_block_sets_block_outer_and_block_inner() {
    let s = parse("display: block");
    assert_eq!(s.display, Some(Value::Specified(Display::Block)));
    assert_eq!(s.flow, Some(Value::Specified(Flow::Block)));
}

#[test]
fn display_flex_sets_block_outer_and_flex_inner() {
    let s = parse("display: flex");
    assert_eq!(s.display, Some(Value::Specified(Display::Block)));
    assert_eq!(s.flow, Some(Value::Specified(Flow::Flex)));
}

#[test]
fn display_inline_sets_inline_outer_and_no_flow() {
    let s = parse("display: inline");
    assert_eq!(s.display, Some(Value::Specified(Display::Inline)));
    // No flow write — inline elements don't have an inner formatting
    // context of their own (they participate in their parent's IFC).
    assert!(s.flow.is_none(), "inline doesn't set flow");
}

#[test]
fn display_inline_block_sets_inline_block_outer_and_block_inner() {
    let s = parse("display: inline-block");
    assert_eq!(s.display, Some(Value::Specified(Display::InlineBlock)));
    assert_eq!(
        s.flow,
        Some(Value::Specified(Flow::Block)),
        "inline-block's inner is block (children lay out as block)"
    );
}

#[test]
fn display_inline_flex_sets_inline_outer_and_flex_inner() {
    let s = parse("display: inline-flex");
    assert_eq!(s.display, Some(Value::Specified(Display::Inline)));
    assert_eq!(s.flow, Some(Value::Specified(Flow::Flex)));
}

#[test]
fn display_none_sets_none_and_no_flow() {
    let s = parse("display: none");
    assert_eq!(s.display, Some(Value::Specified(Display::None)));
    assert!(s.flow.is_none(), "display: none doesn't set flow");
}
