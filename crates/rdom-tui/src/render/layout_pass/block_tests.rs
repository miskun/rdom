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

// ── BFC-1 Phase 5: margin collapsing (CSS 2.1 §8.3.1) ──────────

#[test]
fn adjacent_positive_margins_collapse_to_max() {
    // CSS 2.1 §8.3.1: two adjacent in-flow blocks' bottom + top
    // vertical margins collapse to max(bottom, top) when both are
    // positive. A.margin-bottom = 2, B.margin-top = 3 → gap = 3
    // (not 5).
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
        2 + 3,
        "A bottom (2) + max(A.mb=2, B.mt=3)=3 = 5"
    );
}

#[test]
fn adjacent_margins_equal_collapse_to_one() {
    // Both 3 → max(3, 3) = 3 (not 6).
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
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, b).y,
        1 + 3,
        "1 (A height) + 3 (collapsed margin) = 4"
    );
}

#[test]
fn three_adjacent_siblings_collapse_each_gap_independently() {
    // A.mb=2 / B.mt=4 / B.mb=1 / C.mt=2 → gap1 = max(2,4)=4, gap2 = max(1,2)=2.
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
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(2),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(4),
                MarginValue::Cells(0),
                MarginValue::Cells(1),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, a).y, 0);
    assert_eq!(layout_of(&dom, b).y, 1 + 4, "after A: 1 + max(2, 4) = 5");
    assert_eq!(
        layout_of(&dom, c).y,
        1 + 4 + 1 + 2,
        "after B: prev (5) + B.h (1) + max(B.mb=1, C.mt=2) = 8"
    );
}

// ── Phase 5.2b: upward propagation of merged parent-child margins

#[test]
fn parent_outer_top_margin_propagates_child_margin_upward_to_grandparent() {
    // CSS 2.1 §8.3.1: when a parent collapses with its first
    // block child, the *merged* margin (max of parent.mt +
    // child.mt) surfaces at the parent's OUTER top — it must
    // therefore reach the grandparent's accumulator, not just the
    // parent's local one.
    //
    // GP has two children: A (height: 1) and P. P has C as its
    // first block child.
    //   A.mb = 0
    //   P.mt = 2
    //   C.mt = 5
    // Expected gap between A and P: max(0, 2, 5) = 5.
    // P.y = A.bottom (1) + 5 = 6.
    let mut dom = dom();
    let root = dom.root();
    let gp = dom.create_element("gp");
    let a = dom.create_element("a");
    let p = dom.create_element("p");
    let c = dom.create_element("c");
    dom.append_child(p, c).unwrap();
    dom.append_child(gp, a).unwrap();
    dom.append_child(gp, p).unwrap();
    dom.append_child(root, gp).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked(
            "p",
            TuiStyle::new().height(Size::Fixed(10)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(5),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, gp, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, p).y,
        1 + 5,
        "P.outer_top includes max(P.mt=2, C.mt=5) = 5 → P.y = A.bottom (1) + 5 = 6"
    );
    assert_eq!(
        layout_of(&dom, c).y,
        layout_of(&dom, p).y,
        "C is suppressed inside P — sits at P.y"
    );
}

