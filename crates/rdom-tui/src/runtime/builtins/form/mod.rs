//! `<form>` submission + reset infrastructure.
//!
//! ## Contract (from MDN)
//!
//! - **Submit triggers**:
//!   - Click on `<input type="submit">`, `<button type="submit">`,
//!     or `<button>` without a `type` attribute (HTML default for
//!     buttons in a form is "submit").
//!   - **Implicit submission** (HTML §4.10.21.2): Enter in a
//!     single-line text input fires a `click` at the form's *default
//!     button* — the first submit button it owns, in tree order — which then
//!     submits with that button as the submitter; a disabled default
//!     button makes Enter do nothing. A form with no submit button
//!     submits (submitter `None`) only when the focused input is its
//!     one field that blocks implicit submission.
//! - **Form owner** (HTML §4.10.17.3, `Dom::form_owner`): a control's
//!   form is the `<form>` its `form="id"` attribute names, else its
//!   nearest ancestor `<form>`; a `form` attribute naming no form means
//!   no owner. Submission, reset, [`elements`], [`collect`] and implicit
//!   submission all work on the controls a form *owns*, in tree order
//!   of the whole document — controls outside the form that name it
//!   included.
//! - The `submit` event carries `EventDetail::Submit(SubmitDetail)`
//!   (`Dom::submit_detail`): `submitter` (`SubmitEvent.submitter`) and
//!   the effective `action` / `method` / `enctype` / `target` /
//!   `no_validate` — the submitter's `formaction` / `formmethod` /
//!   `formenctype` / `formtarget` / `formnovalidate` over the form's
//!   attributes (§4.10.19.6). rdom has no navigation; the handler
//!   decides what submitting means. A handler passes the submitter to
//!   [`collect_with_submitter`] so the entry list holds the submitter's
//!   name / value, as `new FormData(form, submitter)`.
//! - **Reset triggers**: click on `<input type="reset">` or
//!   `<button type="reset">`.
//! - Submit fires the `submit` event on the `<form>` element —
//!   bubbling, **cancelable**. `preventDefault()` blocks the
//!   default action (a TUI app's submit handler usually
//!   `preventDefault`s and reads form data via [`collect`]).
//! - Reset fires the `reset` event on the form, also cancelable.
//!   Unless canceled, every control goes back to its default:
//!   `defaultValue` (text controls, ranges), `defaultChecked`
//!   (checkboxes, radios), `defaultSelected` (`<select>` options).
//! - An effective method of `dialog` (the form's `method` or the
//!   submitter's `formmethod`) closes the form's nearest ancestor
//!   `<dialog>` with the submitter's `value` unless the submit was
//!   canceled.
//!
//! ## v1 deliberate simplifications
//!
//! - No client-side validation gate (`required`, `pattern`, `min`,
//!   `max` `valueMissing` blocking submit). Apps validate manually
//!   inside their `submit` handler.
//! - No `formdata` event (would require a `FormData` shim).

use rdom_core::{FormMethod, InputTypeState, ListenerOptions, NodeId};

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Install the form default actions. Two root-level listeners:
/// click (submit / reset trigger), keydown (implicit Enter
/// submit on single-text-input forms).
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();

    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(button) = closest_form_button(ctx.dom, target) else {
            return;
        };
        if ctx.dom.is_actually_disabled(button) {
            return;
        }
        let Some(form) = ctx.dom.form_owner(button) else {
            return;
        };
        match button_action(ctx.dom, button) {
            ButtonAction::Submit => {
                submit(ctx.dom, form, Some(button));
            }
            ButtonAction::Reset => {
                if !fire_reset(ctx.dom, form) {
                    reset_controls(ctx.dom, form);
                }
            }
            ButtonAction::Button => {} // no default action
        }
    })
    .expect("form click listener install");

    // Implicit submission (HTML §4.10.21.2), from Enter in a
    // single-line text-family input.
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        let no_mods = !key.modifiers.ctrl
            && !key.modifiers.shift
            && !key.modifiers.alt
            && !key.modifiers.meta;
        if !no_mods || key.key != "Enter" {
            return;
        }
        if !is_single_line_text_input(ctx.dom, focused) {
            return;
        }
        let Some(form) = ctx.dom.form_owner(focused) else {
            return;
        };
        if let Some(default) = default_button(ctx.dom, form) {
            // The default button's activation behavior does the
            // submitting (and the `method="dialog"` close) through the
            // click listener above, with it as the submitter. A
            // disabled default button blocks implicit submission.
            if !ctx.dom.is_actually_disabled(default) {
                use crate::accessors::TuiAccessorsMut;
                ctx.dom.node_mut(default).click();
            }
            return;
        }
        // No submit button: submit from the form itself, only when
        // the focused input is the one field that blocks implicit
        // submission.
        if count_text_inputs(ctx.dom, form) != 1 {
            return;
        }
        submit(ctx.dom, form, None);
    })
    .expect("form implicit-enter submit listener install");
}

