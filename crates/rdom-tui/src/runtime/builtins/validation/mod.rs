//! Constraint validation (HTML §4.10.20) for the controls rdom ships.
//!
//! ## Contract
//!
//! - **Candidates** — `Dom::will_validate` (rdom-core): a submittable
//!   control that is not barred (disabled, `readonly` text control,
//!   hidden / reset / button input, non-submit button, inside a
//!   `<datalist>`). Only candidates are checked, reported or block a
//!   submission.
//! - **Validity states** — [`validity`] computes a [`ValidityState`]
//!   from the control's current value and attributes (`states`), as the
//!   `ValidityState` getters do — for barred controls too:
//!   - `value_missing` — `required` and an empty value; an unchecked
//!     checkbox; a radio group with a required member and no checked
//!     member; a select with nothing selected or only its placeholder
//!     label option;
//!   - `type_mismatch` — `type=email` (the HTML "valid e-mail address"
//!     grammar, each address of a `multiple` list) and `type=url` (a
//!     simplified absolute-URL check, see DIVERGENCES);
//!   - `pattern_mismatch` — `pattern` against the whole value
//!     (`^(?:pattern)$`, module `pattern`), text / search / url / tel / email /
//!     password only;
//!   - `too_long` / `too_short` — `maxlength` / `minlength` in UTF-16
//!     code units, only while the value was last changed by a user edit
//!     (HTML's dirty value flag: an authored or programmatic value is
//!     exempt);
//!   - `range_underflow` / `range_overflow` / `step_mismatch` — `min` /
//!     `max` / `step` on `type=number` (a range's value is always in
//!     range and on a step, so it never suffers them);
//!   - `bad_input` — `type=number` text that is not a valid
//!     floating-point number;
//!   - `custom_error` — [`set_custom_validity`] with a non-empty message.
//! - **API** — [`check_validity`] fires a cancelable, non-bubbling
//!   `invalid` at an invalid candidate (at every invalid control a form
//!   owns, for a `<form>`) and returns false; [`report_validity`] does
//!   the same and then *reports*: rdom has no validation bubble, so it
//!   focuses the (first) invalid control whose `invalid` was not
//!   canceled. [`validation_message`] is rdom's English message for the
//!   first failing state (`messages`), `""` for a valid or barred control.
//! - **Submission** — `form::submit` runs `interactively_validate`
//!   unless `SubmitDetail::no_validate` (the form's `novalidate`, the
//!   submitter's `formnovalidate`): any invalid control blocks the
//!   `submit` event, whether or not its `invalid` was canceled.

mod messages;
mod pattern;
mod states;
mod syntax;

#[cfg(test)]
mod tests;

use rdom_core::NodeId;

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

pub(crate) use pattern::PatternCache;

/// The validity states of a control (HTML `ValidityState`). Every flag
/// is a state the control *suffers from*; [`ValidityState::valid`] is
/// true when it suffers from none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ValidityState {
    pub value_missing: bool,
    pub type_mismatch: bool,
    pub pattern_mismatch: bool,
    pub too_long: bool,
    pub too_short: bool,
    pub range_underflow: bool,
    pub range_overflow: bool,
    pub step_mismatch: bool,
    pub bad_input: bool,
    pub custom_error: bool,
}

impl ValidityState {
    /// Whether the control satisfies all its constraints — `validity.valid`.
    pub fn valid(&self) -> bool {
        *self == Self::default()
    }
}

/// The validity states of `id`, computed from its current value and
/// attributes (all false for an element without constraints).
pub fn validity(dom: &TuiDom, id: NodeId) -> ValidityState {
    states::compute(dom, id)
}

/// Whether `id` is a candidate for constraint validation that does not
/// satisfy its constraints.
pub fn is_invalid(dom: &TuiDom, id: NodeId) -> bool {
    dom.will_validate(id) && !validity(dom, id).valid()
}

