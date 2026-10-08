//! `<dialog>` show / showModal / close + Esc cancel.
//!
//! ## Contract (from MDN)
//!
//! - `<dialog>` is hidden when the `open` attribute is absent;
//!   visible when present (UA stylesheet flips display).
//! - Methods: `show()` opens non-modally, `showModal()` opens
//!   modally. Both add the `open` attribute. A modal dialog is in the
//!   document's top layer as `TopLayerKind::ModalDialog` (rdom-core's
//!   `Dom::top_layer`) — what `:modal` matches, what renders it above
//!   everything with its `::backdrop`, and what makes the rest of the
//!   document inert to the pointer.
//! - `close(returnValue)` removes `open`, stores the return value,
//!   fires the `close` event on the dialog (non-bubbling).
//! - Esc on a focused element inside a MODAL dialog fires the
//!   `cancel` event (cancelable). If not prevented, the dialog
//!   closes with the existing return value (typically empty).
//!   Non-modal dialogs ignore Esc — matches HTML default.
//!
//! ## v1 deliberate simplifications
//!
//! - The document outside an open modal is inert: Tab cycles inside the
//!   modal, Esc cancels it wherever focus sits, and the pointer outside
//!   it hits its `::backdrop`, whose events go to the dialog
//!   (`hit_test`).
//! - No `closedby` attribute (defaults are baked: modal closes
//!   on Esc, non-modal doesn't).
//!
//! ## Storage of returnValue
//!
//! Stored as the `data-rdom-return-value` attribute. Persists
//! across `show()` calls — `<dialog>` re-opens with whatever
//! value was last set. To clear, call `close("")` or remove the
//! attribute manually.

use rdom_core::{ListenerOptions, NodeId};

use crate::{TuiDom, TuiEvent};

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
    remove_modal(dom, dialog);
    if !was_open {
        fire_toggle(dom, dialog, rdom_core::ToggleState::Closed);
    }
}

/// Open the dialog modally (HTML §4.11.4 `showModal()`): marks the
/// dialog as modal, remembers the previously focused element for
/// `close()`, runs the dialog focusing steps, and — while it stays
/// open — scopes Tab / Shift-Tab to the dialog and lets Esc cancel it
/// wherever focus sits. Pointer events outside are not blocked.
///
/// Polish #5: if the dialog's subtree contains an element with
/// `[autofocus]`, focus transfers to the first such element in
/// document order. Matches MDN's modal-dialog focus behavior.
pub fn show_modal(dom: &mut TuiDom, dialog: NodeId) {
    let was_open = dom.node(dialog).has_attribute("open");
    // Remember where focus was so `close()` can return it (HTML's
    // "previously focused element"), unless it was already inside.
    let previous = dom.focused().filter(|&f| !is_inside(dom, f, dialog));
    if let Some(ext) = dom.node_mut(dialog).ext_mut() {
        ext.dialog_return_focus = previous;
    }
    // HTML §4.11.4 showModal(): the auto and hint popovers that do not
    // hold the dialog are hidden.
    crate::runtime::builtins::popover::hide_unrelated_to(dom, dialog);
    let _ = dom.set_attribute(dialog, "open", "");
    // HTML §4.11.4 showModal(): "add an element to the top layer" — a
    // disconnected dialog opens but cannot be modal.
    let _ = dom.add_to_top_layer(dialog, rdom_core::TopLayerKind::ModalDialog);
    if !was_open {
        fire_toggle(dom, dialog, rdom_core::ToggleState::Closed);
    }
    focusing_steps(dom, dialog);
}

/// The dialog focusing steps (HTML §4.11.4): the first `[autofocus]`
/// descendant, else the first focusable descendant, else the dialog
/// itself when it is focusable. `showModal()` runs them, and so does a
/// `<dialog popover>` shown as a popover (the popover focusing steps).
/// The dialog was closed (`display: none`) when last cascaded; it is
/// rendered now (`tabindex::is_focusable_in_opened`).
pub(crate) fn focusing_steps(dom: &mut TuiDom, dialog: NodeId) {
    crate::runtime::autofocus::focus_within_opened(dom, dialog);
    if !dom.focused().is_some_and(|f| is_inside(dom, f, dialog)) {
        let target = first_focusable_in(dom, dialog).or_else(|| {
            crate::runtime::focus::tabindex::is_focusable_in_opened(dom, dialog, dialog)
                .then_some(dialog)
        });
        if let Some(t) = target {
            crate::runtime::focus::focus_node(dom, Some(t));
        }
    }
}

/// `id == ancestor` or `id` is a descendant of `ancestor`.
fn is_inside(dom: &TuiDom, id: NodeId, ancestor: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
}

