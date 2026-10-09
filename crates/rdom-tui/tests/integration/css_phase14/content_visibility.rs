//! `content-visibility` (CSS Containment 2 §4): `hidden` skips the
//! element's contents — not laid out, painted, hit or focused, its
//! `::before` / `::after` included — while its box stays, sized by
//! `contain-intrinsic-size`; `auto` does so only while the element is not
//! relevant to the user (off-screen), and always contains layout, style
//! and paint.

use rdom_tui::HitTestExt;

use super::*;

fn rect(dom: &TuiDom, id: &str) -> (i32, i32, u16, u16) {
    let r = dom.node(by_id(dom, id)).layout_rect().expect("laid out");
    (r.x, r.y, r.width, r.height)
}

fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf.cell(x, y).unwrap().symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

const MARKUP: &str = r#"<body><div id="c"><p id="t">text</p><button id="b">go</button></div><p id="after">after</p></body>"#;

/// §4.1 `hidden`: the box stays — its border drawn, its size its
/// `contain-intrinsic-size` (size containment) — its contents are not
/// painted, hit or focusable, and keep their own computed styles.
#[test]
fn hidden_skips_the_contents_and_keeps_the_box() {
    let mut dom = doc(MARKUP);
    let css = "p { margin: 0 } #c { content-visibility: hidden; border: solid; contain-intrinsic-size: auto 2 } #c::before { content: 'B' }";
    let buf = paint(&mut dom, css, 20, 8);
    assert_eq!(
        rect(&dom, "c").3,
        4,
        "two rows of intrinsic size and the border"
    );
    assert_eq!(row(&buf, 0), "┌──────────────────┐");
    assert_eq!(
        row(&buf, 1),
        "│                  │",
        "no `::before`, no text"
    );
    assert_eq!(rect(&dom, "after").1, 4);
    assert_ne!(dom.hit_test(1, 1), Some(by_id(&dom, "t")));
    assert_eq!(
        rdom_tui::runtime::focus::tabindex::tab_index(&dom, by_id(&dom, "b")),
        None
    );
    let display = dom.node(by_id(&dom, "t")).computed().unwrap().display;
    assert_eq!(
        display,
        rdom_tui::Display::Block,
        "the content keeps its style"
    );
}

/// §4.2 `auto`: an element on screen renders as `visible` does — with
/// layout, style and paint containment (its overflow is clipped).
#[test]
fn auto_on_screen_renders_with_containment() {
    let mut dom = doc(r#"<body><div id="c"><div id="wide">0123456789</div></div></body>"#);
    let buf = paint(
        &mut dom,
        "#c { content-visibility: auto; width: 4 } #wide { width: 10 }",
        20,
        3,
    );
    assert_eq!(row(&buf, 0), "0123");
}

/// §4.2 `auto`: off-screen, the contents are skipped and the box is sized
/// by `contain-intrinsic-size` — its `auto` the size it was last laid out
/// at (CSS Sizing 4 §6.1).
#[test]
fn auto_off_screen_skips_the_contents() {
    let markup =
        r#"<body><div id="spacer"></div><div id="c"><p>a</p><p>b</p><p>c</p></div></body>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #spacer { height: 20 } #c { content-visibility: auto; contain-intrinsic-size: auto 7 }",
        20,
        5,
    );
    assert_eq!(rect(&dom, "c").3, 3, "remembered from its first layout");
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #spacer { height: 20 } #c { content-visibility: auto; contain-intrinsic-size: 7 }",
        20,
        5,
    );
    assert_eq!(rect(&dom, "c").3, 7, "no `auto`: the length");
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #spacer { height: 1 } #c { content-visibility: auto; contain-intrinsic-size: 7 }",
        20,
        5,
    );
    assert_eq!(rect(&dom, "c").3, 3, "on screen: its content's");
}

/// CSS Sizing 4 §6.1: `contain-intrinsic-size: auto <length>` remembers
/// the size an element had when it last rendered its contents; hiding it
/// keeps that size.
#[test]
fn hidden_keeps_the_last_remembered_size() {
    let markup = r#"<body><div id="c"><p>a</p><p>b</p><p>c</p></div></body>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #c { contain-intrinsic-size: auto 1 }",
        20,
        8,
    );
    assert_eq!(rect(&dom, "c").3, 3);
    let c = by_id(&dom, "c");
    dom.set_attribute(c, "class", "gone").unwrap();
    styled(
        &mut dom,
        "p { margin: 0 } #c { contain-intrinsic-size: auto 1 } .gone { content-visibility: hidden }",
        20,
        8,
    );
    assert_eq!(rect(&dom, "c").3, 3, "the remembered 3, not 1");
}