/// `validationMessage`: rdom's English message for the first failing
/// state (a custom error's own message first), or `""` when `id` is
/// valid or barred.
///
/// HTML leaves the wording to the user agent; rdom's messages are fixed
/// English modeled on Chromium's, checked in this order:
///
/// | state | message |
/// |---|---|
/// | custom error | the `setCustomValidity` message |
/// | value missing | `Please fill out this field.` (checkbox: `Please check this box if you want to proceed.`; radio: `Please select one of these options.`; select: `Please select an item in the list.`) |
/// | type mismatch | `Please enter an email address.` / `Please enter a URL.` |
/// | bad input | `Please enter a number.` |
/// | pattern mismatch | `Please match the requested format.` |
/// | too long | `Please shorten this text to N characters or less (you are currently using M characters).` |
/// | too short | `Please lengthen this text to N characters or more (you are currently using M characters).` |
/// | range underflow | `Value must be greater than or equal to N.` |
/// | range overflow | `Value must be less than or equal to N.` |
/// | step mismatch | `Please enter a valid value. The two nearest valid values are A and B.` |
///
/// Character counts are UTF-16 code units, as the constraint is.
pub fn validation_message(dom: &TuiDom, id: NodeId) -> String {
    if !dom.will_validate(id) {
        return String::new();
    }
    messages::message(dom, id, &validity(dom, id))
}

/// `setCustomValidity(message)`: a non-empty `message` makes `id`
/// suffer from a custom error with that message; `""` clears it.
pub fn set_custom_validity(dom: &mut TuiDom, id: NodeId, message: &str) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.custom_validity = message.to_string();
    }
}

/// `checkValidity()`. On a `<form>`: statically validate it — fire
/// `invalid` at each invalid control it owns, in tree order — and
/// return whether there were none. On a control: fire `invalid` if it is
/// an invalid candidate and return false; true otherwise.
pub fn check_validity(dom: &mut TuiDom, id: NodeId) -> bool {
    if dom.node(id).tag_name() == Some("form") {
        return statically_validate(dom, id).is_empty();
    }
    if !is_invalid(dom, id) {
        return true;
    }
    fire_invalid(dom, id);
    false
}

/// `reportValidity()`: [`check_validity`], then report the problem —
/// focus the control (on a `<form>`, the first invalid control) whose
/// `invalid` event was not canceled. rdom shows no validation bubble.
pub fn report_validity(dom: &mut TuiDom, id: NodeId) -> bool {
    if dom.node(id).tag_name() == Some("form") {
        return interactively_validate(dom, id);
    }
    if !is_invalid(dom, id) {
        return true;
    }
    if fire_invalid(dom, id) {
        report(dom, id);
    }
    false
}

/// HTML §4.10.20.2 "interactively validate the constraints" of `form`:
/// statically validate it, then report the first unhandled invalid
/// control (focus it). True when every owned candidate is valid.
pub(crate) fn interactively_validate(dom: &mut TuiDom, form: NodeId) -> bool {
    let invalid = statically_validate(dom, form);
    if invalid.is_empty() {
        return true;
    }
    if let Some(&(first, _)) = invalid.iter().find(|(_, unhandled)| *unhandled) {
        report(dom, first);
    }
    false
}

/// HTML §4.10.20.2 "statically validate the constraints": snapshot the
/// invalid candidates `form` owns, then fire `invalid` at each in tree
/// order. Returns each with whether its event went uncanceled.
fn statically_validate(dom: &mut TuiDom, form: NodeId) -> Vec<(NodeId, bool)> {
    let invalid: Vec<NodeId> = dom
        .form_listed_elements(form)
        .into_iter()
        .filter(|&id| is_invalid(dom, id))
        .collect();
    invalid
        .into_iter()
        .map(|id| (id, fire_invalid(dom, id)))
        .collect()
}

/// Fire a cancelable, non-bubbling `invalid` at `id`. Returns true when
/// no listener canceled it.
fn fire_invalid(dom: &mut TuiDom, id: NodeId) -> bool {
    let mut ev = TuiEvent::new("invalid");
    ev.event.bubbles = false;
    ev.event.cancelable = true;
    let _ = dom.dispatch_tui_event(id, &mut ev);
    !ev.event.default_prevented()
}

/// Report a problem with `id` to the user: focus it (browsers also show
/// a bubble with the validation message, which rdom does not).
fn report(dom: &mut TuiDom, id: NodeId) {
    use crate::accessors::TuiAccessorsMut;
    dom.node_mut(id).focus();
}
