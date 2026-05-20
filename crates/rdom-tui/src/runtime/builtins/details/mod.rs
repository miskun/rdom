//! `<details>` + `<summary>` disclosure widget — click on the
//! summary toggles the parent `<details>`'s `open` attribute, and
//! fires a `toggle` event.
//!
//! ## Contract (from MDN)
//!
//! - `<details>` has a boolean `open` attribute. Presence = open,
//!   absence = closed. The attribute is a boolean, so **any**
//!   value (including `"false"`) counts as open; removing it is
//!   the only way to close.
//! - `<summary>` is the activation widget. Clicking on a summary
//!   that's a direct child of a details toggles the parent.
//! - Keyboard: Space or Enter on a focused `<summary>` also
//!   toggles. Handled via the click synthesis in C.3's
//!   `<button>` path + a direct summary keydown here.
//! - After the state changes, a `toggle` event fires on the
//!   `<details>` element. Per MDN: non-bubbling, non-cancelable.
//! - `name` attribute for exclusive accordion groups: deferred
//!   (one opening closes others with the same name). Polish
//!   item; not shipping in v1.
//!
//! ## Listener pattern
//!
//! One root click listener + one root keydown listener:
//!
//! - Click: target's `closest("summary")` must find a summary
//!   whose parent is a `<details>`.
//! - Keydown: focused is a summary, key is Enter or Space
//!   (unmodified).

use rdom_core::{ListenerOptions, NodeId};

use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

/// Install the details/summary default actions. Called once
/// from `App::build`.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();

    // Click → toggle.
    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(summary) = closest_summary(ctx.dom, target) else {
            return;
        };
        let Some(details) = parent_details(ctx.dom, summary) else {
            return;
        };
        toggle(ctx.dom, details);
    })
    .expect("details click listener install");

    // Keydown on focused summary → toggle. Enter and Space both.
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if ctx.dom.node(focused).tag_name() != Some("summary") {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        // Modifier combos aren't activation.
        if key.modifiers.ctrl || key.modifiers.alt || key.modifiers.meta {
            return;
        }
        if !matches!(key.key.as_str(), "Enter" | " ") {
            return;
        }
        let Some(details) = parent_details(ctx.dom, focused) else {
            return;
        };
        toggle(ctx.dom, details);
    })
    .expect("details keydown listener install");
}

/// Toggle the `open` attribute on a `<details>` and fire the
/// `toggle` event. Called by both the click and keydown paths.
fn toggle(dom: &mut TuiDom, details: NodeId) {
    let was_open = dom.node(details).has_attribute("open");
    if was_open {
        let _ = dom.remove_attribute(details, "open");
    } else {
        let _ = dom.set_attribute(details, "open", "");
    }
    // MDN: `toggle` doesn't bubble and isn't cancelable.
    let (old_state, new_state) = if was_open {
        (rdom_core::ToggleState::Open, rdom_core::ToggleState::Closed)
    } else {
        (rdom_core::ToggleState::Closed, rdom_core::ToggleState::Open)
    };
    let mut ev = TuiEvent::new("toggle");
    ev.event = ev.event.clone().with_bubbles(false);
    ev.event.detail = rdom_core::EventDetail::Toggle(Box::new(rdom_core::ToggleDetail {
        old_state,
        new_state,
    }));
    let _ = dom.dispatch_tui_event(details, &mut ev);
}

/// Walk up from `id` (inclusive) to the nearest `<summary>`.
fn closest_summary(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("summary") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// The summary's parent — must be a `<details>` for toggle to
/// apply. Returns `None` if the summary is orphaned or has a
/// non-details parent (matches MDN: "A summary whose parent
/// isn't a details is just a generic container").
fn parent_details(dom: &TuiDom, summary: NodeId) -> Option<NodeId> {
    let parent = dom.node(summary).parent_node()?;
    if parent.tag_name() == Some("details") {
        Some(parent.id())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
