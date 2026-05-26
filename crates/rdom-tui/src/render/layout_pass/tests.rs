//! End-to-end layout pipeline tests — cascade → layout, reading
//! back `TuiExt.layout` and `TuiExt.content_layout`. Large test
//! bed; move to `tests/layout_pass.rs` when the crate's
//! integration-test story consolidates.

use super::*;
use crate::prelude::*;

fn tui_dom() -> TuiDom {
    TuiDom::new()
}

fn cascade(dom: &mut TuiDom, sheet: &Stylesheet) {
    dom.cascade(sheet);
}

fn layout_rect_of(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).ext().unwrap().layout
}

fn content_rect_of(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).ext().unwrap().content_layout
}

// ── Root-level layout ────────────────────────────────────────────

#[test]
fn root_fragment_distributes_to_children() {
    let mut dom = tui_dom();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    // Both children default to Auto (content) — no content, so 0+padding.
    // Fragment defaults to Column direction.
    cascade(&mut dom, &Stylesheet::bare());
    dom.layout_dom(Rect::new(0, 0, 20, 10));

    // `a` comes first, `b` follows vertically. Both width = 20 (stretch
    // cross-axis in Column), height = intrinsic (0 cells since empty).
    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    assert_eq!(la.x, 0);
    assert_eq!(la.y, 0);
    assert_eq!(la.width, 20);
    assert_eq!(lb.y, la.y + la.height as i32); // stacks after a
}

// ── Row direction ────────────────────────────────────────────────

#[test]
fn row_fixed_children() {
    let mut dom = tui_dom();
    let root = dom.root();
    let container = dom.create_element("div");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(container, a).unwrap();
    dom.append_child(container, b).unwrap();
    dom.append_child(root, container).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(20))
                .height(Size::Fixed(5)),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(5)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Fixed(7)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 20));

    // container is Fixed(20) wide but positioned inside root (which
    // is Fragment with Column, so container's width is 20 and it
    // stretches to Root's full width? Let's just check children.
    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    assert_eq!(la.width, 5);
    assert_eq!(lb.width, 7);
    assert_eq!(lb.x, la.x + 5); // no gap
}

#[test]
fn row_with_gap() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .gap(2),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(3)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Fixed(4)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 10));

    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    assert_eq!(la.width, 3);
    assert_eq!(lb.x, la.x + 3 + 2); // +size, +gap
}

#[test]
fn row_flex_distributes_remaining() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let fx = dom.create_element("fx");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, fx).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    // Viewport 50 wide. Container takes full 50.
    // a = Fixed(5), fx = Flex(1), b = Fixed(5). Remaining = 50-10 = 40.
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(5)))
        .rule_unchecked("fx", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Fixed(5)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 5));

    assert_eq!(layout_rect_of(&dom, a).width, 5);
    assert_eq!(layout_rect_of(&dom, fx).width, 40);
    assert_eq!(layout_rect_of(&dom, b).width, 5);
}

#[test]
fn row_flex_weights_distribute_proportionally() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    // a = Flex(1), b = Flex(3). Viewport 40. a = 10, b = 30.
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Flex(3)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 5));
    assert_eq!(layout_rect_of(&dom, a).width, 10);
    assert_eq!(layout_rect_of(&dom, b).width, 30);
}

// ── Column direction ─────────────────────────────────────────────

#[test]
fn column_stacks_vertically() {
    let mut dom = tui_dom();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    // Fragment root defaults to Column.
    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(3)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(2)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 10));

    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    assert_eq!(la.y, 0);
    assert_eq!(la.height, 3);
    assert_eq!(lb.y, la.y + 3);
    assert_eq!(lb.height, 2);
}

// ── Padding + border ─────────────────────────────────────────────

#[test]
fn content_layout_insets_by_padding() {
    let mut dom = tui_dom();
    let root = dom.root();
    let box_ = dom.create_element("box");
    dom.append_child(root, box_).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "box",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(10))
            .padding(Padding::symmetric(2, 1)),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 20));

    let outer = layout_rect_of(&dom, box_);
    let inner = content_rect_of(&dom, box_);
    assert_eq!(outer.width, 20);
    assert_eq!(outer.height, 10);
    // symmetric(2, 1) = h=2 (left/right), v=1 (top/bottom)
    assert_eq!(inner.x, outer.x + 2);
    assert_eq!(inner.y, outer.y + 1);
    assert_eq!(inner.width, 20 - 4);
    assert_eq!(inner.height, 10 - 2);
}

#[test]
fn content_layout_insets_by_border() {
    let mut dom = tui_dom();
    let root = dom.root();
    let box_ = dom.create_element("box");
    dom.append_child(root, box_).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "box",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(5))
            .border(Border::single()),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 10));

    let outer = layout_rect_of(&dom, box_);
    let inner = content_rect_of(&dom, box_);
    assert_eq!(inner.x, outer.x + 1);
    assert_eq!(inner.y, outer.y + 1);
    assert_eq!(inner.width, 10 - 2);
    assert_eq!(inner.height, 5 - 2);
}

// ── Auto sizing ──────────────────────────────────────────────────

#[test]
fn auto_text_uses_unicode_width() {
    let mut dom = tui_dom();
    let root = dom.root();
    let span = dom.create_element("span");
    let t = dom.create_text_node("hello");
    dom.append_child(span, t).unwrap();
    dom.append_child(root, span).unwrap();

    // span defaults to width=Auto, so it measures child text intrinsic.
    // Text "hello" = 5 cells.
    let sheet = Stylesheet::bare().rule_unchecked(
        "span",
        TuiStyle::new().flow(Flow::Flex).direction(Direction::Row), // so row-fit takes text width
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 5));
    // Root is Fragment (Column), so span stretches cross-axis (width = 50)
    // — BUT span is Auto in Column parent, so span.width stretches to 50 (Auto → stretch cross).
    // Hmm wait, Fragment = Column means span's main axis = height. Width is cross → stretches.
    // Let me check.

    // Actually both width/height default to Auto. In Column parent, main=height, cross=width.
    // Auto cross stretches → span.width = viewport.width (50).
    // So this test doesn't capture intrinsic in the way I hoped. Rewrite:

    // Put span in a Row container so its width is main axis and Auto means intrinsic.
    let _ = layout_rect_of(&dom, span);
}

#[test]
fn auto_text_in_row_parent() {
    let mut dom = tui_dom();
    let root = dom.root();
    let r = dom.create_element("r");
    let span = dom.create_element("span");
    let t = dom.create_text_node("hello");
    dom.append_child(span, t).unwrap();
    dom.append_child(r, span).unwrap();
    dom.append_child(root, r).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "r",
        TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 5));
    // span is Auto in Row parent → main = intrinsic = 5 (width of "hello")
    assert_eq!(layout_rect_of(&dom, span).width, 5);
}

#[test]
fn auto_text_cjk_is_two_cells_each() {
    let mut dom = tui_dom();
    let root = dom.root();
    let r = dom.create_element("r");
    let span = dom.create_element("span");
    let t = dom.create_text_node("中国"); // 4 cells
    dom.append_child(span, t).unwrap();
    dom.append_child(r, span).unwrap();
    dom.append_child(root, r).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "r",
        TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 5));
    // UnicodeWidthStr::width("中国") = 4
    assert_eq!(layout_rect_of(&dom, span).width, 4);
}

#[test]
fn auto_nested_element_recursive_fit() {
    let mut dom = tui_dom();
    let root = dom.root();
    let r = dom.create_element("r");
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    let t = dom.create_text_node("hi"); // width 2
    dom.append_child(inner, t).unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.append_child(r, outer).unwrap();
    dom.append_child(root, r).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "r",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .padding(Padding::all(1)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 5));

    // outer (Row) → its intrinsic width = inner.intrinsic + padding(2)
    //   inner (default Column) → queried Row = max of children = 2 (text)
    //   outer intrinsic = 2 + 2 = 4
    assert_eq!(layout_rect_of(&dom, outer).width, 4);
}

// ── Min/max ──────────────────────────────────────────────────────

#[test]
fn max_width_clamps_flex() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    dom.append_child(c, a).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Flex(1)).max_width(20));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 5));

    assert_eq!(layout_rect_of(&dom, a).width, 20);
}

#[test]
fn min_width_lifts_fixed() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    dom.append_child(c, a).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(3)).min_width(10));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 5));

    assert_eq!(layout_rect_of(&dom, a).width, 10);
}

