//! `<dialog>` show / showModal / close + Esc cancel.
//!
//! ## Contract (from MDN)
//!
//! - `<dialog>` is hidden when the `open` attribute is absent;
//!   visible when present (UA stylesheet flips display).
//! - Methods: `show()` opens non-modally, `showModal()` opens
//!   modally. Both add the `open` attribute. The modal-vs-non-
//!   modal distinction is tracked via the `data-rdom-modal`
//!   marker attribute (rdom-internal, not standard HTML).
//! - `close(returnValue)` removes `open`, stores the return value,
//!   fires the `close` event on the dialog (non-bubbling).
//! - Esc on a focused element inside a MODAL dialog fires the
//!   `cancel` event (cancelable). If not prevented, the dialog
//!   closes with the existing return value (typically empty).
//!   Non-modal dialogs ignore Esc — matches HTML default.
//!
//! ## v1 deliberate simplifications
//!
//! - No focus trap or `inert` outside the modal — author-level
//!   composition.
//! - No `::backdrop` paint.
//! - No `closedby` attribute (defaults are baked: modal closes
//!   on Esc, non-modal doesn't).
//! - No top-layer / z-index handling — apps lay out the dialog
//!   themselves (e.g. via absolute positioning).
//! - No autofocus on the first focusable on open.
//!
//! ## Storage of returnValue
//!
//! Stored as the `data-rdom-return-value` attribute. Persists
//! across `show()` calls — `<dialog>` re-opens with whatever
//! value was last set. To clear, call `close("")` or remove the
//! attribute manually.

use rdom_core::{ListenerOptions, NodeId};

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Marker attribute set by `show_modal` and cleared by `show` /
/// `close`. Distinguishes modal from non-modal dialogs at runtime.
const MODAL_ATTR: &str = "data-rdom-modal";

/// Attribute that stores the dialog's returnValue between close
/// calls. Mirrors the HTML `dialog.returnValue` IDL property.
const RETURN_VALUE_ATTR: &str = "data-rdom-return-value";

/// Open the dialog non-modally. Idempotent — calling on an
/// already-open dialog clears the modal marker (matches the HTML
/// behavior of `show()` after `showModal()`) and does NOT
/// re-fire the `toggle` event.
pub fn show(dom: &mut TuiDom, dialog: NodeId) {
    let was_open = dom.node(dialog).has_attribute("open");
    let _ = dom.set_attribute(dialog, "open", "");
    let _ = dom.remove_attribute(dialog, MODAL_ATTR);
    if !was_open {
        fire_toggle(dom, dialog, rdom_core::ToggleState::Closed);
    }
}

/// Open the dialog modally. Marks the dialog as modal so the Esc
/// handler treats it correctly. v1 does NOT focus-trap or inert
/// the rest of the document — apps that need that compose it
/// themselves.
///
/// Polish #5: if the dialog's subtree contains an element with
/// `[autofocus]`, focus transfers to the first such element in
/// document order. Matches MDN's modal-dialog focus behavior.
pub fn show_modal(dom: &mut TuiDom, dialog: NodeId) {
    let was_open = dom.node(dialog).has_attribute("open");
    let _ = dom.set_attribute(dialog, "open", "");
    let _ = dom.set_attribute(dialog, MODAL_ATTR, "");
    crate::runtime::autofocus::focus_within(dom, dialog);
    if !was_open {
        fire_toggle(dom, dialog, rdom_core::ToggleState::Closed);
    }
}

