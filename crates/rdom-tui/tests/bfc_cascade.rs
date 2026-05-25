//! BFC-1 Phase 1: cascade tests for `flow` and
//! `establishes_new_bfc`. Pin the cascade-time computation against
//! the spec table from `specs/BFC-1.md` so phase 5's margin-
//! collapse pass can rely on the predicate.

use rdom_style::layout::{Flow, MinSize, Overflow, Position};
use rdom_tui::prelude::*;
use rdom_tui::{CascadeExt, TuiDom};

fn computed(dom: &TuiDom, id: rdom_tui::NodeId) -> rdom_style::ComputedStyle {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .cloned()
        .expect("cascade ran")
}

fn one_element_dom(sheet_rule: TuiStyle) -> (TuiDom, rdom_tui::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let e = dom.create_element("e");
    dom.append_child(root, e).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked("e", sheet_rule);
    dom.cascade(&sheet);
    (dom, e)
}

// ── Flow ──────────────────────────────────────────────────────────

#[test]
fn flow_defaults_to_block() {
    let (dom, e) = one_element_dom(TuiStyle::new());
    assert_eq!(computed(&dom, e).flow, Flow::Block);
}

#[test]
fn display_flex_resolves_to_flow_flex() {
    let (dom, e) = one_element_dom(TuiStyle::new().flow(Flow::Flex));
    assert_eq!(computed(&dom, e).flow, Flow::Flex);
}

#[test]
fn flow_does_not_inherit() {
    // A flex parent's flex flow should NOT propagate to a block
    // child. CSS spec: `display` (and by extension `flow`) is
    // non-inheriting.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("p", TuiStyle::new().flow(Flow::Flex));
    dom.cascade(&sheet);

    assert_eq!(computed(&dom, parent).flow, Flow::Flex);
    assert_eq!(
        computed(&dom, child).flow,
        Flow::Block,
        "child must default back to Block — flow is non-inheriting"
    );
}

// ── BFC formation predicate ──────────────────────────────────────

#[test]
fn block_with_visible_overflow_does_not_establish_new_bfc() {
    let (dom, e) = one_element_dom(TuiStyle::new());
    assert!(!computed(&dom, e).establishes_new_bfc);
}

#[test]
fn flex_container_establishes_new_bfc() {
    let (dom, e) = one_element_dom(TuiStyle::new().flow(Flow::Flex));
    assert!(computed(&dom, e).establishes_new_bfc);
}

#[test]
fn inline_block_establishes_new_bfc() {
    use rdom_style::layout::Display;
    let (dom, e) = one_element_dom(TuiStyle::new().display(Display::InlineBlock));
    assert!(computed(&dom, e).establishes_new_bfc);
}

#[test]
fn overflow_hidden_on_either_axis_establishes_new_bfc() {
    let (dom, x_hidden) = one_element_dom(TuiStyle::new().overflow_x(Overflow::Hidden));
    assert!(computed(&dom, x_hidden).establishes_new_bfc);

    let (dom, y_hidden) = one_element_dom(TuiStyle::new().overflow_y(Overflow::Hidden));
    assert!(computed(&dom, y_hidden).establishes_new_bfc);
}

#[test]
fn overflow_scroll_or_auto_establishes_new_bfc() {
    for ov in [Overflow::Scroll, Overflow::Auto] {
        let (dom, e) = one_element_dom(TuiStyle::new().overflow_x(ov));
        assert!(
            computed(&dom, e).establishes_new_bfc,
            "{ov:?} on overflow-x must establish new BFC"
        );
    }
}

#[test]
fn absolute_positioning_establishes_new_bfc() {
    let (dom, e) = one_element_dom(TuiStyle::new().position(Position::Absolute));
    assert!(computed(&dom, e).establishes_new_bfc);
}

#[test]
fn fixed_positioning_establishes_new_bfc() {
    let (dom, e) = one_element_dom(TuiStyle::new().position(Position::Fixed));
    assert!(computed(&dom, e).establishes_new_bfc);
}

#[test]
fn relative_positioning_does_not_by_itself_establish_new_bfc() {
    // Relative positioning offsets the element after layout but
    // doesn't form a new BFC. Margin collapse should still cross
    // a relatively-positioned parent.
    let (dom, e) = one_element_dom(TuiStyle::new().position(Position::Relative));
    assert!(!computed(&dom, e).establishes_new_bfc);
}

#[test]
fn min_width_auto_alone_does_not_establish_new_bfc() {
    // Sanity: min-width-auto + nothing else doesn't trip the
    // predicate (unrelated to BFC formation).
    let (dom, e) = one_element_dom(TuiStyle::new().min_width(MinSize::Auto));
    assert!(!computed(&dom, e).establishes_new_bfc);
}
