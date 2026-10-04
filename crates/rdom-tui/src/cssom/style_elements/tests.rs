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

// ── P7G-STYLE-HOLDS-1: cheap relevance checks ─────────────────────────

/// A 10 000-deep chain built bottom-up — each new parent adopting the
/// chain so far, a `<style>` at the bottom — in a detached subtree, then
/// inserted. The observer neither recurses (no stack overflow) nor
/// walks the detached build (a `<style>` that is not connected has no
/// sheet, HTML §4.2.6), and the one insertion is relevant.
#[test]
fn a_deep_bottom_up_build_does_not_recurse_and_only_its_insertion_counts() {
    let mut dom: TuiDom = TuiDom::new();
    let elements = super::StyleElements::install(&mut dom);
    elements.dirty.set(false);
    let mut top = style(&mut dom, "p { color: red; }");
    for _ in 0..10_000 {
        let parent = dom.create_element("div");
        dom.append_child(parent, top).unwrap();
        top = parent;
    }
    assert!(!elements.dirty.get(), "a detached build holds no sheet");
    let root = dom.root();
    dom.append_child(root, top).unwrap();
    assert!(elements.dirty.get(), "inserting it adds one");
    elements.dirty.set(false);
    dom.remove_child(root, top).unwrap();
    assert!(elements.dirty.get(), "removing it drops it");
}

/// Mutations under the document that involve no `<style>` leave the
/// set clean; a text edit inside a detached `<style>` too.
#[test]
fn mutations_without_a_connected_style_element_are_not_relevant() {
    let mut dom: TuiDom = TuiDom::new();
    let elements = super::StyleElements::install(&mut dom);
    elements.dirty.set(false);
    let root = dom.root();
    let list = dom.create_element("ul");
    dom.append_child(root, list).unwrap();
    for _ in 0..3 {
        let li = dom.create_element("li");
        let t = dom.create_text_node("x");
        dom.append_child(li, t).unwrap();
        dom.append_child(list, li).unwrap();
    }
    let detached = style(&mut dom, "p {}");
    let text = dom.node(detached).first_child().unwrap().id();
    dom.node_mut(text)
        .set_node_value("p { color: red; }")
        .unwrap();
    assert!(!elements.dirty.get());
}

/// The subtree search itself is iterative: a 10 000-deep chain is
/// searched to its bottom without growing the stack.
#[test]
fn holds_style_searches_a_deep_chain_without_recursing() {
    let mut dom: TuiDom = TuiDom::new();
    let mut with = style(&mut dom, "");
    let mut without = dom.create_element("span");
    for _ in 0..10_000 {
        for top in [&mut with, &mut without] {
            let parent = dom.create_element("div");
            dom.append_child(parent, *top).unwrap();
            *top = parent;
        }
    }
    assert!(super::holds_style(&dom, with));
    assert!(!super::holds_style(&dom, without));
    // A sibling after a deep branch is still searched.
    let s = style(&mut dom, "");
    dom.append_child(without, s).unwrap();
    assert!(super::holds_style(&dom, without));
}

/// CSS Cascade 6 §2.5.1: a prelude-less `@scope` in a `<style>` element
/// scopes to the element's parent — the sheet's owner node is the
/// `<style>` element.
#[test]
fn a_prelude_less_scope_roots_at_the_style_elements_parent() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let inside = dom.create_element("p");
    let outside = dom.create_element("p");
    let s = style(&mut dom, "@scope { p { color: red; } }");
    dom.append_child(section, s).unwrap();
    dom.append_child(section, inside).unwrap();
    dom.append_child(root, section).unwrap();
    dom.append_child(root, outside).unwrap();
    let terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::bare(), terminal).unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, inside), RED);
    assert_ne!(fg(&app, outside), RED);
}
