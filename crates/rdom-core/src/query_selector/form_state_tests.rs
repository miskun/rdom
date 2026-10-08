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

// ─── :indeterminate ─────────────────────────────────────────────────

/// Selectors 4 §14.4.3, HTML §4.16.3: `:indeterminate` matches a
/// checkbox whose `indeterminate` IDL attribute is true (rdom reflects it
/// into an `indeterminate` attribute, as it does `checked`), a radio
/// whose radio button group has no checked member, and a `<progress>`
/// without a `value` attribute.
#[test]
fn indeterminate_matches_checkboxes_radio_groups_and_progress() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let mixed = el(
        &mut dom,
        root,
        "input",
        &[("type", "checkbox"), ("indeterminate", "")],
    );
    let plain = el(&mut dom, root, "input", &[("type", "checkbox")]);
    let text = el(&mut dom, root, "input", &[("indeterminate", "")]);
    let a1 = el(&mut dom, root, "input", &[("type", "radio"), ("name", "a")]);
    let a2 = el(&mut dom, root, "input", &[("type", "radio"), ("name", "a")]);
    let b1 = el(&mut dom, root, "input", &[("type", "radio"), ("name", "b")]);
    let b2 = el(
        &mut dom,
        root,
        "input",
        &[("type", "radio"), ("name", "b"), ("checked", "")],
    );
    let alone = el(&mut dom, root, "input", &[("type", "radio")]);
    let busy = el(&mut dom, root, "progress", &[]);
    let done = el(&mut dom, root, "progress", &[("value", "1")]);
    let div = el(&mut dom, root, "div", &[("indeterminate", "")]);
    for (id, want) in [
        (mixed, true),
        (plain, false),
        (text, false),
        (a1, true),
        (a2, true),
        (b1, false),
        (b2, false),
        (alone, true),
        (busy, true),
        (done, false),
        (div, false),
    ] {
        assert_eq!(is(&dom, id, ":indeterminate"), want, "{id:?}");
        assert_eq!(dom.is_indeterminate(id), want, "{id:?}");
    }
    dom.set_attribute(a2, "checked", "").unwrap();
    assert!(
        !is(&dom, a1, ":indeterminate"),
        "the group has a checked member now"
    );
}

/// A pass answers a radio group's `:indeterminate` once for the whole
/// group (`SelectorCaches`): matching every radio of a long group costs
/// one walk of the tree, not one per radio.
#[test]
fn indeterminate_radio_groups_are_walked_once_per_pass() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    for _ in 0..200 {
        el(&mut dom, root, "input", &[("type", "radio"), ("name", "g")]);
    }
    let list = crate::selectors::parse(":indeterminate").unwrap();
    let mut caches = crate::SelectorCaches::new();
    let radios: Vec<NodeId> = dom.node(root).children().map(|c| c.id()).collect();
    for &r in &radios {
        assert!(dom.matches_list_with(r, &list, None, &mut caches));
    }
    assert_eq!(caches.work().radio_group_walks, 1);
}

// ─── :default ───────────────────────────────────────────────────────

/// Selectors 4 §14.4.2, HTML §4.16.3: `:default` matches a form's
/// default button — its first submit button in tree order (HTML
/// §4.10.21.2), `form=` owners included — a checkbox or radio whose
/// `checked` attribute was authored (`defaultChecked`) and an `<option>`
/// with `selected` (`defaultSelected`). Without a backend those defaults
/// are the attributes.
#[test]
fn default_matches_the_default_button_and_default_checked_controls() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let early = el(&mut dom, root, "input", &[("type", "image"), ("form", "f")]);
    let form = el(&mut dom, root, "form", &[("id", "f")]);
    let plain = el(&mut dom, form, "button", &[("type", "button")]);
    let first = el(&mut dom, form, "button", &[]);
    let second = el(&mut dom, form, "input", &[("type", "submit")]);
    let lone = el(&mut dom, root, "button", &[]);
    let other = el(&mut dom, root, "form", &[]);
    let off = el(&mut dom, other, "button", &[("disabled", "")]);
    let on = el(
        &mut dom,
        form,
        "input",
        &[("type", "checkbox"), ("checked", "")],
    );
    let radio = el(
        &mut dom,
        form,
        "input",
        &[("type", "radio"), ("checked", "")],
    );
    let text = el(&mut dom, form, "input", &[("checked", "")]);
    let select = el(&mut dom, form, "select", &[]);
    let picked = el(&mut dom, select, "option", &[("selected", "")]);
    let opt = el(&mut dom, select, "option", &[]);
    for (id, want) in [
        (early, true),
        (plain, false),
        (first, false),
        (second, false),
        (lone, false),
        (off, true),
        (on, true),
        (radio, true),
        (text, false),
        (select, false),
        (picked, true),
        (opt, false),
        (form, false),
    ] {
        assert_eq!(is(&dom, id, ":default"), want, "{id:?}");
    }
    dom.remove_child(root, early).unwrap();
    assert!(
        is(&dom, first, ":default"),
        "the next submit button takes over"
    );
}