#[test]
fn parent_outer_top_margin_propagates_through_multilevel_chain() {
    // Three levels: GP places M; M places P; P places C.
    // All collapse (no padding/border/BFC anywhere).
    //   A.mb = 0
    //   M.mt = 1
    //   P.mt = 3
    //   C.mt = 7
    // Expected outer-top of M = max(1, 3, 7) = 7.
    // M.y = A.bottom (1) + 7 = 8.
    let mut dom = dom();
    let root = dom.root();
    let gp = dom.create_element("gp");
    let a = dom.create_element("a");
    let m = dom.create_element("m");
    let p = dom.create_element("p");
    let c = dom.create_element("c");
    dom.append_child(p, c).unwrap();
    dom.append_child(m, p).unwrap();
    dom.append_child(gp, a).unwrap();
    dom.append_child(gp, m).unwrap();
    dom.append_child(root, gp).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked(
            "m",
            TuiStyle::new().height(Size::Fixed(10)).margin(Margin::new(
                MarginValue::Cells(1),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new().height(Size::Fixed(8)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(7),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, gp, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, m).y,
        1 + 7,
        "M.outer_top includes max(M.mt=1, P.mt=3, C.mt=7) = 7"
    );
}

#[test]
fn parent_outer_top_chain_stops_at_padding() {
    // GP -> M (no padding) -> P (top padding 2!) -> C (mt=9)
    // The padding on P blocks the chain — only M.mt + P.mt
    // collapse upward. C.mt stays inside P.
    let mut dom = dom();
    let root = dom.root();
    let gp = dom.create_element("gp");
    let a = dom.create_element("a");
    let m = dom.create_element("m");
    let p = dom.create_element("p");
    let c = dom.create_element("c");
    dom.append_child(p, c).unwrap();
    dom.append_child(m, p).unwrap();
    dom.append_child(gp, a).unwrap();
    dom.append_child(gp, m).unwrap();
    dom.append_child(root, gp).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked(
            "m",
            TuiStyle::new().height(Size::Fixed(15)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .height(Size::Fixed(10))
                .padding(Padding::new(2, 0, 0, 0))
                .margin(Margin::new(
                    MarginValue::Cells(4),
                    MarginValue::Cells(0),
                    MarginValue::Cells(0),
                    MarginValue::Cells(0),
                )),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(9),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, gp, LayoutRect::new(0, 0, 80, 24));

    // M.outer_top = collapse(M.mt=2, P.mt=4) = 4.
    // M.y = A.bottom (1) + 4 = 5.
    assert_eq!(
        layout_of(&dom, m).y,
        1 + 4,
        "padding on P blocks chain — M.outer_top = max(2, 4) = 4"
    );
    // P sits at M.y; C sits inside P at P.y + 2 (padding) + 9 (own margin).
    assert_eq!(layout_of(&dom, p).y, 1 + 4, "P at M.y (M-P collapse)");
    assert_eq!(
        layout_of(&dom, c).y,
        layout_of(&dom, p).y + 2 + 9,
        "C inside P: P.y + padding (2) + C.mt (9)"
    );
}

// ── Phase 5.3: empty-block collapse-through ─────────────────────

#[test]
fn empty_block_between_siblings_collapses_top_and_bottom_through() {
    // CSS 2.1 §8.3.1: a block with no content, no padding, no
    // border collapses its top + bottom margins into the
    // surrounding accumulator. Effectively the empty block becomes
    // a single contribution that participates in the collapse with
    // its siblings on both sides.
    //
    // A.mb=2 / E.mt=3 / E.mb=4 / B.mt=1 →
    //   all four collapse into one gap of max(2,3,4,1) = 4.
    // E is an empty block — its top + bottom margins meet because
    // it has no content/padding/border separating them.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let e = dom.create_element("e");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, e).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(2),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "e",
            // No height, no padding, no border, no children → empty
            // collapse-through. Margins 3 / 4.
            TuiStyle::new().margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(4),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(1),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    // a at y=0, height 1 → bottom at y=1.
    // collapse(2, 3, 4, 1) = 4.
    // b sits at 1 + 4 = 5.
    assert_eq!(
        layout_of(&dom, b).y,
        5,
        "all four margins collapse to max(2,3,4,1)=4"
    );
}

// ── Phase 5.2: parent–first/last child collapse ─────────────────

#[test]
fn parent_top_margin_collapses_with_first_child_when_no_padding_or_border() {
    // CSS 2.1 §8.3.1: parent's `margin-top` collapses through to
    // include the first in-flow block child's `margin-top` when
    // the parent has no top padding, no top border, no clearance,
    // and doesn't establish a new BFC.
    //
    // Here parent.mt=4, child.mt=2 → collapsed = max(4, 2) = 4.
    // The collapsed margin lands ABOVE the parent (not between
    // parent and child) — so child.y = parent.y (parent has no
    // padding/border so content starts at outer top), and the
    // parent's outer position absorbs the larger value.
    //
    // We test the OBSERVABLE consequence: the first child's
    // y-offset *relative to the parent* is 0, not the child's
    // own margin-top.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new().height(Size::Fixed(10)).margin(Margin::new(
                MarginValue::Cells(4),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    // Parent is direct child of container at (0,0) — its own
    // top margin would be applied by its own parent, not by
    // `run_block` (which lays out p's children only). So we place
    // p ourselves at y=0 to focus the assertion on the
    // child's position INSIDE p.
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, child).y,
        0,
        "child top margin collapsed through parent (no padding/border) → child.y = parent.y (= 0)"
    );
}

#[test]
fn parent_top_padding_blocks_first_child_collapse() {
    // CSS 2.1 §8.3.1: a non-zero top padding on the parent breaks
    // the parent–first-child top margin collapse. The first child's
    // top margin applies normally INSIDE the parent's padded content
    // area.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .height(Size::Fixed(10))
                .padding(Padding::new(2, 0, 0, 0)),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    // Layout the WHOLE tree through `layout_dom` so the parent's
    // padding is applied to the content rect that `layout_block_children`
    // sees. `run_block` calls layout_block_children directly with
    // a raw container rect (bypassing layout_node's content-rect
    // computation), so use the public entry point instead.
    use crate::render::Rect;
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    // Parent's content area starts at row 2 (after 2-cell top padding).
    // Child's margin-top (3) applies inside that, so child.y = 2 + 3 = 5.
    assert_eq!(
        layout_of(&dom, child).y,
        2 + 3,
        "padding blocks collapse — child sits below padding + own margin"
    );
}

#[test]
fn parent_top_border_blocks_first_child_collapse() {
    use crate::layout::Border;

    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .height(Size::Fixed(10))
                .border(Border::Single),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    use crate::render::Rect;
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, child).y,
        1 + 3,
        "border blocks collapse — child sits below border row + own margin"
    );
}

#[test]
fn parent_overflow_hidden_blocks_first_child_collapse() {
    // CSS 2.1 §8.3.1: `overflow` other than `visible` establishes
    // a new BFC; that blocks parent-child margin collapse.
    use crate::layout::Overflow;

    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .height(Size::Fixed(10))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    use crate::render::Rect;
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, child).y,
        3,
        "overflow: hidden establishes new BFC → child margin applies inside"
    );
}

#[test]
fn adjacent_negative_margins_collapse_to_most_negative() {
    // CSS 2.1 §8.3.1: two negative margins collapse to the most
    // negative (= min). A.mb=-3 B.mt=-1 → gap = -3.
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
                MarginValue::Cells(-3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(-1),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, b).y,
        2 + (-3),
        "A.height (2) + min(-3, -1) = -1 → B at y=-1"
    );
}

