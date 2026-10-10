//! `<dialog>` show / showModal / close + Esc cancel.
//!
//! ## Contract (from MDN)
//!
//! - `<dialog>` is hidden when the `open` attribute is absent;
//!   visible when present (UA stylesheet flips display).
//! - Methods: `show()` opens non-modally, `showModal()` opens
//!   modally. Both add the `open` attribute, and both return
//!   `DomError::InvalidState` where HTML throws `InvalidStateError`. A modal dialog is in the
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
//! - The document outside an open modal is inert (HTML §6.3.2,
//!   `Dom::is_inert`): nothing outside can take the focus (`focus_node`
//!   refuses it, and the focus fixup moves it off), Tab cycles inside the
//!   modal, Esc cancels it wherever focus sits, and the pointer outside
//!   it hits its `::backdrop`, whose events go to the dialog
//!   (`hit_test`) — or nothing, when the dialog is not rendered. A
//!   popover above it from outside it is inert too.
//! - No `closedby` attribute (defaults are baked: modal closes
//!   on Esc, non-modal doesn't).
//!
//! ## Storage of returnValue
//!
//! Stored as the `data-rdom-return-value` attribute. Persists
//! across `show()` calls — `<dialog>` re-opens with whatever
//! value was last set. To clear, call `close("")` or remove the
//! attribute manually.

use rdom_core::{ListenerOptions, NodeId, ToggleState};

use crate::{TuiDom, TuiEvent};

/// Attribute that stores the dialog's returnValue between close
/// calls. Mirrors the HTML `dialog.returnValue` IDL property.
const RETURN_VALUE_ATTR: &str = "data-rdom-return-value";

/// `show()` (HTML §4.11.4): open the dialog non-modally. On a dialog
/// already open non-modally it does nothing; on a modal one it is
/// `DomError::InvalidState` (step 2). Otherwise it fires a cancelable
/// `beforetoggle` (`closed` → `open`) — canceled, it returns — then adds
/// `open` (firing `toggle`), remembers the previously focused element for
/// `close()`, hides the auto and hint popovers that do not hold the
/// dialog, and runs the dialog focusing steps.
pub fn show(dom: &mut TuiDom, dialog: NodeId) -> rdom_core::Result<()> {
    // Steps 1–2.
    if dom.node(dialog).has_attribute("open") {
        if is_modal(dom, dialog) {
            return Err(rdom_core::DomError::InvalidState(
                "show: the dialog is open as a modal dialog",
            ));
        }
        return Ok(());
    }
    // Steps 3–4: a canceled `beforetoggle`, or a listener that opened the
    // dialog, ends the show.
    if !fire_toggle_event(dom, dialog, "beforetoggle", ToggleState::Closed)
        || dom.node(dialog).has_attribute("open")
    {
        return Ok(());
    }
    // Steps 5–6 (the `toggle` fires synchronously: DIVERGENCES §2).
    dom.set_attribute(dialog, "open", "")?;
    fire_toggle_event(dom, dialog, "toggle", ToggleState::Closed);
    // Step 10.
    remember_previous_focus(dom, dialog);
    // Steps 11–15.
    crate::runtime::builtins::popover::hide_unrelated_to(dom, dialog);
    // Step 16.
    focusing_steps(dom, dialog);
    Ok(())
}

/// `showModal()` (HTML §4.11.4 "show a modal dialog"): marks the dialog
/// modal in the top layer — the rest of the document inert (`Dom::is_inert`:
/// Tab, focus, keys and the pointer stay inside it, Esc cancels it
/// wherever focus sits) — remembers the previously focused element for
/// `close()`, hides the auto and hint popovers that do not hold it, and
/// runs the dialog focusing steps.
///
/// HTML's guards: a dialog already modal is left as it is (step 1 — it
/// does not move to the top); an open non-modal dialog (step 2), a
/// disconnected one (step 4) and one showing as a popover (step 5) are
/// `DomError::InvalidState`. Then a cancelable `beforetoggle` (`closed` →
/// `open`) fires (step 6); canceled — or a listener that opened the
/// dialog, disconnected it or showed it as a popover (steps 7–9) — the
/// show ends without an error.
///
/// If the dialog's subtree contains an element with `[autofocus]`, focus
/// moves to the first such element in document order.
pub fn show_modal(dom: &mut TuiDom, dialog: NodeId) -> rdom_core::Result<()> {
    use rdom_core::DomError::InvalidState;
    let open = dom.node(dialog).has_attribute("open");
    // Steps 1–5.
    if open && is_modal(dom, dialog) {
        return Ok(());
    }
    if open {
        return Err(InvalidState("showModal: the dialog is already open"));
    }
    if !dom.contains(dialog) || !dom.node(dialog).is_connected() {
        return Err(InvalidState("showModal: the dialog is not connected"));
    }
    if crate::runtime::builtins::popover::is_showing(dom, dialog) {
        return Err(InvalidState(
            "showModal: the dialog is showing as a popover",
        ));
    }
    // Steps 6–9.
    if !fire_toggle_event(dom, dialog, "beforetoggle", ToggleState::Closed)
        || dom.node(dialog).has_attribute("open")
        || !dom.node(dialog).is_connected()
        || crate::runtime::builtins::popover::is_showing(dom, dialog)
    {
        return Ok(());
    }
    // Steps 11–15: `open`, "is modal", blocked by it, the top layer.
    dom.set_attribute(dialog, "open", "")?;
    dom.add_to_top_layer(dialog, rdom_core::TopLayerKind::ModalDialog)?;
    // Step 10 (synchronously: DIVERGENCES §2).
    fire_toggle_event(dom, dialog, "toggle", ToggleState::Closed);
    // Step 16.
    remember_previous_focus(dom, dialog);
    // Steps 17–19.
    crate::runtime::builtins::popover::hide_unrelated_to(dom, dialog);
    // Step 20.
    focusing_steps(dom, dialog);
    Ok(())
}

