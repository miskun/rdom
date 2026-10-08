//! C11G-CANVAS-FILL — `background-color`'s initial value is
//! `transparent` (CSS Backgrounds 3 §3.2), so a specified `Canvas` (the
//! terminal's own background, `Color::Reset`) paints: the box's cells are
//! blanked in the default background (SGR 49), where `transparent` leaves
//! them to what is beneath. HTML's rendering section gives `dialog` and
//! `[popover]` `background-color: Canvas; color: CanvasText`, so a modal
//! dialog or a popover hides the page under it.

use rdom_tui::runtime::builtins::{dialog, popover};
use rdom_tui::{Color, NodeId, TuiDom, TuiNodeExt};

use crate::css_phase5::{paint, rect, rows};

fn node(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)], text: &str) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(e, t).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

/// A page of `x`s filling 20 × 10.
fn page(dom: &mut TuiDom) {
    let root = dom.root();
    for _ in 0..10 {
        node(dom, root, "p", &[], "xxxxxxxxxxxxxxxxxxxx");
    }
}

/// The cells of `id`'s border box that hold an `x` from the page.
fn page_glyphs_inside(dom: &TuiDom, buf: &rdom_tui::render::Buffer, id: NodeId) -> Vec<(i32, i32)> {
    let r = rect(dom, id);
    let mut seen = Vec::new();
    for y in r.y..r.y + i32::from(r.height) {
        for x in r.x..r.x + i32::from(r.width) {
            if buf.cell(x as u16, y as u16).unwrap().symbol() == "x" {
                seen.push((x, y));
            }
        }
    }
    seen
}

/// The initial value of `background-color` is `transparent` (CSS
/// Backgrounds 3 §3.2): an unstyled element computes it.
#[test]
fn the_initial_background_is_transparent() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = node(&mut dom, root, "div", &[], "a");
    paint(&mut dom, "", 4, 1);
    assert_eq!(dom.node(d).computed().unwrap().bg, Color::TRANSPARENT);
}

/// A modal dialog centred over a page of text hides it: its padding
/// and the cells beside its short line are blank, in the terminal's
/// default background (HTML's `dialog { background-color: Canvas }`).
#[test]
fn a_modal_dialog_hides_the_page_beneath() {
    let mut dom = TuiDom::new();
    page(&mut dom);
    let root = dom.root();
    let d = node(&mut dom, root, "dialog", &[], "hi");
    dialog::show_modal(&mut dom, d).unwrap();
    let buf = paint(&mut dom, "", 20, 10);
    assert_eq!(
        page_glyphs_inside(&dom, &buf, d),
        [],
        "{:#?}",
        rows(&buf, 20, 10)
    );
    let r = rect(&dom, d);
    let pad = buf.cell(r.x as u16 + 1, r.y as u16 + 1).unwrap();
    assert_eq!((pad.symbol(), pad.bg), (" ", Color::Reset));
}

/// A popover menu over a page hides it: the cells right of its shorter
/// item are blank (HTML's `[popover] { background-color: Canvas }`).
#[test]
fn a_popover_menu_hides_the_page_beneath() {
    let mut dom = TuiDom::new();
    page(&mut dom);
    let root = dom.root();
    let m = node(&mut dom, root, "div", &[("popover", "")], "");
    node(&mut dom, m, "div", &[], "Open");
    node(&mut dom, m, "div", &[], "Q");
    popover::show_popover(&mut dom, m).unwrap();
    let buf = paint(&mut dom, "", 20, 10);
    assert_eq!(
        page_glyphs_inside(&dom, &buf, m),
        [],
        "{:#?}",
        rows(&buf, 20, 10)
    );
}

/// A specified `Canvas` (or `reset`) paints the terminal's default
/// background over an ancestor's color, where `transparent` lets it show.
#[test]
fn a_specified_canvas_paints_over_an_ancestor() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = node(&mut dom, root, "div", &[("class", "outer")], "");
    node(&mut dom, outer, "div", &[("class", "canvas")], "a");
    node(&mut dom, outer, "div", &[("class", "reset")], "b");
    node(&mut dom, outer, "div", &[("class", "clear")], "c");
    let buf = paint(
        &mut dom,
        ".outer { background-color: red } .canvas { background-color: Canvas } \
         .reset { background-color: reset } .clear { background-color: transparent }",
        3,
        3,
    );
    let red = Color::Rgb(255, 0, 0);
    assert_eq!(buf.cell(1, 0).unwrap().bg, Color::Reset, "Canvas paints");
    assert_eq!(buf.cell(1, 1).unwrap().bg, Color::Reset, "reset paints");
    assert_eq!(buf.cell(1, 2).unwrap().bg, red, "transparent shows the red");
}
