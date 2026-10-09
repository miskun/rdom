//! C14G-ROOT-ELEMENT — the root fragment is the CSS root element (the
//! Phase 14 gate's decision 1; DIVERGENCES §2, `Dom::root()`): it is
//! cascaded (`:root` and `*` match it, its computed style is real), the
//! top-level elements inherit from it, its background is the canvas's
//! (CSS Backgrounds 3 §2.11.2), and `rlh` reads its line height (CSS
//! Values 4 §6.1.1). Its box stays the initial containing block, so its
//! own box properties do not apply.

use super::*;

/// The cell at `(x, y)`'s background.
fn bg_at(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).unwrap().bg
}

const NAVY: Color = Color::Rgb(0, 0, 128);

/// CSS Conditional 3 §2 with Selectors 4 §14.1: the web's dark-mode
/// pattern — a `:root` custom property overridden inside `@media` — under
/// a terminal detected light keeps the light value (the parse-time mirror
/// took the later rule whatever the condition).
#[test]
fn a_root_variable_in_a_media_block_follows_its_condition() {
    let css = ":root { --bg: #fff } @media (prefers-color-scheme: dark) { :root { --bg: #111 } } \
               #a { background-color: var(--bg) }";
    let mut dom = doc(r#"<p id="a">a</p>"#);
    dom.set_color_scheme(rdom_tui::ColorScheme::Light);
    styled(&mut dom, css, 10, 3);
    let bg = dom.node(by_id(&dom, "a")).computed().unwrap().bg;
    assert_eq!(bg, Color::Rgb(255, 255, 255));
    dom.set_color_scheme(rdom_tui::ColorScheme::Dark);
    styled(&mut dom, css, 10, 3);
    let bg = dom.node(by_id(&dom, "a")).computed().unwrap().bg;
    assert_eq!(bg, Color::Rgb(17, 17, 17));
}

/// CSS Cascade 4 §7: the top-level elements inherit the root's inherited
/// properties — `:root { color }`, the web's most common idiom, colours
/// the page.
#[test]
fn the_top_level_elements_inherit_from_root() {
    let mut dom = doc(r#"<p id="a">a</p>"#);
    styled(&mut dom, ":root { color: rgb(255, 0, 0) }", 10, 3);
    assert_eq!(fg(&dom, "a"), RED);
    let root = dom
        .node(dom.root())
        .computed()
        .expect("the root is styled")
        .fg;
    assert_eq!(root, RED);
}

/// CSS Backgrounds 3 §2.11.2: the root element's background is the
/// canvas's — every cell of the viewport, beneath the content.
#[test]
fn the_root_background_paints_the_canvas() {
    let mut dom = doc(r#"<p id="a">a</p>"#);
    let buf = paint(
        &mut dom,
        ":root { background-color: rgb(0, 0, 128) }",
        10,
        3,
    );
    assert_eq!(bg_at(&buf, 9, 2), NAVY);
    assert_eq!(bg_at(&buf, 0, 0), NAVY);
}

/// §2.11.2 under a transparent root: an HTML document's `<body>`
/// propagates — a `<style>` before it does not take the canvas away (it
/// was the "document element", `display: none`, so nothing painted).
#[test]
fn a_leading_style_element_does_not_take_the_canvas() {
    let mut dom = doc(r#"<style>p { margin: 0 }</style><body id="b"><p>a</p></body>"#);
    let buf = paint(
        &mut dom,
        "body { margin: 0; background-color: rgb(0, 0, 128) }",
        10,
        3,
    );
    assert_eq!(bg_at(&buf, 9, 2), NAVY);
}

/// §2.11.2 is about the root element and its `<body>` only: a toast
/// inserted before the page's body is not the canvas (it was, as the first
/// element child) and paints its own box.
#[test]
fn a_toast_inserted_first_keeps_its_own_background() {
    let mut dom = doc(r#"<div id="toast">t</div><body><p>a</p></body>"#);
    let buf = paint(
        &mut dom,
        "body, p { margin: 0 } body { background-color: rgb(0, 0, 128) } \
         #toast { background-color: rgb(255, 0, 0); width: 3 }",
        10,
        3,
    );
    assert_eq!(bg_at(&buf, 0, 0), RED, "the toast's own box");
    assert_eq!(bg_at(&buf, 9, 2), NAVY, "the body's canvas");
}

/// §2.11.2: under a transparent root fragment, an `<html>` child's
/// background, else its `<body>`'s, is the canvas — a parsed full
/// document propagates as in a browser (pinned: it did before, through
/// the first-element-child rule).
#[test]
fn an_html_tree_under_the_root_propagates() {
    let mut dom = doc(r#"<html><body><p>a</p></body></html>"#);
    let buf = paint(
        &mut dom,
        "body, p { margin: 0 } body { background-color: rgb(0, 0, 128) }",
        10,
        3,
    );
    assert_eq!(bg_at(&buf, 9, 2), NAVY);
}

/// CSS Values 4 §6.1.1: `rlh` is the root element's line height — the
/// root fragment's, for every top-level element and below.
#[test]
fn rlh_reads_the_root() {
    let mut dom = doc(r#"<div id="a"></div><div id="b"><div id="c"></div></div>"#);
    styled(
        &mut dom,
        ":root { line-height: 2 } #a, #c { height: 1rlh }",
        10,
        8,
    );
    let h = |id: &str| dom.node(by_id(&dom, id)).layout_rect().unwrap().height;
    assert_eq!((h("a"), h("c")), (2, 2));
}