/// First focusable element in document order strictly inside `root`,
/// a dialog just opened (`tabindex::is_focusable_in_opened`).
fn first_focusable_in(dom: &TuiDom, root: NodeId) -> Option<NodeId> {
    fn walk(dom: &TuiDom, id: NodeId, root: NodeId) -> Option<NodeId> {
        for child in dom.node(id).child_nodes() {
            let c = child.id();
            if crate::runtime::focus::tabindex::is_focusable_in_opened(dom, c, root) {
                return Some(c);
            }
            if let Some(found) = walk(dom, c, root) {
                return Some(found);
            }
        }
        None
    }
    walk(dom, root, root)
}

/// The open modal dialog that currently owns interaction, if any:
/// with a modal open, the rest of the document is inert, so Tab
/// cycles inside it and Esc cancels it wherever focus sits. The
/// topmost modal dialog in the top layer — the one opened last.
pub fn top_modal(dom: &TuiDom) -> Option<NodeId> {
    dom.top_layer()
        .iter()
        .rev()
        .copied()
        .find(|&d| is_modal(dom, d))
}

/// Take a modal dialog out of the top layer (no-op for any other).
fn remove_modal(dom: &mut TuiDom, dialog: NodeId) {
    if dom.top_layer_kind(dialog) == Some(rdom_core::TopLayerKind::ModalDialog) {
        dom.remove_from_top_layer(dialog);
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
    remove_modal(dom, dialog);
    let _ = dom.set_attribute(dialog, RETURN_VALUE_ATTR, return_value);

    // Fire `toggle` first (the state-change signal), then `close`
    // (the dialog-specific lifecycle event). MDN documents both
    // for HTMLDialogElement; order matches Firefox / Chrome.
    fire_toggle(dom, dialog, rdom_core::ToggleState::Open);

    let mut ev = TuiEvent::new("close");
    ev.event = ev.event.clone().with_bubbles(false);
    crate::tui_event::dispatch_to_live(dom, dialog, &mut ev);
    // Return focus to the previously focused element (HTML §4.11.4)
    // when it is still in the tree; the current focus leaves with the
    // dialog either way if it was inside it.
    let previous = dom
        .node_mut(dialog)
        .ext_mut()
        .and_then(|e| e.dialog_return_focus.take());
    let still_focusable =
        |p: &NodeId| dom.contains(*p) && crate::runtime::focus::tabindex::is_focusable(dom, *p);
    if let Some(prev) = previous.filter(still_focusable) {
        // "Run the focusing steps for previouslyFocusedElement; the
        // viewport should not be scrolled by doing this step."
        crate::runtime::focus::focus_node_with_options(
            dom,
            Some(prev),
            crate::runtime::focus::FocusOptions::new().prevent_scroll(true),
        );
    } else if dom.focused().is_some_and(|f| is_inside(dom, f, dialog)) {
        crate::runtime::focus::focus_node(dom, None);
    }
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
    ev.event.detail = rdom_core::EventDetail::Toggle(Box::new(rdom_core::ToggleDetail::new(
        old_state, new_state,
    )));
    crate::tui_event::dispatch_to_live(dom, dialog, &mut ev);
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

/// True when the dialog is open AND was opened via `show_modal` — in
/// the top layer as a modal dialog (`:modal`). Used by the Esc handler
/// and by the form-method-dialog flow.
pub fn is_modal(dom: &TuiDom, dialog: NodeId) -> bool {
    dom.node(dialog).has_attribute("open")
        && dom.top_layer_kind(dialog) == Some(rdom_core::TopLayerKind::ModalDialog)
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
        // The modal dialog that owns interaction: the one enclosing
        // the focus, else the open modal (the document outside it is
        // inert, so Esc still means "cancel" wherever focus sits).
        let enclosing = ctx
            .dom
            .focused()
            .and_then(|f| enclosing_dialog(ctx.dom, f))
            .filter(|&d| is_modal(ctx.dom, d));
        let Some(dialog) = enclosing.or_else(|| top_modal(ctx.dom)) else {
            return;
        };
        // A close request goes to the most recent close watcher: an auto
        // or hint popover opened above the modal dialog closes first
        // (`popover::light_dismiss`).
        if let Some(top) = crate::runtime::builtins::popover::topmost_close_watcher(ctx.dom)
            && ctx.dom.top_layer_kind(top) == Some(rdom_core::TopLayerKind::Popover)
        {
            return;
        }
        // The Esc is this close request's: no other handler acts on it.
        ctx.event.prevent_default();
        // Fire `cancel` on the dialog — bubbling, cancelable.
        // If a handler `prevent_default`s, we leave the dialog
        // open. Otherwise fall through to close with the
        // current return value (typically empty).
        let mut cancel = TuiEvent::new("cancel");
        if !crate::tui_event::dispatch_to_live(ctx.dom, dialog, &mut cancel)
            || cancel.event.default_prevented()
        {
            return;
        }
        let rv = return_value(ctx.dom, dialog);
        close(ctx.dom, dialog, &rv);
    })
    .expect("dialog Esc-cancel listener install");
}

#[cfg(test)]
mod tests;
