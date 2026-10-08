//! C11-FORM-STATES — the input pseudo-classes of Selectors 4 §14 with
//! HTML §4.16.3's definitions of what each matches.

use crate::{Dom, NodeId};

fn el(dom: &mut Dom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

fn is(dom: &Dom, id: NodeId, sel: &str) -> bool {
    dom.matches(id, sel)
        .unwrap_or_else(|e| panic!("{sel}: {e}"))
}

// ─── :read-only / :read-write ───────────────────────────────────────

/// Selectors 4 §14.3.1, HTML §4.16.3: `:read-write` matches an `input`
/// that `readonly` applies to and that is mutable (neither `readonly`
/// nor disabled), a `textarea` that is neither, and any other element
/// that is an editing host or editable; `:read-only` every other
/// element.
#[test]
fn read_write_follows_htmls_mutability_rules() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let text = el(&mut dom, root, "input", &[]);
    let number = el(&mut dom, root, "input", &[("type", "number")]);
    let ro = el(&mut dom, root, "input", &[("readonly", "")]);
    let off = el(&mut dom, root, "input", &[("disabled", "")]);
    let checkbox = el(&mut dom, root, "input", &[("type", "checkbox")]);
    let range = el(&mut dom, root, "input", &[("type", "range")]);
    let area = el(&mut dom, root, "textarea", &[]);
    let ro_area = el(&mut dom, root, "textarea", &[("readonly", "")]);
    let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
    let in_fs = el(&mut dom, fs, "textarea", &[]);
    let select = el(&mut dom, root, "select", &[]);
    let div = el(&mut dom, root, "div", &[]);
    let host = el(&mut dom, root, "div", &[("contenteditable", "")]);
    let inner = el(&mut dom, host, "p", &[]);
    let frozen = el(&mut dom, host, "p", &[("contenteditable", "false")]);
    let in_frozen = el(&mut dom, frozen, "b", &[]);
    let plain = el(
        &mut dom,
        root,
        "span",
        &[("contenteditable", "PlainText-Only")],
    );
    let ce_input = el(
        &mut dom,
        root,
        "input",
        &[("contenteditable", ""), ("readonly", "")],
    );
    for (id, rw) in [
        (text, true),
        (number, true),
        (ro, false),
        (off, false),
        (checkbox, false),
        (range, false),
        (area, true),
        (ro_area, false),
        (in_fs, false),
        (select, false),
        (div, false),
        (host, true),
        (inner, true),
        (frozen, false),
        (in_frozen, false),
        (plain, true),
        (ce_input, false),
    ] {
        assert_eq!(is(&dom, id, ":read-write"), rw, "{id:?} :read-write");
        assert_eq!(is(&dom, id, ":read-only"), !rw, "{id:?} :read-only");
    }
}

/// Pseudo-class names are ASCII case-insensitive.
#[test]
fn read_only_names_parse_case_insensitively() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let input = el(&mut dom, root, "input", &[]);
    assert!(is(&dom, input, ":READ-WRITE"));
    assert!(!is(&dom, input, ":Read-Only"));
}