#[test]
fn adjacent_mixed_sign_margins_sum_via_positive_plus_negative() {
    // CSS 2.1 §8.3.1: mixed → max(positives) + min(negatives).
    // For two margins +5, -3: result = 5 + (-3) = 2.
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
                MarginValue::Cells(5),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(-3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, b).y,
        2 + (5 + (-3)),
        "A.height (2) + (5 + -3 = 2) = 4"
    );
}

#[test]
fn absolutely_positioned_sibling_does_not_break_margin_adjacency() {
    // CSS 2.1 §9.3: an out-of-flow sibling between two in-flow
    // siblings doesn't disturb their margin adjacency. A.mb=3 and
    // C.mt=2 still collapse as if the absolute B weren't there.
    use crate::layout::Position;

    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let abs = dom.create_element("z");
    let c = dom.create_element("c");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, abs).unwrap();
    dom.append_child(parent, c).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked("z", TuiStyle::new().position(Position::Absolute))
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(
        layout_of(&dom, c).y,
        1 + 3,
        "C.y = A.bottom (1) + max(A.mb=3, C.mt=2) = 4 — out-of-flow Z skipped"
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

// ── Anonymous block box generation (CSS 2.1 §9.2.1.1) ───────────

fn anon_blocks_of(dom: &TuiDom, id: NodeId) -> Vec<crate::ext::AnonymousIfc> {
    dom.node(id).ext().unwrap().anonymous_blocks.clone()
}

#[test]
fn text_only_paragraph_wraps_in_anonymous_block() {
    // CSS 2.1 §9.2.1.1: a block container whose only children are
    // inline-level (here, a single text node) folds those children
    // into one anonymous block establishing an IFC.
    let mut dom = dom();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello world");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();

    cascade(&mut dom, &Stylesheet::bare());
    run_block(&mut dom, p, LayoutRect::new(0, 0, 20, 24));

    let anons = anon_blocks_of(&dom, p);
    assert_eq!(anons.len(), 1, "one anonymous block wraps the text");
    assert_eq!(anons[0].rect.x, 0);
    assert_eq!(anons[0].rect.y, 0);
    assert_eq!(anons[0].rect.width, 20);
    assert_eq!(
        anons[0].rect.height, 1,
        "11-char text fits on one line at width 20"
    );
}

#[test]
fn mixed_text_and_inline_element_share_one_anonymous_block() {
    // `<p>text <em>italic</em> more</p>` — all children are inline-
    // level (text, Display::Inline element, text). They share ONE
    // anonymous block per CSS 2.1 §9.2.1.1 rule 2.
    use crate::layout::Display;
    let mut dom = dom();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("text ");
    let em = dom.create_element("em");
    let t_em = dom.create_text_node("italic");
    dom.append_child(em, t_em).unwrap();
    let t2 = dom.create_text_node(" more");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, em).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("em", TuiStyle::new().display(Display::Inline));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, p, LayoutRect::new(0, 0, 30, 24));

    let anons = anon_blocks_of(&dom, p);
    assert_eq!(
        anons.len(),
        1,
        "consecutive inline-level children share one anonymous block"
    );
    // child_range covers all three direct children (text + em + text).
    assert_eq!(anons[0].child_range, (0, 3));
}

#[test]
fn block_then_text_then_block_produces_anon_block_in_the_middle() {
    // `<div><h1>X</h1>text<h2>Y</h2></div>` — block + inline + block.
    // The inline run (one text node) wraps in an anonymous block
    // that sits between the two block siblings.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("X");
    dom.append_child(h1, h1_t).unwrap();
    let text = dom.create_text_node("middle text");
    let h2 = dom.create_element("h2");
    let h2_t = dom.create_text_node("Y");
    dom.append_child(h2, h2_t).unwrap();
    dom.append_child(parent, h1).unwrap();
    dom.append_child(parent, text).unwrap();
    dom.append_child(parent, h2).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("h1", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("h2", TuiStyle::new().height(Size::Fixed(1)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 20, 24));

    let anons = anon_blocks_of(&dom, parent);
    assert_eq!(anons.len(), 1, "only the middle inline run wraps");
    // Anon block sits at y=1 (below h1), height=1 (text fits).
    assert_eq!(anons[0].rect.y, 1);
    assert_eq!(anons[0].rect.height, 1);
    assert_eq!(anons[0].child_range, (1, 2), "wraps the text node only");

    // The two block children sit at the expected y positions.
    assert_eq!(layout_of(&dom, h1).y, 0);
    assert_eq!(layout_of(&dom, h2).y, 2, "h2 follows anon block");
}

#[test]
fn multiple_inline_runs_separated_by_block_each_get_their_own_anonymous() {
    // text + h1 + text + h2 + text → 3 anonymous blocks interleaved.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let t1 = dom.create_text_node("one");
    let h1 = dom.create_element("h1");
    let h1t = dom.create_text_node("X");
    dom.append_child(h1, h1t).unwrap();
    let t2 = dom.create_text_node("two");
    let h2 = dom.create_element("h2");
    let h2t = dom.create_text_node("Y");
    dom.append_child(h2, h2t).unwrap();
    let t3 = dom.create_text_node("three");
    dom.append_child(parent, t1).unwrap();
    dom.append_child(parent, h1).unwrap();
    dom.append_child(parent, t2).unwrap();
    dom.append_child(parent, h2).unwrap();
    dom.append_child(parent, t3).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("h1", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("h2", TuiStyle::new().height(Size::Fixed(1)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 20, 24));

    let anons = anon_blocks_of(&dom, parent);
    assert_eq!(anons.len(), 3, "three inline runs → three anonymous blocks");
    // Document-order child ranges: [0,1), [2,3), [4,5).
    assert_eq!(anons[0].child_range, (0, 1));
    assert_eq!(anons[1].child_range, (2, 3));
    assert_eq!(anons[2].child_range, (4, 5));
}

#[test]
fn pure_block_container_has_no_anonymous_boxes() {
    // `<div><h1>X</h1><p>Y</p></div>` — all children are block-level.
    // No anon boxes synthesized.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let h1 = dom.create_element("h1");
    let h1t = dom.create_text_node("X");
    dom.append_child(h1, h1t).unwrap();
    let body = dom.create_element("body");
    let bodyt = dom.create_text_node("Y");
    dom.append_child(body, bodyt).unwrap();
    dom.append_child(parent, h1).unwrap();
    dom.append_child(parent, body).unwrap();
    dom.append_child(root, parent).unwrap();

    cascade(&mut dom, &Stylesheet::bare());
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 30, 24));

    assert!(
        anon_blocks_of(&dom, parent).is_empty(),
        "pure-block container has no anonymous boxes"
    );
}