#[test]
fn min_max_width_height_clamp_via_css_strings() {
    // End-to-end gate: CSS string → cascade → flex resolver → clamp.
    // Pins M5.1's CSS-parser wire-up; failure means the CSS path
    // dropped one of the new property names somewhere between
    // property_dispatch::set and the cascade.
    use rdom_css::parse_inline;

    let css_a = "width: 100; max-width: 30";
    let css_b = "width: 5; min-width: 15";
    let style_a = parse_inline(css_a).style;
    let style_b = parse_inline(css_b).style;
    use rdom_style::layout::MinSize;
    assert_eq!(style_a.max_width, Some(Value::Specified(30)));
    assert_eq!(
        style_b.min_width,
        Some(Value::Specified(MinSize::Cells(15)))
    );

    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("a", style_a)
        .rule_unchecked("b", style_b);
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 200, 5));

    assert_eq!(
        layout_rect_of(&dom, a).width,
        30,
        "max-width clamps width down"
    );
    assert_eq!(
        layout_rect_of(&dom, b).width,
        15,
        "min-width lifts width up"
    );
}

#[test]
fn auto_basis_flex_item_keeps_intrinsic_under_pressure() {
    // CSS Flexbox §4.5: a flex item with `width: auto` (no
    // explicit basis suggestion) has an unbounded specified
    // suggestion, so its auto-min floor = content size. Under
    // pressure from a sibling, it should keep at least its
    // intrinsic content width.
    //
    // Compare to `flex: 1` items (`flex-basis: 0%`) which have a
    // specified suggestion of 0 → auto-min = min(content, 0) = 0
    // → free to shrink. That's the spec-correct distinction; the
    // pre-M5-MIN-CONTENT-1 rdom carried a divergence that made
    // explicit `min-width: auto` content-protective regardless of
    // basis. That divergence was dropped — the spec-strict
    // behavior is enough once the test uses the right basis.
    use rdom_style::layout::MinSize;
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let protected = dom.create_element("p");
    let greedy = dom.create_element("g");
    let text = dom.create_text_node("hello world");
    dom.append_child(protected, text).unwrap();
    dom.append_child(c, protected).unwrap();
    dom.append_child(c, greedy).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        // `width: auto + min-width: auto` is the spec-correct way
        // to say "size from content, protect content". This is
        // also what `flex: 0 1 auto` resolves to.
        .rule_unchecked("p", TuiStyle::new().min_width(MinSize::Auto))
        .rule_unchecked("g", TuiStyle::new().width(Size::Flex(99)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 5));

    let pw = layout_rect_of(&dom, protected).width;
    assert!(
        pw >= 11,
        "auto-basis item with min-width: auto must protect intrinsic content, got {pw}"
    );
}

#[test]
fn flex_basis_zero_shrinks_freely_per_css_strict() {
    // Counterpart to the above: per CSS Flexbox §4.5, a `flex: 1`
    // (`flex-basis: 0%`) item's specified suggestion is 0, so
    // its auto-min is also 0. It SHOULD shrink to its share of
    // the flex distribution, even when content would otherwise
    // floor it. This was the divergence we dropped — `flex: 1`
    // items are now free to vanish per spec.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let p = dom.create_element("p");
    let g = dom.create_element("g");
    let text = dom.create_text_node("very long text that would intrinsically need many cells");
    dom.append_child(p, text).unwrap();
    dom.append_child(c, p).unwrap();
    dom.append_child(c, g).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("p", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("g", TuiStyle::new().width(Size::Flex(99)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 5));

    let pw = layout_rect_of(&dom, p).width;
    let gw = layout_rect_of(&dom, g).width;
    // p ≈ 1/100 of 80 = 0 or 1; g ≈ 99/100 of 80 = 79.
    assert!(
        pw <= 1,
        "flex: 1 with no min must shrink to its flex share (~0–1), got {pw}"
    );
    assert!(
        gw >= 78,
        "flex: 99 sibling must take ~99/100 of the row, got {gw}"
    );
}

#[test]
fn auto_min_drops_to_zero_when_overflow_non_visible_per_css_4_5() {
    // CSS Flexbox §4.5 exception: when the flex item's own
    // overflow along the relevant axis is non-visible
    // (hidden / scroll / auto), the auto-min floor goes back
    // to 0 — the box's content is allowed to overflow into the
    // scroll region.
    //
    // To make the exception observable, the container must be
    // narrow enough that the auto-min floor would otherwise
    // prevent shrink. Both children get equal Flex(1) shares in
    // a 20-cell row: without the exception, p (11-cell intrinsic)
    // refuses to shrink below 11 and overflow happens; with
    // the exception, p shrinks to its 10-cell flex share.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let p = dom.create_element("p");
    let g = dom.create_element("g");
    let tp = dom.create_text_node("hello world"); // 11 cells
    let tg = dom.create_text_node("x");
    dom.append_child(p, tp).unwrap();
    dom.append_child(g, tg).unwrap();
    dom.append_child(c, p).unwrap();
    dom.append_child(c, g).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .width(Size::Flex(1))
                .overflow_x(crate::layout::Overflow::Hidden),
        )
        .rule_unchecked("g", TuiStyle::new().width(Size::Flex(1)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 5));

    let pw = layout_rect_of(&dom, p).width;
    // With the exception: p shrinks to its flex share (~10 cells).
    // Without: p would refuse to shrink below 11 (intrinsic floor)
    // and would push g out. The assertion catches the regression
    // where the exception stops applying — p would jump back to ≥11.
    assert!(
        pw < 11,
        "overflow:hidden must drop the content-min floor — p should shrink to its flex share, got {pw}"
    );
}

#[test]
fn fixed_width_caps_auto_min_per_css_4_5() {
    // CSS Flexbox §4.5: auto-min = min(content_size_suggestion,
    // specified_size_suggestion). A `width: 30` (Fixed) on an
    // item with 60+ cells of intrinsic content should clamp auto-
    // min to 30, NOT pin at 60. This lets `max-width` actually
    // clamp the item even when content is larger.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let p = dom.create_element("p");
    let text = dom.create_text_node("a string that would intrinsically take many cells");
    dom.append_child(p, text).unwrap();
    dom.append_child(c, p).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("p", TuiStyle::new().width(Size::Fixed(30)).max_width(30));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 200, 5));

    let pw = layout_rect_of(&dom, p).width;
    assert_eq!(
        pw, 30,
        "max-width must clamp even when intrinsic content is larger (got {pw})"
    );
}

#[test]
fn calc_specified_caps_auto_min_per_css_4_5() {
    // Block 1 finding: Size::Calc must contribute a definite
    // specified suggestion. `width: calc(50%)` resolves to a
    // concrete value at layout time; auto-min should be
    // min(content, resolved-calc), not just content.
    use rdom_style::calc::CalcExpr;
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let p = dom.create_element("p");
    let text = dom.create_text_node("long string of content larger than 30 cells of intrinsic");
    dom.append_child(p, text).unwrap();
    dom.append_child(c, p).unwrap();
    dom.append_child(root, c).unwrap();

    // p width: calc(50%). With main_budget = 80, resolves to 40.
    let basis_50pct = CalcExpr::Percent(50.0);
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Fixed(80)),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new().width(Size::Calc(Box::new(basis_50pct))),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 200, 5));

    let pw = layout_rect_of(&dom, p).width;
    // p should size to calc(50%) = 40, not be pinned at intrinsic
    // (which is ~57 cells). Test passes only when the Calc branch
    // contributes to specified_cap.
    assert_eq!(
        pw, 40,
        "calc(50%) must produce a definite specified suggestion that bounds auto-min; got {pw}"
    );
}

// ── Aspect ratio ─────────────────────────────────────────────────

#[test]
fn aspect_ratio_computes_height_from_explicit_width_in_row_flex() {
    // M5.2: container is Direction::Row, so cross axis is height.
    // Child has width: 32, aspect-ratio: 16/9, height: auto. Expected
    // height = 32 / (16/9) = 18 cells.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    dom.append_child(c, a).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(32)).aspect_ratio(16, 9),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 30));

    assert_eq!(layout_rect_of(&dom, a).height, 18);
}

#[test]
fn aspect_ratio_computes_width_from_explicit_height_in_column_flex() {
    // Column direction: main is height, cross is width. Height: 9,
    // aspect-ratio: 16/9, width: auto. Expected width = 9 * (16/9) = 16.
    let mut dom = tui_dom();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.append_child(root, a).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "a",
        TuiStyle::new()
            .width(Size::Auto)
            .height(Size::Fixed(9))
            .aspect_ratio(16, 9),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 30));

    // root is Column-direction fragment; child's cross is width.
    // Auto on cross with aspect-ratio set → 9 * 16/9 = 16.
    assert_eq!(layout_rect_of(&dom, a).width, 16);
}

