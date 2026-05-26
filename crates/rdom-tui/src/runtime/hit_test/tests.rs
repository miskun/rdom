//! `HitTestExt` tests.
//!
//! Helper `prepared(template, sheet)` runs cascade + layout at a
//! fixed viewport so assertions fire against a fully-realized
//! layout. All tests build the tree imperatively; no parser
//! dependency.

use super::*;
use crate::TuiDom;
use crate::layout::{Border, Direction, Display, Flow, Overflow, Padding, Size};
use crate::render::{LayoutExt, Rect};
use crate::style::{CascadeExt, Color, Stylesheet, TuiStyle};
use rdom_core::NodeId;

fn prepare(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
}

// ── Basic containment ───────────────────────────────────────────────

#[test]
fn point_on_single_block_returns_it() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.hit_test(5, 1), Some(div));
}

#[test]
fn point_outside_viewport_returns_none() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(2)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Well past the div's painted area.
    assert_eq!(dom.hit_test(50, 50), None);
}

#[test]
fn empty_tree_returns_none() {
    let mut dom: TuiDom = TuiDom::new();
    prepare(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 20, 10));
    assert_eq!(dom.hit_test(5, 5), None);
}

// ── Nested blocks — deepest wins ────────────────────────────────────

#[test]
fn deepest_block_wins_on_nesting() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .padding(Padding::all(1)),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Point at (3, 1) sits inside outer's content area AND inside inner.
    assert_eq!(dom.hit_test(3, 1), Some(inner));
}

// ── Paint-order stacking ────────────────────────────────────────────

#[test]
fn paint_order_stacking_later_sibling_wins() {
    // Two absolutely-identical siblings at the same layout rect.
    // (Not achievable in flex without hacks, so use a Fixed container
    // and overlapping Fixed children via scroll — easier: stack two
    // siblings with explicit equal width inside a Column parent and
    // test that the second one is on top. Flex with column means
    // siblings stack vertically; to overlap, I scroll so they
    // both land on the same y.)
    //
    // Simpler: direct approach via nested layout with Visible
    // overflow. Child A has its bounds inside the parent; sibling B
    // also overlaps A. With reverse-document-order descent, B wins.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    // Column layout: a at y=0 h=3, b at y=3 h=3. They don't overlap
    // naturally. Let's just confirm paint order via descent: both are
    // reachable but the reverse-doc-order rule means if a point falls
    // in both, b wins. We can't easily overlap flex siblings without
    // scroll; test the invariant differently via a parent with both
    // children at Fixed positions in a Row direction and equal size
    // but layout_rect conflict forced via scroll_x:
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .gap(0)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Point (2, 1): inside a's rect (0..5). a wins (the only option).
    assert_eq!(dom.hit_test(2, 1), Some(a));
    // Point (7, 1): inside b's rect (5..10).
    assert_eq!(dom.hit_test(7, 1), Some(b));

    // For actual stacking test: force a and b to overlap via scroll on
    // parent. Set parent scroll_x=-5 then re-layout so b shifts left
    // and overlaps a. (scroll is subtracted from main cursor, so
    // positive scroll moves children left; negative moves right.)
    dom.node_mut(parent).ext_mut().unwrap().scroll_x = 0;
    // Actually — the cleanest stacking proof uses overflow:visible with
    // fixed-positioned overlap. Hand-force b's layout rect to overlap a's:
    dom.layout_dom(Rect::new(0, 0, 20, 10)); // fresh layout
    // Now cheat: directly overwrite b.layout to overlap a.
    let overlap = dom.node(a).layout_rect().unwrap();
    dom.node_mut(b).ext_mut().unwrap().layout = overlap;

    // (2, 1) is now inside BOTH a and b (b is later in doc order).
    // Reverse-doc-order descent picks b first.
    assert_eq!(dom.hit_test(2, 1), Some(b));
}

// ── Overflow clipping ───────────────────────────────────────────────

