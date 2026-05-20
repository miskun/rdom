//! Focus management — tabindex ordering, programmatic focus,
//! focus/blur/focusin/focusout event dispatch.
//!
//! ## Sub-modules
//!
//! - [`tabindex`] — `tabindex` attribute parsing, focusable-element
//!   collection, ordering (positive indices ascending, then DOM
//!   order for `tabindex=0`), `Tab` / `Shift+Tab` navigation.
//!
//! ## Events
//!
//! [`focus_node`] is the canonical "change the focus and fire the
//! right events" entry point — the runtime (both Tab navigation
//! and focus-on-click paths) calls it, and apps that want the
//! full browser-ceremony on a programmatic focus change call it
//! instead of raw `dom.set_focused(...)`.
//!
//! Events fire in spec order:
//!
//! 1. `blur` on old (non-bubbling)
//! 2. `focusout` on old (bubbling)
//! 3. `dom.set_focused(new)` commits — `:focus` cascade picks up
//! 4. `focus` on new (non-bubbling)
//! 5. `focusin` on new (bubbling)

pub mod tabindex;

#[cfg(test)]
mod tests;

use rdom_core::{NodeId, NodeType, Position, Selection};

use crate::node::{TuiNodeExt, is_descendant_or_self};
use crate::{TuiDispatchExt, TuiDom, TuiEvent};

/// Change focus. Fires `blur` + `focusout` on the old focus,
/// commits the new focus (which updates the `:focus` pseudo via
/// `Mutation::InteractionChanged`), then fires `focus` + `focusin`
/// on the new.
///
/// Idempotent: if `new_focus == dom.focused()`, no events fire and
/// no mutation happens.
///
/// Pass `None` to clear focus (fires only blur + focusout).
pub fn focus_node(dom: &mut TuiDom, new_focus: Option<NodeId>) {
    let old = dom.focused();
    if old == new_focus {
        return;
    }

    // blur + focusout on the old target.
    if let Some(old_id) = old {
        let mut blur = TuiEvent::blur();
        let _ = dom.dispatch_tui_event(old_id, &mut blur);
        let mut out = TuiEvent::focusout();
        let _ = dom.dispatch_tui_event(old_id, &mut out);
    }

    // Commit the new state — drives :focus cascade via the
    // InteractionChanged mutation the DirtyTracker observes.
    dom.set_focused(new_focus);

    // Seed a collapsed caret for editable focus targets. The
    // mouse-click drag-select path does this automatically (it
    // computes a position from the click coordinates and calls
    // `dom.set_selection(...)`); Tab and programmatic focus had no
    // such path, so a freshly-focused `<input>` had a `None`
    // selection. `editing::perform::insert_at_selection` returns
    // `NoEditableTarget` when the selection is `None`, which means
    // the first keystroke after Tab-focusing an input was silently
    // dropped. Browsers seed a caret on focus (or select-all on
    // most platforms); the simpler caret-at-0 matches what we do
    // for mouse focus and is sufficient for v1.
    if let Some(new_id) = new_focus {
        seed_caret_for_editable_focus(dom, new_id);
    }

    // focus + focusin on the new target.
    if let Some(new_id) = new_focus {
        let mut foc = TuiEvent::focus();
        let _ = dom.dispatch_tui_event(new_id, &mut foc);
        let mut fin = TuiEvent::focusin();
        let _ = dom.dispatch_tui_event(new_id, &mut fin);
    }
}

/// Seed a collapsed caret at the start of `id`'s first text-node
/// child when `id` is editable and the current selection is `None`
/// or points outside `id`'s subtree. No-op for non-editable elements
/// or when a selection already covers this subtree (we don't clobber
/// an existing caret position the user has navigated to).
fn seed_caret_for_editable_focus(dom: &mut TuiDom, id: NodeId) {
    if !dom.node(id).is_editable() {
        return;
    }

    // Preserve any pre-existing selection that already lives inside
    // `id`'s subtree — re-focusing the same element after blur
    // shouldn't reset the user's caret position.
    if let Some(sel) = dom.selection()
        && is_descendant_or_self(dom, sel.focus.node, id)
    {
        return;
    }

    // Find the first Text-node child of `id`. `<input>` and
    // `<textarea>` always have one after `seed_all` runs (called
    // from `App::build`); `contenteditable` elements may or may
    // not — skip seeding if there's none.
    let text_child = dom
        .node(id)
        .child_nodes()
        .find(|n| n.node_type() == NodeType::Text)
        .map(|n| n.id());

    if let Some(text_id) = text_child {
        dom.set_selection(Some(Selection::caret(Position::new(text_id, 0))));
    }
}

/// Walk up from `start` via parent_node, returning the nearest
/// ancestor (including `start` itself) that is tab-focusable.
/// Used by the runtime's focus-on-click path: clicking a
/// non-focusable child focuses the nearest focusable ancestor,
/// matching browser behavior.
///
/// Returns `None` if no ancestor is focusable — e.g., clicking in
/// non-interactive chrome like a decoration element.
pub fn nearest_focusable_ancestor(dom: &TuiDom, start: NodeId) -> Option<NodeId> {
    let mut cur = Some(start);
    while let Some(id) = cur {
        if tabindex::is_focusable(dom, id) {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}