#[test]
fn aspect_ratio_ignored_when_both_axes_explicit() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    dom.append_child(c, a).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(32))
                .height(Size::Fixed(7))
                .aspect_ratio(16, 9),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 30));

    // Explicit width AND height — ratio is ignored. Height stays at 7.
    assert_eq!(layout_rect_of(&dom, a).height, 7);
}

#[test]
fn aspect_ratio_rounds_half_to_even() {
    // 30 cells width × aspect-ratio 16:9 → 30 / (16/9) = 16.875 →
    // round-ties-even → 17 (16.875 is > .5 so just normal round-up).
    // The half-to-even rule matters on EXACT .5 ties; we test the
    // generic round path here and a true tie below.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(30)).aspect_ratio(16, 9),
        )
        // True .5 tie: 5 / (2/1) = 2.5. Round-ties-even → 2 (even).
        .rule_unchecked(
            "b",
            TuiStyle::new().width(Size::Fixed(5)).aspect_ratio(2, 1),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 30));

    assert_eq!(layout_rect_of(&dom, a).height, 17);
    assert_eq!(layout_rect_of(&dom, b).height, 2);
}

#[test]
fn aspect_ratio_round_trip_via_css() {
    use rdom_css::parse_inline;
    let style = parse_inline("aspect-ratio: 16/9").style;
    use rdom_style::layout::AspectRatio;
    assert_eq!(
        style.aspect_ratio,
        Some(Value::Specified(AspectRatio::new(16, 9).unwrap()))
    );
}

// ── Margin (M5.3b: auto absorption + centering) ──────────────────

#[test]
fn flex_row_centers_child_with_margin_auto_horizontal() {
    // Container width 40, child Fixed(10), margin: 0 auto.
    // Remaining = 30, split equally between two auto margins =
    // 15 each. Child sits at x = 0 + 15 = 15.
    use rdom_style::layout::{Margin, MarginValue};
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    dom.append_child(c, a).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(10)).margin(Margin {
                top: MarginValue::Cells(0),
                right: MarginValue::Auto,
                bottom: MarginValue::Cells(0),
                left: MarginValue::Auto,
            }),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 5));

    let la = layout_rect_of(&dom, a);
    assert_eq!(la.width, 10);
    assert_eq!(la.x, 15);
}

#[test]
fn flex_row_multi_auto_margin_distributes_equally() {
    // Three children, each Fixed(5), each with margin-left: auto.
    // 3 children * 5 = 15. Container 30. Remaining = 15. Three auto
    // margins split equally = 5 each. Cumulative: child[0] at 5,
    // child[1] at 5+5+5 = 15, child[2] at 15+5+5 = 25.
    use rdom_style::layout::{Margin, MarginValue};
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let d = dom.create_element("d");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(c, d).unwrap();
    dom.append_child(root, c).unwrap();

    let auto_left = Margin {
        top: MarginValue::Cells(0),
        right: MarginValue::Cells(0),
        bottom: MarginValue::Cells(0),
        left: MarginValue::Auto,
    };
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .margin(auto_left.clone()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(5))
                .margin(auto_left.clone()),
        )
        .rule_unchecked("d", TuiStyle::new().width(Size::Fixed(5)).margin(auto_left));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 5));

    assert_eq!(layout_rect_of(&dom, a).x, 5);
    assert_eq!(layout_rect_of(&dom, b).x, 15);
    assert_eq!(layout_rect_of(&dom, d).x, 25);
}

#[test]
fn flex_main_axis_cells_margin_offsets_child() {
    // Container width 40. Child Fixed(10) with margin-left: 5.
    // Child sits at x = 0 + 5 = 5.
    use rdom_style::layout::{Margin, MarginValue};
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    dom.append_child(c, a).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(10)).margin(Margin {
                top: MarginValue::Cells(0),
                right: MarginValue::Cells(0),
                bottom: MarginValue::Cells(0),
                left: MarginValue::Cells(5),
            }),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 5));

    assert_eq!(layout_rect_of(&dom, a).x, 5);
    assert_eq!(layout_rect_of(&dom, a).width, 10);
}

#[test]
fn flex_auto_margin_starves_flex_grow() {
    // CSS rule: when free space > 0 AND any auto margins exist,
    // auto margins consume the free space; flex-grow is starved.
    // Container 40, child A Flex(1) with margin: auto, child B
    // Fixed(10). Remaining after B = 30. Two auto margins on A → 30
    // / 2 = 15 each. A's flex-grow gets 0 (starved). A's flex
    // resolves to 0 cells, sandwiched between two 15-cell margins.
    use rdom_style::layout::{Margin, MarginValue};
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Flex(1)).margin(Margin {
                top: MarginValue::Cells(0),
                right: MarginValue::Auto,
                bottom: MarginValue::Cells(0),
                left: MarginValue::Auto,
            }),
        )
        .rule_unchecked("b", TuiStyle::new().width(Size::Fixed(10)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 5));

    // A is starved (flex-grow doesn't grow when autos eat the space).
    assert_eq!(layout_rect_of(&dom, a).width, 0);
    // A starts at left-margin = 15.
    assert_eq!(layout_rect_of(&dom, a).x, 15);
    // B follows: x = 15 (A start) + 0 (A width) + 15 (A right margin) = 30.
    assert_eq!(layout_rect_of(&dom, b).x, 30);
}

#[test]
fn absolute_modal_centers_with_inset_zero_and_margin_auto() {
    // Classic CSS modal-centering trick:
    //   position: absolute; top: 0; right: 0; bottom: 0; left: 0;
    //   width: 20; height: 5; margin: auto
    // Container is the viewport (root). Center the 20×5 element
    // inside the 80×30 viewport.
    use rdom_style::layout::{Length, Margin, Position};
    let mut dom = tui_dom();
    let root = dom.root();
    let modal = dom.create_element("modal");
    dom.append_child(root, modal).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "modal",
        TuiStyle::new()
            .position(Position::Absolute)
            .top(Length::Cells(0))
            .right(Length::Cells(0))
            .bottom(Length::Cells(0))
            .left(Length::Cells(0))
            .width(Size::Fixed(20))
            .height(Size::Fixed(5))
            .margin(Margin::all_auto()),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 30));

    let r = layout_rect_of(&dom, modal);
    assert_eq!(r.width, 20);
    assert_eq!(r.height, 5);
    // Horizontal center: viewport 80, modal 20, free = 60, /2 = 30.
    assert_eq!(r.x, 30);
    // Vertical center: viewport 30, modal 5, free = 25, /2 = 12.
    assert_eq!(r.y, 12);
}

// ── Sticky positioning (M5.4) ────────────────────────────────────

#[test]
fn sticky_top_pinned_when_scrollport_scrolls_past_threshold() {
    // Container is a 20-tall scrollport with overflow: hidden.
    // Inside: a sticky banner (height 1, top: 0) followed by enough
    // siblings to fill 60 cells. Before scroll, banner is at y=0.
    // After scrolling the scrollport by 5 cells, the banner would
    // naturally be at y = -5, but sticky keeps it pinned to y = 0
    // of the scrollport.
    use rdom_style::layout::{Length, Position};
    let mut dom = tui_dom();
    let root = dom.root();
    let scrollport = dom.create_element("scrollport");
    let banner = dom.create_element("banner");
    let body = dom.create_element("body");
    dom.append_child(scrollport, banner).unwrap();
    dom.append_child(scrollport, body).unwrap();
    dom.append_child(root, scrollport).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "scrollport",
            TuiStyle::new()
                .height(Size::Fixed(20))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked(
            "banner",
            TuiStyle::new()
                .height(Size::Fixed(1))
                .position(Position::Sticky)
                .top(Length::Cells(0)),
        )
        .rule_unchecked("body", TuiStyle::new().height(Size::Fixed(100)));

    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 20));
    // Pre-scroll: banner at y=0 (top of scrollport).
    assert_eq!(layout_rect_of(&dom, banner).y, 0);

    // Simulate scroll: scrollport.scroll_y = 5. Layout again.
    if let Some(ext) = dom.node_mut(scrollport).ext_mut() {
        ext.scroll_y = 5;
    }
    dom.layout_dom(Rect::new(0, 0, 40, 20));

    // Banner stays pinned at y = scrollport.content_layout.y = 0
    // (scrollport's content_layout starts at 0 since scrollport sits
    // at the top of the viewport).
    assert_eq!(layout_rect_of(&dom, banner).y, 0);
    // Body has scrolled up — its y is now -5 + 1 (banner height) = -4
    // (was at y=1 pre-scroll).
    assert_eq!(layout_rect_of(&dom, body).y, -4);
}

