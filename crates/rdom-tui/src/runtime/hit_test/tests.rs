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
fn position_at_empty_space_in_user_select_none_does_not_snap_out() {
    // rdom-virtualtable repro: clicking the EMPTY area of a `user-select: none`
    // flex container (e.g. a table row's trailing space, past the last cell)
    // must yield no caret — NOT snap to the nearest *selectable* text elsewhere
    // (the page title), which would start a text-selection drag and hijack the
    // grid. The snap fallback must not escalate out of a user-select:none region.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    // A user-select:none flex row with one short cell; cols 5..20 are empty.
    let row = dom.create_element("row");
    let cell = dom.create_element("cell");
    let ct = dom.create_text_node("x");
    dom.append_child(cell, ct).unwrap();
    let cspan = dom.create_element("span");
    dom.append_child(cell, cspan).unwrap();
    dom.append_child(row, cell).unwrap();
    dom.append_child(root, row).unwrap();
    // A separate SELECTABLE block below it (the "title").
    let title = dom.create_element("p");
    let tt = dom.create_text_node("title");
    dom.append_child(title, tt).unwrap();
    let tspan = dom.create_element("span");
    dom.append_child(title, tspan).unwrap();
    dom.append_child(root, title).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "row",
            TuiStyle::new()
                .display(Display::Block)
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1))
                .user_select(UserSelect::None),
        )
        .rule_unchecked(
            "cell",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(5)),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // (10, 0) is the row's empty trailing space — inside the user-select:none
    // row, past its only cell. No caret; must NOT resolve to "title".
    assert_eq!(dom.position_at(10, 0), None);
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
    // document-order walk can't pick it up by accident — only the
    // positioned layer's z order returns it.
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
fn positioned_layer_uses_doc_order_for_auto() {
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

// ── Empty-space snap to nearest inline flow (DRAG-AUTOSCROLL) ────────

/// A point in empty space BELOW every IFC block must snap to the nearest
/// text position — the END of the last block — not return `None`. The
/// `None` made drag-select past the bottom edge (and autoscroll held
/// past the edge) collapse back to the anchor block: the `selectable_text`
/// demo bug where dragging one cell below the last line dropped every
/// block after the anchor's. Browsers snap `caretPositionFromPoint` to
/// the closest text; so does rdom.
#[test]
fn position_at_below_all_blocks_snaps_to_last_block_end() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p1 = dom.create_element("p");
    let t1 = dom.create_text_node("alpha");
    dom.append_child(p1, t1).unwrap();
    let s1 = dom.create_element("span"); // 2nd inline child → IFC
    dom.append_child(p1, s1).unwrap();
    dom.append_child(root, p1).unwrap();
    let p2 = dom.create_element("p");
    let t2 = dom.create_text_node("omega");
    dom.append_child(p2, t2).unwrap();
    let s2 = dom.create_element("span");
    dom.append_child(p2, s2).unwrap();
    dom.append_child(root, p2).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // p1 at row 0, p2 at row 1. A point at row 5 is empty space below
    // both → snaps to the end of p2's "omega" (5 bytes).
    assert_eq!(dom.position_at(3, 5), Some(Position::new(t2, 5)));
}

/// A point ABOVE every block snaps to the first block's start.
#[test]
fn position_at_above_all_blocks_snaps_to_first_block_start() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let spacer = dom.create_element("div"); // pushes the prose down a row
    dom.append_child(root, spacer).unwrap();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let s = dom.create_element("span");
    dom.append_child(p, s).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("div", TuiStyle::new().height(Size::Fixed(3)))
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));

    // The prose sits at row 3 (after the 3-row spacer). A point at the
    // spacer (row 1, above the prose) snaps to the start of "hello".
    assert_eq!(dom.position_at(2, 1), Some(Position::new(t, 0)));
}

// ── pointer-events (HARDENING-2026-09, FOLLOW-WEB) ──────────────────

/// `pointer-events: none` makes an element (and, by inheritance, its
/// subtree) invisible to hit-testing: the point falls through to
/// whatever is beneath. A descendant that sets `pointer-events: auto`
/// is a target again.
#[test]
fn pointer_events_none_falls_through_and_auto_child_is_hittable() {
    use crate::layout::PointerEvents;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let base = dom.create_element("base");
    let overlay = dom.create_element("overlay");
    let button = dom.create_element("button");
    dom.append_child(root, base).unwrap();
    dom.append_child(root, overlay).unwrap();
    dom.append_child(overlay, button).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "base",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(20))
                .height(Size::Fixed(6)),
        )
        .rule_unchecked(
            "overlay",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(20))
                .height(Size::Fixed(6))
                .pointer_events(PointerEvents::None),
        )
        .rule_unchecked(
            "button",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(4))
                .left(crate::layout::Length::Cells(10))
                .width(Size::Fixed(5))
                .height(Size::Fixed(1))
                .pointer_events(PointerEvents::Auto),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 30, 10));
    assert_eq!(
        dom.hit_test(2, 1),
        Some(base),
        "overlay is transparent to the pointer"
    );
    assert_eq!(
        dom.hit_test(12, 4),
        Some(button),
        "auto child inside a none parent is hit"
    );
}

