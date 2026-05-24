//! M6 — `calc()` end-to-end through the full cascade + layout
//! pipeline. Per `SHOWCASE.md` M6 exit criteria: *"Integration tests
//! in `rdom-tui` for layout against `calc()`-expressed lengths."*
//!
//! Each test builds a DOM, applies a stylesheet containing `calc()`
//! expressions, runs cascade + layout, and asserts the resulting
//! `LayoutRect` matches what CSS Values L3 + CSS Box Model §8.4
//! prescribe.

use rdom_style::layout::Size;
use rdom_tui::TuiDom;
use rdom_tui::layout::{LayoutRect, Length};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{LayoutExt, Rect};
use rdom_tui::style::CascadeExt;

/// Mount a single `<child>` inside a fixed-size `<parent>`, apply
/// the stylesheet, cascade + layout, return the child's resolved
/// `LayoutRect`.
fn layout_child(parent_css: &str, child_css: &str, viewport: Rect) -> LayoutRect {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let css = format!("parent {{ {} }} child {{ {} }}", parent_css, child_css);
    let sheet = rdom_css::from_css(&css);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);

    dom.node(child)
        .tui_ext()
        .map(|e| e.layout)
        .expect("child has layout")
}

#[test]
fn calc_full_minus_constant_width() {
    // `width: calc(100% - 4)` inside a 40-cell-wide parent → 36 cells.
    let rect = layout_child(
        "width: 40; height: 10;",
        "width: calc(100% - 4); height: 5;",
        Rect::new(0, 0, 60, 20),
    );
    assert_eq!(rect.width, 36, "calc(100% - 4) of 40 = 36");
    assert_eq!(rect.height, 5);
}

#[test]
fn calc_half_plus_constant_width() {
    // `width: calc(50% + 2)` inside a 40-cell parent → 22.
    let rect = layout_child(
        "width: 40; height: 10;",
        "width: calc(50% + 2); height: 5;",
        Rect::new(0, 0, 60, 20),
    );
    assert_eq!(rect.width, 22);
}

#[test]
fn calc_quarter_height() {
    // `height: calc(100% / 4)` inside a 20-cell-tall parent → 5.
    let rect = layout_child(
        "width: 10; height: 20;",
        "width: 5; height: calc(100% / 4);",
        Rect::new(0, 0, 30, 30),
    );
    assert_eq!(rect.height, 5);
}

#[test]
fn calc_percent_clamps_negative_to_zero() {
    // `width: calc(50% - 100)` of 40 → -80 → clamped to 0.
    let rect = layout_child(
        "width: 40; height: 10;",
        "width: calc(50% - 100); height: 3;",
        Rect::new(0, 0, 60, 20),
    );
    assert_eq!(rect.width, 0);
}

#[test]
fn calc_in_absolute_positioning_left_resolves_against_parent_width() {
    // `position: absolute; left: calc(50% - 4)` against a 40-wide
    // containing block → x = parent.x + 16.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = rdom_css::from_css(
        "parent { width: 40; height: 20; position: relative; } \
         child { width: 8; height: 5; position: absolute; left: calc(50% - 4); top: 2; }",
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 30));

    let child_rect = dom.node(child).tui_ext().map(|e| e.layout).unwrap();
    // Parent at x=0; child.left = 50% of 40 - 4 = 16. Absolute
    // positioning places child at containing-block-x + left.
    assert_eq!(
        child_rect.x, 16,
        "absolute child's left: calc(50% - 4) of 40 → 16"
    );
    assert_eq!(child_rect.y, 2);
}

#[test]
fn calc_in_relative_position_shifts_by_percentage() {
    // `position: relative; left: calc(25% + 1)` inside a 40-wide
    // parent → shift +11 cells from the natural in-flow position.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = rdom_css::from_css(
        "parent { width: 40; height: 20; } \
         child { width: 5; height: 5; position: relative; left: calc(25% + 1); }",
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 30));

    let child_rect = dom.node(child).tui_ext().map(|e| e.layout).unwrap();
    // Natural in-flow position is parent.x (= 0). Shift = 25% of
    // 40 + 1 = 11. Resulting x = 11.
    assert_eq!(child_rect.x, 11);
}

#[test]
fn constant_calc_padding_resolves_at_parse_time() {
    // `padding: calc(2 * 3)` is constant — resolves at parse time.
    // Apply to a parent; child's in-flow rect should reflect the
    // 6-cell padding on each side.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = rdom_css::from_css(
        "parent { width: 40; height: 20; padding: calc(2 * 3); } \
         child { width: 10; height: 5; }",
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 30));

    let child_rect = dom.node(child).tui_ext().map(|e| e.layout).unwrap();
    // Padding 6 on all sides → child starts at (6, 6).
    assert_eq!(child_rect.x, 6);
    assert_eq!(child_rect.y, 6);
}

#[test]
fn calc_top_in_absolute_resolves_against_parent_height() {
    // `top: calc(25% + 3)` of a 20-tall parent → 5 + 3 = 8.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let sheet = rdom_css::from_css(
        "parent { width: 40; height: 20; position: relative; } \
         child { width: 5; height: 3; position: absolute; left: 0; top: calc(25% + 3); }",
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 30));

    let child_rect = dom.node(child).tui_ext().map(|e| e.layout).unwrap();
    assert_eq!(child_rect.y, 8);
}

#[test]
fn calc_nested_resolves_correctly() {
    // `width: calc(calc(50% + 4) - 2)` of 40 → (20+4)-2 = 22.
    let rect = layout_child(
        "width: 40; height: 10;",
        "width: calc(calc(50% + 4) - 2); height: 5;",
        Rect::new(0, 0, 60, 20),
    );
    assert_eq!(rect.width, 22);
}

#[test]
fn calc_inside_paint_pipeline_doesnt_panic() {
    // End-to-end sanity: a percent-bearing calc that goes through
    // cascade + layout + paint without panicking.
    use rdom_tui::App;
    use rdom_tui::render::{Terminal, TestBackend};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer");
    let inner = dom.create_element("inner");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = rdom_css::from_css(
        "outer { width: 30; height: 10; } \
         inner { width: calc(100% - 4); height: calc(100% - 2); }",
    );
    let backend = TestBackend::new(40, 20);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty()
        .expect("paint succeeds with calc layout");

    // Length::Auto / Length::Calc both render with `Length` type
    // path — pin import so the use is exercised.
    let _ = Length::Auto;
    let _ = Size::Auto;
}
