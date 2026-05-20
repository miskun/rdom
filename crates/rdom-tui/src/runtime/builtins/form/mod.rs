//! `<form>` submission + reset infrastructure.
//!
//! ## Contract (from MDN)
//!
//! - **Submit triggers**:
//!   - Click on `<input type="submit">`, `<button type="submit">`,
//!     or `<button>` without a `type` attribute (HTML default for
//!     buttons in a form is "submit").
//!   - **Implicit submission**: pressing Enter inside a form whose
//!     only submittable single-line text input is the focused one
//!     submits the form.
//! - **Reset triggers**: click on `<input type="reset">` or
//!   `<button type="reset">`.
//! - Submit fires the `submit` event on the `<form>` element —
//!   bubbling, **cancelable**. `preventDefault()` blocks the
//!   default action (a TUI app's submit handler usually
//!   `preventDefault`s and reads form data via [`collect`]).
//! - Reset fires the `reset` event on the form, also cancelable.
//!   Default reset action (restoring `defaultValue` / `defaultChecked`)
//!   is deferred to polish — v1 just fires the event so apps can
//!   react.
//!
//! ## v1 deliberate simplifications
//!
//! - No automatic field reset on `reset` event (would require
//!   `defaultValue` / `defaultChecked` tracking the cascade
//!   doesn't yet provide).
//! - No `formaction` / `formmethod` overrides on individual buttons.
//! - No client-side validation gate (`required`, `pattern`, `min`,
//!   `max` `valueMissing` blocking submit). Apps validate manually
//!   inside their `submit` handler.
//! - No `formdata` event (would require a `FormData` shim).

use rdom_core::{ListenerOptions, NodeId};

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
        if ctx.dom.node(button).has_attribute("disabled") {
            return;
        }
        let Some(form) = enclosing_form(ctx.dom, button) else {
            return;
        };
        match button_action(ctx.dom, button) {
            ButtonAction::Submit => {
                let prevented = fire_submit(ctx.dom, form, Some(button));
                // `<form method="dialog">` integration: when
                // submit isn't prevented, close the enclosing
                // dialog with the submit button's `value` as
                // the returnValue. Matches MDN's HTMLDialogElement
                // form-submission behavior.
                if !prevented
                    && is_dialog_form(ctx.dom, form)
                    && let Some(dialog) =
                        crate::runtime::builtins::dialog::enclosing_dialog(ctx.dom, form)
                {
                    let rv = ctx
                        .dom
                        .node(button)
                        .get_attribute("value")
                        .unwrap_or("")
                        .to_string();
                    crate::runtime::builtins::dialog::close(ctx.dom, dialog, &rv);
                }
            }
            ButtonAction::Reset => {
                let _ = fire_reset(ctx.dom, form);
            }
            ButtonAction::Button => {} // no default action
        }
    })
    .expect("form click listener install");

    // Implicit Enter submission: per HTML, Enter pressed in a
    // single-line text-family input that's the only such input
    // in its form submits the form. v1 uses the broader rule
    // "the focused input is a single-line text-family input AND
    // it's the only such input in its enclosing form" — close
    // enough to the spec for TUI apps without parsing every
    // edge case.
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
        let Some(form) = enclosing_form(ctx.dom, focused) else {
            return;
        };
        if count_text_inputs(ctx.dom, form) != 1 {
            return;
        }
        // Implicit-Enter submit: HTML reports submitter=None.
        let prevented = fire_submit(ctx.dom, form, None);
        if !prevented
            && is_dialog_form(ctx.dom, form)
            && let Some(dialog) = crate::runtime::builtins::dialog::enclosing_dialog(ctx.dom, form)
        {
            // No submit button known on the implicit-Enter
            // path — close with empty returnValue.
            crate::runtime::builtins::dialog::close(ctx.dom, dialog, "");
        }
    })
    .expect("form implicit-enter submit listener install");
}

/// Walk descendants of `form` and collect every form-controlled
/// element's `(name, value)` pair. Apps call this from their
/// `submit` handler to read the form's data.
///
/// Rules (v1):
/// - Only elements with a non-empty `name` attribute participate.
/// - `disabled` elements are skipped.
/// - `<input type="checkbox">` / `<input type="radio">` only
///   contribute when `checked` (matches HTML form-encoding).
/// - Checkbox value defaults to `"on"` when no `value` attribute
///   is set (HTML rule).
/// - Text-family inputs and `<textarea>` contribute their current
///   text content.
/// - `<select>` / `<button>` not supported in v1.
pub fn collect(dom: &TuiDom, form: NodeId) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk_collect(dom, form, &mut out);
    out
}

// ── Internals ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonAction {
    Submit,
    Reset,
    Button,
}

/// HTML default for `<button>` is `type="submit"` when inside a
/// form and the attribute is absent. `<input type=submit/reset/button>`
/// uses the literal type. Anything else is `Button` (no default).
fn button_action(dom: &TuiDom, id: NodeId) -> ButtonAction {
    let node = dom.node(id);
    let ty = node.get_attribute("type");
    match (node.tag_name(), ty) {
        (Some("button"), None)
        | (Some("button"), Some("submit"))
        | (Some("input"), Some("submit")) => ButtonAction::Submit,
        (Some("button"), Some("reset")) | (Some("input"), Some("reset")) => ButtonAction::Reset,
        _ => ButtonAction::Button,
    }
}

