//! BFC-1 Phase 2 — unit tests for `layout_block_children`.
//!
//! These tests bypass `layout_children`'s dispatch (which hasn't
//! been wired yet — phase 4) and call `layout_block_children`
//! directly to pin the width formula + auto margins + min/max
//! behavior. After phase 4 lands, these become parity tests for
//! the live dispatch path.

#![cfg(test)]

use rdom_core::Dom;

use crate::ext::TuiExt;
use crate::layout::{Border, LayoutRect, Margin, MarginValue, Padding, Size};
use crate::node::TuiNodeExt;
use crate::prelude::*;
use crate::render::layout_pass::block::layout_block_children;
use crate::style::ComputedStyle;

fn dom() -> TuiDom {
    TuiDom::new()
}

fn cascade(dom: &mut TuiDom, sheet: &Stylesheet) {
    dom.cascade(sheet);
}

fn layout_of(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).ext().unwrap().layout
}

fn run_block(dom: &mut TuiDom, container_id: NodeId, container_rect: LayoutRect) {
    let computed = dom
        .node(container_id)
        .computed()
        .cloned()
        .unwrap_or_else(ComputedStyle::initial);
    layout_block_children(
        dom as &mut Dom<TuiExt>,
        container_id,
        container_rect,
        &computed,
    );
}

// ── Width formula ────────────────────────────────────────────────

#[test]
fn block_child_with_width_auto_fills_containing_block() {
    // CSS 2.1 §10.3.3: width auto + non-auto margins → width
    // absorbs the leftover after frame + margins.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare(); // no rules — defaults
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    assert_eq!(r.x, 0);
    assert_eq!(r.width, 80, "auto width fills containing block");
}

#[test]
fn block_child_with_width_fixed_respects_declared() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("c", TuiStyle::new().width(Size::Fixed(30)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, child).width, 30);
}

#[test]
fn block_child_with_width_percent_resolves_against_containing_block() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("c", TuiStyle::new().width(Size::Percent(50)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, child).width,
        40,
        "50% of 80-cell containing block = 40"
    );
}

#[test]
fn block_child_with_explicit_left_right_margins_shrinks_auto_width() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new().margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Cells(5),
            MarginValue::Cells(0),
            MarginValue::Cells(3),
        )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    assert_eq!(r.x, 3, "left margin shifts x");
    assert_eq!(r.width, 72, "width = 80 - 3 left margin - 5 right margin");
}

// ── Auto margins ─────────────────────────────────────────────────

#[test]
fn auto_margin_left_pushes_block_to_right_edge() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new().width(Size::Fixed(20)).margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Cells(0),
            MarginValue::Cells(0),
            MarginValue::Auto,
        )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    assert_eq!(
        r.x, 60,
        "margin-left: auto absorbs 60 cells, pushing child to x=60"
    );
    assert_eq!(r.width, 20);
}

#[test]
fn auto_margin_right_pushes_block_to_left_edge() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new().width(Size::Fixed(20)).margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Auto,
            MarginValue::Cells(0),
            MarginValue::Cells(0),
        )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    assert_eq!(r.x, 0, "child stays at x=0");
    assert_eq!(r.width, 20);
}

#[test]
fn both_auto_margins_center_block() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new().width(Size::Fixed(40)).margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Auto,
            MarginValue::Cells(0),
            MarginValue::Auto,
        )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    assert_eq!(r.x, 20, "centered: (80 - 40) / 2 = 20");
    assert_eq!(r.width, 40);
}

#[test]
fn both_auto_margins_with_odd_leftover_distribute_odd_cell_to_right() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new().width(Size::Fixed(41)).margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Auto,
            MarginValue::Cells(0),
            MarginValue::Auto,
        )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    // Leftover = 80 - 41 = 39. Half (floor) = 19. ML = 19, MR = 20.
    assert_eq!(
        r.x, 19,
        "odd leftover splits 19/20 (right gets the odd cell)"
    );
}

// ── Min/max width ────────────────────────────────────────────────