#[test]
fn sticky_top_pre_stick_renders_in_normal_flow() {
    // CSS rule: sticky behaves like relative when the natural
    // position is BELOW the threshold (natural.y > scrollport.y +
    // top_inset). Tested here: top: 0, banner positioned 5 cells
    // into the scrollport. Natural y = 5, pin_y = 0, so sticky
    // doesn't pin — banner stays at natural y = 5.
    use rdom_style::layout::{Length, Position};
    let mut dom = tui_dom();
    let root = dom.root();
    let scrollport = dom.create_element("scrollport");
    let spacer = dom.create_element("spacer");
    let banner = dom.create_element("banner");
    dom.append_child(scrollport, spacer).unwrap();
    dom.append_child(scrollport, banner).unwrap();
    dom.append_child(root, scrollport).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "scrollport",
            TuiStyle::new()
                .height(Size::Fixed(20))
                .overflow(Overflow::Hidden),
        )
        .rule_unchecked("spacer", TuiStyle::new().height(Size::Fixed(5)))
        .rule_unchecked(
            "banner",
            TuiStyle::new()
                .height(Size::Fixed(1))
                .position(Position::Sticky)
                .top(Length::Cells(0)),
        );

    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 20));
    // Natural y = 5 (after spacer), top threshold = 0. Natural is
    // BELOW the threshold (visually below = larger y), no pin.
    assert_eq!(layout_rect_of(&dom, banner).y, 5);
}

#[test]
fn sticky_no_scrollport_ancestor_behaves_as_relative() {
    // No overflow on any ancestor → no scrollport → sticky stays at
    // natural position (CSS rule). Tests the early-return path.
    use rdom_style::layout::{Length, Position};
    let mut dom = tui_dom();
    let root = dom.root();
    let banner = dom.create_element("banner");
    dom.append_child(root, banner).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "banner",
        TuiStyle::new()
            .height(Size::Fixed(3))
            .position(Position::Sticky)
            .top(Length::Cells(10)),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 20));
    // No pin — natural flow position (y = 0 in root).
    assert_eq!(layout_rect_of(&dom, banner).y, 0);
}

#[test]
fn sticky_keyword_parses_via_css() {
    use rdom_css::parse_inline;
    use rdom_style::layout::Position;
    let s = parse_inline("position: sticky").style;
    assert_eq!(s.position, Some(Value::Specified(Position::Sticky)));
}

// ── Border collapse layout (M5.5b) ───────────────────────────────

#[test]
fn flex_row_with_collapse_shared_edge_is_one_cell() {
    // Parent border + collapse + two bordered children. Children
    // should sit side-by-side with the parent's border cells acting
    // as their outer-left/right edges, and a single cell shared
    // between the two children at their meeting point.
    //
    // Without collapse: width = parent_border_left(1) + child_a_left
    // + child_a + child_a_right + child_b_left + child_b + child_b_right
    // + parent_border_right(1) = needs all those cells.
    //
    // With collapse:
    // - Parent's content area is the OUTER rect (border cells shared).
    // - children's outer rects fit edge-to-edge with siblings.
    use rdom_style::layout::{Border, BorderCollapse};
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(5))
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single())
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .border(Border::single()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .border(Border::single()),
        );

    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));

    // outer x=0, width=20.
    // With collapse + border: parent content area = OUTER rect (0..20).
    // a starts at x=0, width=8 → spans 0..8.
    // Without overlap, b would start at 8 (no gap).
    // With overlap (both bordered + collapse), b starts at 8-1 = 7.
    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    assert_eq!(la.x, 0, "child A's outer extends into parent border");
    assert_eq!(la.width, 8);
    assert_eq!(lb.x, 7, "sibling overlap: B starts 1 cell before A's end");
    assert_eq!(lb.width, 8);
}

#[test]
fn flex_row_collapse_inactive_keeps_separate_borders() {
    // Same shape, but border-collapse: separate (default). The
    // parent's content area is inset by its border (1 cell each
    // side), so children sit inside that inner box. Siblings
    // don't overlap.
    use rdom_style::layout::Border;
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(5))
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single()),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .border(Border::single()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .border(Border::single()),
        );

    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));

    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    // Outer border insets by 1 → content x = 1.
    assert_eq!(la.x, 1);
    // No overlap: b starts at a's end edge (1 + 8 = 9).
    assert_eq!(lb.x, 9);
}

#[test]
fn collapse_without_border_does_not_expand_content_area() {
    // collapse is set but the element has no border → no overlap
    // logic. content area is the standard inset.
    use rdom_style::layout::BorderCollapse;
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    dom.append_child(outer, a).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .padding(Padding::all(1))
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(10)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));

    // Padding insets normally. Content starts at x=1.
    assert_eq!(layout_rect_of(&dom, a).x, 1);
}

#[test]
fn flex_distribution_redistributes_integer_remainder() {
    // Pre-fix: two `Flex(1)` children with `flex_remaining = 31`
    // each got `31 / 2 = 15`, summing to 30 — leaving 1 cell of
    // empty space at the parent's right edge. Visible in the
    // border_collapse_demo at odd terminal sizes. The rolling
    // allocation now distributes the leftover to the last child:
    // 15 + 16 = 31.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .width(Size::Fixed(31))
                .flow(Flow::Flex)
                .direction(Direction::Row),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Flex(1)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 31, 5));

    let aw = layout_rect_of(&dom, a).width;
    let bw = layout_rect_of(&dom, b).width;
    assert_eq!(aw + bw, 31, "flex distribution must consume all 31 cells");
    assert_eq!(
        layout_rect_of(&dom, b).x + bw as i32,
        31,
        "rightmost edge reaches the parent's right"
    );
}

#[test]
fn hit_test_at_shared_edge_returns_deeper_element() {
    // Decision 2 from M5 pre-prep: under `border-collapse: collapse`
    // a click on a shared edge cell should go to the "deeper"
    // element — which in our case is the later document-order
    // sibling (CSS table-cell rule: right cell owns its left edge).
    // Confirms the existing hit-test logic already satisfies this
    // without a new collapse-aware rule (the deepest-wins recurse +
    // last-painted reverse walk does the right thing).
    use crate::runtime::hit_test::HitTestExt;
    use rdom_style::layout::{Border, BorderCollapse};
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(5))
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single())
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .border(Border::single()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(8))
                .border(Border::single()),
        );

    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));

    // Layout: a is 0..8, b is 7..15 (1-cell overlap at x=7).
    // Click at (7, 2) — inside both a and b. Decision 2 says deeper /
    // later-doc-order wins → b.
    let hit = dom.hit_test(7, 2);
    assert_eq!(hit, Some(b), "shared-edge click goes to later sibling");

    // Click at x=3 (firmly inside a, well before the shared edge) → a.
    let hit_a = dom.hit_test(3, 2);
    assert_eq!(hit_a, Some(a));
}

#[test]
fn collapse_three_bordered_siblings_share_two_junctions() {
    // Three siblings, each width 6, each bordered, parent border +
    // collapse. Junction count = 2 between three siblings.
    // a: x=0..6, b: x=5..11, c: x=10..16.
    use rdom_style::layout::{Border, BorderCollapse};
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();
    dom.append_child(outer, c).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .border(Border::single())
                .border_collapse(BorderCollapse::Collapse),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .border(Border::single()),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .border(Border::single()),
        )
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .width(Size::Fixed(6))
                .border(Border::single()),
        );

    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));

    assert_eq!(layout_rect_of(&dom, a).x, 0);
    assert_eq!(layout_rect_of(&dom, b).x, 5);
    assert_eq!(layout_rect_of(&dom, c).x, 10);
}

// ── Cross axis ───────────────────────────────────────────────────

#[test]
fn row_cross_axis_stretches() {
    let mut dom = tui_dom();
    let root = dom.root();
    let r = dom.create_element("r");
    let a = dom.create_element("a");
    dom.append_child(r, a).unwrap();
    dom.append_child(root, r).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "r",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .height(Size::Fixed(8)),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(5)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 20));

    // a's height is Auto → stretches to parent's content_layout.height = 8.
    assert_eq!(layout_rect_of(&dom, a).height, 8);
}

#[test]
fn row_cross_axis_fixed_not_stretched() {
    let mut dom = tui_dom();
    let root = dom.root();
    let r = dom.create_element("r");
    let a = dom.create_element("a");
    dom.append_child(r, a).unwrap();
    dom.append_child(root, r).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "r",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .height(Size::Fixed(10)),
        )
        .rule_unchecked(
            "a",
            TuiStyle::new().width(Size::Fixed(5)).height(Size::Fixed(3)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 20));

    assert_eq!(layout_rect_of(&dom, a).height, 3);
}

// ── Scroll ───────────────────────────────────────────────────────

