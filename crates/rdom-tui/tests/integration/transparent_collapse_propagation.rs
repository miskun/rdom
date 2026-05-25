//! Transparent intermediate propagation for `border-collapse: collapse`.
//!
//! Surfaced by the M2 chrome visual: the canonical pattern
//! `<outer border collapse> > <header border> + <body no-border> >
//! <sidebar border> + <main border>` previously rendered the
//! header's bottom border and sidebar/main's top borders as TWO
//! adjacent rows (not shared) because the borderless `<body>`
//! between them broke the sibling-overlap chain. CSS tables
//! handle this — `<tbody>` and `<tr>` are transparent for
//! collapse purposes. rdom's extension of `border-collapse` to
//! any flex container now propagates collapse-sharing through
//! borderless container intermediates too.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

#[test]
fn header_and_body_share_border_through_transparent_intermediate() {
    // The canonical TUI shell shape. <body> has no own border but
    // contains <sidebar> + <main> which both do. Under collapse,
    // <header>'s bottom should share a row with the descendants'
    // tops through the transparent <body>.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app");
    let header = dom.create_element("header_el");
    let body = dom.create_element("body_el");
    let sidebar = dom.create_element("sidebar_el");
    let main = dom.create_element("main_el");
    dom.append_child(root, app).unwrap();
    dom.append_child(app, header).unwrap();
    dom.append_child(app, body).unwrap();
    dom.append_child(body, sidebar).unwrap();
    dom.append_child(body, main).unwrap();

    let css = r#"
        app        { display: flex;
 width: 80; height: 24; flex-direction: column;
                     border: solid; border-collapse: collapse; }
        header_el  { height: 3; border: solid; }
        body_el    { display: flex;
 flex: 1; flex-direction: row; }
        sidebar_el { width: 28; border: solid; }
        main_el    { flex: 1; border: solid; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let header_rect = dom.node(header).layout_rect().expect("header laid out");
    let body_rect = dom.node(body).layout_rect().expect("body laid out");
    let sidebar_rect = dom.node(sidebar).layout_rect().expect("sidebar laid out");
    let main_rect = dom.node(main).layout_rect().expect("main laid out");

    // Header occupies rows 0..3 (its own outer rect). Its bottom
    // border paints at row 2.
    assert_eq!(header_rect.y, 0, "header at top");
    assert_eq!(header_rect.height, 3, "header height 3");

    // Body should overlap with header by 1 row — its outer top is
    // at row 2 (same as header's bottom border). Without the
    // transparent-intermediate fix, body's y would be 3.
    assert_eq!(
        body_rect.y, 2,
        "body's outer rect should start at header's bottom border row \
         (sibling overlap via transparent intermediate); got {body_rect:?}"
    );

    // Sidebar and main inside body — they have their own borders,
    // so they share with header's bottom too.
    assert_eq!(sidebar_rect.y, 2, "sidebar's top at the shared border row");
    assert_eq!(main_rect.y, 2, "main's top at the shared border row");
}

#[test]
fn header_and_borderless_intermediate_with_no_bordered_descendants_dont_overlap() {
    // Sanity check the inverse: if the borderless intermediate
    // contains only content-bearing leaves (text), the
    // transparent propagation should NOT trigger an overlap —
    // there's nothing to share with. Header bottom and body
    // intermediate's outer should stay at adjacent (non-shared) rows.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app");
    let header = dom.create_element("header_el");
    let body = dom.create_element("body_el");
    let text_child = dom.create_element("text_child");
    let text = dom.create_text_node("hi");
    dom.append_child(root, app).unwrap();
    dom.append_child(app, header).unwrap();
    dom.append_child(app, body).unwrap();
    dom.append_child(body, text_child).unwrap();
    dom.append_child(text_child, text).unwrap();

    let css = r#"
        app        { display: flex;
 width: 30; height: 10; flex-direction: column;
                     border: solid; border-collapse: collapse; }
        header_el  { height: 3; border: solid; }
        body_el    { display: flex;
 flex: 1; flex-direction: column; }
        text_child { height: 1; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));

    let header_rect = dom.node(header).layout_rect().expect("header laid out");
    let body_rect = dom.node(body).layout_rect().expect("body laid out");

    // No bordered descendants in body — no border to share. Body's
    // outer should sit just after header without overlap.
    assert_eq!(header_rect.height, 3);
    assert_eq!(
        body_rect.y, 3,
        "body should NOT overlap header — no bordered descendants to share with; got {body_rect:?}"
    );
}

#[test]
fn directly_bordered_sibling_still_overlaps() {
    // Regression check: the pre-existing "both immediate siblings
    // have borders" case still triggers overlap. (Same shape the
    // M5.5b collapse_two_bordered_siblings test pins, replicated
    // here to ensure the helper-based logic doesn't regress it.)
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer_el");
    let a = dom.create_element("a_el");
    let b = dom.create_element("b_el");
    dom.append_child(root, outer).unwrap();
    dom.append_child(outer, a).unwrap();
    dom.append_child(outer, b).unwrap();

    let css = r#"
        outer_el { display: flex;
 width: 20; height: 5; flex-direction: column;
                   border: solid; border-collapse: collapse; }
        a_el     { height: 2; border: solid; }
        b_el     { flex: 1; border: solid; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 5));

    let b_rect = dom.node(b).layout_rect().expect("b laid out");
    // a is at (0, 0, 20, 2) — bottom border at row 1. b's outer
    // top should be at row 1 (sibling overlap, both have borders).
    assert_eq!(
        b_rect.y, 1,
        "b should overlap a — both direct siblings have borders; got {b_rect:?}"
    );
}
