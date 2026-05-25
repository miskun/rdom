//! `flex: <n>` shorthand — parses + actually fills remaining flex
//! space. Surfaces the canonical CSS idiom for "fill remaining
//! space in flex," which is what every CSS author reaches for
//! when they want a chrome layout's body to fill what the header
//! leaves behind.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

fn layout_at(mut html_shape: impl FnMut(&mut TuiDom), css: &str, viewport: Rect) -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    html_shape(&mut dom);
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    dom
}

fn rect(dom: &TuiDom, id: rdom_tui::NodeId) -> rdom_tui::layout::LayoutRect {
    dom.node(id).layout_rect().expect("laid out")
}

#[test]
fn flex_one_fills_remaining_space_in_column_parent() {
    // The chrome canonical pattern. Parent column-flex with a
    // fixed-height header and a body that should fill the rest.
    let mut body_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let app = dom.create_element("app");
            let header = dom.create_element("header_el");
            let body = dom.create_element("body_el");
            body_id = Some(body);
            dom.append_child(root, app).unwrap();
            dom.append_child(app, header).unwrap();
            dom.append_child(app, body).unwrap();
        },
        r#"
            app        { width: 80; height: 24; flex-direction: column; }
            header_el  { height: 3; }
            body_el    { flex: 1; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    let body = body_id.unwrap();
    let body_rect = rect(&dom, body);
    assert_eq!(
        body_rect.height, 21,
        "body should fill remaining = 24 - 3 = 21, got {body_rect:?}"
    );
}

#[test]
fn flex_one_fills_remaining_space_in_row_parent() {
    // Mirror of the column case. Parent row-flex with a fixed-
    // width sidebar and a main column that should fill the rest.
    let mut main_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let row = dom.create_element("row_el");
            let sidebar = dom.create_element("side_el");
            let main = dom.create_element("main_el");
            main_id = Some(main);
            dom.append_child(root, row).unwrap();
            dom.append_child(row, sidebar).unwrap();
            dom.append_child(row, main).unwrap();
        },
        r#"
            row_el    { width: 80; height: 10; flex-direction: row; }
            side_el   { width: 28; }
            main_el   { flex: 1; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    let main = main_id.unwrap();
    let main_rect = rect(&dom, main);
    assert_eq!(
        main_rect.width, 52,
        "main should fill remaining = 80 - 28 = 52, got {main_rect:?}"
    );
}

#[test]
fn flex_none_falls_back_to_auto() {
    // `flex: none` is CSS shorthand for `flex: 0 0 auto` — the
    // child does not grow, does not shrink, basis is content.
    // In rdom that maps to `Size::Auto` on the main axis (intrinsic).
    let mut child_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let parent = dom.create_element("parent");
            let child = dom.create_element("child");
            let text = dom.create_text_node("hi");
            dom.append_child(root, parent).unwrap();
            dom.append_child(parent, child).unwrap();
            dom.append_child(child, text).unwrap();
            child_id = Some(child);
        },
        r#"
            parent { width: 80; height: 10; flex-direction: row; }
            child  { flex: none; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    let child = child_id.unwrap();
    let child_rect = rect(&dom, child);
    assert_eq!(
        child_rect.width, 2,
        "child should size to intrinsic content (\"hi\" = 2 cells), got {child_rect:?}"
    );
}

#[test]
fn flex_auto_grows_like_flex_one() {
    // `flex: auto` = `1 1 auto` — grows like `flex: 1` in our
    // simplified model (basis differences ignored until full
    // flex-grow / flex-shrink tracking lands).
    let mut child_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let parent = dom.create_element("parent");
            let fixed = dom.create_element("fixed_el");
            let child = dom.create_element("auto_el");
            dom.append_child(root, parent).unwrap();
            dom.append_child(parent, fixed).unwrap();
            dom.append_child(parent, child).unwrap();
            child_id = Some(child);
        },
        r#"
            parent   { width: 80; height: 10; flex-direction: row; }
            fixed_el { width: 30; }
            auto_el  { flex: auto; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    let child = child_id.unwrap();
    let child_rect = rect(&dom, child);
    assert_eq!(
        child_rect.width, 50,
        "child should fill remaining = 80 - 30 = 50, got {child_rect:?}"
    );
}

#[test]
fn flex_two_takes_double_share() {
    // `flex: 2` should give the child twice the share of remaining
    // space compared to `flex: 1`. Two flex-1 siblings + one
    // flex-2 sibling: shares are 1:1:2 of remaining space.
    let mut a_id = None;
    let mut b_id = None;
    let mut c_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let parent = dom.create_element("parent");
            let a = dom.create_element("a");
            let b = dom.create_element("b");
            let c = dom.create_element("c");
            dom.append_child(root, parent).unwrap();
            dom.append_child(parent, a).unwrap();
            dom.append_child(parent, b).unwrap();
            dom.append_child(parent, c).unwrap();
            a_id = Some(a);
            b_id = Some(b);
            c_id = Some(c);
        },
        r#"
            parent { width: 80; height: 10; flex-direction: row; }
            a      { flex: 1; }
            b      { flex: 1; }
            c      { flex: 2; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    assert_eq!(rect(&dom, a_id.unwrap()).width, 20, "a = 80 * 1/4 = 20");
    assert_eq!(rect(&dom, b_id.unwrap()).width, 20, "b = 80 * 1/4 = 20");
    assert_eq!(rect(&dom, c_id.unwrap()).width, 40, "c = 80 * 2/4 = 40");
}
