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

// ─── :in-range / :out-of-range ──────────────────────────────────────

/// Selectors 4 §14.3.3–§14.3.4, HTML §4.16.3: a number input with `min`
/// / `max` is in or out of range by its value; one without limits is
/// neither; a range input always has limits and is always in range (its
/// value is sanitized into them); the date-like states compare their
/// parsed values; a disabled control (barred from constraint validation)
/// is neither.
#[test]
fn in_range_and_out_of_range_style_through_a_sheet() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let low = node(
        &mut dom,
        root,
        "input",
        &[("type", "number"), ("min", "5"), ("value", "3")],
    );
    let ok = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "number"),
            ("min", "5"),
            ("max", "9"),
            ("value", "7"),
        ],
    );
    let free = node(
        &mut dom,
        root,
        "input",
        &[("type", "number"), ("value", "7")],
    );
    let slider = node(&mut dom, root, "input", &[("type", "range")]);
    let late = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "date"),
            ("max", "2026-01-31"),
            ("value", "2026-02-01"),
        ],
    );
    let month = node(
        &mut dom,
        root,
        "input",
        &[("type", "month"), ("min", "2026-03"), ("value", "2026-03")],
    );
    let week = node(
        &mut dom,
        root,
        "input",
        &[("type", "week"), ("min", "2026-W10"), ("value", "2026-W09")],
    );
    let night = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "time"),
            ("min", "22:00"),
            ("max", "06:00"),
            ("value", "23:30"),
        ],
    );
    let noon = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "time"),
            ("min", "22:00"),
            ("max", "06:00"),
            ("value", "12:00"),
        ],
    );
    let stamp = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "datetime-local"),
            ("min", "2026-10-14T09:00"),
            ("value", "2026-10-14 08:59"),
        ],
    );
    let off = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "number"),
            ("min", "5"),
            ("value", "3"),
            ("disabled", ""),
        ],
    );
    // Through an App: it seeds the number fields' text from `value` and
    // installs the form hooks.
    let app = app(
        dom,
        ":in-range { color: blue } :out-of-range { color: red }",
    );
    for (id, want, what) in [
        (low, RED, "under min"),
        (ok, BLUE, "inside"),
        (free, UNSTYLED, "no limits"),
        (slider, BLUE, "range"),
        (late, RED, "after max"),
        (month, BLUE, "at min"),
        (week, RED, "before min"),
        (night, BLUE, "inside a reversed time range"),
        (noon, RED, "outside a reversed time range"),
        (stamp, RED, "a minute early, normalized separator"),
        (off, UNSTYLED, "barred"),
    ] {
        let got = app_fg(&app, id);
        if want == UNSTYLED {
            // Neither rule: the UA's color (`:disabled` grays a field).
            assert!(got != RED && got != BLUE, "{what}: {got:?}");
        } else {
            assert_eq!(got, want, "{what}");
        }
    }
}

/// A value or a limit changed restyles the control.
#[test]
fn a_new_value_or_limit_restyles_the_range_state() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let n = node(
        &mut dom,
        root,
        "input",
        &[("type", "number"), ("max", "10"), ("value", "4")],
    );
    let mut app = app(
        dom,
        ":in-range { color: blue } :out-of-range { color: red }",
    );
    assert_eq!(app_fg(&app, n), BLUE);
    rdom_tui::runtime::builtins::input::set_value(app.dom_mut(), n, "12");
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, n), RED);
    app.dom_mut().set_attribute(n, "max", "20").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, n), BLUE);
}

/// HTML §4.10.5.1.7–§4.10.5.1.12: a date-like value before `min` suffers
/// from an underflow, after `max` from an overflow — `:invalid` too.
#[test]
fn an_out_of_range_date_is_invalid() {
    use rdom_tui::runtime::builtins::validation;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = node(
        &mut dom,
        root,
        "input",
        &[
            ("type", "date"),
            ("min", "2026-10-01"),
            ("value", "2026-09-30"),
        ],
    );
    validation::install(&mut dom);
    let v = validation::validity(&dom, d);
    assert!(v.range_underflow && !v.range_overflow);
    assert!(validation::is_invalid(&dom, d));
}

// ─── :default ───────────────────────────────────────────────────────

/// Selectors 4 §14.4.2, HTML §4.16.3: the default button, the authored
/// `checked` / `selected` — which stay the defaults after the user
/// changes the live state (rdom keeps the defaults beside the live
/// attributes, DIVERGENCES §2).
#[test]
fn default_styles_through_a_sheet_and_follows_the_defaults_not_the_live_state() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let form = node(&mut dom, root, "form", &[]);
    let go = node(&mut dom, form, "button", &[]);
    let again = node(&mut dom, form, "button", &[]);
    let cb = node(
        &mut dom,
        form,
        "input",
        &[("type", "checkbox"), ("checked", "")],
    );
    let select = node(&mut dom, form, "select", &[]);
    let a = node(&mut dom, select, "option", &[]);
    let b = node(&mut dom, select, "option", &[("selected", "")]);
    let mut app = app(dom, ":default { color: red }");
    assert_eq!(app_fg(&app, go), RED);
    assert_ne!(app_fg(&app, again), RED, "a later submit button");
    assert_eq!(app_fg(&app, cb), RED);
    assert_eq!(app_fg(&app, b), RED);
    assert_ne!(app_fg(&app, a), RED);
    // The user unchecks the box: it is still the default-checked one.
    use rdom_tui::TuiAccessorsMut;
    app.dom_mut().node_mut(cb).click();
    app.advance(0).unwrap();
    assert!(!app.dom().node(cb).has_attribute("checked"));
    assert_eq!(app_fg(&app, cb), RED);
    // A new default changes the match.
    app.dom_mut()
        .node_mut(cb)
        .set_default_checked(false)
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, cb), UNSTYLED);
}

/// A submit button inserted before the default one takes `:default`
/// from it — a change far from the old default in the tree.
#[test]
fn an_earlier_submit_button_takes_default_from_the_old_one() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let form = node(&mut dom, root, "form", &[]);
    let head = node(&mut dom, form, "div", &[]);
    let foot = node(&mut dom, form, "div", &[]);
    let old = node(&mut dom, foot, "button", &[]);
    let mut app = app(dom, "button:default { color: red }");
    assert_eq!(app_fg(&app, old), RED);
    let new = app.dom_mut().create_element("button");
    app.dom_mut().append_child(head, new).unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, new), RED);
    assert_ne!(app_fg(&app, old), RED);
}