#[test]
fn overflow_hidden_clips_children_hit_area() {
    // Parent with padding + overflow:hidden. A child that extends
    // into the padding area shouldn't be hittable there — the hit
    // stays on the parent.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .padding(Padding::all(1))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // (0, 0) is in parent's padding (outer rect) but not content.
    // Overflow:Hidden means hit stays on parent.
    assert_eq!(dom.hit_test(0, 0), Some(parent));
    // (2, 1) is inside the content area AND inside child.
    assert_eq!(dom.hit_test(2, 1), Some(child));
}

#[test]
fn overflow_visible_allows_child_hit_past_parent_rect() {
    // With overflow:Visible, a child whose layout rect happens to
    // extend beyond the parent is still hittable at those positions
    // — matches CSS.
    //
    // In our flex model children are laid out strictly within the
    // content area, so this test exercises a *forced* overlap: we
    // override child.layout to sit past parent's right edge.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 30, 10));

    // Force child to sit partly past parent.
    let mut rect = dom.node(child).layout_rect().unwrap();
    rect.x = 3;
    rect.width = 8; // extends to x=11, past parent's x=5 edge
    dom.node_mut(child).ext_mut().unwrap().layout = rect;

    // Point (1, 1): inside parent, not inside child.
    assert_eq!(dom.hit_test(1, 1), Some(parent));
    // Point (4, 1): inside BOTH parent and child.
    assert_eq!(dom.hit_test(4, 1), Some(child));
    // Point (9, 1): outside parent's rect but inside child's. With
    // overflow:Visible on the parent, the paint paints the child here,
    // so hit test must match.
    //
    // BUT: descent enters the parent only if the point is in the
    // parent's outer rect. (9, 1) is outside parent → we never
    // descend into child. This is CSS-divergent: spec says
    // overflow:visible allows child hit, but in rdom-tui v1 the
    // descent is gated on parent containment. This is a documented
    // limitation; app-level elements that need "child escapes parent"
    // can use z-index + absolute layout (not shipping v1).
    //
    // For v1 we assert the *current* behavior: miss.
    assert_eq!(dom.hit_test(9, 1), None);
}

// ── IFC fragment lookup ─────────────────────────────────────────────

#[test]
fn ifc_block_returns_fragment_owner() {
    // <p>text <code>X</code></p>
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("ab ");
    let code = dom.create_element("code");
    let ct = dom.create_text_node("X");
    dom.append_child(code, ct).unwrap();
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, code).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("code", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Fragments on row 0: "ab " (x 0..3) owned by p, "X" (x 3..4)
    // owned by code.
    // (0, 0) → p owns "ab ".
    let path = dom.hit_test_path(0, 0);
    assert_eq!(path.last().copied(), Some(p));
    // (3, 0) → code owns "X".
    let path = dom.hit_test_path(3, 0);
    assert_eq!(path.last().copied(), Some(code));
    assert!(
        path.contains(&p),
        "path must include p (IFC block) as ancestor"
    );
}

#[test]
fn ifc_click_in_padding_returns_ifc_block_not_fragment() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("X");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap(); // trigger IFC
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .padding(Padding::all(1))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // (0, 0) is in p's outer rect but NOT its content area.
    // Overflow:Hidden + outside content → hit stays on p itself
    // (IFC fragment lookup is inside content only, and content-gate
    // stops descent).
    assert_eq!(dom.hit_test(0, 0), Some(p));
}

#[test]
fn ifc_wrapped_inline_hittable_on_any_line() {
    // <p> with an inline element whose text wraps across lines.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("aaa bbb ccc");
    dom.append_child(b, bt).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(4)),
        )
        .rule_unchecked("b", TuiStyle::new().display(Display::Inline).bold(true));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Width 4 forces wrap: "aaa" (line 0), "bbb" (line 1), "ccc" (line 2).
    // Cell (0, 0) → b. Cell (0, 1) → b on second line. Cell (0, 2) → b on third.
    for y in 0..3 {
        let path = dom.hit_test_path(0, y);
        let last = path.last().copied();
        assert_eq!(last, Some(b), "wrapped <b> must be hittable on line {y}");
        assert!(path.contains(&p));
    }
}

// ── Path structure ──────────────────────────────────────────────────