// ── Inline-block atomic packing in IFC (phase 3.5b) ─────────────

#[test]
fn inline_block_inside_paragraph_packs_as_atomic_and_brackets_render() {
    // CSS 2.1 §10.8 + BFC-1 phase 3.5b: a `Display::InlineBlock`
    // child of an IFC participates as an atomic inline-level box.
    // UA pseudos (`<button>`'s `[ ]`) paint at the fragment's
    // rect via `paint_inline_content`'s single-row-chrome path.
    use crate::render::{Buffer, PaintExt, Rect};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("hi ");
    let btn = dom.create_element("button");
    let btn_t = dom.create_text_node("X");
    dom.append_child(btn, btn_t).unwrap();
    let t2 = dom.create_text_node(" ok");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, btn).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();
    // SUB-2 workaround: trailing inline element so the IFC predicate
    // (which requires at least one inline ELEMENT child) fires for
    // text-only-plus-inline-block content.
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();

    // Cascade with UA defaults — that's where `<button>` gets
    // `display: inline-block` + `[ ` / ` ]` pseudo chrome and
    // `<span>` gets `display: inline`.
    dom.cascade(&Stylesheet::new());

    // Full pipeline.
    let viewport = Rect::new(0, 0, 30, 5);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    // Read row 0: should contain "hi", "[ X ]" (button UA pseudo
    // chrome), and "ok".
    let mut row = String::new();
    for x in 0..30 {
        if let Some(c) = buf.cell(x, 0)
            && !c.is_spacer()
        {
            row.push_str(c.symbol());
        }
    }
    // Stronger check: "hi" must precede "[ X ]" must precede "ok",
    // with a space separating each (pending-space-before-atom path).
    let i_hi = row.find("hi").expect("hi missing");
    let i_btn = row.find("[ X ]").expect("button UA chrome missing");
    let i_ok = row.find("ok").expect("ok missing");
    assert!(
        i_hi < i_btn && i_btn < i_ok,
        "inline-flow order broken: got {row:?}"
    );
    // Separators (collapsed whitespace) must survive around the
    // atom — without `pending_space` honored, this would be
    // "hi[ X ]ok".
    assert!(
        row.contains("hi [ X ] ok"),
        "missing separator space around atomic inline-block: got {row:?}"
    );
}

// ── Selection / inline-flow lookup (phase 3.4) ──────────────────

#[test]
fn inline_flow_for_text_resolves_anon_box() {
    use crate::ext::TuiExt;
    use crate::render::inline::{InlineFlow, inline_flow_for_text, inline_flow_layout};
    use crate::render::layout_pass::block::layout_block_children;
    use rdom_core::Dom;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let h1 = dom.create_element("h1");
    let h1t = dom.create_text_node("X");
    dom.append_child(h1, h1t).unwrap();
    let text = dom.create_text_node("middle");
    dom.append_child(parent, h1).unwrap();
    dom.append_child(parent, text).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("h1", TuiStyle::new().height(Size::Fixed(1)));
    dom.cascade(&sheet);

    let parent_rect = LayoutRect::new(0, 0, 20, 10);
    if let Some(ext) = dom.node_mut(parent).ext_mut() {
        ext.layout = parent_rect;
        ext.content_layout = parent_rect;
    }
    let parent_computed = dom.node(parent).computed().cloned().unwrap_or_default();
    layout_block_children(
        &mut dom as &mut Dom<TuiExt>,
        parent,
        parent_rect,
        &parent_computed,
    );

    // The "middle" text node is inside an anonymous block on
    // `parent`. The new helper should find it.
    let flow = inline_flow_for_text(&dom, text).expect("text resolves to a flow");
    assert!(
        matches!(flow, InlineFlow::Anonymous { container, .. } if container == parent),
        "text in anon box resolves to InlineFlow::Anonymous on the parent, got {flow:?}",
    );

    // Looking up the layout returns a non-empty IFC.
    let (layout, rect) = inline_flow_layout(&dom, flow).expect("anon flow has layout + rect");
    assert!(!layout.lines.is_empty(), "anon IFC has at least one line");
    assert_eq!(rect.y, 1, "anon box at y=1 (below h1)");
}

// ── Hit-test integration (phase 3.3) ────────────────────────────

