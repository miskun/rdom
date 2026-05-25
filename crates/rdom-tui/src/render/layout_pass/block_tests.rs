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
