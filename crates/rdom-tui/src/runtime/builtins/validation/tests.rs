//! Constraint validation tests (HTML §4.10.20): validity states per
//! control, barring, the element / form API and the `invalid` event.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::accessors::{TuiAccessors, TuiAccessorsMut};
use crate::runtime::editing::perform::{Edit, perform_edit};

/// A `<form>` under the root.
fn form_dom() -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    (dom, form)
}

fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

fn input(dom: &mut TuiDom, parent: NodeId, attrs: &[(&str, &str)]) -> NodeId {
    let i = el(dom, parent, "input", attrs);
    crate::runtime::builtins::input::ensure_seeded(dom, i);
    i
}

fn option(dom: &mut TuiDom, parent: NodeId, value: &str, selected: bool) -> NodeId {
    let o = el(dom, parent, "option", &[("value", value)]);
    let t = dom.create_text_node(if value.is_empty() { "Pick" } else { value });
    dom.append_child(o, t).unwrap();
    if selected {
        dom.set_attribute(o, "selected", "").unwrap();
    }
    o
}

fn validity(dom: &TuiDom, id: NodeId) -> crate::ValidityState {
    dom.node(id).validity().expect("a control has a validity")
}

/// Type `text` at the end of an input / textarea as the user would:
/// through the editing pipeline.
fn user_types(dom: &mut TuiDom, control: NodeId, text: &str) {
    crate::runtime::builtins::input::ensure_seeded(dom, control);
    let t = dom
        .node(control)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .expect("seeded text child");
    let end = dom.node(t).node_value().map(str::len).unwrap_or(0);
    let outcome = perform_edit(
        dom,
        Edit {
            node: t,
            range: end..end,
            text: text.to_string(),
        },
    );
    assert_eq!(
        outcome,
        crate::runtime::editing::perform::EditOutcome::Applied
    );
}

/// Log of `invalid` events: the target of each.
fn record_invalid(dom: &mut TuiDom, ids: &[NodeId]) -> Rc<RefCell<Vec<NodeId>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    for &id in ids {
        let l = log.clone();
        dom.add_event_listener(id, "invalid", ListenerOptions::default(), move |ctx| {
            l.borrow_mut().push(ctx.event.target.unwrap());
        })
        .unwrap();
    }
    log
}

// ── valueMissing ────────────────────────────────────────────────────

#[test]
fn required_empty_text_controls_suffer_from_being_missing() {
    let (mut dom, form) = form_dom();
    let text = input(&mut dom, form, &[("required", "")]);
    let area = el(&mut dom, form, "textarea", &[("required", "")]);
    let optional = input(&mut dom, form, &[]);
    assert!(validity(&dom, text).value_missing);
    assert!(!validity(&dom, text).valid());
    assert!(validity(&dom, area).value_missing);
    assert!(validity(&dom, optional).valid());

    dom.node_mut(text).set_value("x").unwrap();
    dom.node_mut(area).set_value("y").unwrap();
    assert!(validity(&dom, text).valid());
    assert!(validity(&dom, area).valid());
}

#[test]
fn a_required_checkbox_is_missing_until_checked() {
    let (mut dom, form) = form_dom();
    let cb = input(&mut dom, form, &[("type", "checkbox"), ("required", "")]);
    assert!(validity(&dom, cb).value_missing);
    dom.node_mut(cb).set_checked(true).unwrap();
    assert!(validity(&dom, cb).valid());
}

/// HTML §4.10.5.1.18: a radio group is missing when any member is
/// required and no member is checked — every member suffers.
#[test]
fn a_radio_group_with_a_required_member_is_missing_until_one_is_checked() {
    let (mut dom, form) = form_dom();
    let a = input(
        &mut dom,
        form,
        &[("type", "radio"), ("name", "g"), ("required", "")],
    );
    let b = input(&mut dom, form, &[("type", "radio"), ("name", "g")]);
    let other = input(&mut dom, form, &[("type", "radio"), ("name", "h")]);
    assert!(validity(&dom, a).value_missing);
    assert!(validity(&dom, b).value_missing, "the whole group suffers");
    assert!(validity(&dom, other).valid(), "another group does not");
    dom.node_mut(b).set_checked(true).unwrap();
    assert!(validity(&dom, a).valid());
    assert!(validity(&dom, b).valid());
}