#[test]
fn max_width_clamps_block_smaller() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("c", TuiStyle::new().width(Size::Fixed(50)).max_width(30));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, child).width, 30, "max-width clamps");
}

#[test]
fn min_width_floors_block_larger() {
    use crate::layout::MinSize;

    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .min_width(MinSize::Cells(25)),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, child).width, 25, "min-width floors");
}

// ── Vertical stacking ────────────────────────────────────────────

#[test]
fn block_children_stack_vertically_in_document_order() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(parent, c).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(2)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(3)))
        .rule_unchecked("c", TuiStyle::new().height(Size::Fixed(1)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, a).y, 0);
    assert_eq!(layout_of(&dom, a).height, 2);
    assert_eq!(layout_of(&dom, b).y, 2, "b stacks below a (no margin yet)");
    assert_eq!(layout_of(&dom, b).height, 3);
    assert_eq!(layout_of(&dom, c).y, 5, "c stacks below b");
}

#[test]
fn block_children_overflow_below_container() {
    // CSS 2.1: block children with `overflow: visible` (default)
    // are placed at their natural heights even when they exceed
    // the container. The container overflows below — no shrinking.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(20)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(20)));
    cascade(&mut dom, &sheet);
    // Container only 10 cells tall — children overflow.
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 10));

    assert_eq!(layout_of(&dom, a).height, 20);
    assert_eq!(layout_of(&dom, b).y, 20);
    assert_eq!(layout_of(&dom, b).height, 20);
}

#[test]
fn vertical_margins_stack_additively_phase_2() {
    // Phase 2 doesn't implement margin collapse (phase 5). Two
    // siblings with top/bottom margins simply add — top of B sits
    // at (bottom of A + A's margin-bottom + B's margin-top).
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(2),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, a).y, 0);
    assert_eq!(
        layout_of(&dom, b).y,
        2 + 2 + 3,
        "A bottom (2) + A margin-bottom (2) + B margin-top (3) = 7"
    );
}

// ── Out-of-flow children skipped ────────────────────────────────

#[test]
fn absolutely_positioned_child_does_not_advance_cursor() {
    use crate::layout::Position;

    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let abs = dom.create_element("z");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, abs).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(2)))
        .rule_unchecked("z", TuiStyle::new().position(Position::Absolute))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(3)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    // `b` should stack right after `a` — `z` is out of flow.
    assert_eq!(layout_of(&dom, b).y, 2);
}

#[test]
fn display_none_child_does_not_advance_cursor() {
    use crate::layout::Display;

    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let hidden = dom.create_element("z");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, hidden).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(2)))
        .rule_unchecked("z", TuiStyle::new().display(Display::None))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(3)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, b).y, 2);
}

// ── Padding / border on parent affect cursor base, not child width ─

#[test]
fn padding_on_block_container_is_handled_by_layout_node() {
    // `layout_block_children` receives the container rect AFTER
    // padding+border insets are applied (the caller — `layout_node`
    // — does that via `compute_content_area_collapsed`). Verify
    // by calling with a pre-inset container directly. (Once dispatch
    // wires in phase 4, this is the natural flow.)
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    cascade(&mut dom, &Stylesheet::bare());
    // Caller-provided inset: container starts at (3, 1).
    run_block(&mut dom, parent, LayoutRect::new(3, 1, 70, 20));

    let r = layout_of(&dom, child);
    assert_eq!(r.x, 3, "child anchored at container.x");
    assert_eq!(r.y, 1, "child anchored at container.y");
    assert_eq!(r.width, 70);
}

// ── Sanity: border + padding eat into child width ───────────────

#[test]
fn child_with_padding_and_border_reduces_intrinsic_width_via_layout_node() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .padding(Padding {
                top: 0,
                right: 2,
                bottom: 0,
                left: 2,
            })
            .border(Border::Single),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    // Child's OUTER width is 80 (fills containing block on auto
    // width); padding/border eat into the content area, which
    // `layout_node` handles. The block pass writes outer rect only.
    assert_eq!(layout_of(&dom, child).width, 80);
}
