//! Forced pseudo-classes: an inspector's "force element state" (CAPTURE
//! phase 4, `specs/CAPTURE.md` §Edits — DevTools' `CSS.forcePseudoState`).
//! An element forced into `:hover` styles as hovered without the pointer
//! being over it, so a tool can see and edit a state it cannot hold by
//! hand.
//!
//! A forced state is for rendering only: it is honored by a matching pass
//! that asks for it ([`SelectorCaches::for_rendering`] — a backend's
//! cascade), never by [`Dom::matches`] / `querySelector` from app code, and
//! setting one fires no `MutationObserver` record — app code cannot tell
//! a state is forced. The backend restyles after forcing one.
//!
//! [`SelectorCaches::for_rendering`]: crate::SelectorCaches::for_rendering

use crate::dom::Dom;
use crate::node_id::NodeId;

/// A user-action pseudo-class an element can be forced into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ForcedState {
    /// `:hover`.
    Hover,
    /// `:active`.
    Active,
    /// `:focus`.
    Focus,
    /// `:focus-visible`.
    FocusVisible,
    /// `:focus-within`.
    FocusWithin,
}

impl ForcedState {
    fn bit(self) -> u8 {
        match self {
            ForcedState::Hover => 1,
            ForcedState::Active => 2,
            ForcedState::Focus => 4,
            ForcedState::FocusVisible => 8,
            ForcedState::FocusWithin => 16,
        }
    }

    /// The state a pseudo-class name (`"hover"`, `":focus-visible"`) names.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name.trim_start_matches(':') {
            "hover" => ForcedState::Hover,
            "active" => ForcedState::Active,
            "focus" => ForcedState::Focus,
            "focus-visible" => ForcedState::FocusVisible,
            "focus-within" => ForcedState::FocusWithin,
            _ => return None,
        })
    }

    /// Every state, in declaration order.
    pub const ALL: [ForcedState; 5] = [
        ForcedState::Hover,
        ForcedState::Active,
        ForcedState::Focus,
        ForcedState::FocusVisible,
        ForcedState::FocusWithin,
    ];
}

impl<Ext> Dom<Ext> {
    /// Force `id` into `state` (`on`) or stop forcing it. Fires no record:
    /// the caller restyles. A rendering match ([`crate::SelectorCaches::
    /// for_rendering`]) then matches the pseudo-class on `id` itself.
    pub fn set_forced_state(&mut self, id: NodeId, state: ForcedState, on: bool) {
        let bits = self.forced_bits(id);
        let next = if on {
            bits | state.bit()
        } else {
            bits & !state.bit()
        };
        self.forced.retain(|(n, _)| *n != id);
        if next != 0 {
            self.forced.push((id, next));
        }
    }

    /// The states `id` is forced into.
    pub fn forced_states(&self, id: NodeId) -> Vec<ForcedState> {
        let bits = self.forced_bits(id);
        ForcedState::ALL
            .into_iter()
            .filter(|s| bits & s.bit() != 0)
            .collect()
    }

    /// Every forced element and its states.
    pub fn forced_elements(&self) -> Vec<(NodeId, Vec<ForcedState>)> {
        self.forced
            .iter()
            .map(|(n, _)| (*n, self.forced_states(*n)))
            .collect()
    }

    /// Stop forcing every state on every element.
    pub fn clear_forced_states(&mut self) {
        self.forced.clear();
    }

    /// Whether a rendering match takes `id` as in `state`.
    pub(crate) fn is_forced(&self, id: NodeId, state: ForcedState) -> bool {
        self.forced_bits(id) & state.bit() != 0
    }

    fn forced_bits(&self, id: NodeId) -> u8 {
        self.forced
            .iter()
            .find(|(n, _)| *n == id)
            .map_or(0, |(_, b)| *b)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::ForcedState;
    use crate::{Dom, Mutation, MutationObserver, SelectorCaches};

    struct Count(Rc<Cell<usize>>);
    impl MutationObserver<()> for Count {
        fn observe(&mut self, _: &mut Dom<()>, _: &Mutation) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn a_forced_state_matches_in_a_rendering_pass_only() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let b = dom.create_element("button");
        dom.append_child(root, b).unwrap();
        let records = Rc::new(Cell::new(0));
        dom.add_mutation_observer(Box::new(Count(records.clone())));
        dom.set_forced_state(b, ForcedState::Hover, true);
        assert_eq!(records.get(), 0, "forcing fires no record");
        let hover = crate::selectors::parse(":hover").unwrap();
        assert!(dom.matches_list_with(b, &hover, None, &mut SelectorCaches::for_rendering()));
        assert!(
            !dom.matches(b, ":hover").unwrap(),
            "app code sees the real state"
        );
        assert!(
            !dom.matches_list_with(root, &hover, None, &mut SelectorCaches::for_rendering()),
            "only the element itself"
        );
        assert_eq!(dom.forced_states(b), [ForcedState::Hover]);
        dom.set_forced_state(b, ForcedState::FocusVisible, true);
        dom.set_forced_state(b, ForcedState::Hover, false);
        assert_eq!(dom.forced_states(b), [ForcedState::FocusVisible]);
        dom.clear_forced_states();
        assert!(dom.forced_elements().is_empty());
        assert_eq!(
            ForcedState::from_name(":focus-within"),
            Some(ForcedState::FocusWithin)
        );
        assert_eq!(ForcedState::from_name("checked"), None);
    }
}
