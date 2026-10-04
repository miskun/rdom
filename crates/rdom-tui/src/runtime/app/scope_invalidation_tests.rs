//! `C1G-INVALIDATION`: style invalidation reads every selector the
//! cascade matches — the `<scope-start>` / `<scope-end>` of an `@scope`
//! (CSS Cascade 6 §2.5), which are not rule selectors, and the argument
//! of `:is()` (`SimpleSelector::Is`, the nesting selector `&`, CSS
//! Nesting 1 §2). A `+` / `~` in either can make a change on one element
//! restyle a sibling's subtree (Selectors 4 §16.3 / §16.4).

use rdom_core::NodeId;

use crate::Color;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;
use crate::{TuiDom, TuiNodeExt};

const RED: Color = Color::Rgb(255, 0, 0);

/// A `<main>` holding `<div id=x>` and `<div id=b class=b><p></p></div>`,
/// under `css`, one frame drawn. Returns `(app, x, b, p)`.
fn app_with(css: &str, b_class: &str) -> (App<TestBackend>, NodeId, NodeId, NodeId) {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    app_with_sheet(parsed.stylesheet, b_class)
}

fn app_with_sheet(sheet: Stylesheet, b_class: &str) -> (App<TestBackend>, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let main = dom.create_element("main");
    dom.append_child(root, main).unwrap();
    let x = dom.create_element("div");
    dom.append_child(main, x).unwrap();
    let b = dom.create_element("div");
    dom.set_attribute(b, "class", b_class).unwrap();
    dom.append_child(main, b).unwrap();
    let p = dom.create_element("p");
    dom.append_child(b, p).unwrap();
    let mut app = App::with_backend(
        dom,
        Stylesheet::new(),
        Terminal::new(TestBackend::new(20, 6)).unwrap(),
    )
    .unwrap();
    app.push_stylesheet(sheet);
    app.advance(0).unwrap();
    (app, x, b, p)
}

fn fg(app: &App<TestBackend>, id: NodeId) -> Color {
    app.dom()
        .node(id)
        .tui_ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.fg)
        .expect("cascaded")
}

/// Cascade 6 §2.5.1: every element `<scope-start>` matches is a scoping
/// root. Inserting `.a` before `.b` makes `.b` a root of
/// `@scope (.a + .b)`, so its `<p>` takes the scoped rule.
#[test]
fn a_sibling_insert_under_a_scope_prelude_restyles() {
    let (mut app, x, b, p) = app_with("@scope (.a + .b) { p { color: red } }", "b");
    assert_ne!(fg(&app, p), RED);
    let main = app.dom().node(x).parent_node().unwrap().id();
    let a = app.dom_mut().create_element("i");
    app.dom_mut().set_attribute(a, "class", "a").unwrap();
    app.dom_mut().insert_before(main, a, Some(b)).unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), RED, "`.b` became a scoping root");
}

/// The same root appearing through a class change on the previous
/// sibling: the sibling trigger must read the scope prelude.
#[test]
fn a_class_change_left_of_a_sibling_combinator_in_a_scope_prelude_restyles() {
    let (mut app, x, _b, p) = app_with("@scope (.a + .b) { p { color: red } }", "b");
    assert_ne!(fg(&app, p), RED);
    app.dom_mut().set_attribute(x, "class", "a").unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), RED, "`.b` became a scoping root");
}

/// Cascade 6 §2.5.1: an element inside a scoping limit is out of scope.
/// A class change on the previous sibling makes `.b` a limit through
/// `<scope-end>`'s `+`, so its `<p>` loses the scoped rule.
#[test]
fn a_class_change_matching_a_scope_limit_restyles() {
    let (mut app, x, _b, p) = app_with("@scope (main) to (.lim + .b) { p { color: red } }", "b");
    assert_eq!(fg(&app, p), RED, "in scope before the change");
    app.dom_mut().set_attribute(x, "class", "lim").unwrap();
    app.advance(0).unwrap();
    assert_ne!(fg(&app, p), RED, "`.b` became a scoping limit");
}

/// CSS Nesting 1 §2: a nested rule's `&` is `:is(<parent list>)`, so a
/// sibling combinator in the parent's selector sits inside an `:is()`
/// of the nested rule's selector. The sheet holds the nested rule alone
/// (built in Rust, as a host may), so no other rule exposes the `+`.
#[test]
fn a_sibling_combinator_reached_through_nested_amp_restyles() {
    use rdom_style::{RuleContext, StyleSelector, TuiStyle};
    let parent = StyleSelector::parse(".a + .b, .q + .b").unwrap();
    let nested = StyleSelector::parse_nested("p", &parent).unwrap();
    let mut sheet = Stylesheet::bare();
    sheet.add_style_rule(&nested, TuiStyle::new().fg(RED), RuleContext::default());
    let (mut app, x, _b, p) = app_with_sheet(sheet, "b");
    assert_ne!(fg(&app, p), RED);
    app.dom_mut().set_attribute(x, "class", "a").unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app, p), RED, "`.b` matches `:is(.a + .b, .q + .b)`");
}
