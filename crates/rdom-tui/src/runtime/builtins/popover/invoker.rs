//! Popover invokers (HTML §6.12.3): a button's `popovertarget` names the
//! popover its activation shows, hides or toggles, by its
//! `popovertargetaction`.

use rdom_core::NodeId;

use super::algorithms::{self, Hide};
use super::stack::is_inclusive_ancestor;
use super::{is_showing, popover_state};
use crate::TuiDom;

/// A `popovertargetaction` state (an enumerated attribute: `toggle` is
/// both defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Toggle,
    Show,
    Hide,
}

fn action(dom: &TuiDom, button: NodeId) -> Action {
    match dom.node(button).get_attribute("popovertargetaction") {
        Some(v) if v.eq_ignore_ascii_case("show") => Action::Show,
        Some(v) if v.eq_ignore_ascii_case("hide") => Action::Hide,
        _ => Action::Toggle,
    }
}

/// HTML "get the popover target element" of `node`: for a button (a
/// `<button>`, an `<input>` in the Button, Submit, Reset or Image state)
/// that is not disabled and is not a form's submit button, the element
/// its `popovertarget` names, when that has a `popover` attribute.
pub(super) fn popover_target(dom: &TuiDom, node: NodeId) -> Option<NodeId> {
    if !super::super::button::is_button_like(dom, node) || dom.is_actually_disabled(node) {
        return None;
    }
    if dom.form_owner(node).is_some() && dom.is_submit_button(node) {
        return None;
    }
    let id = dom.node(node).get_attribute("popovertarget")?;
    let target = dom.get_element_by_id(id)?;
    popover_state(dom, target).map(|_| target)
}

/// The activation behavior of the button that `event_target` (a click's
/// target) is or is inside: HTML "popover target attribute activation
/// behavior".
pub(super) fn activate(dom: &mut TuiDom, event_target: NodeId) {
    let mut cur = Some(event_target);
    let button = loop {
        let Some(n) = cur else {
            return;
        };
        if super::super::button::is_button_like(dom, n) {
            break n;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    };
    let Some(popover) = popover_target(dom, button) else {
        return;
    };
    // A click inside a popover that lives inside its own invoker is the
    // popover's, not the invoker's.
    if is_inclusive_ancestor(dom, popover, event_target)
        && is_inclusive_ancestor(dom, button, popover)
        && popover != button
    {
        return;
    }
    let showing = is_showing(dom, popover);
    match action(dom, button) {
        Action::Show if showing => {}
        Action::Hide if !showing => {}
        // HTML: "run the hide popover algorithm given popover, true,
        // true, false, and node".
        _ if showing => {
            let _ = algorithms::hide(
                dom,
                popover,
                Hide {
                    throw: false,
                    source: Some(button),
                    ..Hide::THROWING
                },
            );
        }
        _ => {
            let _ = algorithms::show(dom, popover, false, Some(button));
        }
    }
}