/// The group behind `valueMissing` is the form-owner group: a checked
/// same-named radio in another form does not satisfy it, one pointing
/// at this form with `form=` does.
#[test]
fn required_radio_validity_follows_the_form_owner_group() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let fa = el(&mut dom, root, "form", &[("id", "a")]);
    let fb = el(&mut dom, root, "form", &[("id", "b")]);
    let req = input(
        &mut dom,
        fa,
        &[("type", "radio"), ("name", "g"), ("required", "")],
    );
    let other_form = input(&mut dom, fb, &[("type", "radio"), ("name", "g")]);
    dom.node_mut(other_form).set_checked(true).unwrap();
    assert!(
        validity(&dom, req).value_missing,
        "another form's radio is another group"
    );
    assert!(validity(&dom, other_form).valid());
    let pointing = input(
        &mut dom,
        root,
        &[("type", "radio"), ("name", "g"), ("form", "a")],
    );
    assert!(
        validity(&dom, pointing).value_missing,
        "form=a joins a's group"
    );
    dom.node_mut(pointing).set_checked(true).unwrap();
    assert!(validity(&dom, req).valid());
}

/// HTML §4.10.7: a required select is missing when nothing is selected
/// or only its placeholder label option (first option, value `""`,
/// child of the select, single-select with display size 1).
#[test]
fn a_required_select_is_missing_on_its_placeholder_or_no_selection() {
    let (mut dom, form) = form_dom();
    let sel = el(&mut dom, form, "select", &[("required", "")]);
    let placeholder = option(&mut dom, sel, "", true);
    let a = option(&mut dom, sel, "a", false);
    assert!(validity(&dom, sel).value_missing);
    dom.remove_attribute(placeholder, "selected").unwrap();
    dom.set_attribute(a, "selected", "").unwrap();
    assert!(validity(&dom, sel).valid());

    // In an <optgroup>, an empty-valued first option is a real choice.
    let grouped = el(&mut dom, form, "select", &[("required", "")]);
    let group = el(&mut dom, grouped, "optgroup", &[]);
    option(&mut dom, group, "", true);
    assert!(validity(&dom, grouped).valid());

    // A multi-select is missing only with nothing selected.
    let multi = el(
        &mut dom,
        form,
        "select",
        &[("required", ""), ("multiple", "")],
    );
    let m = option(&mut dom, multi, "", false);
    assert!(validity(&dom, multi).value_missing);
    dom.set_attribute(m, "selected", "").unwrap();
    assert!(validity(&dom, multi).valid());
}

// ── typeMismatch ────────────────────────────────────────────────────

#[test]
fn email_inputs_suffer_a_type_mismatch_on_an_invalid_address() {
    let (mut dom, form) = form_dom();
    let email = input(&mut dom, form, &[("type", "email")]);
    for (value, mismatch) in [
        ("", false),
        ("a@b", false),
        ("first.last+tag@sub.example.test", false),
        ("a@", true),
        ("@b", true),
        ("a b@c", true),
        ("a@-b", true),
        ("a@b..c", true),
        ("a@b,c@d", true),
    ] {
        dom.node_mut(email).set_value(value).unwrap();
        assert_eq!(validity(&dom, email).type_mismatch, mismatch, "{value:?}");
    }
    let many = input(&mut dom, form, &[("type", "email"), ("multiple", "")]);
    dom.node_mut(many).set_value("a@b, c@d").unwrap();
    assert!(validity(&dom, many).valid());
    dom.node_mut(many).set_value("a@b,oops").unwrap();
    assert!(validity(&dom, many).type_mismatch);
}

#[test]
fn url_inputs_suffer_a_type_mismatch_without_a_scheme() {
    let (mut dom, form) = form_dom();
    let url = input(&mut dom, form, &[("type", "URL")]);
    for (value, mismatch) in [
        ("", false),
        ("https://example.test/a?b", false),
        ("mailto:a@b", false),
        ("example.test", true),
        ("https://exa mple.test", true),
        ("1http://x", true),
        ("http:", true),
    ] {
        dom.node_mut(url).set_value(value).unwrap();
        assert_eq!(validity(&dom, url).type_mismatch, mismatch, "{value:?}");
    }
}

// ── patternMismatch ─────────────────────────────────────────────────