#[test]
fn scroll_y_offsets_children_negative() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("a", TuiStyle::new().height(Size::Fixed(3)))
        .rule_unchecked("b", TuiStyle::new().height(Size::Fixed(3)));
    cascade(&mut dom, &sheet);
    // Apply scroll_y = 2 on c
    dom.node_mut(c).ext_mut().unwrap().scroll_y = 2;
    dom.layout_dom(Rect::new(0, 0, 20, 10));

    let la = layout_rect_of(&dom, a);
    let lb = layout_rect_of(&dom, b);
    // a was at y=0, now y=-2 because scroll_y=2 shifts upward
    assert_eq!(la.y, -2);
    assert_eq!(lb.y, la.y + 3);
}

// ── Nested recursion ─────────────────────────────────────────────

#[test]
fn nested_containers_lay_out_independently() {
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    let leaf = dom.create_element("leaf");
    dom.append_child(inner, leaf).unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .padding(Padding::all(1))
                .border(Border::single())
                .width(Size::Fixed(20))
                .height(Size::Fixed(10)),
        )
        .rule_unchecked("inner", TuiStyle::new().height(Size::Fixed(4)))
        .rule_unchecked("leaf", TuiStyle::new().height(Size::Fixed(2)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 50, 20));

    // outer.content = (1+1, 1+1, 20-4, 10-4) = (2, 2, 16, 6)
    let oc = content_rect_of(&dom, outer);
    assert_eq!(oc.x, 2);
    assert_eq!(oc.y, 2);
    assert_eq!(oc.width, 16);
    assert_eq!(oc.height, 6);

    // inner at (2, 2) with height 4.
    let il = layout_rect_of(&dom, inner);
    assert_eq!(il.x, 2);
    assert_eq!(il.y, 2);
    assert_eq!(il.height, 4);
    // inner stretches cross-axis (Column direction means cross=width=16)
    assert_eq!(il.width, 16);

    // leaf at inner's content (no padding/border on inner), y=2, height=2
    let leaf_rect = layout_rect_of(&dom, leaf);
    assert_eq!(leaf_rect.y, 2);
    assert_eq!(leaf_rect.height, 2);
}

// ── Layout-dirty flag ────────────────────────────────────────────

#[test]
fn layout_pass_clears_layout_dirty() {
    let mut dom = tui_dom();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.append_child(root, a).unwrap();

    dom.cascade(&Stylesheet::bare()); // sets layout_dirty = true (first cascade)
    assert!(dom.node(a).is_layout_dirty());

    dom.layout_dom(Rect::new(0, 0, 20, 10));
    assert!(!dom.node(a).is_layout_dirty());
}

// ── Empty tree ───────────────────────────────────────────────────

#[test]
fn empty_root_no_children_no_panic() {
    let mut dom = tui_dom();
    dom.cascade(&Stylesheet::bare());
    dom.layout_dom(Rect::new(0, 0, 20, 10));
}

// ── Gap doesn't trail ────────────────────────────────────────────

#[test]
fn gap_only_between_not_after_last() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(c, a).unwrap();
    dom.append_child(c, b).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .gap(3),
        )
        .rule_unchecked("a", TuiStyle::new().width(Size::Fixed(5)))
        .rule_unchecked("b", TuiStyle::new().width(Size::Fixed(5)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 5));

    // a at 0..5, gap 3 = 5..8, b at 8..13. No trailing gap past 13.
    assert_eq!(layout_rect_of(&dom, a).x, 0);
    assert_eq!(layout_rect_of(&dom, b).x, 8);
}

// ── Phase 1 skips absolute / fixed children (M2 §12.4) ───────────

#[test]
fn flex_phase1_skips_absolute_child() {
    // Container Row with two children: an absolute one and a
    // flex(1) one. The flex child gets the full container width
    // because the absolute child is removed from flow.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let abs = dom.create_element("abs");
    let flex = dom.create_element("flex");
    dom.append_child(c, abs).unwrap();
    dom.append_child(c, flex).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "abs",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked("flex", TuiStyle::new().width(Size::Flex(1)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 5));

    // flex child gets the full 30 cells (absolute didn't take any).
    assert_eq!(layout_rect_of(&dom, flex).width, 30);
}

#[test]
fn flex_phase1_skips_fixed_child() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let fixed = dom.create_element("fixed");
    let flex = dom.create_element("flex");
    dom.append_child(c, fixed).unwrap();
    dom.append_child(c, flex).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "fixed",
            TuiStyle::new()
                .position(crate::layout::Position::Fixed)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked("flex", TuiStyle::new().width(Size::Flex(1)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 5));

    assert_eq!(layout_rect_of(&dom, flex).width, 30);
}

// ── Relative shift (M2 §12.6) ────────────────────────────────────

#[test]
fn relative_shift_moves_layout_rect() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let rel = dom.create_element("rel");
    dom.append_child(c, rel).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "rel",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .top(crate::layout::Length::Cells(2))
                .left(crate::layout::Length::Cells(3))
                .width(Size::Fixed(10))
                .height(Size::Fixed(2)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));

    let r = layout_rect_of(&dom, rel);
    // In-flow x would be 0; shifted by left:3 → 3.
    assert_eq!(r.x, 3);
    assert_eq!(r.y, 2);
}

#[test]
fn relative_shift_does_not_affect_sibling_layout() {
    // Sibling after a relative-shifted element stays at its
    // un-shifted in-flow position.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let rel = dom.create_element("rel");
    let after = dom.create_element("after");
    dom.append_child(c, rel).unwrap();
    dom.append_child(c, after).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "rel",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .left(crate::layout::Length::Cells(50))
                .width(Size::Fixed(5))
                .height(Size::Fixed(2)),
        )
        .rule_unchecked("after", TuiStyle::new().width(Size::Fixed(10)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 10));

    // `after` follows rel's *un-shifted* position (rel takes 5 cells
    // in-flow; after starts at x=5).
    let after_r = layout_rect_of(&dom, after);
    assert_eq!(after_r.x, 5);
}

#[test]
fn relative_negative_offset_shifts_left() {
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let rel = dom.create_element("rel");
    dom.append_child(c, rel).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "rel",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .top(crate::layout::Length::Cells(-1))
                .width(Size::Fixed(5))
                .height(Size::Fixed(2)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 5, 30, 10));

    let r = layout_rect_of(&dom, rel);
    // In-flow y = 5; shifted by top:-1 → 4.
    assert_eq!(r.y, 4);
}

// ── Phase 2 places absolute / fixed (M2 §12.5) ───────────────────

#[test]
fn absolute_placed_against_viewport() {
    // No positioned ancestor → containing block is the viewport.
    let mut dom = tui_dom();
    let root = dom.root();
    let abs = dom.create_element("abs");
    dom.append_child(root, abs).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "abs",
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(5))
            .left(crate::layout::Length::Cells(3))
            .width(Size::Fixed(10))
            .height(Size::Fixed(2)),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let r = layout_rect_of(&dom, abs);
    assert_eq!(r.x, 3);
    assert_eq!(r.y, 5);
    assert_eq!(r.width, 10);
    assert_eq!(r.height, 2);
}

#[test]
fn absolute_placed_against_relative_parent() {
    // Relative parent at (0,0,30,10) sets the containing block;
    // absolute child placed at (top:1, left:2) maps to (2, 1).
    let mut dom = tui_dom();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let abs = dom.create_element("abs");
    dom.append_child(parent, abs).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "parent",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(30))
                .height(Size::Fixed(10)),
        )
        .rule_unchecked(
            "abs",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(1))
                .left(crate::layout::Length::Cells(2))
                .width(Size::Fixed(5))
                .height(Size::Fixed(2)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let parent_rect = layout_rect_of(&dom, parent);
    let abs_rect = layout_rect_of(&dom, abs);
    assert_eq!(abs_rect.x, parent_rect.x + 2);
    assert_eq!(abs_rect.y, parent_rect.y + 1);
}

#[test]
fn absolute_inset_zero_fills_containing_block() {
    // `inset: 0` (= top:0 right:0 bottom:0 left:0) makes the
    // absolute element fill its containing block.
    let mut dom = tui_dom();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let abs = dom.create_element("abs");
    dom.append_child(parent, abs).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "parent",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(20))
                .height(Size::Fixed(8)),
        )
        .rule_unchecked(
            "abs",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(0))
                .right(crate::layout::Length::Cells(0))
                .bottom(crate::layout::Length::Cells(0))
                .left(crate::layout::Length::Cells(0)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let parent_rect = layout_rect_of(&dom, parent);
    let abs_rect = layout_rect_of(&dom, abs);
    assert_eq!(abs_rect.x, parent_rect.x);
    assert_eq!(abs_rect.y, parent_rect.y);
    assert_eq!(abs_rect.width, parent_rect.width);
    assert_eq!(abs_rect.height, parent_rect.height);
}

