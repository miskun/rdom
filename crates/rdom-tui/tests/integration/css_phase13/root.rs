//! C13-ROOT-BLOCK: the document root lays its children out as block flow
//! in the initial containing block (CSS 2.1 §10.1: the ICB has the
//! viewport's dimensions; §9.4.1: it holds a block formatting context),
//! as a browser lays out `<body>`'s children — not as the items of a
//! viewport flex column. Each test pins one root special case the column
//! made and the block container removes.

use rdom_tui::render::{Buffer, Color};
use rdom_tui::{LayoutRect, TuiAccessors, TuiDocAccessors, TuiDom, TuiNodeExt};

use super::{by_id, doc, paint, rows};

fn rect(dom: &TuiDom, id: &str) -> LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().unwrap()
}

fn hit(dom: &TuiDom, x: i32, y: i32) -> Option<rdom_tui::NodeId> {
    dom.element_from_point(x, y).map(|n| n.id())
}

fn bg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf.cell(x, y).expect("in the buffer").bg
}

/// CSS 2.1 §8.3.1: adjacent vertical margins of in-flow block siblings
/// collapse — bottom 2 then top 3 leave 3 rows, at the root as in a
/// `<body>` (the column added them: 5).
#[test]
fn a_root_childs_margins_collapse_with_its_siblings() {
    let mut dom = doc(r#"<div id="a">a</div><div id="b">b</div>"#);
    paint(
        &mut dom,
        "#a { margin-bottom: 2 } #b { margin-top: 3 }",
        20,
        10,
    );
    assert_eq!(rect(&dom, "a").y, 0);
    assert_eq!(rect(&dom, "b").y, 4);
}

/// CSS 2.1 §9.5: a root child floats — the ICB lays out block flow — and
/// the line boxes after it are shortened beside it.
#[test]
fn a_root_child_floats() {
    let mut dom = doc(r#"<div id="f">F</div><p id="p">ab</p>"#);
    let buf = paint(
        &mut dom,
        "#f { float: right; width: 3 } p { margin: 0 }",
        10,
        4,
    );
    assert_eq!(rect(&dom, "f").x, 7);
    assert_eq!(rect(&dom, "f").y, 0);
    assert_eq!(rect(&dom, "p").y, 0);
    assert_eq!(rows(&buf)[0], "ab     F");
}

/// CSS 2.1 §10.5: a percentage height is of the containing block's
/// height, and the ICB's is the viewport's (§10.1) — definite whether or
/// not the box would have grown in a column: `height: 50%` is half the
/// viewport, and so definite for a `100%` child.
#[test]
fn height_50_percent_on_a_root_child_is_definite_against_the_viewport() {
    let mut dom = doc(r#"<div id="a"><div id="b">x</div></div><div id="c">c</div>"#);
    paint(&mut dom, "#a { height: 50% } #b { height: 100% }", 20, 12);
    assert_eq!(rect(&dom, "a").height, 6);
    assert_eq!(rect(&dom, "b").height, 6);
    assert_eq!(rect(&dom, "c").y, 6);
}

/// CSS 2.1 §10.6.3 / §10.5: an `auto`-height root child's height is its
/// content's, and indefinite — a percentage height inside it is `auto`
/// (the column made one with `flex-grow` definite).
#[test]
fn an_auto_root_child_is_indefinite_even_when_it_would_grow() {
    let mut dom = doc(r#"<div id="a"><div id="b">x</div></div>"#);
    paint(&mut dom, "#a { flex-grow: 1 } #b { height: 100% }", 20, 12);
    assert_eq!(rect(&dom, "a").height, 1);
    assert_eq!(rect(&dom, "b").height, 1);
}

/// CSS 2.1 §9.2.2 / §10.3.9: inline blocks at the root are inline-level —
/// each as wide as its content, side by side in one line box, the space
/// between them kept (the column stacked and needed an exception not to
/// stretch them).
#[test]
fn root_level_inline_blocks_hug_their_content_in_one_line() {
    let mut dom = doc(r#"<span id="a" class="ib">ab</span> <span id="b" class="ib">cd</span>"#);
    let buf = paint(&mut dom, ".ib { display: inline-block }", 20, 4);
    assert_eq!(rect(&dom, "a"), LayoutRect::new(0, 0, 2, 1));
    assert_eq!(rect(&dom, "b"), LayoutRect::new(3, 0, 2, 1));
    assert_eq!(rows(&buf)[0], "ab cd");
}

/// CSS 2.1 §17.5.2.2: a table in block flow is as wide as its content
/// asks, with no root exception, and its auto margins centre it
/// (§10.3.3, the table wrapper box being block-level).
#[test]
fn a_root_level_table_hugs_its_content() {
    let mut dom = doc(r#"<table id="t"><tr><td>ab</td><td>cd</td></tr></table>"#);
    paint(
        &mut dom,
        "td { padding: 0 } table { margin: 0 auto }",
        20,
        4,
    );
    assert_eq!(rect(&dom, "t").width, 4);
    assert_eq!(rect(&dom, "t").x, 8);
}

/// CSS Overflow 3 §3: an `overflow: auto` box shows a scrollbar only when
/// its content overflows it. The README's data table in its scrolling
/// wrapper, the wrapper a root child in a viewport shorter than it: its
/// `auto` height is its content's plus the horizontal bar's row (CSS 2.1
/// §10.6.3) whatever the viewport's — block flow does not shrink a box
/// to fit — so it shows the horizontal bar only, its gutter column free,
/// as when it is nested in a block. (The column flex-shrank it to the
/// viewport's 8 rows and drew a vertical bar.)
#[test]
fn a_root_level_overflow_auto_block_shows_no_spurious_scrollbar() {
    let table = r#"<table>
             <thead><tr><th>File</th><th>Size</th></tr></thead>
             <tbody>
               <tr><td>notes.txt</td><td>12</td></tr>
               <tr><td>build-output.log</td><td>4096</td></tr>
               <tr><td>b</td><td>7</td></tr>
             </tbody>
           </table>"#;
    let mut dom = doc(&format!(r#"<div id="w">{table}</div>"#));
    paint(
        &mut dom,
        "#w { width: 20; overflow: auto } \
         table { width: max-content; border-collapse: collapse } th, td { border: solid }",
        30,
        8,
    );
    let w = by_id(&dom, "w");
    assert_eq!(dom.node(w).scroll_width(), Some(27));
    assert_eq!(dom.node(w).scroll_height(), Some(10));
    assert_eq!(rect(&dom, "w").height, 10);
    let content = dom.node(w).tui_ext().unwrap().content_layout;
    assert_eq!(
        (content.width, content.height),
        (20, 9),
        "a horizontal bar, no vertical one"
    );
}

/// CSS 2.1 §10.5: `height: 100%` on a root child is the viewport's height,
/// and stays definite below it.
#[test]
fn height_100_percent_on_a_root_child_fills_the_viewport() {
    let mut dom = doc(r#"<div id="app"><div id="main">m</div></div>"#);
    paint(
        &mut dom,
        "#app { height: 100% } #main { height: 50% }",
        20,
        10,
    );
    assert_eq!(rect(&dom, "app"), LayoutRect::new(0, 0, 20, 10));
    assert_eq!(rect(&dom, "main").height, 5);
}

/// CSS 2.1 §10.5 (the CHANGELOG's upgrade item `sc-root-block`): a
/// percentage height resolves only against a definite one, so a shell
/// below a top-level `<body>` fills the screen only when every ancestor
/// down to it has `height: 100%` — `.app { height: 100% }` alone is its
/// content's height, `<body>`'s being `auto`.
#[test]
fn a_shell_under_body_fills_the_screen_only_with_every_ancestor_full_height() {
    let markup = r#"<body><div class="app" id="app">a</div></body>"#;
    let mut dom = doc(markup);
    paint(&mut dom, ".app { height: 100% }", 20, 10);
    assert_eq!(
        rect(&dom, "app").height,
        1,
        "body is auto: the shell's 100% is auto"
    );
    let mut dom = doc(markup);
    paint(&mut dom, "body, .app { height: 100% }", 20, 10);
    assert_eq!(rect(&dom, "app"), LayoutRect::new(0, 0, 20, 10));
}

/// The ICB is no flex container: `flex: 1` on a root child grows nothing
/// (CSS Flexbox §7: `flex` applies to flex items) — the app shell fills
/// the screen with `height: 100%` and lays out its own column.
#[test]
fn flex_on_a_root_child_does_not_grow_it_but_a_full_height_shell_does() {
    let mut dom = doc(r#"<div id="a">a</div>"#);
    paint(&mut dom, "#a { flex: 1 }", 20, 10);
    assert_eq!(rect(&dom, "a").height, 1);

    let mut dom = doc(
        r#"<div id="app"><header id="h">h</header><main id="m">m</main><footer id="f">f</footer></div>"#,
    );
    paint(
        &mut dom,
        "#app { height: 100%; display: flex; flex-direction: column } #m { flex: 1 }",
        20,
        10,
    );
    assert_eq!(rect(&dom, "m"), LayoutRect::new(0, 1, 20, 8));
    assert_eq!(rect(&dom, "f").y, 9);
}

/// CSS Backgrounds 3 §2.11.2: the root element's background paints the
/// whole canvas — the viewport below a content-high page too. The root
/// element is the root fragment (C14G-ROOT-ELEMENT; a first top-level
/// element's background was the canvas's before it).
#[test]
fn the_root_elements_background_paints_the_canvas() {
    let mut dom = doc(r#"<div id="app">hi</div>"#);
    let buf = paint(&mut dom, ":root { background: rgb(1, 2, 3) }", 10, 4);
    assert_eq!(rect(&dom, "app").height, 1);
    for (x, y) in [(0, 0), (9, 0), (0, 3), (9, 3)] {
        assert_eq!(bg(&buf, x, y), Color::Rgb(1, 2, 3), "({x}, {y})");
    }
}

/// CSS Backgrounds 3 §2.11.2: "the used value of `background` [on the
/// propagating element] is transparent" once it is the canvas's — so a
/// translucent `<body>` one composites once, its box no darker than the
/// canvas.
#[test]
fn the_propagated_background_paints_once() {
    let mut dom = doc(r#"<body id="app">hi</body>"#);
    let buf = paint(
        &mut dom,
        "body { margin: 0; background: rgb(200 0 0 / 50%) }",
        10,
        4,
    );
    assert_eq!(bg(&buf, 5, 0), bg(&buf, 5, 3), "the box as the canvas");
    assert_ne!(bg(&buf, 5, 3), Color::Reset);
}

/// CSS Backgrounds 3 §2.11.2: for an HTML document whose `html` element
/// has a transparent background, `body`'s background is the canvas's.
#[test]
fn a_transparent_html_propagates_bodys_background() {
    let mut dom = doc(r#"<html><body id="b">hi</body></html>"#);
    let buf = paint(&mut dom, "body { background: rgb(4, 5, 6) }", 10, 4);
    assert_eq!(bg(&buf, 9, 3), Color::Rgb(4, 5, 6));
}

/// CSS 2.1 §10.1, §10.3.3, §10.6.3: an element root is laid out as a block
/// in the ICB — the viewport's width less its margins, its height its
/// content's unless set; `html { height: 100% }` fills the viewport.
#[test]
fn an_element_root_is_a_block_in_the_initial_containing_block() {
    let mut dom = TuiDom::with_root_tag("html");
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, "<p>x</p>", root).expect("markup parses");
    paint(&mut dom, "p { margin: 0 } html { margin: 0 1 }", 20, 10);
    assert_eq!(
        dom.node(root).layout_rect(),
        Some(LayoutRect::new(1, 0, 18, 1))
    );
    paint(&mut dom, "p { margin: 0 } html { height: 100% }", 20, 10);
    assert_eq!(
        dom.node(root).layout_rect(),
        Some(LayoutRect::new(0, 0, 20, 10))
    );
}

/// CSS 2.1 §9.2.1.1: a root-level inline sits in a line of the ICB's
/// anonymous block box, and a point on its text hits it (UI Events'
/// hit test: the topmost box under the point) — as an inline in the
/// anonymous line of any block container with block children does.
#[test]
fn an_inline_in_an_anonymous_line_is_hit() {
    let mut dom = doc(r#"<p id="p">x</p><a id="a" href="h">link</a>"#);
    paint(&mut dom, "p { margin: 0 }", 20, 4);
    assert_eq!(hit(&dom, 1, 1), Some(by_id(&dom, "a")));

    let mut dom = doc(r#"<div id="d"><p>x</p>text <b id="b">bold</b></div>"#);
    paint(&mut dom, "p { margin: 0 }", 20, 4);
    assert_eq!(hit(&dom, 6, 1), Some(by_id(&dom, "b")));
    assert_eq!(hit(&dom, 1, 1), Some(by_id(&dom, "d")));
}
