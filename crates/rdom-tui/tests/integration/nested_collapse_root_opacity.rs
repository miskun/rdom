//! Nested `border-collapse: collapse` collapse-root opacity.
//!
//! Per CSS 2.1 §17.6.2.1 the `<table>` element is the boundary of
//! its collapse group: nothing outside the table collapses with its
//! borders, and nested `<table>` elements form their own self-
//! contained groups. rdom extends `border-collapse: collapse` to
//! non-table elements (DIVERGENCES.md), so the spec's table-equals-
//! boundary rule translates to: an element that DECLARES
//! `border-collapse: collapse` (rather than just inheriting it) is
//! a sealed sub-group — its outer border is opaque to the outer
//! collapse group via transparent intermediates.
//!
//! Surfaced by the showcase: the border-collapse demo's outer ring
//! was fusing with `<main>`'s ring (T-joints visible at `<main>`'s
//! top + bottom) and dragging `<details class="source-disclosure">`
//! against `<main>`'s left border (lost 1-cell content inset, `┼`
//! glyph at the disclosure's top-left). The chrome had a single
//! collapse group spanning `.app → <main> → view-content →
//! .border-collapse-demo → cols/rows`.

use rdom_tui::App;
use rdom_tui::render::{Rect, Terminal, TestBackend};
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

#[test]
fn declared_collapse_child_keeps_outer_content_inset() {
    // `outer` declares `border-collapse: collapse` + own border.
    // `mid` is a transparent intermediate (no border).
    // `inner` ALSO declares `border-collapse: collapse` + own
    // border — it's a sealed sub-group, equivalent to a nested
    // `<table>` inside a `<td>`.
    //
    // Expected (collapse-root model): `outer`'s parent-edge inset
    // for `mid` should be (1, 1, 1, 1) — `inner` is opaque, so
    // `outer` reserves its own content inset and `inner` sits
    // INSIDE `outer`'s content area, not coincident with its
    // border ring.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer_el");
    let mid = dom.create_element("mid_el");
    let inner = dom.create_element("inner_el");
    dom.append_child(root, outer).unwrap();
    dom.append_child(outer, mid).unwrap();
    dom.append_child(mid, inner).unwrap();

    let css = r#"
        outer_el { display: flex;
 width: 20; height: 10; flex-direction: column;
                   border: solid; border-collapse: collapse; }
        mid_el   { display: flex;
 flex: 1; flex-direction: column; }
        inner_el { display: flex;
 flex: 1; flex-direction: column;
                   border: solid; border-collapse: collapse; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 10));

    let outer_rect = dom.node(outer).layout_rect().expect("outer laid out");
    let inner_rect = dom.node(inner).layout_rect().expect("inner laid out");

    // outer's outer rect is the full viewport (0,0,20,10).
    assert_eq!((outer_rect.x, outer_rect.y), (0, 0));
    assert_eq!((outer_rect.width, outer_rect.height), (20, 10));

    // inner's outer rect should sit STRICTLY inside outer's outer
    // rect — at least 1 cell of inset on every edge because outer's
    // border ring should not be shared with inner (inner is a
    // sealed collapse-root, opaque to the outer group).
    assert!(
        inner_rect.x > outer_rect.x,
        "inner.x must be at least 1 cell inside outer's left border; \
         got inner={inner_rect:?} outer={outer_rect:?}"
    );
    assert!(
        inner_rect.y > outer_rect.y,
        "inner.y must be at least 1 cell inside outer's top border; \
         got inner={inner_rect:?} outer={outer_rect:?}"
    );
    assert!(
        inner_rect.x + (inner_rect.width as i32) < outer_rect.x + (outer_rect.width as i32),
        "inner's right edge must be at least 1 cell inside outer's right border; \
         got inner={inner_rect:?} outer={outer_rect:?}"
    );
    assert!(
        inner_rect.y + (inner_rect.height as i32) < outer_rect.y + (outer_rect.height as i32),
        "inner's bottom edge must be at least 1 cell inside outer's bottom border; \
         got inner={inner_rect:?} outer={outer_rect:?}"
    );
}

#[test]
fn declared_collapse_child_paints_outer_corners_as_clean_corners() {
    // Paint-side companion: render the structure to a TestBackend
    // and assert that outer's four corner cells are clean square-
    // corner glyphs (┌┐└┘), NOT T-junctions (┬┴├┤) — that would
    // mean inner's edges fused with outer's edges.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("outer_el");
    let mid = dom.create_element("mid_el");
    let inner = dom.create_element("inner_el");
    dom.append_child(root, outer).unwrap();
    dom.append_child(outer, mid).unwrap();
    dom.append_child(mid, inner).unwrap();

    let sheet = rdom_css::from_css(
        r#"
        outer_el { display: flex;
 width: 100%; height: 100%; flex-direction: column;
                   border: solid; border-collapse: collapse; }
        mid_el   { display: flex;
 flex: 1; flex-direction: column; }
        inner_el { display: flex;
 flex: 1; flex-direction: column;
                   border: solid; border-collapse: collapse; }
        "#,
    );

    let backend = TestBackend::new(20, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app_rt = App::with_backend(dom, sheet, terminal).unwrap();
    app_rt.draw_if_dirty().unwrap();

    let bytes = app_rt.terminal().backend().bytes();
    let s = String::from_utf8_lossy(bytes);

    // outer's top edge — full horizontal `─` plus square corners.
    // Without the fix, inner's left/right edges intrude and produce
    // `┬` joints somewhere along the top row.
    assert!(
        !s.contains("┬"),
        "outer's top border must not contain T-junctions from inner's edges; \
         got rendered output:\n{s}"
    );
    assert!(
        !s.contains("┴"),
        "outer's bottom border must not contain T-junctions from inner's edges; \
         got rendered output:\n{s}"
    );
}

#[test]
fn inherited_collapse_child_still_propagates_through_transparent_intermediate() {
    // Regression guard: the chrome's `<header>` ↔ `<sidebar>`
    // overlap-through-`<body>` pattern relies on inherited
    // `border-collapse: collapse` propagating through borderless
    // intermediates. The collapse-root opacity rule must ONLY fire
    // on elements that DECLARE the property — pure inheritance
    // must keep the existing transparent-intermediate behavior.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app_el");
    let header = dom.create_element("header_el");
    let body = dom.create_element("body_el");
    let sidebar = dom.create_element("sidebar_el");
    let main = dom.create_element("main_el");
    dom.append_child(root, app).unwrap();
    dom.append_child(app, header).unwrap();
    dom.append_child(app, body).unwrap();
    dom.append_child(body, sidebar).unwrap();
    dom.append_child(body, main).unwrap();

    // `app` declares collapse — root of the group.
    // `header`, `body`, `sidebar`, `main` only INHERIT it.
    // Expected: `header` ↔ `sidebar`/`main` still overlap by 1 row
    // through the transparent `body` (matches today's behavior).
    let css = r#"
        app_el     { display: flex;
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
    let sidebar_rect = dom.node(sidebar).layout_rect().expect("sidebar laid out");
    let main_rect = dom.node(main).layout_rect().expect("main laid out");

    assert_eq!(header_rect.y, 0);
    assert_eq!(
        sidebar_rect.y, 2,
        "sidebar should overlap header's bottom border via transparent body \
         (inherited-only collapse must still propagate); got {sidebar_rect:?}"
    );
    assert_eq!(
        main_rect.y, 2,
        "main should overlap header's bottom border via transparent body \
         (inherited-only collapse must still propagate); got {main_rect:?}"
    );
}