/// Close the dialog. Removes the `open` attribute, stores
/// `return_value` for later reads, fires the `close` event
/// (non-bubbling, non-cancelable per HTML).
///
/// `return_value` may be empty — both browser-`close()` (no arg)
/// and the empty-string overload land here.
pub fn close(dom: &mut TuiDom, dialog: NodeId, return_value: &str) {
    if !dom.node(dialog).has_attribute("open") {
        return; // Already closed — no event, no state change.
    }
    let _ = dom.remove_attribute(dialog, "open");
    let _ = dom.remove_attribute(dialog, MODAL_ATTR);
    let _ = dom.set_attribute(dialog, RETURN_VALUE_ATTR, return_value);

    // Fire `toggle` first (the state-change signal), then `close`
    // (the dialog-specific lifecycle event). MDN documents both
    // for HTMLDialogElement; order matches Firefox / Chrome.
    fire_toggle(dom, dialog, rdom_core::ToggleState::Open);

    let mut ev = TuiEvent::new("close");
    ev.event = ev.event.clone().with_bubbles(false);
    let _ = dom.dispatch_tui_event(dialog, &mut ev);
}

/// Fire a non-bubbling `toggle` event with typed
/// `EventDetail::Toggle` carrying the state transition.
/// `old_state` is the state before the transition; the new state
/// is its inverse.
fn fire_toggle(dom: &mut TuiDom, dialog: NodeId, old_state: rdom_core::ToggleState) {
    let new_state = match old_state {
        rdom_core::ToggleState::Open => rdom_core::ToggleState::Closed,
        rdom_core::ToggleState::Closed => rdom_core::ToggleState::Open,
    };
    let mut ev = TuiEvent::new("toggle");
    ev.event = ev.event.clone().with_bubbles(false);
    ev.event.detail = rdom_core::EventDetail::Toggle(Box::new(rdom_core::ToggleDetail {
        old_state,
        new_state,
    }));
    let _ = dom.dispatch_tui_event(dialog, &mut ev);
}

/// Read the dialog's `returnValue` (set by the last `close()`).
/// Empty string when the dialog has never been closed.
pub fn return_value(dom: &TuiDom, dialog: NodeId) -> String {
    dom.node(dialog)
        .get_attribute(RETURN_VALUE_ATTR)
        .unwrap_or("")
        .to_string()
}

/// Direct assignment to the dialog's `returnValue` — the
/// `dialog.returnValue = "x"` IDL setter. Does NOT close the
/// dialog or fire any events; just updates the stored value
/// so a subsequent `close()` (or `return_value()` read) sees
/// the new string.
pub fn set_return_value(dom: &mut TuiDom, dialog: NodeId, value: &str) {
    let _ = dom.set_attribute(dialog, RETURN_VALUE_ATTR, value);
}

/// True when the dialog is open AND was opened via `show_modal`.
/// Used by the Esc handler and by the form-method-dialog flow.
pub fn is_modal(dom: &TuiDom, dialog: NodeId) -> bool {
    dom.node(dialog).has_attribute("open") && dom.node(dialog).has_attribute(MODAL_ATTR)
}

/// Walk up from `id` (inclusive) to the nearest `<dialog>`
/// ancestor. Used by the form-method-dialog flow — submit handler
/// closes the dialog containing the form. Returns `None` when the
/// element isn't inside a dialog.
pub fn enclosing_dialog(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("dialog") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Install the dialog default actions. One root-level keydown
/// listener: Esc on focused inside an open MODAL dialog fires
/// `cancel`; if not prevented, closes with the current returnValue.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();
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
        if key.key != "Escape" || !no_mods {
            return;
        }
        let Some(dialog) = enclosing_dialog(ctx.dom, focused) else {
            return;
        };
        if !is_modal(ctx.dom, dialog) {
            return;
        }
        // Fire `cancel` on the dialog — bubbling, cancelable.
        // If a handler `prevent_default`s, we leave the dialog
        // open. Otherwise fall through to close with the
        // current return value (typically empty).
        let mut cancel = TuiEvent::new("cancel");
        let _ = ctx.dom.dispatch_tui_event(dialog, &mut cancel);
        if cancel.event.default_prevented() {
            return;
        }
        let rv = return_value(ctx.dom, dialog);
        close(ctx.dom, dialog, &rv);
    })
    .expect("dialog Esc-cancel listener install");
}

#[cfg(test)]
mod tests;
