//! Popover light dismiss (HTML §6.12.2 "light dismiss open popovers")
//! and the close requests popovers answer.
//!
//! - **Pointer** — the mouse router reports each press ([`pointer_down`])
//!   and release ([`pointer_up`]), of any button: the release hides
//!   every auto / hint popover above the *topmost clicked popover* — the
//!   popover the point is in, or the one whose invoker it is on, whichever
//!   is higher — when the press began in that same one (outside every
//!   popover, both: all of them).
//! - **Esc** — a close request goes to the most recent close watcher: the
//!   topmost of the modal dialogs and the auto / hint popovers in the top
//!   layer ([`topmost_close_watcher`]). A popover there is hidden; a
//!   modal dialog is the dialog builtin's (`cancel`, then `close`). One
//!   press, one close: the handler that acts claims the key.
//!
//! HTML has no focus-based light dismiss: focus leaving a popover leaves
//! it open.

use rdom_core::{NodeId, TopLayerKind};

use super::algorithms::{self, Hide, hide_all_until, nearest_inclusive_open_popover, showing_list};
use super::invoker::popover_target;
use super::{PopoverState, is_showing, popover_state};
use crate::TuiDom;

/// Where the last press began: its topmost clicked popover.
#[derive(Debug, Default)]
struct PointerDown(Option<NodeId>);

fn any_light_dismissable(dom: &TuiDom) -> bool {
    !showing_list(dom, PopoverState::Auto).is_empty()
        || !showing_list(dom, PopoverState::Hint).is_empty()
}

/// A press at `target` (`None`: on nothing).
pub(crate) fn pointer_down(dom: &mut TuiDom, target: Option<NodeId>) {
    if !any_light_dismissable(dom) {
        return;
    }
    let clicked = target.and_then(|t| topmost_clicked_popover(dom, t));
    dom.set_document_data(PointerDown(clicked));
}

/// A release at `target`. Returns whether it hid a popover.
pub(crate) fn pointer_up(dom: &mut TuiDom, target: Option<NodeId>) -> bool {
    if !any_light_dismissable(dom) {
        return false;
    }
    let ancestor = target.and_then(|t| topmost_clicked_popover(dom, t));
    let down = dom.remove_document_data::<PointerDown>().and_then(|d| d.0);
    if ancestor != down {
        return false;
    }
    let before = dom.top_layer().len();
    hide_all_until(dom, ancestor, false, true);
    dom.top_layer().len() != before
}

/// HTML "topmost clicked popover": of the nearest open popover holding
/// `node` and the nearest one `node` (or an ancestor) is an invoker of,
/// the one higher in the stacks.
fn topmost_clicked_popover(dom: &TuiDom, node: NodeId) -> Option<NodeId> {
    let clicked = nearest_inclusive_open_popover(dom, node);
    let invoked = nearest_inclusive_target_popover(dom, node);
    if stack_position(dom, clicked) > stack_position(dom, invoked) {
        clicked
    } else {
        invoked
    }
}

/// HTML "nearest inclusive target popover for invoker": the showing auto
/// or hint popover that `node` or its nearest ancestor invokes.
fn nearest_inclusive_target_popover(dom: &TuiDom, node: NodeId) -> Option<NodeId> {
    let mut cur = Some(node);
    while let Some(id) = cur {
        if let Some(target) = popover_target(dom, id)
            && matches!(
                popover_state(dom, target),
                Some(PopoverState::Auto | PopoverState::Hint)
            )
            && is_showing(dom, target)
        {
            return Some(target);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// HTML "popover stack position": 0 for none, else the place in the auto
/// list, the hint list counting above it.
fn stack_position(dom: &TuiDom, popover: Option<NodeId>) -> usize {
    let Some(p) = popover else {
        return 0;
    };
    let auto = showing_list(dom, PopoverState::Auto);
    let hint = showing_list(dom, PopoverState::Hint);
    if let Some(i) = hint.iter().position(|&h| h == p) {
        return i + auto.len() + 1;
    }
    auto.iter().position(|&a| a == p).map_or(0, |i| i + 1)
}

/// The most recent close watcher: the topmost modal dialog or auto /
/// hint popover in the top layer.
pub fn topmost_close_watcher(dom: &TuiDom) -> Option<NodeId> {
    dom.top_layer()
        .iter()
        .rev()
        .copied()
        .find(|&id| match dom.top_layer_kind(id) {
            Some(TopLayerKind::ModalDialog) => true,
            Some(TopLayerKind::Popover) => matches!(
                algorithms::opened_mode(dom, id),
                Some(PopoverState::Auto | PopoverState::Hint)
            ),
            _ => false,
        })
}

/// Esc: hide the topmost close watcher when it is a popover (HTML's close
/// watcher of an auto or hint popover runs "hide popover" with focus
/// restored and events). Returns whether it did.
pub(super) fn close_request(dom: &mut TuiDom) -> bool {
    let Some(top) = topmost_close_watcher(dom) else {
        return false;
    };
    if dom.top_layer_kind(top) != Some(TopLayerKind::Popover) {
        return false;
    }
    let _ = algorithms::hide(
        dom,
        top,
        Hide {
            throw: false,
            ..Hide::THROWING
        },
    );
    true
}
