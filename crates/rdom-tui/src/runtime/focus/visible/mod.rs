//! `:focus-visible` heuristics — when the UA makes focus evident
//! (Selectors 4 §13.2). rdom-core keeps the bit
//! ([`Dom::focus_visible`](rdom_core::Dom::focus_visible)) and matches
//! the selector; this module decides it from input modality, following
//! the browsers (and the WICG `focus-visible` polyfill they converged
//! on):
//!
//! - a key press without Ctrl / Alt / Super (typing, Tab, arrows, Enter
//!   — not a shortcut chord) makes the focus evident, both the current
//!   focus and whatever the key moves it to ([`note_key`]);
//! - focus moved by a pointer event is evident only on an element that
//!   takes keyboard input — a text control or editing host
//!   ([`TuiNodeExt::is_editable`]) — so a clicked button, toggle or
//!   select shows no indicator while a clicked text field does
//!   ([`note_pointer_focus`]);
//! - focus moved by script outside an input event leaves the bit alone,
//!   so it keeps the previously focused element's visibility, as the
//!   spec asks; script focus from a click handler counts as pointer
//!   focus.
//!
//! A pointer press that does not move focus leaves the bit alone, as in
//! browsers: the element keeps the visibility it was focused with.

use crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::node::TuiNodeExt;

/// A key press (or repeat) without Ctrl / Alt / Super is keyboard use:
/// the focus becomes evident.
pub(crate) fn note_key(dom: &mut TuiDom, key: KeyEvent) {
    if key.kind == KeyEventKind::Release {
        return;
    }
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return;
    }
    dom.set_focus_visible(true);
}

/// After a pointer event: if it moved focus off `before` onto an
/// element, that focus is evident iff the element takes keyboard input.
pub(crate) fn note_pointer_focus(dom: &mut TuiDom, before: Option<NodeId>) {
    let after = dom.focused();
    if after == before {
        return;
    }
    if let Some(now) = after {
        let evident = dom.node(now).is_editable();
        dom.set_focus_visible(evident);
    }
}

#[cfg(test)]
mod tests;
