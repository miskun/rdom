//! `autofocus` attribute tests.

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::autofocus;
use crate::runtime::builtins::dialog;
use crate::style::Stylesheet;

fn test_app(dom: TuiDom) -> App<TestBackend> {
    let backend = TestBackend::new(20, 5);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, Stylesheet::new(), terminal).unwrap()
}

// ── App mount ────────────────────────────────────────────────────

#[test]
fn app_build_focuses_first_autofocus_element() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("button");
    let b = dom.create_element("button");
    dom.set_attribute(b, "autofocus", "").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let app = test_app(dom);
    assert_eq!(app.dom().focused(), Some(b));
}

#[test]
fn document_order_wins_when_multiple_autofocus() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let first = dom.create_element("button");
    dom.set_attribute(first, "autofocus", "").unwrap();
    let second = dom.create_element("button");
    dom.set_attribute(second, "autofocus", "").unwrap();
    dom.append_child(root, first).unwrap();
    dom.append_child(root, second).unwrap();

    let app = test_app(dom);
    assert_eq!(app.dom().focused(), Some(first));
}

#[test]
fn no_autofocus_attribute_does_not_set_focus() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let b = dom.create_element("button");
    dom.append_child(root, b).unwrap();

    let app = test_app(dom);
    assert_eq!(app.dom().focused(), None);
}

#[test]
fn non_focusable_autofocus_target_is_skipped() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p"); // not implicitly focusable
    dom.set_attribute(p, "autofocus", "").unwrap();
    let btn = dom.create_element("button");
    dom.set_attribute(btn, "autofocus", "").unwrap();
    dom.append_child(root, p).unwrap();
    dom.append_child(root, btn).unwrap();

    let app = test_app(dom);
    // <p> isn't focusable, so we fall through to the next
    // [autofocus] candidate.
    assert_eq!(app.dom().focused(), Some(btn));
}

#[test]
fn disabled_autofocus_element_is_skipped() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let b1 = dom.create_element("button");
    dom.set_attribute(b1, "autofocus", "").unwrap();
    dom.set_attribute(b1, "disabled", "").unwrap();
    let b2 = dom.create_element("button");
    dom.set_attribute(b2, "autofocus", "").unwrap();
    dom.append_child(root, b1).unwrap();
    dom.append_child(root, b2).unwrap();

    let app = test_app(dom);
    assert_eq!(app.dom().focused(), Some(b2));
}

#[test]
fn pre_existing_focus_is_not_clobbered() {
    // If an explicit set_focused was applied before App::build,
    // autofocus's initial-mount pass should respect it.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("button");
    let b = dom.create_element("button");
    dom.set_attribute(b, "autofocus", "").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.set_focused(Some(a));

    let app = test_app(dom);
    assert_eq!(app.dom().focused(), Some(a));
}

// ── Modal dialog integration ─────────────────────────────────────

#[test]
fn dialog_show_modal_focuses_autofocus_descendant() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    let btn = dom.create_element("button");
    dom.set_attribute(btn, "autofocus", "").unwrap();
    dom.append_child(dlg, btn).unwrap();
    dom.append_child(root, dlg).unwrap();
    let mut app = test_app(dom);

    dialog::show_modal(app.dom_mut(), dlg);
    assert_eq!(app.dom().focused(), Some(btn));
}

#[test]
fn dialog_show_modal_without_autofocus_descendant_leaves_focus_alone() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let dlg = dom.create_element("dialog");
    let btn = dom.create_element("button"); // no autofocus
    dom.append_child(dlg, btn).unwrap();
    dom.append_child(root, dlg).unwrap();
    let mut app = test_app(dom);
    let other = app.dom_mut().create_element("button");
    let root_id = app.dom().root();
    app.dom_mut().append_child(root_id, other).unwrap();
    app.dom_mut().set_focused(Some(other));

    dialog::show_modal(app.dom_mut(), dlg);
    assert_eq!(app.dom().focused(), Some(other));
}

// ── focus_within ─────────────────────────────────────────────────

#[test]
fn focus_within_scans_subtree_only() {
    // autofocus on a sibling of the scan root shouldn't change
    // focus when we scan the root's subtree — `focus_within` is
    // a no-op when no match exists.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("div");
    let leaf = dom.create_element("p");
    dom.append_child(a, leaf).unwrap();
    dom.append_child(root, a).unwrap();
    let mut app = test_app(dom);
    let anchor = app.dom_mut().create_element("button");
    let root_id = app.dom().root();
    app.dom_mut().append_child(root_id, anchor).unwrap();
    app.dom_mut().set_focused(Some(anchor));

    autofocus::focus_within(app.dom_mut(), a);
    // No autofocus inside `a` → focus untouched, still `anchor`.
    assert_eq!(app.dom().focused(), Some(anchor));
}

#[test]
fn focus_within_finds_deep_descendant() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let middle = dom.create_element("div");
    let inner = dom.create_element("button");
    dom.set_attribute(inner, "autofocus", "").unwrap();
    dom.append_child(middle, inner).unwrap();
    dom.append_child(outer, middle).unwrap();
    dom.append_child(root, outer).unwrap();
    let mut app = test_app(dom);

    autofocus::focus_within(app.dom_mut(), outer);
    assert_eq!(app.dom().focused(), Some(inner));
}