/// HTML §4.10.5.3.6: the pattern must match the whole value
/// (`^(?:pattern)$`); an empty value or an invalid pattern never
/// mismatches; `pattern` does not apply to a checkbox.
#[test]
fn pattern_matches_the_whole_value() {
    let (mut dom, form) = form_dom();
    let code = input(&mut dom, form, &[("pattern", "[a-z]{3}")]);
    for (value, mismatch) in [("", false), ("abc", false), ("abcd", true), ("ab", true)] {
        dom.node_mut(code).set_value(value).unwrap();
        assert_eq!(validity(&dom, code).pattern_mismatch, mismatch, "{value:?}");
    }
    let alt = input(&mut dom, form, &[("pattern", "a|b"), ("value", "ab")]);
    assert!(
        validity(&dom, alt).pattern_mismatch,
        "alternation is grouped"
    );
    let broken = input(&mut dom, form, &[("pattern", "("), ("value", "x")]);
    assert!(
        validity(&dom, broken).valid(),
        "an invalid pattern is ignored"
    );
    let cb = input(
        &mut dom,
        form,
        &[("type", "checkbox"), ("pattern", "x"), ("value", "on")],
    );
    assert!(validity(&dom, cb).valid());
}

// ── tooLong / tooShort ──────────────────────────────────────────────

/// HTML §4.10.5.3.1: `maxlength` / `minlength` constrain only a value
/// the user last edited, measured in UTF-16 code units.
#[test]
fn length_limits_apply_only_to_a_user_edited_value() {
    let (mut dom, form) = form_dom();
    let short = input(&mut dom, form, &[("maxlength", "3"), ("value", "abcdef")]);
    assert!(validity(&dom, short).valid(), "an authored value is exempt");
    user_types(&mut dom, short, "g");
    assert!(validity(&dom, short).too_long);
    dom.node_mut(short).set_value("abcdef").unwrap();
    assert!(
        validity(&dom, short).valid(),
        "a programmatic value is exempt again"
    );

    let long = input(&mut dom, form, &[("minlength", "3")]);
    user_types(&mut dom, long, "\u{1F600}");
    assert!(
        validity(&dom, long).too_short,
        "one astral char is two code units"
    );
    user_types(&mut dom, long, "a");
    assert!(validity(&dom, long).valid(), "three code units");

    let area = el(&mut dom, form, "textarea", &[("maxlength", "1")]);
    user_types(&mut dom, area, "ab");
    assert!(validity(&dom, area).too_long);
}

// ── rangeUnderflow / rangeOverflow / stepMismatch / badInput ────────

#[test]
fn number_inputs_check_min_max_and_step() {
    let (mut dom, form) = form_dom();
    let n = input(
        &mut dom,
        form,
        &[
            ("type", "number"),
            ("min", "1"),
            ("max", "10"),
            ("step", "2"),
        ],
    );
    let check = |dom: &mut TuiDom, v: &str| {
        dom.node_mut(n).set_value(v).unwrap();
        let s = validity(dom, n);
        (s.range_underflow, s.range_overflow, s.step_mismatch)
    };
    assert_eq!(check(&mut dom, "5"), (false, false, false));
    assert_eq!(check(&mut dom, "-1"), (true, false, false));
    assert_eq!(check(&mut dom, "11"), (false, true, false));
    assert_eq!(check(&mut dom, "4"), (false, false, true), "base is min");
    assert_eq!(check(&mut dom, "1.0e1"), (false, false, true));
    assert_eq!(check(&mut dom, ""), (false, false, false));

    dom.set_attribute(n, "step", "ANY").unwrap();
    assert_eq!(check(&mut dom, "4"), (false, false, false), "step=any");
    dom.set_attribute(n, "step", "0.1").unwrap();
    assert_eq!(
        check(&mut dom, "1.3"),
        (false, false, false),
        "no float noise"
    );

    dom.set_attribute(n, "step", "bogus").unwrap();
    assert_eq!(
        check(&mut dom, "2.5"),
        (false, false, true),
        "default step 1"
    );
}

#[test]
fn a_number_input_with_unparsable_text_suffers_bad_input() {
    let (mut dom, form) = form_dom();
    let n = input(&mut dom, form, &[("type", "number"), ("required", "")]);
    dom.node_mut(n).set_value("1-2").unwrap();
    let s = validity(&dom, n);
    assert!(s.bad_input);
    assert!(!s.value_missing && !s.step_mismatch);
    dom.node_mut(n).set_value("inf").unwrap();
    assert!(validity(&dom, n).bad_input, "not an HTML float");
    dom.node_mut(n).set_value("-.5").unwrap();
    let s = validity(&dom, n);
    assert!(!s.bad_input, "-.5 is a valid float");
    assert!(s.step_mismatch, "base 0, step 1");
}

