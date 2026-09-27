//! Live `<style>` sheets (`P7-LIVE-STYLE-1`) through a headless `App`.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

/// A `<style>` element holding `css`, not yet inserted.
fn style(dom: &mut TuiDom, css: &str) -> NodeId {
    let s = dom.create_element("style");
    let t = dom.create_text_node(css);
    dom.append_child(s, t).unwrap();
    s
}

/// `<p>x</p>` plus the given `<style>` sources (in document order,
/// before the paragraph), under an App whose own sheet is `sheet`.
fn app_with(styles: &[&str], sheet: Stylesheet) -> (App<TestBackend>, NodeId, Vec<NodeId>) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mut ids = Vec::new();
    for css in styles {
        let s = style(&mut dom, css);
        dom.append_child(root, s).unwrap();
        ids.push(s);
    }
    let p = dom.create_element("p");
    let t = dom.create_text_node("x");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    let terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, p, ids)
}

fn app(styles: &[&str]) -> (App<TestBackend>, NodeId, Vec<NodeId>) {
    app_with(styles, Stylesheet::bare())
}

fn fg(app: &App<TestBackend>, id: NodeId) -> Color {
    app.dom().node(id).computed().expect("cascaded").fg
}

fn text_of(app: &App<TestBackend>, style: NodeId) -> NodeId {
    app.dom().node(style).first_child().unwrap().id()
}

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);
const GREEN: Color = Color::Rgb(0, 128, 0);

#[test]
fn a_style_element_present_at_mount_applies() {
    let (app, p, _) = app(&["p { color: red; }"]);
    assert_eq!(fg(&app, p), RED);
}

#[test]
fn editing_a_style_elements_text_restyles_on_the_next_frame() {
    let (mut app, p, styles) = app(&["p { color: red; }"]);
    let text = text_of(&app, styles[0]);
    app.dom_mut()
        .node_mut(text)
        .set_node_value("p { color: blue; }")
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), BLUE);

    // Replacing the text child (textContent) is a child-list change.
    app.dom_mut()
        .set_text_content(styles[0], "p { color: green; }")
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), GREEN);
}

#[test]
fn inserting_a_style_element_adds_its_rules() {
    let (mut app, p, _) = app(&[]);
    assert_eq!(fg(&app, p), Color::Reset);
    let s = style(app.dom_mut(), "p { color: red; }");
    // Nested inside an inserted subtree, as a parsed template would be.
    let wrapper = app.dom_mut().create_element("div");
    app.dom_mut().append_child(wrapper, s).unwrap();
    let root = app.dom().root();
    app.dom_mut().append_child(root, wrapper).unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), RED);
}

#[test]
fn removing_a_style_element_drops_its_rules() {
    let (mut app, p, styles) = app(&["p { color: red; }"]);
    let root = app.dom().root();
    app.dom_mut().remove_child(root, styles[0]).unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), Color::Reset);
}

#[test]
fn style_elements_cascade_in_document_order() {
    let (mut app, p, styles) = app(&["p { color: red; }", "p { color: blue; }"]);
    assert_eq!(fg(&app, p), BLUE, "the later sheet wins the tie");
    // Move the first sheet after the second.
    let root = app.dom().root();
    app.dom_mut()
        .insert_before(root, styles[0], Some(p))
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), RED);
}

#[test]
fn app_sheets_cascade_after_style_elements() {
    let sheet = Stylesheet::bare().rule_unchecked("p", TuiStyle::new().fg(GREEN));
    let (app, p, _) = app_with(&["p { color: red; }"], sheet);
    assert_eq!(
        fg(&app, p),
        GREEN,
        "an App sheet, like an adopted sheet, follows the document's sheets"
    );
}

#[test]
fn the_inline_style_cssom_path_still_works() {
    let (mut app, p, _) = app(&["p { color: red; }"]);
    app.dom_mut()
        .set_attribute(p, "style", "color: blue")
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), BLUE);
}

#[test]
fn parse_warnings_are_reported_and_refreshed() {
    let (mut app, _, styles) = app(&["p { colr: red; }"]);
    assert_eq!(app.style_element_warnings().len(), 1);
    let text = text_of(&app, styles[0]);
    app.dom_mut()
        .node_mut(text)
        .set_node_value("p { color: red; }")
        .unwrap();
    app.advance(0).unwrap();
    assert!(app.style_element_warnings().is_empty());
}