#[test]
fn anonymous_block_hit_test_resolves_to_source_text_node() {
    use crate::ext::TuiExt;
    use crate::render::layout_pass::block::layout_block_children;
    use crate::render::{PaintExt, Rect};
    use crate::runtime::hit_test::HitTestExt;
    use rdom_core::Dom;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let h1 = dom.create_element("h1");
    let h1t = dom.create_text_node("X");
    dom.append_child(h1, h1t).unwrap();
    let text = dom.create_text_node("middle");
    let h2 = dom.create_element("h2");
    let h2t = dom.create_text_node("Y");
    dom.append_child(h2, h2t).unwrap();
    dom.append_child(parent, h1).unwrap();
    dom.append_child(parent, text).unwrap();
    dom.append_child(parent, h2).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("h1", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("h2", TuiStyle::new().height(Size::Fixed(1)));
    dom.cascade(&sheet);

    let parent_rect = LayoutRect::new(0, 0, 20, 10);
    if let Some(ext) = dom.node_mut(parent).ext_mut() {
        ext.layout = parent_rect;
        ext.content_layout = parent_rect;
    }
    if let Some(ext) = dom.node_mut(root).ext_mut() {
        ext.layout = parent_rect;
        ext.content_layout = parent_rect;
    }
    // Lay out child blocks so hit-test descent works through them.
    let parent_computed = dom.node(parent).computed().cloned().unwrap_or_default();
    layout_block_children(
        &mut dom as &mut Dom<TuiExt>,
        parent,
        parent_rect,
        &parent_computed,
    );

    // Paint once so any internal caches are warm (no-op for hit-
    // test but matches the runtime sequence).
    let mut buf = crate::render::Buffer::empty(Rect::new(0, 0, 20, 10));
    dom.paint_dom(&mut buf, Rect::new(0, 0, 20, 10));

    // Click at (2, 1) — inside the anon box's row, second cell.
    // Should resolve to a Position inside the "middle" text node
    // with byte offset 2 ("middle"[0..2] = "mi").
    let pos = dom
        .position_at(2, 1)
        .expect("anon-box click resolves to a position");
    assert_eq!(pos.node, text, "click routes to the middle text node");
    assert_eq!(pos.offset, 2, "byte offset within the text node");
}

// ── Paint integration (phase 3.2) ───────────────────────────────

#[test]
fn anonymous_block_text_paints_at_anon_block_rect() {
    // Verify that paint_anonymous_blocks actually puts glyphs on
    // the buffer at the anon box's y. Constructs a block container
    // with mixed children (block + text + block), runs the block
    // layout pass, then a paint pass restricted to the parent's
    // content area.
    use crate::ext::TuiExt;
    use crate::render::layout_pass::block::layout_block_children;
    use crate::render::{Buffer, PaintExt, Rect};
    use rdom_core::Dom;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let h1 = dom.create_element("h1");
    let h1t = dom.create_text_node("X");
    dom.append_child(h1, h1t).unwrap();
    let text = dom.create_text_node("middle");
    let h2 = dom.create_element("h2");
    let h2t = dom.create_text_node("Y");
    dom.append_child(h2, h2t).unwrap();
    dom.append_child(parent, h1).unwrap();
    dom.append_child(parent, text).unwrap();
    dom.append_child(parent, h2).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("h1", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("h2", TuiStyle::new().height(Size::Fixed(1)));
    dom.cascade(&sheet);
    // Lay out the parent's children directly so we sidestep the
    // global dispatch (phase 4). After this the parent has 1 anon
    // box at y=1 wrapping "middle".
    let parent_rect = LayoutRect::new(0, 0, 20, 10);
    if let Some(ext) = dom.node_mut(parent).ext_mut() {
        ext.layout = parent_rect;
        ext.content_layout = parent_rect;
    }
    let parent_computed = dom.node(parent).computed().cloned().unwrap_or_default();
    layout_block_children(
        &mut dom as &mut Dom<TuiExt>,
        parent,
        parent_rect,
        &parent_computed,
    );

    // Now run the global paint pass. Since paint_node only paints
    // ELEMENT children + anonymous boxes (text nodes don't paint
    // standalone), the "middle" text should reach the buffer via
    // paint_anonymous_blocks.
    let viewport = Rect::new(0, 0, 20, 10);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    // Read row 1 (where the anon block sits) and confirm "middle"
    // appears there.
    let mut row1 = String::new();
    for x in 0..20 {
        if let Some(c) = buf.cell(x, 1)
            && !c.is_spacer()
        {
            row1.push_str(c.symbol());
        }
    }
    assert!(
        row1.contains("middle"),
        "anon-box text should paint at row 1; got {row1:?}"
    );
}

#[test]
fn out_of_flow_children_do_not_break_inline_runs() {
    // text + absolute + text → ONE anonymous block (the absolute
    // is removed from flow before partitioning, so the two text
    // nodes are adjacent in the filtered sequence).
    use crate::layout::Position;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let t1 = dom.create_text_node("one ");
    let abs = dom.create_element("abs");
    let t2 = dom.create_text_node("two");
    dom.append_child(parent, t1).unwrap();
    dom.append_child(parent, abs).unwrap();
    dom.append_child(parent, t2).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet =
        Stylesheet::bare().rule_unchecked("abs", TuiStyle::new().position(Position::Absolute));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 30, 24));

    let anons = anon_blocks_of(&dom, parent);
    assert_eq!(
        anons.len(),
        1,
        "two text nodes adjacent in flow share one anon block"
    );
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

// ── Phase 6.1: auto height sums children + margin collapse ──────

#[test]
fn auto_height_block_sums_two_fixed_children() {
    // CSS 2.1 §10.6.3: a block with `height: auto` takes the
    // vertical extent of its in-flow content (top of first child
    // to bottom of last child).
    //
    // Two fixed-height children (3 + 5 = 8) → parent.height = 8.
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(3)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(5)));
    cascade(&mut dom, &sheet);
    // Full pipeline so layout_node updates parent.height after
    // block_layout returns the measured content height.
    dom.layout_dom(Rect::new(0, 0, 40, 50));

    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        8,
        "auto height = sum of children = 3 + 5"
    );
}