/// With a control-state hook, `defaultChecked` / `defaultSelected` come
/// from the backend, which keeps them apart from the live attributes.
#[test]
fn default_asks_the_backend_for_default_checkedness() {
    use crate::ControlState;
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let flipped = el(
        &mut dom,
        root,
        "input",
        &[("type", "checkbox"), ("checked", "")],
    );
    let authored = el(
        &mut dom,
        root,
        "input",
        &[("type", "checkbox"), ("data-default", "")],
    );
    let select = el(&mut dom, root, "select", &[]);
    let picked = el(&mut dom, select, "option", &[("selected", "")]);
    let initial = el(&mut dom, select, "option", &[("data-default", "")]);
    dom.set_control_state_hook(Some(|dom, id, state| match state {
        ControlState::DefaultChecked | ControlState::DefaultSelected => {
            dom.has_attribute(id, "data-default")
        }
        _ => false,
    }));
    assert!(!is(&dom, flipped, ":default"));
    assert!(is(&dom, authored, ":default"));
    assert!(!is(&dom, picked, ":default"));
    assert!(is(&dom, initial, ":default"));
}

/// A pass finds each form's default button once (`SelectorCaches`):
/// matching `:default` on every button of a long form costs one walk of
/// the form's controls.
#[test]
fn default_buttons_are_found_once_per_form_per_pass() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let form = el(&mut dom, root, "form", &[]);
    let buttons: Vec<NodeId> = (0..100)
        .map(|_| el(&mut dom, form, "button", &[]))
        .collect();
    let list = crate::selectors::parse(":default").unwrap();
    let mut caches = crate::SelectorCaches::new();
    let hits = buttons
        .iter()
        .filter(|&&b| dom.matches_list_with(b, &list, None, &mut caches))
        .count();
    assert_eq!(hits, 1);
    assert_eq!(caches.work().default_button_walks, 1);
}

// ─── :in-range / :out-of-range ──────────────────────────────────────

/// Selectors 4 §14.3.3–§14.3.4, HTML §4.16.3: `:in-range` matches a
/// candidate for constraint validation that has range limitations and
/// suffers from neither an underflow nor an overflow, `:out-of-range`
/// one that has them and suffers from either. Both questions are the
/// backend's (values and their parsing live there); without a hook
/// nothing has range limitations.
#[test]
fn in_range_asks_the_backend_about_limits_and_the_value() {
    use crate::ControlState;
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let inside = el(&mut dom, root, "input", &[("type", "number"), ("min", "1")]);
    let outside = el(
        &mut dom,
        root,
        "input",
        &[("type", "number"), ("min", "1"), ("data-out", "")],
    );
    let unlimited = el(&mut dom, root, "input", &[("type", "number")]);
    let barred = el(
        &mut dom,
        root,
        "input",
        &[
            ("type", "number"),
            ("min", "1"),
            ("data-out", ""),
            ("disabled", ""),
        ],
    );
    let div = el(&mut dom, root, "div", &[("min", "1")]);
    for id in [inside, outside, unlimited, barred, div] {
        assert!(!is(&dom, id, ":in-range"), "no hook: {id:?}");
        assert!(!is(&dom, id, ":out-of-range"), "no hook: {id:?}");
    }
    dom.set_control_state_hook(Some(|dom, id, state| match state {
        ControlState::RangeLimited => dom.has_attribute(id, "min"),
        ControlState::OutOfRange => dom.has_attribute(id, "data-out"),
        _ => false,
    }));
    for (id, range) in [
        (inside, Some(true)),
        (outside, Some(false)),
        (unlimited, None),
        (barred, None),
        (div, None),
    ] {
        assert_eq!(is(&dom, id, ":in-range"), range == Some(true), "{id:?}");
        assert_eq!(
            is(&dom, id, ":out-of-range"),
            range == Some(false),
            "{id:?}"
        );
    }
}

// ─── :user-valid / :user-invalid ────────────────────────────────────

/// Selectors 4 §14.4.4–§14.4.5, HTML §4.16.3: `:user-valid` matches an
/// `input`, `textarea` or `select` whose *user validity* is true (the
/// backend's flag: the user committed a change, or a submission was
/// attempted), that is a candidate for constraint validation and
/// satisfies its constraints; `:user-invalid` one that does not. Without
/// a backend no element has interacted with the user.
#[test]
fn user_validity_combines_the_backends_flag_with_validity() {
    use crate::ControlState;
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let fresh = el(&mut dom, root, "input", &[("data-bad", "")]);
    let good = el(&mut dom, root, "input", &[("data-user", "")]);
    let bad = el(
        &mut dom,
        root,
        "textarea",
        &[("data-user", ""), ("data-bad", "")],
    );
    let select = el(
        &mut dom,
        root,
        "select",
        &[("data-user", ""), ("data-bad", "")],
    );
    let button = el(
        &mut dom,
        root,
        "button",
        &[("data-user", ""), ("data-bad", "")],
    );
    let barred = el(
        &mut dom,
        root,
        "input",
        &[("data-user", ""), ("data-bad", ""), ("disabled", "")],
    );
    let div = el(&mut dom, root, "div", &[("data-user", "")]);
    let probe = [fresh, good, bad, select, button, barred, div];
    for id in probe {
        assert!(
            !is(&dom, id, ":user-valid") && !is(&dom, id, ":user-invalid"),
            "no hook: {id:?}"
        );
    }
    dom.set_validity_hook(Some(|dom, id| !dom.has_attribute(id, "data-bad")));
    dom.set_control_state_hook(Some(|dom, id, state| {
        state == ControlState::UserValidity && dom.has_attribute(id, "data-user")
    }));
    for (id, user) in [
        (fresh, None),
        (good, Some(true)),
        (bad, Some(false)),
        (select, Some(false)),
        (button, None),
        (barred, None),
        (div, None),
    ] {
        assert_eq!(is(&dom, id, ":user-valid"), user == Some(true), "{id:?}");
        assert_eq!(is(&dom, id, ":user-invalid"), user == Some(false), "{id:?}");
    }
}
