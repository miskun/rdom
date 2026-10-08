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
