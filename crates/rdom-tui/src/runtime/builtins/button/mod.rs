//! `<button>` keyboard activation — Enter and Space on a
//! focused button synthesize a `click` event.
//!
//! ## Contract (from MDN)
//!
//! - Enter and Space both activate a focused `<button>`.
//! - The synthesized click is a normal DOM click event — bubbles,
//!   cancelable, fires on the button. Handlers that listen for
//!   `click` receive it; an `event.preventDefault()` on the click
//!   suppresses any downstream effects (e.g. the anchor-dispatch
//!   listener from C.2 respects it the same way).
//! - `disabled` buttons don't fire (focus-nav already skips
//!   disabled, so a disabled button can't be the focus target
//!   of a keydown — covered by Phase C.1).
//! - The synthetic click carries `is_synthetic = true` so
//!   runtime default-actions (C.2 `<a href>`, future `<form>`
//!   submission) can distinguish keyboard-origin clicks from
//!   mouse clicks if they ever need to.
//!
//! ## Why a root listener
//!
//! Same reason as `a_href`: browsers don't install per-element
//! listeners, they have built-in default actions. One
//! registration at the document root handles every button the
//! app will ever contain.
//!
//! ## `type` attribute
//!
//! Ship type-agnostic activation for C.3. `type="submit"` /
//! `type="reset"` → form submission / reset behavior arrives in
//! C.4c when `<form>` lands. For now all three types synthesize
//! a plain click.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use rdom_core::ListenerOptions;

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Install the button keyboard-activation default action. Called
/// once from `App::build`.
///
/// C.4c expansion: the same Enter/Space activation also fires on
/// `<input type="submit">`, `<input type="reset">`, and
/// `<input type="button">` — these behave as buttons for keyboard
/// activation, then the form builtin (C.4c) handles the resulting
/// click as a submit / reset trigger.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        // Focused element must be a `<button>` or one of the
        // button-like `<input>` variants.
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if !is_button_like(ctx.dom, focused) {
            return;
        }
        // Read the typed keyboard payload off the dispatching event.
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        // Modifier combos (Ctrl-Enter, Alt-Space, …) are NOT
        // activation — they belong to clipboard / selection /
        // editing paths. Bare Enter or Space only.
        if key.modifiers.ctrl || key.modifiers.alt || key.modifiers.meta {
            return;
        }
        let triggers = matches!(key.key.as_str(), "Enter" | " ");
        if !triggers {
            return;
        }
        // Synthesize the click.
        let fake_mouse = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let mut click = TuiEvent::click(fake_mouse);
        click.event = click.event.clone().with_synthetic(true);
        let _ = ctx.dom.dispatch_tui_event(focused, &mut click);
    })
    .expect("root button keydown listener install");
}

/// True for `<button>` and for the button-like `<input>` types
/// (`submit`, `reset`, `button`). These three input types render
/// as buttons in HTML and share its keyboard activation.
pub(super) fn is_button_like(dom: &TuiDom, id: rdom_core::NodeId) -> bool {
    let node = dom.node(id);
    match node.tag_name() {
        Some("button") => true,
        Some("input") => matches!(
            node.get_attribute("type"),
            Some("submit") | Some("reset") | Some("button")
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