/// A scrolled IFC block resolves fragment owners through the scrolled
/// content rect, like paint and the caret do: after scrolling two lines,
/// the link on the third line is what sits on the block's first row.
#[test]
fn scrolled_ifc_block_hits_the_fragment_owner_visible_on_that_row() {
    use crate::layout::WhiteSpace;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let a1 = dom.create_element("a");
    let a2 = dom.create_element("a");
    let t1 = dom.create_text_node("first");
    let gap = dom.create_text_node("\n\n");
    let t2 = dom.create_text_node("third");
    dom.append_child(a1, t1).unwrap();
    dom.append_child(a2, t2).unwrap();
    dom.append_child(p, a1).unwrap();
    dom.append_child(p, gap).unwrap();
    dom.append_child(p, a2).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .width(Size::Fixed(20))
                // One visible row of three lines, so a scroll of two
                // rows is within the clamp and puts the third line on top.
                .height(Size::Fixed(1))
                .white_space(WhiteSpace::Pre)
                .overflow_y(Overflow::Scroll)
                .padding(Padding::all(0))
                .border(Border::none()),
        )
        .rule_unchecked("a", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 30, 10));
    assert_eq!(
        dom.hit_test(1, 0),
        Some(a1),
        "unscrolled: first link on row 0"
    );
    dom.node_mut(p).ext_mut().unwrap().scroll_y = 2;
    dom.layout_dom(Rect::new(0, 0, 30, 10));
    assert_eq!(
        dom.hit_test(1, 0),
        Some(a2),
        "scrolled by two: the third line's link is on row 0"
    );
}

// ── Stacking contexts (CSS 2.1 Appendix E; D-M2-3 / D-M2-4) ────────

fn abs_box() -> TuiStyle {
    TuiStyle::new()
        .position(crate::layout::Position::Absolute)
        .top(crate::layout::Length::Cells(0))
        .left(crate::layout::Length::Cells(0))
        .width(Size::Fixed(3))
        .height(Size::Fixed(1))
}

/// `D-M2-3`: the click goes to the higher sibling context, not to the
/// `z: 100` descendant of the lower one.
#[test]
fn nested_context_click_goes_to_the_higher_sibling_context() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let a1 = dom.create_element("a1");
    let b = dom.create_element("b");
    dom.append_child(a, a1).unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked("a", abs_box().z_index(crate::layout::ZIndex::Value(1)))
        .rule_unchecked("a1", abs_box().z_index(crate::layout::ZIndex::Value(100)))
        .rule_unchecked("b", abs_box().z_index(crate::layout::ZIndex::Value(2)));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(dom.hit_test(1, 0), Some(b));
}

/// `D-M2-4`: a negative `z-index` sits under the context's in-flow
/// content but over the context's own box.
#[test]
fn negative_z_click_falls_to_in_flow_content_above_it() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let wrap = dom.create_element("wrap");
    let c = dom.create_element("c");
    let n = dom.create_element("n");
    dom.append_child(wrap, c).unwrap();
    dom.append_child(wrap, n).unwrap();
    dom.append_child(root, wrap).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "wrap",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .z_index(crate::layout::ZIndex::Value(0))
                .width(Size::Fixed(10))
                .height(Size::Fixed(2)),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "n",
            abs_box()
                .width(Size::Fixed(10))
                .z_index(crate::layout::ZIndex::Value(-1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(dom.hit_test(1, 0), Some(c), "in-flow content above z:-1");
    assert_eq!(
        dom.hit_test(7, 0),
        Some(n),
        "z:-1 above the context's own box"
    );
}

/// Appendix E layer 6: a `position: relative` box catches the click
/// over a later in-flow sibling it overlaps.
#[test]
fn relative_element_catches_click_over_a_later_sibling() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let wrap = dom.create_element("wrap");
    let r = dom.create_element("r");
    let s = dom.create_element("s");
    dom.append_child(wrap, r).unwrap();
    dom.append_child(wrap, s).unwrap();
    dom.append_child(root, wrap).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked("wrap", TuiStyle::new().width(Size::Fixed(10)))
        .rule_unchecked(
            "r",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .top(crate::layout::Length::Cells(1))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("s", TuiStyle::new().height(Size::Fixed(1)));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(dom.hit_test(1, 1), Some(r));
}

/// CSS 2.1 §11.1.1: an absolutely positioned box clipped by a scroll
/// container above its containing block is not hittable there.
#[test]
fn absolute_clipped_by_a_scroll_container_is_not_hittable_outside_it() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("list");
    let item = dom.create_element("item");
    let pop = dom.create_element("pop");
    dom.append_child(item, pop).unwrap();
    dom.append_child(list, item).unwrap();
    dom.append_child(root, list).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "list",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(2))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked(
            "item",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("pop", abs_box().top(crate::layout::Length::Cells(4)));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 10, 8));
    assert_eq!(dom.hit_test(0, 4), None);
}