/// The form's entry list with no submitter — `new FormData(form)`:
/// every form-controlled element's `(name, value)` pair, and no button
/// entry at all. A `submit` handler that wants the clicked button's
/// entry calls [`collect_with_submitter`] instead.
///
/// Rules (v1):
/// - Only elements with a non-empty `name` attribute participate.
/// - Actually disabled controls are skipped: own `disabled`, or inside a
///   `<fieldset disabled>` outside its first `<legend>` (HTML
///   §4.10.18.5, `Dom::is_actually_disabled`).
/// - `<input type="checkbox">` / `<input type="radio">` only
///   contribute when `checked` (matches HTML form-encoding).
/// - Checkbox value defaults to `"on"` when no `value` attribute
///   is set (HTML rule).
/// - Text-family inputs and `<textarea>` contribute their current
///   text content.
/// - `<select>` contributes one pair per selected option that is not
///   disabled — by its own attribute or a disabled `<optgroup>`
///   (HTML §4.10.21.4).
/// - Buttons (`<button>`, `<input type=submit|reset|button>`) are not
///   collected; see [`collect_with_submitter`] for the submitter.
/// - `<input type=hidden>` contributes its `value` attribute.
pub fn collect(dom: &TuiDom, form: NodeId) -> Vec<(String, String)> {
    collect_with_submitter(dom, form, None)
}

/// The entry list for a submission by `submitter` — HTML §4.10.21.4,
/// `new FormData(form, submitter)`. As [`collect`], plus one entry for
/// the submitter, in tree order, when it is a submit button (`<button>`
/// with a missing / invalid / `submit` type, `<input type=submit>`) that
/// has a non-empty `name` and is not disabled; its value is its `value`
/// attribute, or `""`. Any other button never contributes.
///
/// A `submit` handler passes the event's submitter:
/// `ctx.event.detail.as_submit().and_then(|s| s.submitter)`.
pub fn collect_with_submitter(
    dom: &TuiDom,
    form: NodeId,
    submitter: Option<NodeId>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for id in dom.form_listed_elements(form) {
        collect_entry(dom, id, submitter, &mut out);
    }
    out
}

// ── Internals ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonAction {
    Submit,
    Reset,
    Button,
}

/// HTML §4.10.6: a `<button>` is a submit button unless its `type` is
/// `reset` or `button` — the missing and the invalid value default are
/// both Submit (`Dom::is_submit_button`). `<input type=submit/reset/button>`
/// uses its type state. Keywords match ASCII case-insensitively
/// (§2.3.3). Anything else is `Button` (no default action).
fn button_action(dom: &TuiDom, id: NodeId) -> ButtonAction {
    if dom.is_submit_button(id) {
        return ButtonAction::Submit;
    }
    let reset = match dom.node(id).tag_name() {
        Some("button") => dom
            .node(id)
            .get_attribute("type")
            .is_some_and(|t| t.eq_ignore_ascii_case("reset")),
        _ => dom.input_type_state(id) == Some(InputTypeState::Reset),
    };
    if reset {
        ButtonAction::Reset
    } else {
        ButtonAction::Button
    }
}

/// The form's default button (HTML §4.10.21.2): the first submit button
/// in tree order whose form owner is `form`, disabled or not.
fn default_button(dom: &TuiDom, form: NodeId) -> Option<NodeId> {
    dom.form_listed_elements(form)
        .into_iter()
        .find(|&id| dom.is_submit_button(id))
}

