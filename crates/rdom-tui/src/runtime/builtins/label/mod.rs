//! `<label>` click default — focus the associated form control.
//!
//! ## Contract (from MDN)
//!
//! Association resolves in two ways (both supported):
//!
//! 1. **Explicit** — `<label for="id">` references an element
//!    by `id`. The first matching element in document order wins.
//! 2. **Implicit** — `<label>` wraps exactly one labelable
//!    control among its descendants. The first such descendant
//!    is the associated control.
//!
//! ### Labelable elements
//!
//! Per HTML living standard: `<button>`, `<input>` (except
//! `type="hidden"`), `<meter>`, `<output>`, `<progress>`,
//! `<select>`, `<textarea>`. Anything else is ignored.
//!
//! ### Click behavior
//!
//! Clicking a label:
//! 1. Moves focus to the associated control.
//! 2. For `<input type="checkbox">` / `<input type="radio">`,
//!    also re-dispatches a click on the control so it toggles.
//!    The re-dispatched click bubbles back through the label
//!    (the input is a descendant), so the label listener uses
//!    a `target != control` check to skip its own
//!    re-dispatches — otherwise the dispatch chain
//!    `label → input → label → input → …` would loop forever.
//!    Other labelables — `<button>`, `<select>`, `<textarea>`,
//!    `<meter>`, `<output>`, `<progress>` — get focus only;
//!    their activation semantics differ from checkbox/radio's
//!    flip-the-state model and re-dispatching click is not what
//!    those controls want.
//!
//! The listener respects `event.preventDefault()` — an author
//! `click` handler on the label can short-circuit the focus
//! transfer.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::runtime::builtins::toggle;
use crate::tui_event::{TuiDispatchExt, TuiEvent};

/// Install the label-click default action. Called once from
/// `App::build`.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();
    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(label) = closest_label(ctx.dom, target) else {
            return;
        };
        // Only handle unmodified clicks — leave the door open
        // for Ctrl-click / Shift-click custom handling later.
        let Some(control) = associated_control(ctx.dom, label) else {
            return;
        };
        crate::runtime::focus::focus_node(ctx.dom, Some(control));

        // For checkbox/radio, also re-dispatch click on the
        // control so the toggle builtin's click listener flips
        // the state.
        //
        // Loop breaker: only re-dispatch if the original target
        // is something OTHER than the control itself. When the
        // user clicks the input glyph directly, `target == control`
        // and the toggle builtin's own click listener has already
        // handled it — re-dispatching would toggle the state a
        // second time (cancelling itself). When our re-dispatched
        // click bubbles back through the label (input is a
        // descendant), `target == control` again and we skip,
        // breaking the otherwise-infinite chain
        // `label → input → label → input → …`.
        //
        // Note we DON'T gate on `is_synthetic` — the mouse router
        // marks every click as synthetic when it synthesizes the
        // click from a mousedown/mouseup pair, so that flag can't
        // distinguish "user-originated" from "re-dispatched."
        if toggle::is_toggle(ctx.dom, control) && target != control {
            let fake_mouse = MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::empty(),
            };
            let mut click = TuiEvent::click(fake_mouse);
            click.event = click.event.clone().with_synthetic(true);
            let _ = ctx.dom.dispatch_tui_event(control, &mut click);
        }
    })
    .expect("root label click listener install");
}

/// Walk up from `id` (inclusive) to the nearest `<label>`
/// element. Returns `None` when no label ancestor exists.
fn closest_label(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).tag_name() == Some("label") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Find the form control this label associates with. Tries the
/// explicit `for` attribute first (by `id`), then falls back to
/// the first labelable descendant (implicit wrap). Returns
/// `None` when neither resolves.
///
/// Public for `TuiAccessors::label_control` (step 30c) — same
/// HTML-spec resolution rule.
pub fn associated_control(dom: &TuiDom, label: NodeId) -> Option<NodeId> {
    // Explicit: `for="id"`.
    if let Some(id_ref) = dom.node(label).get_attribute("for")
        && !id_ref.is_empty()
    {
        // Search the whole tree for an element with matching
        // `id`. Same semantics as HTML — first match wins.
        let root = dom.root();
        let id_ref_owned = id_ref.to_string();
        if let Some(found) = find_by_id(dom, root, &id_ref_owned) {
            return Some(found);
        }
        return None;
    }
    // Implicit: first labelable descendant.
    find_labelable_descendant(dom, label)
}

fn find_by_id(dom: &TuiDom, root: NodeId, needle: &str) -> Option<NodeId> {
    if dom.node(root).get_attribute("id") == Some(needle) {
        return Some(root);
    }
    for child in dom.node(root).child_nodes() {
        if let Some(found) = find_by_id(dom, child.id(), needle) {
            return Some(found);
        }
    }
    None
}

fn find_labelable_descendant(dom: &TuiDom, start: NodeId) -> Option<NodeId> {
    for child in dom.node(start).child_nodes() {
        let id = child.id();
        if is_labelable(dom, id) {
            return Some(id);
        }
        if let Some(found) = find_labelable_descendant(dom, id) {
            return Some(found);
        }
    }
    None
}

/// HTML living-standard "labelable element" list.
fn is_labelable(dom: &TuiDom, id: NodeId) -> bool {
    let node = dom.node(id);
    let Some(tag) = node.tag_name() else {
        return false;
    };
    match tag {
        "button" | "meter" | "output" | "progress" | "select" | "textarea" => true,
        // Input is labelable UNLESS type="hidden".
        "input" => !matches!(node.get_attribute("type"), Some("hidden")),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