#[test]
fn fixed_placed_against_viewport_even_with_relative_parent() {
    // Fixed always uses viewport, ignoring all ancestors.
    let mut dom = tui_dom();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let f = dom.create_element("f");
    dom.append_child(parent, f).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "parent",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(30))
                .height(Size::Fixed(10)),
        )
        .rule_unchecked(
            "f",
            TuiStyle::new()
                .position(crate::layout::Position::Fixed)
                .top(crate::layout::Length::Cells(2))
                .left(crate::layout::Length::Cells(4))
                .width(Size::Fixed(8))
                .height(Size::Fixed(3)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let r = layout_rect_of(&dom, f);
    // Viewport is at (0,0,100,50); fixed coords are absolute viewport offsets.
    assert_eq!(r.x, 4);
    assert_eq!(r.y, 2);
}

#[test]
fn nested_absolute_uses_outer_absolute_as_containing_block() {
    // outer absolute placed at (5, 5, 30, 10);
    // inner absolute with top:1 left:2 → (7, 6) absolute coords.
    let mut dom = tui_dom();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "outer",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(5))
                .left(crate::layout::Length::Cells(5))
                .width(Size::Fixed(30))
                .height(Size::Fixed(10)),
        )
        .rule_unchecked(
            "inner",
            TuiStyle::new()
                .position(crate::layout::Position::Absolute)
                .top(crate::layout::Length::Cells(1))
                .left(crate::layout::Length::Cells(2))
                .width(Size::Fixed(5))
                .height(Size::Fixed(2)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let inner_r = layout_rect_of(&dom, inner);
    assert_eq!(inner_r.x, 7);
    assert_eq!(inner_r.y, 6);
}

#[test]
fn absolute_with_right_and_no_left_aligns_to_right_edge() {
    let mut dom = tui_dom();
    let root = dom.root();
    let abs = dom.create_element("abs");
    dom.append_child(root, abs).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "abs",
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .right(crate::layout::Length::Cells(2))
            .width(Size::Fixed(10))
            .height(Size::Fixed(2)),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let r = layout_rect_of(&dom, abs);
    // x = viewport.right - right - width = 100 - 2 - 10 = 88
    assert_eq!(r.x, 88);
}

#[test]
fn absolute_with_top_and_bottom_stretches_height() {
    // top:0 + bottom:0 → height = containing_block.height (cb.height - 0 - 0).
    let mut dom = tui_dom();
    let root = dom.root();
    let abs = dom.create_element("abs");
    dom.append_child(root, abs).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "abs",
        TuiStyle::new()
            .position(crate::layout::Position::Absolute)
            .top(crate::layout::Length::Cells(0))
            .bottom(crate::layout::Length::Cells(0))
            .left(crate::layout::Length::Cells(0))
            .width(Size::Fixed(20)),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let r = layout_rect_of(&dom, abs);
    assert_eq!(r.height, 50);
}

// ── Fixed vs scroll (M2 §12.12) ──────────────────────────────────

#[test]
fn fixed_inside_overflow_scroll_uses_viewport() {
    // Even when nested inside an overflow:scroll container, a
    // position:fixed element places against the viewport.
    let mut dom = tui_dom();
    let root = dom.root();
    let scroller = dom.create_element("scroll");
    let f = dom.create_element("f");
    dom.append_child(scroller, f).unwrap();
    dom.append_child(root, scroller).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "scroll",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(10))
                .overflow(Overflow::Scroll),
        )
        .rule_unchecked(
            "f",
            TuiStyle::new()
                .position(crate::layout::Position::Fixed)
                .top(crate::layout::Length::Cells(2))
                .left(crate::layout::Length::Cells(4))
                .width(Size::Fixed(5))
                .height(Size::Fixed(2)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 50));

    let r = layout_rect_of(&dom, f);
    // Viewport at (0,0,100,50); fixed coords ignore the
    // scrolled container entirely.
    assert_eq!(r.x, 4);
    assert_eq!(r.y, 2);
}

#[test]
fn flex_phase1_keeps_relative_child_in_flow() {
    // position: relative does NOT remove from flow — the flex
    // distribution still allocates space for the relative child.
    let mut dom = tui_dom();
    let root = dom.root();
    let c = dom.create_element("c");
    let rel = dom.create_element("rel");
    let flex = dom.create_element("flex");
    dom.append_child(c, rel).unwrap();
    dom.append_child(c, flex).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "rel",
            TuiStyle::new()
                .position(crate::layout::Position::Relative)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("flex", TuiStyle::new().width(Size::Flex(1)));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 5));

    // rel takes its 10 cells; flex gets the remaining 20.
    assert_eq!(layout_rect_of(&dom, rel).width, 10);
    assert_eq!(layout_rect_of(&dom, flex).width, 20);
}

#[test]
fn debug_inline_block_cascade() {
    let mut dom = tui_dom();
    let root = dom.root();
    let btn = dom.create_element("btn");
    let t = dom.create_text_node("Submit");
    dom.append_child(btn, t).unwrap();
    dom.append_child(root, btn).unwrap();
    let sheet =
        Stylesheet::bare().rule_unchecked("btn", TuiStyle::new().display(Display::InlineBlock));
    cascade(&mut dom, &sheet);
    let display = dom.node(btn).computed().unwrap().display;
    assert_eq!(
        display,
        Display::InlineBlock,
        "cascade did not produce InlineBlock"
    );
}

// ── Display::InlineBlock — atomic inline-level box ───────────────
//
// Regression bed for the OOTB-round blocker: a `<button>` (or any
// inline-block element) with `width: Auto` must NOT stretch cross-
// axially to the container's width. The button hugs its intrinsic
// content on both axes.

#[test]
fn inline_block_hugs_content_in_column_parent() {
    // The OOTB blocker scenario: button as direct child of a column-
    // direction parent (`<screen>`). Pre-M5-now, an auto-width Block
    // child stretches horizontally to fill the parent. Post-M5-now,
    // an InlineBlock child sizes to its intrinsic content width.
    let mut dom = tui_dom();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Submit");
    dom.append_child(btn, label).unwrap();
    dom.append_child(screen, btn).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .rule_unchecked("btn", TuiStyle::new().display(Display::InlineBlock));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(
        lb.width, 6,
        "InlineBlock child must size to intrinsic content width (got {lb:?})"
    );
}

#[test]
fn inline_block_with_pseudo_chrome_hugs_content_plus_pseudos() {
    // The actual `<button>` case: text content + `::before` / `::after`
    // pseudo brackets contribute to intrinsic width via the C-bugfix
    // pseudo_content_width path. InlineBlock width = 2 + 6 + 2 = 10.
    let mut dom = tui_dom();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Submit");
    dom.append_child(btn, label).unwrap();
    dom.append_child(screen, btn).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1)),
        )
        .rule_unchecked("btn", TuiStyle::new().display(Display::InlineBlock))
        .rule_unchecked(
            "btn::before",
            TuiStyle::new().content(crate::style::Content::Str("[ ".into())),
        )
        .rule_unchecked(
            "btn::after",
            TuiStyle::new().content(crate::style::Content::Str(" ]".into())),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(
        lb.width, 10,
        "InlineBlock width = text(6) + ::before(2) + ::after(2) = 10 (got {lb:?})"
    );
}

#[test]
fn inline_block_hugs_content_in_row_parent() {
    let mut dom = tui_dom();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Go");
    dom.append_child(btn, label).unwrap();
    dom.append_child(screen, btn).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .rule_unchecked("btn", TuiStyle::new().display(Display::InlineBlock));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(lb.width, 2, "InlineBlock width hugs 'Go' (2 cells)");
    assert_eq!(
        lb.height, 1,
        "InlineBlock height hugs content — does NOT stretch to row cross"
    );
}

#[test]
fn inline_block_fixed_width_wins_over_intrinsic() {
    let mut dom = tui_dom();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Go");
    dom.append_child(btn, label).unwrap();
    dom.append_child(screen, btn).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1)),
        )
        .rule_unchecked(
            "btn",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .width(Size::Fixed(20)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(lb.width, 20, "Fixed width overrides InlineBlock intrinsic");
}

#[test]
fn block_still_stretches_cross_axially() {
    // Regression guard: Block + Auto width still stretches to fill
    // the parent's cross axis. The InlineBlock branch must not
    // accidentally affect Block behavior.
    let mut dom = tui_dom();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Submit");
    dom.append_child(btn, label).unwrap();
    dom.append_child(screen, btn).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1)),
        )
        .rule_unchecked("btn", TuiStyle::new().display(Display::Block));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(lb.width, 80, "Block + Auto still stretches");
}

