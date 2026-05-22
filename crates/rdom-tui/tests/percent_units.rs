//! `Size::Percent` resolution — end-to-end through CSS parser +
//! cascade + layout.
//!
//! The substrate previously dropped `%` units as "non-cell" along
//! with `px`/`em`/`rem`/`ch`. That divergence was wrong: `%` is
//! *relative to parent dimensions*, not to pixels or font sizes,
//! and resolves naturally at layout time. Restored as a first-class
//! sizing unit in 0.2.0 M2; these tests pin the resolution.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

fn layout_width_of(dom: &TuiDom, id: rdom_tui::NodeId) -> u16 {
    dom.node(id).layout_rect().expect("laid out").width
}

fn layout_height_of(dom: &TuiDom, id: rdom_tui::NodeId) -> u16 {
    dom.node(id).layout_rect().expect("laid out").height
}

#[test]
fn percent_width_main_axis_resolves_against_parent_content() {
    // Parent: row direction, width 80 (cells). Child: width: 50%.
    // Expected: child main-axis (width in row) = 80 * 0.5 = 40.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent { width: 80; height: 10; flex-direction: row; }
        child { width: 50%; height: 5; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 30));

    assert_eq!(
        layout_width_of(&dom, child),
        40,
        "child width = 50% × 80 = 40"
    );
}

#[test]
fn percent_height_main_axis_resolves_against_parent_content() {
    // Parent: column direction, height 20. Child: height 25%.
    // Expected: child main-axis (height in column) = 20 * 0.25 = 5.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent { width: 30; height: 20; flex-direction: column; }
        child { width: 30; height: 25%; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 30));

    assert_eq!(
        layout_height_of(&dom, child),
        5,
        "child height = 25% × 20 = 5"
    );
}

#[test]
fn percent_cross_axis_resolves_against_container_cross() {
    // Parent: row direction (cross = height), height 20.
    // Child: height 50% (cross axis for a row child).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let child = dom.create_element("child");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent { width: 40; height: 20; flex-direction: row; }
        child { width: 10; height: 50%; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 30));

    assert_eq!(
        layout_height_of(&dom, child),
        10,
        "child cross-axis (height) = 50% × 20 = 10"
    );
}

#[test]
fn percent_100_fills_parent_content_area() {
    // The showcase pattern: shell at 100% × 100%, header at fixed
    // height 3, body at 100% × 100% fills the rest. Verifies the
    // common "fill parent" use case end-to-end.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app");
    let body = dom.create_element("body");
    dom.append_child(root, app).unwrap();
    dom.append_child(app, body).unwrap();

    let css = r#"
        app { width: 100%; height: 100%; flex-direction: column; }
        body { width: 100%; height: 100%; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    let viewport = Rect::new(0, 0, 60, 24);
    dom.layout_dom(viewport);

    let app_rect = dom.node(app).layout_rect().expect("app laid out");
    assert_eq!(
        (app_rect.width, app_rect.height),
        (60, 24),
        "app at 100%×100% fills the viewport"
    );
    let body_rect = dom.node(body).layout_rect().expect("body laid out");
    assert_eq!(
        (body_rect.width, body_rect.height),
        (60, 24),
        "body at 100%×100% fills its parent (app)"
    );
}

#[test]
fn percent_alongside_flex_distributes_correctly() {
    // Parent width 100, three children: width 30%, 40, 1fr.
    // Expected: child A = 30, child B = 40, child C = remaining = 30.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(parent, c).unwrap();

    let css = r#"
        parent { width: 100; height: 10; flex-direction: row; }
        a { width: 30%; height: 5; }
        b { width: 40; height: 5; }
        c { width: 1fr; height: 5; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 100, 30));

    assert_eq!(layout_width_of(&dom, a), 30, "a = 30% × 100");
    assert_eq!(layout_width_of(&dom, b), 40, "b = fixed 40");
    assert_eq!(
        layout_width_of(&dom, c),
        30,
        "c = remaining 30 (100 - 30 - 40)"
    );
}