#[test]
fn auto_height_block_includes_collapsed_inter_sibling_margin() {
    // Auto height includes the gap between siblings — the
    // collapsed margin space counts as content extent.
    //   a.h=2, a.mb=3, b.mt=2, b.h=4.
    //   collapsed gap = max(3, 2) = 3.
    //   total = 2 + 3 + 4 = 9.
    use crate::render::Rect;
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
                MarginValue::Cells(3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(4)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 50));

    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        2 + 3 + 4,
        "auto height includes collapsed inter-sibling margin gap"
    );
}

#[test]
fn block_height_fixed_overflows_excess_children() {
    // CSS 2.1 §10.6.3: a block with a declared height keeps that
    // declared value — children that exceed overflow below
    // (overflow: visible) or get clipped (overflow: hidden). The
    // box itself does NOT grow.
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().height(Size::Fixed(4)))
        .rule_unchecked("c", TuiStyle::new().height(Size::Fixed(10)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        4,
        "fixed-height parent stays 4 even with 10-tall child"
    );
    // Child still takes its declared height; it just overflows.
    assert_eq!(dom.node(child).ext().unwrap().layout.height, 10);
}

// ── Phase 6.3: min/max-height clamping ───────────────────────────

#[test]
fn min_height_floors_block_above_content() {
    // CSS 2.1 §10.7: `min-height` is a lower bound; if content
    // would be shorter, the block stretches to min-height.
    use crate::layout::MinSize;
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().min_height(MinSize::Cells(8)))
        .rule_unchecked("c", TuiStyle::new().height(Size::Fixed(3)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        8,
        "parent floored to min-height even though content is only 3"
    );
}

#[test]
fn max_height_caps_block_below_content() {
    // CSS 2.1 §10.7: `max-height` is an upper bound; content
    // overflows the box visually (paint clips by overflow setting).
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().max_height(5))
        .rule_unchecked("c", TuiStyle::new().height(Size::Fixed(12)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        5,
        "parent capped at max-height even though content is 12"
    );
}

#[test]
fn min_height_keeps_empty_block_open_blocking_collapse_through() {
    // Phase 5.3 + 6.3 interaction: an empty block with min-height >
    // 0 is NOT collapse-through (its min-height pins top + bottom
    // edges apart). Adjacent siblings' margins don't merge through
    // it; the min-height creates separation.
    //
    //   A.mb=4, E.mt=2, E.mb=3, E.min-height=2, B.mt=5
    //   Without E: collapse = max(4, 5) = 5.
    //   With E pinned: gap = max(4, 2) + 2 (E.height) + max(3, 5) = 4 + 2 + 5 = 11.
    use crate::layout::MinSize;
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let e = dom.create_element("e");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, e).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(4),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "e",
            TuiStyle::new()
                .min_height(MinSize::Cells(2))
                .margin(Margin::new(
                    MarginValue::Cells(2),
                    MarginValue::Cells(0),
                    MarginValue::Cells(3),
                    MarginValue::Cells(0),
                )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(5),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    // A at y=0 (h=1). E.top: collapse A.mb=4 with E.mt=2 → 4.
    // E starts at y=1+4=5, height=2 (min-height pin), bottom at 7.
    // B.top: collapse E.mb=3 with B.mt=5 → 5. B at y=7+5=12.
    assert_eq!(layout_of(&dom, b).y, 12);
}

// ── Phase 6.2: percent-height needs definite parent ─────────────

#[test]
fn percent_height_resolves_against_definite_parent() {
    // CSS 2.1 §10.5: `height: <percent>` resolves to parent's
    // content height when parent's height is *definite*. Here the
    // parent has `height: Fixed(20)` → child's `height: 50%` = 10.
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().height(Size::Fixed(20)))
        .rule_unchecked("c", TuiStyle::new().height(Size::Percent(50)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    assert_eq!(
        dom.node(child).ext().unwrap().layout.height,
        10,
        "50% of definite parent height (20) = 10"
    );
}

#[test]
fn percent_height_falls_to_auto_when_parent_height_is_indefinite() {
    // CSS 2.1 §10.5: when the parent's height is indefinite
    // (`height: auto` and not pinned by min/max), a child's
    // percent height resolves to `auto` — i.e. intrinsic content.
    //
    //   parent.height = auto
    //   child.height = 50%
    //   child has fixed-height grandchild = 3
    // Expected: child.height = 3 (intrinsic, NOT 50% of anything).
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    let gc = dom.create_element("g");
    dom.append_child(child, gc).unwrap();
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        // parent has no height declared → Auto, and Auto here
        // doesn't pin to a finite outer because the root viewport
        // size flows down through Auto cascade (rdom subtlety: a
        // top-level Auto on `<p>` falls through to grand-parent
        // viewport, but for percent-resolution purposes that
        // outer is also indefinite per CSS 2.1).
        .rule_unchecked("c", TuiStyle::new().height(Size::Percent(50)))
        .rule_unchecked("g", TuiStyle::new().height(Size::Fixed(3)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    assert_eq!(
        dom.node(child).ext().unwrap().layout.height,
        3,
        "indefinite parent → percent height falls to auto (intrinsic = 3)"
    );
}

#[test]
fn auto_height_block_with_overflow_hidden_includes_descendant_margins() {
    // With `overflow: hidden` (new BFC), the parent traps its
    // children's margins. The auto height includes everything,
    // including the first child's top margin (no escape upward
    // because the BFC blocks it).
    //
    //   parent: overflow: hidden, height: auto
    //   a.mt=3, a.h=2
    //   total height = 3 (top margin) + 2 (a.h) = 5.
    use crate::layout::Overflow;
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    dom.append_child(parent, a).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().overflow(Overflow::Hidden))
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(3),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 50));

    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        3 + 2,
        "BFC parent traps child's top margin → height includes it"
    );
}

