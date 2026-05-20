//! `<input>` value-attribute mirror + seed + helper tests.

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::input::{seed_all, set_value, value};
use crate::style::Stylesheet;

fn test_app(dom: TuiDom) -> App<TestBackend> {
    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, Stylesheet::new(), terminal).unwrap()
}

// ── seed_all ──────────────────────────────────────────────────────

#[test]
fn seed_all_creates_text_child_from_value_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "hello").unwrap();
    dom.append_child(root, input).unwrap();

    seed_all(&mut dom);

    assert_eq!(value(&dom, input), "hello");
}

#[test]
fn seed_all_creates_empty_text_child_when_no_value_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.append_child(root, input).unwrap();

    seed_all(&mut dom);

    // Empty string still produces a (single, empty) text child so
    // the editing pipeline has something to position the caret on.
    assert_eq!(value(&dom, input), "");
    assert_eq!(dom.node(input).child_nodes().count(), 1);
}

#[test]
fn seed_all_is_idempotent_when_text_already_matches_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "hi").unwrap();
    let t = dom.create_text_node("hi");
    dom.append_child(input, t).unwrap();
    dom.append_child(root, input).unwrap();

    let before = dom.node(input).child_nodes().next().map(|c| c.id());
    seed_all(&mut dom);
    let after = dom.node(input).child_nodes().next().map(|c| c.id());

    // Same text node id — wasn't replaced.
    assert_eq!(before, after);
}

#[test]
fn seed_all_re_seeds_when_text_disagrees_with_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "fresh").unwrap();
    let stale = dom.create_text_node("stale");
    dom.append_child(input, stale).unwrap();
    dom.append_child(root, input).unwrap();

    seed_all(&mut dom);

    assert_eq!(value(&dom, input), "fresh");
}

#[test]
fn seed_all_handles_inputs_nested_inside_other_elements() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "nested").unwrap();
    dom.append_child(form, input).unwrap();
    dom.append_child(root, form).unwrap();

    seed_all(&mut dom);

    assert_eq!(value(&dom, input), "nested");
}

// ── App::build wires seed_all automatically ───────────────────────

#[test]
fn app_build_seeds_all_inputs_in_the_tree() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "auto").unwrap();
    dom.append_child(root, input).unwrap();

    let app = test_app(dom);

    assert_eq!(value(app.dom(), input), "auto");
}

// ── set_value ─────────────────────────────────────────────────────

#[test]
fn set_value_writes_attribute_and_text_child() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.append_child(root, input).unwrap();
    seed_all(&mut dom);

    set_value(&mut dom, input, "set");

    assert_eq!(value(&dom, input), "set");
    assert_eq!(dom.node(input).get_attribute("value"), Some("set"));
}