/// Walk up from `id` (inclusive) to the nearest `<button>` or
/// button-like `<input>`. Returns `None` if no button is on the
/// click target's path.
fn closest_form_button(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("button")
            || matches!(
                dom.input_type_state(n),
                Some(InputTypeState::Submit | InputTypeState::Reset | InputTypeState::Button)
            )
        {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// The form's controls — HTML `form.elements` (§4.10.3): every listed
/// element (`<button>`, `<fieldset>`, `<input>`, `<object>`, `<output>`,
/// `<select>`, `<textarea>`) whose form owner is `form`, in tree order
/// of the whole document, so a control outside the form that names it
/// with `form="id"` is included and one inside that names another form
/// is not (`Dom::form_listed_elements`).
///
/// Public for [`crate::accessors::TuiAccessors::form_elements`]
/// (step 31).
pub fn elements(dom: &TuiDom, form: NodeId) -> Vec<NodeId> {
    dom.form_listed_elements(form)
}

/// What one run of the form submission algorithm did.
///
/// Returned by [`crate::accessors::TuiAccessorsMut::form_request_submit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubmitOutcome {
    /// The `submit` event fired and no listener canceled it; the form's
    /// default action ran (a method-`dialog` form closed its dialog).
    Submitted,
    /// The `submit` event fired and a listener called
    /// `preventDefault()`; nothing else happened.
    Canceled,
    /// The node is not a `<form>`: nothing happened (the accessors'
    /// wrong-tag no-op).
    NotAForm,
}

/// The form submission algorithm (HTML §4.10.21.3) from `submitter` —
/// the one path click activation, implicit submission and
/// `requestSubmit()` share:
///
/// 1. fire a bubbling, cancelable `submit` at `form` carrying
///    `EventDetail::Submit(Dom::submit_detail(form, submitter))`;
/// 2. canceled → [`SubmitOutcome::Canceled`];
/// 3. otherwise, when the effective method is `dialog` (the form's
///    `method`, or the submit button's `formmethod`), close the form's
///    nearest ancestor `<dialog>` with the submitter's `value` (`""`
///    without a submitter) as its `returnValue`.
///
/// `submitter` is the clicked submit button, the default button on
/// implicit submission, or `None` for an implicit submission from a
/// form with no submit button and for `requestSubmit()` without one.
/// rdom has no navigation: the `submit` handler decides what submitting
/// means.
pub(crate) fn submit(dom: &mut TuiDom, form: NodeId, submitter: Option<NodeId>) -> SubmitOutcome {
    let detail = dom.submit_detail(form, submitter);
    let method = detail.method;
    let mut ev = TuiEvent::new("submit");
    ev.event.detail = rdom_core::EventDetail::Submit(Box::new(detail));
    let _ = dom.dispatch_tui_event(form, &mut ev);
    if ev.event.default_prevented() {
        return SubmitOutcome::Canceled;
    }
    if method == FormMethod::Dialog
        && let Some(dialog) = crate::runtime::builtins::dialog::enclosing_dialog(dom, form)
    {
        let rv = submitter
            .and_then(|b| dom.node(b).get_attribute("value"))
            .unwrap_or("")
            .to_string();
        crate::runtime::builtins::dialog::close(dom, dialog, &rv);
    }
    SubmitOutcome::Submitted
}

/// `form.requestSubmit(submitter)` (HTML §4.10.3): a `submitter` must
/// be a submit button (`DomError::TypeError` otherwise) whose form
/// owner is `form` (`DomError::NotFound` otherwise); then the shared
/// [`submit`] path runs with it. `form` must be a `<form>`.
pub(crate) fn request_submit(
    dom: &mut TuiDom,
    form: NodeId,
    submitter: Option<NodeId>,
) -> rdom_core::Result<SubmitOutcome> {
    if let Some(s) = submitter {
        if !dom.is_submit_button(s) {
            return Err(rdom_core::DomError::TypeError(
                "requestSubmit: the submitter is not a submit button",
            ));
        }
        if dom.form_owner(s) != Some(form) {
            return Err(rdom_core::DomError::NotFound);
        }
    }
    Ok(submit(dom, form, submitter))
}

/// HTML §4.10.21.5 reset algorithm: every control whose form owner is
/// `form` goes back to its default — text controls and ranges to `defaultValue`,
/// checkboxes and radios to `defaultChecked` (`FORM-DEFAULTS-1`), a
/// `<select>`'s options to `defaultSelected` (`P6G-FORM-RESET-1`). No
/// `input` / `change` events fire, as on the web.
fn reset_controls(dom: &mut TuiDom, form: NodeId) {
    for id in dom.form_listed_elements(form) {
        match dom.node(id).tag_name() {
            Some("input") if crate::runtime::builtins::toggle::is_toggle(dom, id) => {
                crate::runtime::builtins::toggle::reset_to_default(dom, id);
            }
            Some("input") | Some("textarea") => {
                crate::runtime::builtins::input::reset_to_default(dom, id);
            }
            Some("select") => {
                crate::runtime::builtins::select::reset_to_default(dom, id);
            }
            _ => {}
        }
    }
}

fn fire_reset(dom: &mut TuiDom, form: NodeId) -> bool {
    let mut ev = TuiEvent::new("reset");
    let _ = dom.dispatch_tui_event(form, &mut ev);
    ev.event.default_prevented()
}

/// Single-line text-family input (`node::is_text_input`). Excludes
/// `<textarea>` (multi-line, where Enter inserts a newline).
fn is_single_line_text_input(dom: &TuiDom, id: NodeId) -> bool {
    crate::node::is_text_input(dom, id)
}

/// The form's fields that block implicit submission (HTML §4.10.21.2):
/// its owned single-line text inputs.
fn count_text_inputs(dom: &TuiDom, form: NodeId) -> usize {
    dom.form_listed_elements(form)
        .into_iter()
        .filter(|&id| is_single_line_text_input(dom, id))
        .count()
}

/// The entries one owned control contributes to the entry list.
fn collect_entry(
    dom: &TuiDom,
    id: NodeId,
    submitter: Option<NodeId>,
    out: &mut Vec<(String, String)>,
) {
    use InputTypeState as T;
    let node = dom.node(id);
    if !dom.is_actually_disabled(id) {
        let name = node.get_attribute("name").unwrap_or("").to_string();
        if !name.is_empty() {
            // The clippy::collapsible_match suggestion here is unsafe:
            // collapsing `if node.has_attribute("checked")` into a guard
            // would let unchecked checkboxes/radios fall through to the
            // generic `(Some("input"), _)` arm below and submit their
            // text value — matching the browser, which excludes
            // unchecked checkbox values from form submission, requires
            // the explicit no-op on miss.
            #[allow(clippy::collapsible_match, clippy::collapsible_if)]
            match (node.tag_name(), dom.input_type_state(id)) {
                (_, Some(T::Checkbox | T::Radio)) => {
                    if node.has_attribute("checked") {
                        let value = node.get_attribute("value").unwrap_or("on").to_string();
                        out.push((name, value));
                    }
                }
                // HTML §4.10.21.4: a button contributes only as the
                // submitter, and only a submit button can be one. Its
                // value is the `value` attribute or `""` (value mode
                // default) — not the "Submit" default label browsers
                // send (DIVERGENCES).
                (_, Some(T::Submit | T::Reset | T::Button)) | (Some("button"), _) => {
                    if submitter == Some(id) && button_action(dom, id) == ButtonAction::Submit {
                        let value = node.get_attribute("value").unwrap_or("").to_string();
                        out.push((name, value));
                    }
                }
                (_, Some(T::Hidden)) => {
                    if let Some(value) = node.get_attribute("value") {
                        out.push((name, value.to_string()));
                    }
                }
                (Some("input"), _) => {
                    out.push((name, crate::runtime::builtins::input::value(dom, id)));
                }
                (Some("textarea"), _) => {
                    let mut text = String::new();
                    for child in node.child_nodes() {
                        if child.node_type() == rdom_core::NodeType::Text
                            && let Some(s) = child.node_value()
                        {
                            text.push_str(s);
                        }
                    }
                    out.push((name, text));
                }
                (Some("select"), _) => {
                    // HTML §4.10.21.4: one (name, value) entry per
                    // option that is selected and not disabled (own
                    // attribute or disabled `<optgroup>` parent). A
                    // multi-select yields several entries; a single-
                    // select at most one.
                    use crate::runtime::builtins::select;
                    let selected = select::selected_options(dom, id);
                    for opt in selected
                        .into_iter()
                        .filter(|&o| !select::option_disabled(dom, o))
                    {
                        let value = crate::runtime::builtins::select::option_value(dom, opt);
                        out.push((name.clone(), value));
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;