/// HTML's "previously focused element" of a dialog being shown — the
/// focus, unless it is already inside — for `close()` to return to.
fn remember_previous_focus(dom: &mut TuiDom, dialog: NodeId) {
    let previous = dom.focused().filter(|&f| !is_inside(dom, f, dialog));
    if let Some(ext) = dom.node_mut(dialog).ext_mut() {
        ext.dialog_return_focus = previous;
    }
}

/// The dialog focusing steps (HTML §4.11.4): the first `[autofocus]`
/// descendant, else the focus delegate (the first focusable
/// descendant), else the dialog itself — step 4, "set control to
/// subject": a dialog with nothing focusable in it takes the focus, so
/// none is left on the page it makes inert. `showModal()` runs them, and
/// so does a `<dialog popover>` shown as a popover (the popover focusing
/// steps). The dialog was closed (`display: none`) when last cascaded; it
/// is rendered now (`tabindex::is_focusable_in_opened`). A dialog that
/// is still not rendered (a `display: none` ancestor) takes nothing.
pub(crate) fn focusing_steps(dom: &mut TuiDom, dialog: NodeId) {
    crate::runtime::autofocus::focus_within_opened(dom, dialog);
    if !dom.focused().is_some_and(|f| is_inside(dom, f, dialog)) {
        let target = first_focusable_in(dom, dialog).or_else(|| {
            crate::runtime::focus::tabindex::renders_in_opened(dom, dialog, dialog)
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
    // Iterative: a DOM may be any depth (C16G-DEPTH-CAPS).
    dom.descendants(root)
        .find(|&c| crate::runtime::focus::tabindex::is_focusable_in_opened(dom, c, root))
}

/// The open modal dialog that currently owns interaction, if any:
/// with a modal open, the rest of the document is inert, so Tab
/// cycles inside it and Esc cancels it wherever focus sits. The
/// topmost modal dialog in the top layer — the one opened last; the
/// dialog the document is blocked by (`Dom::blocking_modal`).
pub fn top_modal(dom: &TuiDom) -> Option<NodeId> {
    dom.blocking_modal()
}

/// Take a modal dialog out of the top layer (no-op for any other): the
/// close steps' "remove an element from the top layer".
fn remove_modal(dom: &mut TuiDom, dialog: NodeId) {
    if dom.top_layer_kind(dialog) == Some(rdom_core::TopLayerKind::ModalDialog) {
        // "Request an element to be removed from the top layer" (CSS
        // Position 4 §3.3): it waits there while `overlay` transitions.
        crate::runtime::top_layer::request_removal(dom, dialog);
    }
}

/// Close the dialog (HTML §4.11.4 "close the dialog"). Fires a
/// non-cancelable `beforetoggle` (`open` → `closed`), then — unless a
/// listener closed the dialog first — removes the `open` attribute,
/// stores `return_value` for later reads, fires `toggle` and the `close`
/// event (non-bubbling, non-cancelable per HTML).
///
/// `return_value` may be empty — both browser-`close()` (no arg)
/// and the empty-string overload land here.
pub fn close(dom: &mut TuiDom, dialog: NodeId, return_value: &str) {
    if !dom.node(dialog).has_attribute("open") {
        return; // Already closed — no event, no state change.
    }
    // Steps 2–3.
    fire_toggle_event(dom, dialog, "beforetoggle", ToggleState::Open);
    if !dom.node(dialog).has_attribute("open") {
        return;
    }
    let _ = dom.remove_attribute(dialog, "open");
    remove_modal(dom, dialog);
    let _ = dom.set_attribute(dialog, RETURN_VALUE_ATTR, return_value);

    // Fire `toggle` first (the state-change signal), then `close`
    // (the dialog-specific lifecycle event). MDN documents both
    // for HTMLDialogElement; order matches Firefox / Chrome.
    fire_toggle_event(dom, dialog, "toggle", ToggleState::Open);

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

/// Fire a non-bubbling `beforetoggle` or `toggle` (a `ToggleEvent`)
/// with typed `EventDetail::Toggle` carrying the state transition:
/// `old_state` is the state before it, the new state its inverse. Only a
/// `beforetoggle` that opens is cancelable (HTML §4.11.4: `show()` step 3,
/// `showModal()` step 6; the close's is not). Returns whether it went
/// uncanceled.
fn fire_toggle_event(dom: &mut TuiDom, dialog: NodeId, ty: &str, old_state: ToggleState) -> bool {
    let new_state = match old_state {
        ToggleState::Open => ToggleState::Closed,
        ToggleState::Closed => ToggleState::Open,
    };
    let mut ev = TuiEvent::new(ty);
    ev.event.bubbles = false;
    ev.event.cancelable = ty == "beforetoggle" && new_state == ToggleState::Open;
    ev.event.detail = rdom_core::EventDetail::Toggle(Box::new(rdom_core::ToggleDetail::new(
        old_state, new_state,
    )));
    crate::tui_event::dispatch_to_live(dom, dialog, &mut ev) && !ev.event.default_prevented()
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