// ── Phase 7: extended coverage (WPT-style scenarios) ─────────────

// ── 7a — Margin collapse edge cases ──────────────────────────────

#[test]
fn parent_bottom_margin_collapses_with_last_child() {
    // Symmetric to the parent-first-child top collapse: when the
    // parent has no bottom padding/border and doesn't establish a
    // new BFC, the last in-flow block child's `margin-bottom`
    // collapses through the parent. The visible consequence: the
    // parent's effective outer bottom margin includes the child's.
    let mut dom = dom();
    let root = dom.root();
    let gp = dom.create_element("gp");
    let p = dom.create_element("p");
    let c = dom.create_element("c");
    let next = dom.create_element("n");
    dom.append_child(p, c).unwrap();
    dom.append_child(gp, p).unwrap();
    dom.append_child(gp, next).unwrap();
    dom.append_child(root, gp).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(7),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked("n", TuiStyle::new().height(Size::Fixed(1)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, gp, LayoutRect::new(0, 0, 80, 24));

    // p height includes c's height but NOT c's margin-bottom
    // (which escaped upward via P's outer bottom collapse).
    // After P: gap = max(P.mb=3, C.mb=7) = 7. N at y = P.bottom (2) + 7 = 9.
    assert_eq!(
        layout_of(&dom, next).y,
        2 + 7,
        "N.y = max(P.mb=3, C.mb=7) above gp.y_cursor after P"
    );
}

#[test]
fn display_none_sibling_does_not_break_margin_adjacency() {
    use crate::layout::Display;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let h = dom.create_element("h");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, h).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(4),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked("h", TuiStyle::new().display(Display::None))
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(1)).margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    // display:none child is filtered out before run partitioning,
    // so A.mb and B.mt collapse as if H weren't there.
    assert_eq!(layout_of(&dom, b).y, 1 + 4);
}

#[test]
fn negative_top_with_positive_bottom_partially_cancels() {
    // A.mb=+5, B.mt=-2 → mixed → 5 + -2 = 3.
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
                MarginValue::Cells(5),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(-2),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, b).y, 2 + (5 - 2));
}