// ── Phase 5 architect gate: the path keeps the ancestor chain ───────

/// B2: a hit inside a positioned box still reports the full ancestor
/// chain, root-most first — `relative` / `sticky` boxes and absolute
/// boxes alike (`elements_from_point` documents the chain).
#[test]
fn path_through_a_positioned_box_keeps_its_ancestors() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pane = dom.create_element("pane");
    let rel = dom.create_element("rel");
    let button = dom.create_element("button");
    let abs = dom.create_element("abs");
    dom.append_child(rel, button).unwrap();
    dom.append_child(rel, abs).unwrap();
    dom.append_child(pane, rel).unwrap();
    dom.append_child(root, pane).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "pane",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5))
                .overflow(Overflow::Auto),
        )
        .rule_unchecked(
            "rel",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "button",
            TuiStyle::new().width(Size::Fixed(4)).height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "abs",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(2))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(3))
                .height(Size::Fixed(1)),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));
    assert_eq!(dom.hit_test_path(1, 0), vec![pane, rel, button]);
    assert_eq!(dom.hit_test_path(1, 2), vec![pane, rel, abs]);
}

/// B2, nested contexts: the context root and the ancestors between it
/// and the hit both appear.
#[test]
fn path_through_nested_stacking_contexts_keeps_every_root() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let mid = dom.create_element("mid");
    let a1 = dom.create_element("a1");
    dom.append_child(mid, a1).unwrap();
    dom.append_child(a, mid).unwrap();
    dom.append_child(root, a).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked("a", abs_box().z_index(crate::layout::ZIndex::Value(1)))
        .rule_unchecked("mid", TuiStyle::new().height(Size::Fixed(1)))
        .rule_unchecked("a1", abs_box().z_index(crate::layout::ZIndex::Value(2)));
    prepare(&mut dom, &sheet, Rect::new(0, 0, 10, 5));
    assert_eq!(dom.hit_test_path(1, 0), vec![a, mid, a1]);
}

/// A stacking-context root with `pointer-events: none` is never on the
/// path, but its `auto` content is still hit, and its bare area falls
/// through to what lies beneath.
#[test]
fn transparent_context_root_falls_through_but_its_content_is_hittable() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let under = dom.create_element("under");
    let ctx = dom.create_element("ctx");
    let child = dom.create_element("child");
    dom.append_child(ctx, child).unwrap();
    dom.append_child(root, under).unwrap();
    dom.append_child(root, ctx).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "under",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "ctx",
            abs_box()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .z_index(crate::layout::ZIndex::Value(1))
                .pointer_events(crate::layout::PointerEvents::None),
        )
        .rule_unchecked(
            "child",
            TuiStyle::new()
                .width(Size::Fixed(3))
                .height(Size::Fixed(1))
                .pointer_events(crate::layout::PointerEvents::Auto),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 10));
    assert_eq!(
        dom.hit_test_path(1, 0),
        vec![child],
        "content of a transparent root"
    );
    assert_eq!(
        dom.hit_test_path(8, 2),
        vec![under],
        "bare area falls through"
    );
}

// ── POINTER-EVENTS-IFC-1: pointer-events inside inline content ──────

/// `pointer-events: none` on an inline formatting context's block
/// hides the block, not its `auto` inline descendants: a click on
/// `<b>` inside a transparent `<p>` hits `<b>` (without the `<p>` on
/// the path); a click on the `<p>`'s own text falls through.
#[test]
fn transparent_ifc_block_still_exposes_its_auto_inline_descendants() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let under = dom.create_element("under");
    let p = dom.create_element("p");
    let lead = dom.create_text_node("aaaa ");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("bbbb");
    dom.append_child(b, bt).unwrap();
    dom.append_child(p, lead).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(root, under).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "under",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0))
                .width(Size::Fixed(20))
                .height(Size::Fixed(1))
                .pointer_events(crate::layout::PointerEvents::None),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .display(Display::Inline)
                .pointer_events(crate::layout::PointerEvents::Auto),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));
    assert_eq!(
        dom.hit_test_path(6, 0),
        vec![b],
        "the auto inline is hit, the block is not on the path"
    );
    assert_eq!(
        dom.hit_test_path(1, 0),
        vec![under],
        "the block's own text falls through"
    );
}

/// The reverse: an inline with `pointer-events: none` inside an `auto`
/// block resolves to the block (the nearest non-transparent ancestor).
#[test]
fn transparent_inline_resolves_to_its_nearest_auto_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let lead = dom.create_text_node("aaaa ");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("bbbb");
    dom.append_child(b, bt).unwrap();
    dom.append_child(p, lead).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .display(Display::Inline)
                .pointer_events(crate::layout::PointerEvents::None),
        );
    prepare(&mut dom, &sheet, Rect::new(0, 0, 20, 5));
    assert_eq!(dom.hit_test_path(6, 0), vec![p]);
}