/// A range's value is always in range and on a step (HTML sanitizes
/// it), so it never suffers those states.
#[test]
fn a_range_input_never_underflows_overflows_or_mismatches_its_step() {
    let (mut dom, form) = form_dom();
    let r = input(
        &mut dom,
        form,
        &[
            ("type", "range"),
            ("min", "0"),
            ("max", "10"),
            ("step", "4"),
            ("value", "3"),
        ],
    );
    assert!(validity(&dom, r).valid());
}

// ── customError, messages, willValidate ─────────────────────────────

#[test]
fn set_custom_validity_sets_and_clears_a_custom_error() {
    let (mut dom, form) = form_dom();
    let text = input(&mut dom, form, &[]);
    dom.node_mut(text).set_custom_validity("Taken").unwrap();
    assert!(validity(&dom, text).custom_error);
    assert_eq!(
        dom.node(text).validation_message().as_deref(),
        Some("Taken")
    );
    assert!(!dom.node_mut(text).check_validity());
    dom.node_mut(text).set_custom_validity("").unwrap();
    assert!(validity(&dom, text).valid());
    assert_eq!(dom.node(text).validation_message().as_deref(), Some(""));
}

#[test]
fn validation_messages_describe_the_first_failing_state() {
    let (mut dom, form) = form_dom();
    let msg = |dom: &TuiDom, id| dom.node(id).validation_message().unwrap();
    let text = input(&mut dom, form, &[("required", "")]);
    assert_eq!(msg(&dom, text), "Please fill out this field.");
    let cb = input(&mut dom, form, &[("type", "checkbox"), ("required", "")]);
    assert_eq!(
        msg(&dom, cb),
        "Please check this box if you want to proceed."
    );
    let radio = input(&mut dom, form, &[("type", "radio"), ("required", "")]);
    assert_eq!(msg(&dom, radio), "Please select one of these options.");
    let sel = el(&mut dom, form, "select", &[("required", "")]);
    assert_eq!(msg(&dom, sel), "Please select an item in the list.");
    let email = input(&mut dom, form, &[("type", "email"), ("value", "x")]);
    assert_eq!(msg(&dom, email), "Please enter an email address.");
    let url = input(&mut dom, form, &[("type", "url"), ("value", "x")]);
    assert_eq!(msg(&dom, url), "Please enter a URL.");
    let pat = input(&mut dom, form, &[("pattern", "\\d+"), ("value", "x")]);
    assert_eq!(msg(&dom, pat), "Please match the requested format.");
    let n = input(
        &mut dom,
        form,
        &[
            ("type", "number"),
            ("min", "2"),
            ("max", "4"),
            ("value", "1"),
        ],
    );
    assert_eq!(msg(&dom, n), "Value must be greater than or equal to 2.");
    dom.node_mut(n).set_value("5").unwrap();
    assert_eq!(msg(&dom, n), "Value must be less than or equal to 4.");
    dom.node_mut(n).set_value("2.5").unwrap();
    assert_eq!(
        msg(&dom, n),
        "Please enter a valid value. The two nearest valid values are 2 and 3."
    );
    dom.node_mut(n).set_value("x").unwrap();
    assert_eq!(msg(&dom, n), "Please enter a number.");
    let long = input(&mut dom, form, &[("maxlength", "2")]);
    user_types(&mut dom, long, "abc");
    assert_eq!(
        msg(&dom, long),
        "Please shorten this text to 2 characters or less (you are currently using 3 characters)."
    );
    let short = input(&mut dom, form, &[("minlength", "4")]);
    user_types(&mut dom, short, "abc");
    assert_eq!(
        msg(&dom, short),
        "Please lengthen this text to 4 characters or more (you are currently using 3 characters)."
    );
    let div = el(&mut dom, form, "div", &[]);
    assert_eq!(dom.node(div).validation_message(), None);
    assert_eq!(dom.node(div).validity(), None);
}

