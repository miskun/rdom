//! `flex-shrink` — when total declared sizes exceed the main
//! axis budget, items shrink proportional to `flex_shrink * basis`
//! instead of overflowing the parent. Default `flex_shrink: 1`
//! per CSS spec. `flex-shrink: 0` opts out (item keeps declared
//! size and contributes to overflow).
//!
//! Closes Finding 4 of the M2 visual review: `height: 100%` on a
//! flex child alongside a fixed-size sibling no longer silently
//! overflows the viewport — it shrinks to fit.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

fn layout_at(mut html: impl FnMut(&mut TuiDom), css: &str, viewport: Rect) -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    html(&mut dom);
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    dom
}

#[test]
fn percent_height_with_fixed_sibling_shrinks_to_fit() {
    // The chrome canonical pattern. Header is Fixed(3), body uses
    // `height: 100%` which resolves to parent's full content
    // height — total exceeds the budget. With flex-shrink default
    // 1 on both, body shrinks proportionally to fit.
    let mut header_id = None;
    let mut body_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let app = dom.create_element("app");
            let header = dom.create_element("header_el");
            let body = dom.create_element("body_el");
            dom.append_child(root, app).unwrap();
            dom.append_child(app, header).unwrap();
            dom.append_child(app, body).unwrap();
            header_id = Some(header);
            body_id = Some(body);
        },
        r#"
            app       { width: 80; height: 24; flex-direction: column; }
            header_el { height: 3; }
            body_el   { height: 100%; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    let header_rect = dom
        .node(header_id.unwrap())
        .layout_rect()
        .expect("header laid out");
    let body_rect = dom
        .node(body_id.unwrap())
        .layout_rect()
        .expect("body laid out");

    // Both children participate in shrink with shrink:1 each. Sum
    // is 3 + 24 = 27; budget = 24; overflow = 3. Distribution by
    // basis * shrink: header takes 3/27, body takes 24/27 of the
    // overflow. Header shrinks by ~0.33 → 0, body shrinks by
    // ~2.67 → 3 (Bresenham keeps the total exact at 3).
    assert_eq!(header_rect.height, 3, "header keeps 3 (shrunk to floor)");
    assert_eq!(body_rect.height, 21, "body shrinks 24 → 21");
    assert_eq!(
        header_rect.height + body_rect.height,
        24,
        "total fits the parent's budget"
    );
}

#[test]
fn flex_shrink_zero_protects_size() {
    // Sidebar uses `flex-shrink: 0` — should keep its declared
    // 28-cell width even when the row overflows. The flex sibling
    // (using `flex: 1`) takes the remaining space.
    let mut sidebar_id = None;
    let mut main_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let row = dom.create_element("row");
            let sidebar = dom.create_element("sidebar_el");
            let main = dom.create_element("main_el");
            dom.append_child(root, row).unwrap();
            dom.append_child(row, sidebar).unwrap();
            dom.append_child(row, main).unwrap();
            sidebar_id = Some(sidebar);
            main_id = Some(main);
        },
        r#"
            row        { width: 80; height: 10; flex-direction: row; }
            sidebar_el { width: 28; flex-shrink: 0; }
            main_el    { flex: 1; }
        "#,
        Rect::new(0, 0, 80, 24),
    );
    let sidebar_rect = dom
        .node(sidebar_id.unwrap())
        .layout_rect()
        .expect("sidebar laid out");
    let main_rect = dom
        .node(main_id.unwrap())
        .layout_rect()
        .expect("main laid out");

    assert_eq!(
        sidebar_rect.width, 28,
        "sidebar at fixed 28 (flex-shrink: 0 protects)"
    );
    assert_eq!(main_rect.width, 52, "main fills remaining = 80 - 28");
}

#[test]
fn shrink_respects_min_width() {
    // With `min-width` set, shrinking can't take a child below its
    // floor. Item stops shrinking at min; remaining overflow stays
    // as overflow (matches CSS spec).
    let mut aa_id = None;
    let mut bb_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let parent = dom.create_element("parent");
            let aa = dom.create_element("aa");
            let bb = dom.create_element("bb");
            dom.append_child(root, parent).unwrap();
            dom.append_child(parent, aa).unwrap();
            dom.append_child(parent, bb).unwrap();
            aa_id = Some(aa);
            bb_id = Some(bb);
        },
        r#"
            parent { width: 20; height: 5; flex-direction: row; }
            aa     { width: 30; min-width: 10; }
            bb     { width: 30; }
        "#,
        Rect::new(0, 0, 50, 10),
    );
    let aa_rect = dom.node(aa_id.unwrap()).layout_rect().expect("aa laid out");
    let bb_rect = dom.node(bb_id.unwrap()).layout_rect().expect("bb laid out");

    // Total declared = 60, budget = 20, overflow = 40. Both shrink
    // proportionally. aa's min-width: 10 floor; bb has no min.
    assert!(
        aa_rect.width >= 10,
        "aa doesn't shrink below min-width: 10 (got {aa_rect:?})"
    );
    assert!(bb_rect.width <= 20, "bb shrinks (got {bb_rect:?})");
}

#[test]
fn no_overflow_means_no_shrink() {
    // Sanity check the inverse: when total fits, nothing shrinks.
    // Custom tags (avoiding UA-defined `<a>` which is
    // `display: inline`).
    let mut aa_id = None;
    let mut bb_id = None;
    let dom = layout_at(
        |dom| {
            let root = dom.root();
            let parent = dom.create_element("parent");
            let aa = dom.create_element("aa");
            let bb = dom.create_element("bb");
            dom.append_child(root, parent).unwrap();
            dom.append_child(parent, aa).unwrap();
            dom.append_child(parent, bb).unwrap();
            aa_id = Some(aa);
            bb_id = Some(bb);
        },
        r#"
            parent { width: 50; height: 5; flex-direction: row; }
            aa     { width: 10; }
            bb     { width: 10; }
        "#,
        Rect::new(0, 0, 50, 10),
    );
    assert_eq!(dom.node(aa_id.unwrap()).layout_rect().unwrap().width, 10);
    assert_eq!(dom.node(bb_id.unwrap()).layout_rect().unwrap().width, 10);
}
