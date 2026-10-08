//! The pointer state of pseudo-elements — which one is hovered and which
//! one is active (Selectors 4 §9.2 / §9.4) — read by a rule whose
//! pseudo-element is followed by user-action pseudo-classes
//! (`::before:hover`, §3.6.3; [`Rule::pseudo_state`](crate::Rule)).
//!
//! rdom-core keeps the hovered and active *elements*. A pseudo-element
//! has no node, and rdom-core knows no pseudo-element slots (it stays
//! renderer-free), so the backend keeps this state as document data
//! ([`Dom::document_data`]): the runtime writes it from the hit test
//! ([`HitTestExt::hit_test_pseudo`](crate::HitTestExt::hit_test_pseudo))
//! while the sheets hold such a rule ([`set_tracked`]), the cascade's rule
//! matching reads it ([`matches`]), and a change names the hosts to
//! restyle ([`take_dirty`]), which the App's frame cascades with the
//! dirty tracker's roots. One pseudo-element is hovered at a time: the
//! innermost under the pointer.

use rdom_core::{Dom, NodeId};
use rdom_style::{PseudoElementTarget, UserActionState};

use crate::ext::{PseudoSlot, TuiExt};
use crate::style::Stylesheet;

/// A pseudo-element: its host and slot.
pub(crate) type PseudoRef = (NodeId, PseudoSlot);

/// The document's pseudo-element pointer state.
#[derive(Debug, Default)]
struct PseudoPointer {
    /// The sheets hold a rule that reads it: the runtime keeps it.
    tracked: bool,
    hovered: Option<PseudoRef>,
    active: Option<PseudoRef>,
    /// Hosts whose pseudo-element state changed since the last drain.
    dirty: Vec<NodeId>,
}

/// Whether any of `sheets` has a rule a pseudo-element's pointer state
/// decides (`::before:hover`): only then is the state worth keeping.
pub(crate) fn sheets_read_it<'a>(sheets: impl IntoIterator<Item = &'a Stylesheet>) -> bool {
    sheets
        .into_iter()
        .any(|s| s.rules().iter().any(|r| !r.pseudo_state.is_empty()))
}

/// Whether the runtime keeps the state ([`set_tracked`]).
pub(crate) fn is_tracked(dom: &Dom<TuiExt>) -> bool {
    dom.document_data::<PseudoPointer>()
        .is_some_and(|p| p.tracked)
}

/// Keep the state (the sheets read it) or not. Turning it off forgets
/// the hovered and active pseudo-elements, restyling their hosts.
pub(crate) fn set_tracked(dom: &mut Dom<TuiExt>, tracked: bool) {
    if tracked == is_tracked(dom) {
        return;
    }
    if !tracked {
        set_hovered(dom, None);
        set_active(dom, None);
    }
    state_mut(dom).tracked = tracked;
}

/// Point the hovered pseudo-element at `next`. Returns whether it moved.
pub(crate) fn set_hovered(dom: &mut Dom<TuiExt>, next: Option<PseudoRef>) -> bool {
    set(dom, next, |p| &mut p.hovered)
}

/// Point the active pseudo-element at `next`. Returns whether it moved.
pub(crate) fn set_active(dom: &mut Dom<TuiExt>, next: Option<PseudoRef>) -> bool {
    set(dom, next, |p| &mut p.active)
}

fn set(
    dom: &mut Dom<TuiExt>,
    next: Option<PseudoRef>,
    field: impl Fn(&mut PseudoPointer) -> &mut Option<PseudoRef>,
) -> bool {
    if next.is_none() && dom.document_data::<PseudoPointer>().is_none() {
        return false;
    }
    let state = state_mut(dom);
    let prev = std::mem::replace(field(state), next);
    if prev == next {
        return false;
    }
    state.dirty.extend(prev.map(|(host, _)| host));
    state.dirty.extend(next.map(|(host, _)| host));
    true
}

/// The hosts to restyle — those whose pseudo-element state changed since
/// the last call and are still in the document.
pub(crate) fn take_dirty(dom: &mut Dom<TuiExt>) -> Vec<NodeId> {
    let mut hosts = match dom.document_data_mut::<PseudoPointer>() {
        Some(state) => std::mem::take(&mut state.dirty),
        None => return Vec::new(),
    };
    hosts.retain(|&h| dom.contains(h) && dom.node(h).is_connected());
    hosts
}

/// Whether `state` holds of `host`'s `target` pseudo-element (Selectors
/// 4 §3.6.3): an empty set always does; the focus pseudo-classes never
/// do; `:hover` / `:active` when the pseudo-element is the hovered /
/// active one.
pub(crate) fn matches(
    dom: &Dom<TuiExt>,
    host: NodeId,
    target: &PseudoElementTarget,
    state: UserActionState,
) -> bool {
    if state.is_empty() {
        return true;
    }
    let slot = match target {
        PseudoElementTarget::Before => PseudoSlot::Before,
        PseudoElementTarget::After => PseudoSlot::After,
        PseudoElementTarget::Marker => PseudoSlot::Marker,
        PseudoElementTarget::FirstLetter => PseudoSlot::FirstLetter,
        _ => return false,
    };
    let me = Some((host, slot));
    let (hovered, active) = dom
        .document_data::<PseudoPointer>()
        .map_or((false, false), |p| (p.hovered == me, p.active == me));
    state.matches(hovered, active)
}

/// The state, created on first use.
fn state_mut(dom: &mut Dom<TuiExt>) -> &mut PseudoPointer {
    if dom.document_data::<PseudoPointer>().is_none() {
        dom.set_document_data(PseudoPointer::default());
    }
    dom.document_data_mut::<PseudoPointer>()
        .expect("just inserted")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TuiDom;

    /// A host that left the document is not restyled: its id may be
    /// stale (DOM §4.2.3), and a detached subtree has no cascade.
    #[test]
    fn dirty_hosts_still_in_the_document_are_drained_once() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let (a, b) = (dom.create_element("a"), dom.create_element("b"));
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        assert!(set_hovered(&mut dom, Some((a, PseudoSlot::Before))));
        assert!(!set_hovered(&mut dom, Some((a, PseudoSlot::Before))));
        assert!(set_hovered(&mut dom, Some((b, PseudoSlot::After))));
        dom.remove_child(root, a).unwrap();
        assert_eq!(take_dirty(&mut dom), [b]);
        assert!(take_dirty(&mut dom).is_empty());
    }

    /// Forgetting the state restyles the hosts that lost it.
    #[test]
    fn untracking_forgets_the_hovered_and_active_pseudo_elements() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        dom.append_child(root, a).unwrap();
        set_tracked(&mut dom, true);
        set_active(&mut dom, Some((a, PseudoSlot::Marker)));
        take_dirty(&mut dom);
        set_tracked(&mut dom, false);
        assert_eq!(take_dirty(&mut dom), [a]);
        let marker = &PseudoElementTarget::Marker;
        assert!(!matches(&dom, a, marker, UserActionState::ACTIVE));
    }
}
