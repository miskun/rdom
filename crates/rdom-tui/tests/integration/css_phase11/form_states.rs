//! C11-FORM-STATES — the input pseudo-classes (Selectors 4 §14, HTML
//! §4.16.3) through a sheet, and the restyle when the state they read
//! changes.

use rdom_tui::{NodeId, TuiDom};

use super::{BLUE, RED, UNSTYLED, app, app_fg, cascade, fg};

/// An element with attributes, appended to `parent`.
fn node(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

// ─── :read-only / :read-write ───────────────────────────────────────

/// Selectors 4 §14.3.1, HTML §4.16.3: a mutable text field and an
/// editing host's content are `:read-write`; a `readonly` field and
/// everything else `:read-only`.
#[test]
fn read_only_and_read_write_style_through_a_sheet() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let field = node(&mut dom, root, "input", &[]);
    let locked = node(&mut dom, root, "input", &[("readonly", "")]);
    let host = node(&mut dom, root, "div", &[("contenteditable", "")]);
    let p = node(&mut dom, host, "p", &[]);
    let div = node(&mut dom, root, "div", &[]);
    cascade(
        &mut dom,
        ":read-write { color: red } :read-only { color: blue }",
    );
    assert_eq!(fg(&dom, field), RED);
    assert_eq!(fg(&dom, locked), BLUE);
    assert_eq!(fg(&dom, p), RED);
    assert_eq!(fg(&dom, div), BLUE);
}

/// `readonly` toggled on a field, `contenteditable` on an ancestor, and a
/// `:has(:read-write)` anchor all restyle on the next frame.
#[test]
fn mutability_changes_restyle() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let form = node(&mut dom, root, "form", &[]);
    let field = node(&mut dom, form, "input", &[]);
    let host = node(&mut dom, root, "div", &[]);
    let p = node(&mut dom, host, "p", &[]);
    let mut app = app(
        dom,
        "input:read-write { color: red } p:read-write { color: blue } \
         form:has(:read-write) { color: red }",
    );
    assert_eq!(app_fg(&app, field), RED);
    assert_eq!(app_fg(&app, form), RED);
    assert_eq!(app_fg(&app, p), UNSTYLED);
    app.dom_mut().set_attribute(field, "readonly", "").unwrap();
    app.dom_mut()
        .set_attribute(host, "contenteditable", "true")
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, field), UNSTYLED);
    assert_eq!(app_fg(&app, form), UNSTYLED);
    assert_eq!(app_fg(&app, p), BLUE);
}

// ─── :indeterminate ─────────────────────────────────────────────────

/// Selectors 4 §14.4.3, HTML §4.16.3: checking one radio of a group
/// with nothing checked takes `:indeterminate` from every member — the
/// ones not touched included — and from a `:has(:indeterminate)` anchor.
#[test]
fn checking_a_radio_restyles_its_whole_group() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let form = node(&mut dom, root, "form", &[]);
    let fs = node(&mut dom, form, "fieldset", &[]);
    let a = node(&mut dom, fs, "input", &[("type", "radio"), ("name", "g")]);
    let other = node(&mut dom, form, "div", &[]);
    let b = node(
        &mut dom,
        other,
        "input",
        &[("type", "radio"), ("name", "g")],
    );
    let mut app = app(
        dom,
        "input:indeterminate { color: red } fieldset:has(:indeterminate) { color: blue }",
    );
    assert_eq!(app_fg(&app, a), RED);
    assert_eq!(app_fg(&app, b), RED);
    assert_eq!(app_fg(&app, fs), BLUE);
    app.dom_mut().set_attribute(b, "checked", "").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, b), UNSTYLED);
    assert_eq!(
        app_fg(&app, a),
        UNSTYLED,
        "a's own attributes did not change"
    );
    assert_eq!(app_fg(&app, fs), UNSTYLED);
}

/// A checkbox's indeterminate flag set through the accessor restyles it.
#[test]
fn setting_a_checkboxs_indeterminate_flag_restyles_it() {
    use rdom_tui::TuiAccessorsMut;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = node(&mut dom, root, "input", &[("type", "checkbox")]);
    let mut app = app(dom, "input:indeterminate { color: red }");
    assert_eq!(app_fg(&app, cb), UNSTYLED);
    app.dom_mut().node_mut(cb).set_indeterminate(true).unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, cb), RED);
}

/// Found with C11-FORM-STATES: a validity flip no mutation reports
/// (`set_custom_validity`) restyled the control but not a
/// `:has(:invalid)` anchor around it — the per-frame validity marks
/// went around the tracker's `:has()` walk.
#[test]
fn a_custom_validity_restyles_a_has_invalid_anchor() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    // No form or fieldset around it: those have a validity of their own,
    // whose flip would restyle their whole subtree.
    let row = node(&mut dom, root, "div", &[]);
    let field = node(&mut dom, row, "input", &[]);
    let mut app = app(dom, "div:has(:invalid) { color: red }");
    assert_eq!(app_fg(&app, row), UNSTYLED);
    rdom_tui::runtime::builtins::validation::set_custom_validity(app.dom_mut(), field, "no");
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, row), RED);
}