#[test]
fn path_includes_full_ancestor_chain() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(b, c).unwrap();
    dom.append_child(a, b).unwrap();
    dom.append_child(root, a).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5)),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new().width(Size::Fixed(8)).height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    let path = dom.hit_test_path(1, 0);
    // Order: outermost → innermost = [a, b, c]. Root Fragment never
    // appears (it has no layout rect of its own).
    assert_eq!(path, vec![a, b, c]);
}

#[test]
fn path_empty_when_nothing_hit() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new().width(Size::Fixed(3)).height(Size::Fixed(1)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.hit_test_path(50, 50), Vec::<NodeId>::new());
}

// ── Interaction with paint-order stacking via reverse doc order ─────

#[test]
fn reverse_document_order_descent_picks_last_sibling() {
    // Build two siblings whose layout rects are forced to the same
    // area. The *second* (last document-order) must win.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let first = dom.create_element("first");
    let second = dom.create_element("second");
    dom.append_child(root, first).unwrap();
    dom.append_child(root, second).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "first",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(2)),
        )
        .rule_unchecked(
            "second",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(2)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Force second to overlap first exactly.
    let r = dom.node(first).layout_rect().unwrap();
    dom.node_mut(second).ext_mut().unwrap().layout = r;

    // Inside both — second wins (reverse-document-order descent).
    assert_eq!(dom.hit_test(2, 1), Some(second));
}

// ── Border + padding are part of the element (HTML semantics) ───────

#[test]
fn hit_on_border_returns_the_element() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(6))
            .height(Size::Fixed(4))
            .border(Border::single())
            .fg(Color::Rgb(255, 255, 255)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // (0, 0) sits on the top-left border corner.
    assert_eq!(dom.hit_test(0, 0), Some(div));
}

// ── position_at (Phase 6.5.2 prep) ──────────────────────────────────

use crate::layout::UserSelect;
use rdom_core::Position;

#[test]
fn position_at_outside_viewport_returns_none() {
    let mut dom: TuiDom = TuiDom::new();
    prepare(&mut dom, &Stylesheet::bare(), Rect::new(0, 0, 20, 10));
    assert_eq!(dom.position_at(99, 99), None);
}

#[test]
fn position_at_inside_text_fragment_returns_source_position() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    // Need a second inline child to trigger IFC detection.
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Cell 0 of "hello" → byte 0 of text node `t`.
    assert_eq!(dom.position_at(0, 0), Some(Position::new(t, 0)));
    // Cell 2 of "hello" → byte 2 (the 'l').
    assert_eq!(dom.position_at(2, 0), Some(Position::new(t, 2)));
    // Cell 4 of "hello" → byte 4 (the second 'l' finished, 'o' next).
    assert_eq!(dom.position_at(4, 0), Some(Position::new(t, 4)));
}

#[test]
fn position_at_across_inline_element_boundary_uses_text_node_of_fragment() {
    // <p>ab<code>XY</code>cd</p>: fragments are
    //   "ab" (text_node=t1, offset 0),
    //   "XY" (text_node=t2, offset 0),
    //   "cd" (text_node=t3, offset 0).
    // Hit on each zone should return the correct text_node.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("ab");
    dom.append_child(p, t1).unwrap();
    let code = dom.create_element("code");
    let t2 = dom.create_text_node("XY");
    dom.append_child(code, t2).unwrap();
    dom.append_child(p, code).unwrap();
    let t3 = dom.create_text_node("cd");
    dom.append_child(p, t3).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("code", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // Row 0: "abXYcd". Cells 0-1 → t1; cells 2-3 → t2; cells 4-5 → t3.
    assert_eq!(dom.position_at(0, 0), Some(Position::new(t1, 0)));
    assert_eq!(dom.position_at(1, 0), Some(Position::new(t1, 1)));
    assert_eq!(dom.position_at(2, 0), Some(Position::new(t2, 0)));
    assert_eq!(dom.position_at(3, 0), Some(Position::new(t2, 1)));
    assert_eq!(dom.position_at(4, 0), Some(Position::new(t3, 0)));
    assert_eq!(dom.position_at(5, 0), Some(Position::new(t3, 1)));
}

#[test]
fn position_at_on_cjk_grapheme_snaps_to_start() {
    // CJK fragments are 2 cells wide. Clicking on either cell
    // should return the same byte offset (start of the grapheme).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("中文");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // "中" is 3 UTF-8 bytes at offset 0. Cells 0 and 1 both fall
    // on "中" — the cells_to_bytes walker consumes the full
    // 2-cell grapheme before moving on.
    assert_eq!(dom.position_at(0, 0), Some(Position::new(t, 0)));
    assert_eq!(dom.position_at(1, 0), Some(Position::new(t, 0)));
    // Cell 2 lands on "文" (starts at byte 3).
    assert_eq!(dom.position_at(2, 0), Some(Position::new(t, 3)));
    assert_eq!(dom.position_at(3, 0), Some(Position::new(t, 3)));
}

#[test]
fn position_at_respects_user_select_none() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hi");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .user_select(UserSelect::None),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // user-select: none on the IFC block suppresses all positions
    // inside it.
    assert_eq!(dom.position_at(0, 0), None);
    assert_eq!(dom.position_at(1, 0), None);
}