/// Walk up from `id` (inclusive) to the nearest `<button>` or
/// button-like `<input>`. Returns `None` if no button is on the
/// click target's path.
fn closest_form_button(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        let node = dom.node(n);
        match node.tag_name() {
            Some("button") => return Some(n),
            Some("input") => {
                if matches!(
                    node.get_attribute("type"),
                    Some("submit") | Some("reset") | Some("button")
                ) {
                    return Some(n);
                }
            }
            _ => {}
        }
        cur = node.parent_node().map(|p| p.id());
    }
    None
}

/// Walk up from `id` (exclusive of `<form>` self-match — i.e.
/// inclusive, but a form IS its own enclosing form) to the
/// nearest `<form>` ancestor.
fn enclosing_form(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("form") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Collect every form-control descendant of `form` in
/// document order. Form controls per HTML's "listed elements"
/// definition: `<button>`, `<fieldset>`, `<input>`,
/// `<object>`, `<output>`, `<select>`, `<textarea>`. Excludes
/// `<form>` itself (consistent with `form.elements`).
///
/// Public for [`crate::accessors::TuiAccessors::form_elements`]
/// (step 31).
pub fn elements(dom: &TuiDom, form: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_elements(dom, form, form, &mut out);
    out
}

fn walk_elements(dom: &TuiDom, form: NodeId, id: NodeId, out: &mut Vec<NodeId>) {
    if id != form && is_form_control(dom, id) {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_elements(dom, form, child.id(), out);
    }
}

fn is_form_control(dom: &TuiDom, id: NodeId) -> bool {
    matches!(
        dom.node(id).tag_name(),
        Some("button" | "fieldset" | "input" | "object" | "output" | "select" | "textarea")
    )
}

/// Fire a `submit` event on `form` with typed
/// `EventDetail::Submit { submitter }`. `submitter` is the
/// element that triggered submission (the clicked `<button>` /
/// `<input type=submit>`), or `None` for implicit-Enter submits
/// where there's no clicked button.
///
/// Returns `true` when the submit was `preventDefault`-ed.
/// Callers chain post-submit defaults (the `<form method="dialog">`
/// auto-close) on the not-prevented case.
///
/// `pub(crate)` so [`crate::accessors::TuiAccessorsMut::form_request_submit`]
/// (step 31) can fire the same event with the same detail
/// shape as the implicit/button-triggered paths.
pub(crate) fn fire_submit(dom: &mut TuiDom, form: NodeId, submitter: Option<NodeId>) -> bool {
    let mut ev = TuiEvent::new("submit");
    ev.event.detail =
        rdom_core::EventDetail::Submit(Box::new(rdom_core::SubmitDetail { submitter }));
    let _ = dom.dispatch_tui_event(form, &mut ev);
    ev.event.default_prevented()
}

fn fire_reset(dom: &mut TuiDom, form: NodeId) -> bool {
    let mut ev = TuiEvent::new("reset");
    let _ = dom.dispatch_tui_event(form, &mut ev);
    ev.event.default_prevented()
}

/// True for `<form method="dialog">` (case-insensitive). Used by
/// the post-submit hook to decide whether to close an enclosing
/// dialog with the submit button's `value`.
fn is_dialog_form(dom: &TuiDom, form: NodeId) -> bool {
    matches!(
        dom.node(form)
            .get_attribute("method")
            .map(|s| s.to_ascii_lowercase()),
        Some(ref m) if m == "dialog"
    )
}

/// Single-line text-family input: an `<input>` whose `type` is
/// one of the text-family values (or absent). Excludes
/// `<textarea>` (multi-line, where Enter inserts a newline).
fn is_single_line_text_input(dom: &TuiDom, id: NodeId) -> bool {
    if dom.node(id).tag_name() != Some("input") {
        return false;
    }
    matches!(
        dom.node(id).get_attribute("type"),
        None | Some("text")
            | Some("password")
            | Some("email")
            | Some("url")
            | Some("tel")
            | Some("search")
            | Some("number")
    )
}

fn count_text_inputs(dom: &TuiDom, form: NodeId) -> usize {
    fn walk(dom: &TuiDom, id: NodeId, count: &mut usize) {
        if is_single_line_text_input(dom, id) {
            *count += 1;
        }
        for child in dom.node(id).child_nodes() {
            walk(dom, child.id(), count);
        }
    }
    let mut n = 0;
    walk(dom, form, &mut n);
    n
}

fn walk_collect(dom: &TuiDom, id: NodeId, out: &mut Vec<(String, String)>) {
    let node = dom.node(id);
    if !node.has_attribute("disabled") {
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
            match (node.tag_name(), node.get_attribute("type")) {
                (Some("input"), Some("checkbox")) | (Some("input"), Some("radio")) => {
                    if node.has_attribute("checked") {
                        let value = node.get_attribute("value").unwrap_or("on").to_string();
                        out.push((name, value));
                    }
                }
                (Some("input"), Some("submit"))
                | (Some("input"), Some("reset"))
                | (Some("input"), Some("button"))
                | (Some("input"), Some("hidden")) => {
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
                    // Multi-select submits each selected option
                    // as a separate (name, value) pair (matches
                    // URLSearchParams array convention). Single-
                    // select submits the one selected option —
                    // or nothing when no option is selected.
                    let selected = crate::runtime::builtins::select::selected_options(dom, id);
                    for opt in selected {
                        let value = crate::runtime::builtins::select::option_value(dom, opt);
                        out.push((name.clone(), value));
                    }
                }
                _ => {}
            }
        }
    }
    for child in node.child_nodes() {
        walk_collect(dom, child.id(), out);
    }
}

#[cfg(test)]
mod tests;