/// HTML §4.10.20.1: disabled controls (own attribute or a disabled
/// fieldset), `readonly` text controls, hidden / reset / button inputs,
/// non-submit buttons and anything in a `<datalist>` are barred from
/// constraint validation — `checkValidity()` is true and fires nothing,
/// the message is empty, while `validity` still reports the states.
#[test]
fn barred_controls_are_not_validated() {
    let (mut dom, form) = form_dom();
    let disabled = input(&mut dom, form, &[("required", ""), ("disabled", "")]);
    let fieldset = el(&mut dom, form, "fieldset", &[("disabled", "")]);
    let in_fieldset = input(&mut dom, fieldset, &[("required", "")]);
    let readonly = input(&mut dom, form, &[("required", ""), ("readonly", "")]);
    let hidden = input(&mut dom, form, &[("type", "hidden"), ("required", "")]);
    let plain = el(&mut dom, form, "button", &[("type", "button")]);
    dom.node_mut(plain).set_custom_validity("x").unwrap();
    let barred = [disabled, in_fieldset, readonly, hidden, plain];
    let log = record_invalid(&mut dom, &barred);
    for id in barred {
        assert!(!dom.node(id).will_validate(), "{id:?}");
        assert!(dom.node_mut(id).check_validity(), "{id:?}");
        assert_eq!(dom.node(id).validation_message().as_deref(), Some(""));
    }
    assert!(log.borrow().is_empty());
    assert!(
        validity(&dom, disabled).value_missing,
        "the getter still reports"
    );
    assert!(dom.node_mut(form).check_validity(), "nothing to validate");

    // readonly does not apply to a checkbox, so it is not barred.
    let cb = input(
        &mut dom,
        form,
        &[("type", "checkbox"), ("required", ""), ("readonly", "")],
    );
    assert!(dom.node(cb).will_validate());
    assert!(!dom.node_mut(cb).check_validity());
}

// ── checkValidity / reportValidity and the invalid event ────────────

/// HTML §4.10.20.3: `checkValidity()` fires a cancelable, non-bubbling
/// `invalid` at an invalid control and returns false; a valid control
/// fires nothing.
#[test]
fn check_validity_fires_a_cancelable_non_bubbling_invalid_event() {
    let (mut dom, form) = form_dom();
    let text = input(&mut dom, form, &[("required", "")]);
    let seen = Rc::new(Cell::new((0u32, false, false)));
    let s = seen.clone();
    dom.add_event_listener(text, "invalid", ListenerOptions::default(), move |ctx| {
        let (n, _, _) = s.get();
        s.set((n + 1, ctx.event.cancelable, ctx.event.bubbles));
    })
    .unwrap();
    let bubbled = Rc::new(Cell::new(0u32));
    let b = bubbled.clone();
    let root = dom.root();
    dom.add_event_listener(root, "invalid", ListenerOptions::default(), move |_| {
        b.set(b.get() + 1);
    })
    .unwrap();
    assert!(!dom.node_mut(text).check_validity());
    assert_eq!(seen.get(), (1, true, false));
    assert_eq!(bubbled.get(), 0, "invalid does not bubble");
    dom.node_mut(text).set_value("ok").unwrap();
    assert!(dom.node_mut(text).check_validity());
    assert_eq!(seen.get().0, 1);
}

/// `reportValidity()` also reports the problem: rdom has no validation
/// bubble, so it focuses the control — unless `invalid` was canceled.
#[test]
fn report_validity_focuses_the_control_unless_invalid_is_canceled() {
    let (mut dom, form) = form_dom();
    let text = input(&mut dom, form, &[("required", "")]);
    assert!(!dom.node_mut(text).report_validity());
    assert_eq!(dom.focused(), Some(text));

    let other = input(&mut dom, form, &[("required", "")]);
    dom.add_event_listener(other, "invalid", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default();
    })
    .unwrap();
    assert!(!dom.node_mut(other).report_validity());
    assert_eq!(
        dom.focused(),
        Some(text),
        "a canceled invalid is not reported"
    );
}

/// HTML §4.10.20.2: form `checkValidity()` fires `invalid` at every
/// invalid control it owns, in tree order, and returns false.
#[test]
fn form_check_validity_fires_invalid_at_every_invalid_control() {
    let (mut dom, form) = form_dom();
    let a = input(&mut dom, form, &[("required", "")]);
    let ok = input(&mut dom, form, &[("value", "x")]);
    let b = el(&mut dom, form, "textarea", &[("required", "")]);
    let root = dom.root();
    let outside = input(&mut dom, root, &[("required", "")]);
    let log = record_invalid(&mut dom, &[a, ok, b, outside]);
    assert!(!dom.node_mut(form).check_validity());
    assert_eq!(*log.borrow(), vec![a, b]);
    assert_eq!(dom.focused(), None, "checkValidity reports nothing");
}