#[test]
fn position_at_user_select_none_inherits_to_subtree() {
    // user-select inherits; a child inside a user-select: none
    // parent is also unselectable even without its own declaration.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let wrapper = dom.create_element("wrapper");
    let p = dom.create_element("p");
    let t = dom.create_text_node("hi");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(wrapper, p).unwrap();
    dom.append_child(root, wrapper).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("wrapper", TuiStyle::new().user_select(UserSelect::None))
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // `p` doesn't declare user_select but inherits None from
    // wrapper.
    assert_eq!(dom.position_at(0, 0), None);
}

// ── M2 §12.9-12.10 — Hit-test reverse z-order ────────────────────

#[test]
fn higher_z_index_catches_click_first() {
    // High-z element placed FIRST in the document so the reverse-
    // document-order fallback can't pick it up by accident — only
    // proper z-list logic returns it.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let hi = dom.create_element("hi");
    let lo = dom.create_element("lo");
    dom.append_child(root, hi).unwrap();
    dom.append_child(root, lo).unwrap();

    let base = || {
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .left(crate::layout::Length::Cells(0))
            .width(Size::Fixed(5))
            .height(Size::Fixed(2))
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked("hi", base().z_index(crate::layout::ZIndex::Value(5)))
        .rule_unchecked("lo", base().z_index(crate::layout::ZIndex::Value(1)));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.hit_test(2, 0), Some(hi));
}

#[test]
fn positioned_catches_click_over_in_flow_content() {
    // In-flow `bar` lives under the absolutely-positioned
    // `tooltip`. Click in the overlap → tooltip wins.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let tip = dom.create_element("tip");
    let bar = dom.create_element("bar");
    dom.append_child(root, tip).unwrap();
    dom.append_child(root, bar).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "bar",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "tip",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(5))
                .height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.hit_test(2, 0), Some(tip));
    assert_eq!(dom.hit_test(10, 0), Some(bar));
}

#[test]
fn z_list_among_positioned_uses_doc_order_for_auto() {
    // Two z-index:auto positioned siblings — later in document
    // wins (matches paint order, last paint sits on top).
    let mut dom: TuiDom = TuiDom::new();
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
            .width(Size::Fixed(5))
            .height(Size::Fixed(2))
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked("a", base())
        .rule_unchecked("b", base());
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.hit_test(2, 0), Some(b));
}

#[test]
fn click_outside_positioned_falls_through_to_in_flow() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let tip = dom.create_element("tip");
    let bar = dom.create_element("bar");
    dom.append_child(root, tip).unwrap();
    dom.append_child(root, bar).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "bar",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "tip",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(2))
                .left(crate::layout::Length::Cells(10))
                .width(Size::Fixed(3))
                .height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.hit_test(5, 1), Some(bar));
}

#[test]
fn position_at_in_non_ifc_returns_none() {
    // Pure block layout without inline children — no IFC, no
    // fragments, no position.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    assert_eq!(dom.position_at(3, 1), None);
}