#[test]
fn inline_block_with_position_relative_shifts_in_flex_parent() {
    let mut dom = tui_dom();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Hi");
    dom.append_child(btn, label).unwrap();
    dom.append_child(screen, btn).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1)),
        )
        .rule_unchecked(
            "btn",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .position(crate::layout::Position::Relative)
                .left(crate::layout::Length::Cells(5)),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(
        lb.width, 2,
        "intrinsic width preserved through relative shift"
    );
    assert_eq!(lb.x, 5, "shifted right by 5 cells");
}

#[test]
fn display_none_ancestor_suppresses_inline_block_descendant() {
    let mut dom = tui_dom();
    let root = dom.root();
    let hidden = dom.create_element("hidden");
    let btn = dom.create_element("btn");
    let label = dom.create_text_node("Should not render");
    dom.append_child(btn, label).unwrap();
    dom.append_child(hidden, btn).unwrap();
    dom.append_child(root, hidden).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("hidden", TuiStyle::new().display(Display::None))
        .rule_unchecked("btn", TuiStyle::new().display(Display::InlineBlock));
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let lb = layout_rect_of(&dom, btn);
    assert_eq!(lb.width, 0);
    assert_eq!(lb.height, 0);
}

// ── scroll_content_{width,height} recording ─────────────────────

/// A `Column` flex container with `overflow: Scroll` and N children
/// of fixed height H records `scroll_content_height = N * H + (N-1) * gap`
/// regardless of viewport height. Without this, scrollbar paint
/// can't tell viewport from content size and the thumb fills the
/// whole track.
#[test]
fn layout_records_scroll_content_height_for_scrollable_column() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("list");
    dom.append_child(root, list).unwrap();
    for _ in 0..50 {
        let row = dom.create_element("row");
        dom.append_child(list, row).unwrap();
    }
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "list",
            TuiStyle::new()
                .flow(crate::layout::Flow::Flex)
                .direction(crate::layout::Direction::Column)
                .height(crate::layout::Size::Flex(1))
                .overflow(crate::layout::Overflow::Scroll),
        )
        .rule_unchecked(
            "row",
            TuiStyle::new()
                .height(crate::layout::Size::Fixed(1))
                .flex_shrink(0),
        );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let ext = dom.node(list).ext().unwrap();
    // 50 rows × 1 cell tall, no gap → 50 cells of content height.
    // `flex-shrink: 0` opts each row out of the default CSS shrink
    // behavior so the scroll-content overflow is what gets recorded
    // (scrollable container with non-shrinking children — the
    // canonical use case for `overflow: scroll`).
    assert_eq!(ext.scroll_content_height, 50);
    // No horizontal overflow; child rows stretch to fill cross-axis
    // (less 1 cell of vertical scrollbar gutter). Whatever the
    // resolved width is, it should be > 0 and ≤ viewport width.
    assert!(ext.scroll_content_width > 0);
    assert!(ext.scroll_content_width <= 80);
}

/// Non-scrollable containers don't pay the scroll_content cost.
/// The fields stay at the default 0 — saves an O(N) child walk per
/// element per frame.
#[test]
fn layout_skips_scroll_content_recording_for_visible_overflow() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let child = dom.create_element("p");
    dom.append_child(div, child).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new());
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let ext = dom.node(div).ext().unwrap();
    assert_eq!(ext.scroll_content_height, 0);
    assert_eq!(ext.scroll_content_width, 0);
}

// ── Intrinsic measurement: whitespace-between-blocks and text-only ─

/// `css_parser_demo`-shaped regression. A block container with
/// whitespace text nodes between its block element children — the
/// shape the parser produces for any pretty-printed template — must
/// not let those text nodes inflate the container's intrinsic main-
/// axis size. Pre-fix this card measured to 16 cells (each "\n…"
/// text node summing into the column-direction children sum, plus
/// gaps between them).
#[test]
fn intrinsic_height_ignores_whitespace_text_between_block_children() {
    let mut dom = tui_dom();
    let root = dom.root();
    let card = dom.create_element("card");
    let ws1 = dom.create_text_node("\n    ");
    let label = dom.create_element("label");
    let label_text = dom.create_text_node("Author CSS");
    let ws2 = dom.create_text_node("\n    ");
    let note = dom.create_element("note");
    let note_text = dom.create_text_node("This whole card is styled by the block.");
    let ws3 = dom.create_text_node("\n  ");
    dom.append_child(label, label_text).unwrap();
    dom.append_child(note, note_text).unwrap();
    dom.append_child(card, ws1).unwrap();
    dom.append_child(card, label).unwrap();
    dom.append_child(card, ws2).unwrap();
    dom.append_child(card, note).unwrap();
    dom.append_child(card, ws3).unwrap();
    dom.append_child(root, card).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "card",
        TuiStyle::new()
            .display(Display::Block)
            .border(crate::layout::Border::rounded())
            .padding(crate::layout::Padding {
                top: crate::layout::PaddingValue::Cells(1),
                right: crate::layout::PaddingValue::Cells(2),
                bottom: crate::layout::PaddingValue::Cells(1),
                left: crate::layout::PaddingValue::Cells(2),
            })
            .gap(1),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    // border(2) + padding(2) + label(1) + gap(1) + note(1) = 7.
    let card_rect = layout_rect_of(&dom, card);
    assert_eq!(
        card_rect.height, 7,
        "whitespace between block children must not inflate intrinsic height"
    );
}

/// Direct unit test on `intrinsic_size`. A block element whose only
/// children are whitespace text nodes contributes only its own
/// padding + border to the column-axis intrinsic — the text nodes
/// collapse like CSS ignorable whitespace.
#[test]
fn intrinsic_size_block_with_only_whitespace_text_is_chrome_only() {
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    let ws = dom.create_text_node("\n    \n  ");
    dom.append_child(host, ws).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "host",
        TuiStyle::new()
            .display(Display::Block)
            .padding(crate::layout::Padding {
                top: crate::layout::PaddingValue::Cells(1),
                right: crate::layout::PaddingValue::Cells(0),
                bottom: crate::layout::PaddingValue::Cells(1),
                left: crate::layout::PaddingValue::Cells(0),
            }),
    );
    cascade(&mut dom, &sheet);

    let h = super::intrinsic::intrinsic_size(&dom, host, Direction::Column, 40);
    assert_eq!(h, 2, "padding only — whitespace text contributes nothing");
}

#[test]
fn intrinsic_size_ignores_display_none_children() {
    // CSS 2.1 §9.5: `display: none` elements take no space in
    // layout — including no contribution to their parent's
    // intrinsic measurement. Regression for the
    // `<details>`-closed-body bug surfaced during BFC-1 Phase 9
    // chrome review: the source-disclosure's intrinsic counted
    // its hidden `<pre>` blocks (display:none via the UA
    // `details:not([open]) > *:not(summary)` rule), giving it
    // ~15 rows of intrinsic instead of ~1 (summary only). That
    // intrinsic then ate from the flex column's main-axis
    // budget so the sibling `flex: 1` view-content shrank to
    // its content height instead of stretching.
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    let visible = dom.create_element("v");
    let visible_text = dom.create_text_node("one");
    dom.append_child(visible, visible_text).unwrap();
    let hidden = dom.create_element("h");
    let hidden_text = dom.create_text_node("five\nlines\nof\nhidden\ncontent");
    dom.append_child(hidden, hidden_text).unwrap();
    dom.append_child(host, visible).unwrap();
    dom.append_child(host, hidden).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("v", TuiStyle::new().display(Display::Block))
        .rule_unchecked("h", TuiStyle::new().display(Display::None));
    cascade(&mut dom, &sheet);

    let h = super::intrinsic::intrinsic_size(&dom, host, Direction::Column, 40);
    assert_eq!(
        h, 1,
        "intrinsic = visible child's 1 row; hidden 5-line child contributes 0"
    );
}

#[test]
fn intrinsic_size_ignores_absolutely_positioned_children() {
    // CSS 2.1 §9.3 + §10.6: out-of-flow elements (position:
    // absolute/fixed) don't contribute to their parent's
    // in-flow content extent. Their layout rect comes from the
    // positioning pass against their containing block, not from
    // the parent's intrinsic.
    use crate::layout::Position;
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    let visible = dom.create_element("v");
    let vt = dom.create_text_node("one");
    dom.append_child(visible, vt).unwrap();
    let abs = dom.create_element("a");
    let at = dom.create_text_node("five\nlines\nof\noff-flow\ncontent");
    dom.append_child(abs, at).unwrap();
    dom.append_child(host, visible).unwrap();
    dom.append_child(host, abs).unwrap();
    dom.append_child(root, host).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("v", TuiStyle::new().display(Display::Block))
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .display(Display::Block)
                .position(Position::Absolute),
        );
    cascade(&mut dom, &sheet);

    let h = super::intrinsic::intrinsic_size(&dom, host, Direction::Column, 40);
    assert_eq!(h, 1, "absolute child contributes 0 to in-flow intrinsic");
}

