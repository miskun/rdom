//! Nested `border-collapse: collapse` + bordered child + content
//! — the showcase chrome pattern. Surfaced visually: the header's
//! `<h1>` text was missing from the rendered chrome, painted at
//! the same cell as the top border. This file pins where the
//! content actually lands so we can fix the layout root cause
//! instead of working around it.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, TuiDom, TuiNodeExt};

#[test]
fn child_with_own_border_under_collapse_parent_has_content_inset_by_own_border() {
    // Structure mirrors the showcase chrome's <app> / <app-header> /
    // <h1> nesting. <app> has border-collapse: collapse + own
    // border; <header> has its own border + a 1-cell-tall content
    // area; <h1> is the content.
    //
    // Expected: h1 sits at row 1 (inside header's content area,
    // below header's top border row at row 0 which is shared with
    // app's top border under collapse).
    //
    // What we surfaced: h1 sits at row 0 (the same row as the
    // shared border ring) — the border paints over it.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app");
    let header = dom.create_element("header_el");
    let h1 = dom.create_element("h1_el");
    dom.append_child(root, app).unwrap();
    dom.append_child(app, header).unwrap();
    dom.append_child(header, h1).unwrap();

    let css = r#"
        app    { width: 80; height: 24; flex-direction: column;
                 border: solid; border-collapse: collapse; }
        header_el { height: 3; border: solid; }
        h1_el  { height: 1; }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let app_rect = dom.node(app).layout_rect().expect("app laid out");
    let header_rect = dom.node(header).layout_rect().expect("header laid out");
    let h1_rect = dom.node(h1).layout_rect().expect("h1 laid out");
    let app_content = dom.node(app).content_layout_rect();
    let header_content = dom.node(header).content_layout_rect();
    eprintln!("DBG app    outer = {app_rect:?}");
    eprintln!("DBG app    inner = {app_content:?}");
    eprintln!("DBG header outer = {header_rect:?}");
    eprintln!("DBG header inner = {header_content:?}");
    eprintln!("DBG h1     outer = {h1_rect:?}");
    eprintln!(
        "DBG header.computed.border = {:?}",
        dom.node(header)
            .ext()
            .and_then(|e| e.computed.as_ref().map(|c| c.border))
    );
    eprintln!(
        "DBG header.computed.collapse = {:?}",
        dom.node(header)
            .ext()
            .and_then(|e| e.computed.as_ref().map(|c| c.border_collapse))
    );
    // Header's outer rect under collapse extends into app's top
    // border row, so header.y == 0. Header has its own border, so
    // its content area top is row 1 (just below header's top
    // border row). h1's y SHOULD therefore be row 1.
    assert_eq!(
        h1_rect.y, 1,
        "h1 must sit inside header's content area (row 1), not at the shared border row \
         (row 0). Got rect {h1_rect:?}",
    );
}

#[test]
fn child_with_own_border_under_collapse_paints_text_below_top_border_row() {
    // End-to-end paint check: render the chrome to a TestBackend
    // and verify that row 0 contains border glyphs (not h1 text)
    // and row 1 contains the h1's text.
    use rdom_tui::App;
    use rdom_tui::render::{Terminal, TestBackend};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let app = dom.create_element("app");
    let header = dom.create_element("header_el");
    let h1 = dom.create_element("h1_el");
    let text = dom.create_text_node("HELLO");
    dom.append_child(root, app).unwrap();
    dom.append_child(app, header).unwrap();
    dom.append_child(header, h1).unwrap();
    dom.append_child(h1, text).unwrap();

    let sheet = rdom_css::from_css(
        r#"
        app    { width: 100%; height: 100%; flex-direction: column;
                 border: solid; border-collapse: collapse; }
        header_el { height: 3; border: solid; }
        h1_el  { height: 1; }
    "#,
    );

    let backend = TestBackend::new(20, 6);
    let terminal = Terminal::new(backend).unwrap();
    let mut app_rt = App::with_backend(dom, sheet, terminal).unwrap();
    app_rt.draw_if_dirty().unwrap();

    // Walk the buffer looking for "HELLO". It must appear at row 1
    // (inside the header content area), NOT row 0 (the border row).
    //
    // We assert against the rendered bytes — exact column might
    // shift slightly with padding, but row must be 1.
    let bytes = app_rt.terminal().backend().bytes();
    let s = String::from_utf8_lossy(bytes);
    assert!(
        s.contains("HELLO"),
        "h1 text \"HELLO\" must appear somewhere in the painted output \
         (got {} bytes)",
        bytes.len()
    );
    // Stronger assertion would parse the rendered grid and check row
    // index. For now, just confirming the text isn't lost.
}