#[test]
fn parent_with_only_empty_collapse_through_children_has_zero_content() {
    // All children are empty-collapse-through. Their margins fold
    // together; with no real content, the parent's content height
    // is 0 (and the merged margin escapes upward).
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let e1 = dom.create_element("e1");
    let e2 = dom.create_element("e2");
    dom.append_child(parent, e1).unwrap();
    dom.append_child(parent, e2).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "e1",
            TuiStyle::new().margin(Margin::new(
                MarginValue::Cells(2),
                MarginValue::Cells(0),
                MarginValue::Cells(3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "e2",
            TuiStyle::new().margin(Margin::new(
                MarginValue::Cells(1),
                MarginValue::Cells(0),
                MarginValue::Cells(4),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    // Parent has no real content; all margins collapse through.
    // Phase 6.1 auto-height should resolve to 0 (no content
    // extent) — margins escaped upward.
    assert_eq!(
        dom.node(parent).ext().unwrap().layout.height,
        0,
        "all-empty parent collapses to 0 content height"
    );
}

// ── 7b — Width edge cases ────────────────────────────────────────

#[test]
fn min_width_floors_under_auto_margin_distribution() {
    // CSS 2.1 §10.3.3 + §10.7: min-width clamp wins even when
    // auto margins would have given the box less.
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
            .min_width(MinSize::Cells(60))
            .margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Auto,
                MarginValue::Cells(0),
                MarginValue::Auto,
            )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    assert_eq!(layout_of(&dom, child).width, 60, "min-width clamp wins");
}

#[test]
fn max_width_caps_before_auto_margins() {
    // Auto margins distribute the LEFTOVER. With width: auto, the
    // child would normally fill the container (80). With max-width:
    // 30, the box caps at 30 and auto margins fight over 50 cells
    // leftover → 25 each (LTR — odd leftover, but 50 is even).
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new().max_width(30).margin(Margin::new(
            MarginValue::Cells(0),
            MarginValue::Auto,
            MarginValue::Cells(0),
            MarginValue::Auto,
        )),
    );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    let r = layout_of(&dom, child);
    assert_eq!(r.width, 30, "max-width caps");
    assert_eq!(r.x, 25, "leftover (50) splits 25/25");
}

#[test]
fn width_percent_resolves_against_definite_containing_block() {
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let child = dom.create_element("c");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().width(Size::Fixed(40)))
        .rule_unchecked("c", TuiStyle::new().width(Size::Percent(25)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 40, 24));

    assert_eq!(layout_of(&dom, child).width, 10, "25% of 40 = 10");
}

// ── 7c — Height resolution chains ────────────────────────────────

#[test]
fn percent_height_chains_through_three_definite_levels() {
    // GP (Fixed 40) -> P (50% = 20) -> C (50% = 10)
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let gp = dom.create_element("gp");
    let p = dom.create_element("p");
    let c = dom.create_element("c");
    dom.append_child(p, c).unwrap();
    dom.append_child(gp, p).unwrap();
    dom.append_child(root, gp).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("gp", TuiStyle::new().height(Size::Fixed(40)))
        .rule_unchecked("p", TuiStyle::new().height(Size::Percent(50)))
        .rule_unchecked("c", TuiStyle::new().height(Size::Percent(50)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 60));

    assert_eq!(dom.node(p).ext().unwrap().layout.height, 20);
    assert_eq!(dom.node(c).ext().unwrap().layout.height, 10);
}

#[test]
fn percent_height_breaks_chain_at_auto_ancestor() {
    // GP (Fixed 40) -> P (Auto) -> C (50%)
    // Chain breaks at P (Auto). C's percent falls back to intrinsic.
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let gp = dom.create_element("gp");
    let p = dom.create_element("p");
    let c = dom.create_element("c");
    let inner = dom.create_element("i");
    dom.append_child(c, inner).unwrap();
    dom.append_child(p, c).unwrap();
    dom.append_child(gp, p).unwrap();
    dom.append_child(root, gp).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("gp", TuiStyle::new().height(Size::Fixed(40)))
        // P is Auto — breaks the chain.
        .rule_unchecked("c", TuiStyle::new().height(Size::Percent(50)))
        .rule_unchecked("i", TuiStyle::new().height(Size::Fixed(3)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 60));

    assert_eq!(
        dom.node(c).ext().unwrap().layout.height,
        3,
        "percent height falls to intrinsic (inner fixed 3) because parent P is Auto"
    );
}

// ── 7d — Nested formatting contexts ──────────────────────────────

#[test]
fn block_inside_flex_inside_block() {
    // Outer is block; middle is flex column with one child; that
    // child is a block with two children. Each formatting context
    // applies its own rules without leaking into siblings.
    use crate::render::Rect;
    let mut dom = dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let middle = dom.create_element("middle");
    let inner = dom.create_element("inner");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(inner, a).unwrap();
    dom.append_child(inner, b).unwrap();
    dom.append_child(middle, inner).unwrap();
    dom.append_child(outer, middle).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        // outer is block-flow (default)
        .rule_unchecked(
            "middle",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .height(Size::Fixed(20)),
        )
        // inner: block-flow under flex parent. Its own block children
        // stack. inner.height = stretch (cross axis is row; main is
        // column → main = inner's natural).
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(5)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(7)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 60));

    let a_rect = dom.node(a).ext().unwrap().layout;
    let b_rect = dom.node(b).ext().unwrap().layout;
    // Block children stack inside inner.
    assert_eq!(a_rect.y, 0, "a at inner's content top");
    assert_eq!(b_rect.y, 5, "b stacks below a");
    assert_eq!(a_rect.height, 5);
    assert_eq!(b_rect.height, 7);
}

// ── 7e — Positioned / relative interactions ──────────────────────

#[test]
fn relative_positioned_child_does_not_shift_subsequent_block_siblings() {
    // CSS 2.1 §9.4.3: relative positioning offsets the element
    // visually but DOES NOT remove it from flow — subsequent
    // siblings lay out as if the relative child was at its
    // original position.
    use crate::layout::{Length, Position};
    use crate::render::Rect;
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
            TuiStyle::new()
                .height(Size::Fixed(3))
                .position(Position::Relative)
                .top(Length::Cells(5)),
        )
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(2)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 50));

    // b sits at A's natural bottom (a.height = 3, b.y = 3), even
    // though a was visually shifted to y=5.
    assert_eq!(
        dom.node(b).ext().unwrap().layout.y,
        3,
        "b's position is independent of A's relative shift"
    );
}

// ── 7f — Gap interactions ────────────────────────────────────────

#[test]
fn block_gap_adds_to_collapsed_margins() {
    // CSS3 Box Alignment + CSS 2.1: row-gap stacks ON TOP of the
    // collapsed margin between block siblings. They don't merge.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().gap(2))
        .rule_unchecked(
            "a",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(3),
                MarginValue::Cells(0),
            )),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().height(Size::Fixed(2)).margin(Margin::new(
                MarginValue::Cells(1),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
                MarginValue::Cells(0),
            )),
        );
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    // a.bottom = 2. collapsed margin = max(3, 1) = 3. + gap = 2.
    // b.y = 2 + 3 + 2 = 7.
    assert_eq!(layout_of(&dom, b).y, 2 + 3 + 2);
}

#[test]
fn block_gap_not_applied_before_first_or_after_last() {
    // Gap is BETWEEN adjacent siblings, not around the outside.
    let mut dom = dom();
    let root = dom.root();
    let parent = dom.create_element("p");
    let a = dom.create_element("a");
    dom.append_child(parent, a).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().gap(5))
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(2)));
    cascade(&mut dom, &sheet);
    run_block(&mut dom, parent, LayoutRect::new(0, 0, 80, 24));

    // Single child: no gap. y=0.
    assert_eq!(layout_of(&dom, a).y, 0);
}