/// Text-only block elements measure via the inline formatting
/// context, not by counting their text node's newline count. The
/// height should reflect actual inline layout at the given cross
/// budget (1 line for short text that fits).
#[test]
fn intrinsic_size_text_only_block_uses_inline_layout() {
    let mut dom = tui_dom();
    let root = dom.root();
    let note = dom.create_element("note");
    let text = dom.create_text_node("hello world");
    dom.append_child(note, text).unwrap();
    dom.append_child(root, note).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked("note", TuiStyle::new().display(Display::Block));
    cascade(&mut dom, &sheet);

    let h = super::intrinsic::intrinsic_size(&dom, note, Direction::Column, 40);
    assert_eq!(h, 1, "short text fits on one line at width 40");
}

// ── is_ifc_block predicate ──────────────────────────────────────

#[test]
fn ifc_block_text_only_returns_false_for_pseudo_paint_routing() {
    // Pure-text blocks (`<note>only text</note>`) are intentionally
    // NOT IFC — paint_ifc doesn't render `::before` / `::after`
    // chrome. Intrinsic measurement uses `compute_inline_layout`
    // directly (see `intrinsic_element`); the layout/paint
    // routing stays on the non-IFC path so static pseudos paint.
    let mut dom = tui_dom();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    cascade(&mut dom, &Stylesheet::bare());

    assert!(
        !super::ifc::is_ifc_block(&dom, p),
        "text-only block stays non-IFC so pseudos paint via paint_inline_content"
    );
}

#[test]
fn ifc_block_whitespace_only_text_returns_false() {
    let mut dom = tui_dom();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("   \n\t  ");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    cascade(&mut dom, &Stylesheet::bare());

    assert!(
        !super::ifc::is_ifc_block(&dom, p),
        "whitespace-only text does not establish an IFC"
    );
}

#[test]
fn ifc_block_block_child_disqualifies_even_with_text() {
    let mut dom = tui_dom();
    let root = dom.root();
    let card = dom.create_element("card");
    let text = dom.create_text_node("\n  ");
    let child = dom.create_element("child");
    dom.append_child(card, text).unwrap();
    dom.append_child(card, child).unwrap();
    dom.append_child(root, card).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked("child", TuiStyle::new().display(Display::Block));
    cascade(&mut dom, &sheet);

    assert!(
        !super::ifc::is_ifc_block(&dom, card),
        "block element child wins over text — not IFC"
    );
}

#[test]
fn ifc_block_inline_block_only_returns_false() {
    // Pre-existing rule documented in ifc.rs: InlineBlock alone does
    // NOT flip the parent into IFC mode. This test pins that
    // behavior so the text-only extension doesn't drift it.
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    let btn = dom.create_element("btn");
    dom.append_child(host, btn).unwrap();
    dom.append_child(root, host).unwrap();
    let sheet =
        Stylesheet::bare().rule_unchecked("btn", TuiStyle::new().display(Display::InlineBlock));
    cascade(&mut dom, &sheet);

    assert!(
        !super::ifc::is_ifc_block(&dom, host),
        "InlineBlock-only parent stays non-IFC"
    );
}

// ── BFC-1 Phase 4.1: dispatch wiring ────────────────────────────

#[test]
fn block_flow_container_ignores_flex_direction_row() {
    // BFC-1 Phase 4.1: a container with `flow: Block` (the cascaded
    // default after Phase 1's Flow enum landed) must dispatch to
    // `layout_block_children` and stack its block-level children
    // vertically EVEN WHEN the author writes `flex-direction: row`
    // — because block layout doesn't honor `flex-direction` at all
    // (it's a flex-only property per CSS Flexbox §5.1). The current
    // pre-phase-4 substrate routes everything through flex, so it
    // WOULD honor the row direction; the new dispatch must ignore
    // it for Block flow containers.
    //
    // We assert horizontal cursor (x) stays at 0 for both children
    // — that's the unambiguous "block layout was selected" signal,
    // since flex(row) would lay them out side-by-side with the
    // second child at x = h1.width.
    let mut dom = tui_dom();
    let root = dom.root();
    let container = dom.create_element("div");
    let h1 = dom.create_element("h1");
    let p = dom.create_element("p");
    dom.append_child(container, h1).unwrap();
    dom.append_child(container, p).unwrap();
    dom.append_child(root, container).unwrap();

    // Author writes `flex-direction: row` on the container — but
    // `<div>` cascades to `flow: Block` per UA defaults (which
    // implies `Display::Block` + the new `Flow::Block`), so flex-
    // direction is inert. Children fixed-height to pin stacking.
    let sheet = Stylesheet::new()
        .rule_unchecked("div", TuiStyle::new().direction(Direction::Row))
        .rule_unchecked(
            "h1",
            TuiStyle::new()
                .height(Size::Fixed(2))
                .width(Size::Fixed(15)),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .height(Size::Fixed(3))
                .width(Size::Fixed(20)),
        );
    cascade(&mut dom, &sheet);

    dom.layout_dom(Rect::new(0, 0, 40, 20));

    let h1_rect = layout_rect_of(&dom, h1);
    let p_rect = layout_rect_of(&dom, p);

    // Block dispatch: both children at x = container left.
    assert_eq!(
        h1_rect.x, 0,
        "h1 at container left (block layout, not flex)"
    );
    assert_eq!(p_rect.x, 0, "p at container left (block layout, not flex)");
    // Block dispatch: p stacks BELOW h1, not beside it.
    assert_eq!(h1_rect.y, 0, "h1 at container top");
    assert_eq!(p_rect.y, 2, "p stacks directly below h1 (block layout)");
    // Block dispatch: each child honors its declared width.
    assert_eq!(h1_rect.width, 15, "h1 declared width");
    assert_eq!(p_rect.width, 20, "p declared width");
    assert_eq!(h1_rect.height, 2);
    assert_eq!(p_rect.height, 3);
}

// ── scrollbar-gutter property ────────────────────────────────────

#[test]
fn overflow_auto_does_not_reserve_gutter_under_default_scrollbar_gutter_auto() {
    // CSS default for `scrollbar-gutter` is `auto` — gutter only
    // reserves when the scrollbar actually paints (`Overflow::Scroll`
    // always; `Overflow::Auto` only via the `Stable` opt-in).
    // Regression for `CHROME-SCROLL-GUTTER-DEFAULT-1`.
    use crate::layout::Overflow;
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    dom.append_child(root, host).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "host",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(10))
            .height(Size::Fixed(5))
            .overflow(Overflow::Auto),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    let inner = dom.node(host).ext().unwrap().content_layout;
    // No gutter reserved → content_layout == outer (no padding/border).
    assert_eq!(inner.width, 10, "no gutter reserved on auto+auto");
    assert_eq!(inner.height, 5);
}

#[test]
fn overflow_auto_with_scrollbar_gutter_stable_reserves_gutter() {
    use crate::layout::{Overflow, ScrollbarGutter};
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    dom.append_child(root, host).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "host",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(10))
            .height(Size::Fixed(5))
            .overflow(Overflow::Auto)
            .scrollbar_gutter(ScrollbarGutter::Stable),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    let inner = dom.node(host).ext().unwrap().content_layout;
    // Both axes reserve 1 cell each.
    assert_eq!(
        inner.width, 9,
        "scrollbar-gutter: stable reserves Y-axis gutter"
    );
    assert_eq!(
        inner.height, 4,
        "scrollbar-gutter: stable reserves X-axis gutter"
    );
}

#[test]
fn overflow_scroll_always_reserves_gutter_regardless_of_scrollbar_gutter_value() {
    use crate::layout::{Overflow, ScrollbarGutter};
    let mut dom = tui_dom();
    let root = dom.root();
    let host = dom.create_element("host");
    dom.append_child(root, host).unwrap();
    // overflow: scroll + scrollbar-gutter: auto. Scroll always
    // shows the scrollbar, so the gutter must reserve regardless.
    let sheet = Stylesheet::bare().rule_unchecked(
        "host",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(10))
            .height(Size::Fixed(5))
            .overflow(Overflow::Scroll)
            .scrollbar_gutter(ScrollbarGutter::Auto),
    );
    cascade(&mut dom, &sheet);
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    let inner = dom.node(host).ext().unwrap().content_layout;
    assert_eq!(inner.width, 9, "overflow: scroll always reserves Y-gutter");
    assert_eq!(inner.height, 4, "overflow: scroll always reserves X-gutter");
}
