//! HTML §6.12's stack walks: hide popovers until, hide popover stack
//! until, topmost popover ancestor, nearest inclusive open popover,
//! topmost auto or hint popover.
//!
//! Each walk is bounded by what it sees when it starts: "hide popover
//! stack until" hides a slice of the list fixed up front, then makes one
//! more pass — with no events — over whatever listeners managed to show
//! meanwhile (HTML refuses that anyway: show popover step 2).

use rdom_core::NodeId;

use super::PopoverState;
use super::algorithms::{self, Hide};
use super::state::{hint_stack_parent, opened_mode, showing_list, walk_list};
use crate::TuiDom;

/// HTML §6.12 "hide popovers until" `endpoint` (`None`: the document).
pub(super) fn hide_popovers_until(
    dom: &mut TuiDom,
    endpoint: Option<NodeId>,
    focus_previous: bool,
    fire_events: bool,
) {
    // Step 1.
    let endpoint_is_hint =
        endpoint.is_some_and(|e| showing_list(dom, PopoverState::Hint).contains(&e));
    // Step 2.
    hide_stack_until(
        dom,
        endpoint,
        PopoverState::Hint,
        focus_previous,
        fire_events,
    );
    // Steps 3–4: a hint endpoint keeps the auto popover its stack hangs
    // from.
    let auto_endpoint = if endpoint_is_hint {
        hint_stack_parent(dom)
    } else {
        endpoint
    };
    // Step 5.
    hide_stack_until(
        dom,
        auto_endpoint,
        PopoverState::Auto,
        focus_previous,
        fire_events,
    );
}

/// HTML §6.12 "hide popover stack until": hide the `mode` popovers above
/// `endpoint` (all of them when it is `None` or not in the list), top
/// down, then — without events — any that showing popovers' listeners
/// put above it meanwhile.
pub(super) fn hide_stack_until(
    dom: &mut TuiDom,
    endpoint: Option<NodeId>,
    mode: PopoverState,
    focus_previous: bool,
    fire_events: bool,
) {
    // Steps 1–4.
    let list = walk_list(dom, mode);
    let last_hide_index = endpoint
        .and_then(|e| list.iter().position(|&p| p == e))
        .map_or(0, |i| i + 1);
    let (to_remain, to_hide) = list.split_at(last_hide_index);
    // Step 5.
    for &p in to_hide.iter().rev() {
        let _ = algorithms::hide(dom, p, Hide::quiet(focus_previous, fire_events));
    }
    // Steps 6–8: fireEvents is ignored on this pass.
    for p in walk_list(dom, mode).into_iter().rev() {
        if to_remain.contains(&p) {
            continue;
        }
        let _ = algorithms::hide(dom, p, Hide::quiet(focus_previous, false));
    }
}

/// HTML §6.12 "topmost popover ancestor" of `new` (a popover about to
/// show when `is_popover`, else a top-layer element such as a modal
/// dialog) through the node tree and through `source`: the last popover
/// of the showing auto list followed by the hint list that holds `new`
/// or `source`.
pub(super) fn topmost_popover_ancestor(
    dom: &TuiDom,
    new: NodeId,
    source: Option<NodeId>,
    is_popover: bool,
) -> Option<NodeId> {
    debug_assert!(is_popover || source.is_none());
    // Step 4.
    let mut combined = showing_list(dom, PopoverState::Auto);
    combined.extend(showing_list(dom, PopoverState::Hint));
    // Steps 5–8: the last item that `new` / `source` is a descendant of.
    let last_holding = |node: NodeId| {
        combined
            .iter()
            .rposition(|&p| p != node && is_inclusive_ancestor(dom, p, node))
    };
    let popover_index = last_holding(new);
    let source_index = source.and_then(last_holding);
    // Steps 9–11.
    popover_index.max(source_index).map(|i| combined[i])
}

/// HTML "nearest inclusive open popover": `node` or its nearest ancestor
/// that is a showing popover opened as `auto` or `hint`.
pub(super) fn nearest_inclusive_open_popover(dom: &TuiDom, node: NodeId) -> Option<NodeId> {
    let mut cur = Some(node);
    while let Some(id) = cur {
        if opened_mode(dom, id).is_some() {
            return Some(id);
        }
        cur = parent(dom, id);
    }
    None
}

/// HTML "topmost auto or hint popover": the last showing hint popover,
/// else the last showing auto one.
pub(super) fn topmost_auto_or_hint(dom: &TuiDom) -> Option<NodeId> {
    showing_list(dom, PopoverState::Hint)
        .last()
        .or(showing_list(dom, PopoverState::Auto).last())
        .copied()
}

fn parent(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    dom.node(id).parent_node().map(|p| p.id())
}

/// `ancestor == id` or `ancestor` contains `id`.
pub(super) fn is_inclusive_ancestor(dom: &TuiDom, ancestor: NodeId, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = parent(dom, n);
    }
    false
}