/// Form `reportValidity()` focuses the first invalid control whose
/// `invalid` event was not canceled.
#[test]
fn form_report_validity_focuses_the_first_unhandled_invalid_control() {
    let (mut dom, form) = form_dom();
    let a = input(&mut dom, form, &[("required", "")]);
    let b = input(&mut dom, form, &[("required", "")]);
    dom.add_event_listener(a, "invalid", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default();
    })
    .unwrap();
    assert!(!dom.node_mut(form).report_validity());
    assert_eq!(dom.focused(), Some(b));
    dom.node_mut(a).set_value("x").unwrap();
    dom.node_mut(b).set_value("y").unwrap();
    assert!(dom.node_mut(form).report_validity());
}

// ── P7-VALIDATION-SELECTORS-1 ───────────────────────────────────────

/// `validation::install` hooks the real validity states into the
/// selector engine: `:invalid` / `:valid` follow values, custom errors
/// and the form's owned controls.
#[test]
fn installed_hook_drives_valid_and_invalid() {
    let (mut dom, form) = form_dom();
    crate::runtime::builtins::validation::install(&mut dom);
    let text = input(&mut dom, form, &[("required", "")]);
    assert!(dom.matches(text, "input:invalid:required").unwrap());
    assert!(dom.matches(form, "form:invalid").unwrap());
    dom.node_mut(text).set_value("x").unwrap();
    assert!(dom.matches(text, ":valid").unwrap());
    assert!(dom.matches(form, ":valid").unwrap());
    dom.node_mut(text).set_custom_validity("no").unwrap();
    assert!(dom.matches(text, ":invalid").unwrap());
}

/// The UA sheet does not style `:invalid` / `:valid` (browsers show
/// only the bubble), so a page without author validity rules pays for
/// no validity walk.
#[test]
fn the_ua_sheet_has_no_validity_rules() {
    assert!(!super::marks::uses_validity(&crate::Stylesheet::new()));
    let author = crate::Stylesheet::bare().rule_unchecked(
        "form :not(:where(input:invalid)) + p",
        crate::TuiStyle::new(),
    );
    assert!(super::marks::uses_validity(&author));
}

/// Selectors re-match when validity changes without a cascade-dirtying
/// mutation: a textarea's text edit (character data) and
/// `set_custom_validity` (no mutation at all) both restyle the control
/// and its form on the next frame.
#[test]
fn validity_changes_restyle_on_the_next_frame() {
    use crate::render::{Terminal, TestBackend};
    use crate::runtime::app::App;
    use crate::style::cascade::computed_of;
    use crate::style::{Color, Stylesheet, TuiStyle};

    let (mut dom, form) = form_dom();
    let area = el(&mut dom, form, "textarea", &[("required", "")]);
    let t = dom.create_text_node("");
    dom.append_child(area, t).unwrap();
    let text = input(&mut dom, form, &[("value", "ok")]);
    let red = Color::Rgb(255, 0, 0);
    let sheet = Stylesheet::new()
        .rule_unchecked("input:invalid", TuiStyle::new().fg(red))
        .rule_unchecked("textarea:invalid", TuiStyle::new().fg(red))
        .rule_unchecked("form:invalid", TuiStyle::new().bg(red));
    let mut app =
        App::with_backend(dom, sheet, Terminal::new(TestBackend::new(40, 6)).unwrap()).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(computed_of(app.dom(), area).fg, red);
    assert_eq!(computed_of(app.dom(), form).bg, red);
    assert_ne!(computed_of(app.dom(), text).fg, red);

    app.dom_mut().node_mut(t).set_node_value("filled").unwrap();
    app.draw_if_dirty().unwrap();
    assert_ne!(
        computed_of(app.dom(), area).fg,
        red,
        "textarea edit restyles"
    );
    assert_ne!(computed_of(app.dom(), form).bg, red, "and its form");

    app.dom_mut()
        .node_mut(text)
        .set_custom_validity("taken")
        .unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(
        computed_of(app.dom(), text).fg,
        red,
        "custom error restyles"
    );
    assert_eq!(computed_of(app.dom(), form).bg, red);
}
