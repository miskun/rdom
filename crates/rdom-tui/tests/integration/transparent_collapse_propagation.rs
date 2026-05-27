//! Direct-children-only collapse semantics (BORDER-MODEL-1).
//!
//! Historically this file pinned the **transparent intermediate
//! propagation** rule: a borderless container would pass border-
//! sharing signals up from its bordered descendants, so the
//! chrome's `<header>` ↔ `<sidebar>`/`<main>` overlap fired
//! through an intervening `<body>` without its own border.
//!
//! BORDER-MODEL-1 retired that propagation. `border-collapse` is
//! now non-inheriting, and within a collapse-declaring container
//! only **direct children** participate. A borderless intermediate
//! that wants its bordered grandchildren to share with the outer
//! group must either (a) declare its own border so it can share
//! with its sibling under the outer collapse, or (b) restructure
//! so the bordered elements are direct siblings of the outer
//! collapse-declaring container.
//!
//! These tests pin the new direct-only semantics.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

#[test]
fn direct_bordered_siblings_share_under_collapse() {
    // Canonical pattern: collapse on parent + two direct children
    // both bordered → outer rects overlap by 1 cell at the shared
    // edge. Mirrors the showcase chrome's `.app-body { display:
    // flex; border-collapse: collapse } > .sidebar + .main`.
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
                   border-collapse: collapse; }
        a_el     { height: 2; border: solid; }
        b_el     { flex: 1; border: solid; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 5));

    let b_rect = dom.node(b).layout_rect().expect("b laid out");
    assert_eq!(
        b_rect.y, 1,
        "b's outer top must coincide with a's outer bottom (sibling overlap); got {b_rect:?}"
    );
}

#[test]
fn borderless_intermediate_does_not_propagate_borders_upward() {
    // BORDER-MODEL-1's central simplification: a borderless
    // container does NOT inherit "effective border" from its
    // bordered children. Under collapse on the outer container,
    // sibling-overlap fires only when both DIRECT children actually
    // have a border on the shared edge.
    //
    // Setup:
    //   outer (collapse + border)
    //     ├ header (border)
    //     └ body  (no border)
    //         ├ sidebar (border)
    //         └ main    (border)
    //
    // Old (recursive) behavior: body's `has_effective_border_on_edge(Top)`
    // returned true via sidebar/main → sibling-overlap fired →
    // body.y == 2 (overlapping header).
    //
    // New (direct-only) behavior: body has no border.top → no
    // overlap → body.y == 3 (just after header). Authors who want
    // the old behavior either give body its own border or
    // restructure so sidebar/main are direct children of outer.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer_el");
    let header = dom.create_element("header_el");
    let body = dom.create_element("body_el");
    let sidebar = dom.create_element("sidebar_el");
    let main = dom.create_element("main_el");
    dom.append_child(root, outer).unwrap();
    dom.append_child(outer, header).unwrap();
    dom.append_child(outer, body).unwrap();
    dom.append_child(body, sidebar).unwrap();
    dom.append_child(body, main).unwrap();

    let css = r#"
        outer_el   { display: flex;
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

    let body_rect = dom.node(body).layout_rect().expect("body laid out");
    assert_eq!(
        body_rect.y, 3,
        "body should NOT overlap header — body has no own border so direct-only \
         collapse can't share through it; got {body_rect:?}"
    );
}

#[test]
fn borderless_intermediate_with_no_bordered_descendants_dont_overlap() {
    // Inverse sanity: when the borderless intermediate has no
    // bordered descendants at all, there's nothing the old model
    // would have shared either. Today's behavior unchanged.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app_el");
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
        app_el     { display: flex;
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

    assert_eq!(header_rect.height, 3);
    assert_eq!(
        body_rect.y, 3,
        "body should NOT overlap header — no bordered descendants; got {body_rect:?}"
    );
}
