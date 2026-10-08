//! Per-element form state, kept off the common `TuiExt` path
//! (`P7G-FORM-STATE-BOX-1`): only form controls and `<form>`s use it,
//! so it lives behind a box created on first use and every other
//! element pays one pointer.

use std::cell::OnceCell;

use rdom_core::{ControlState, NodeId};

use crate::TuiDom;
use crate::runtime::builtins::validation::{self, PatternCache};

/// The [`rdom_core::ControlStateHook`] rdom-tui installs
/// (`validation::install`): the defaults the form builtins keep beside
/// the live attributes, and the range states `validation` computes.
pub(crate) fn control_state(dom: &TuiDom, id: NodeId, state: ControlState) -> Option<bool> {
    use crate::accessors::TuiAccessors;
    Some(match state {
        ControlState::DefaultChecked => dom.node(id).default_checked().unwrap_or(false),
        ControlState::DefaultSelected => dom.node(id).default_selected().unwrap_or(false),
        ControlState::RangeLimited => validation::range_limited(dom, id),
        ControlState::UserValidity => user_validity(dom, id),
        ControlState::OutOfRange => {
            let v = validation::validity(dom, id);
            v.range_underflow || v.range_overflow
        }
        // `ControlState` is `#[non_exhaustive]`: a question added
        // upstream gets an arm here; until then, the substrate's answer.
        _ => {
            debug_assert!(false, "control_state: unanswered {state:?}");
            return None;
        }
    })
}

/// The form-only state of one element.
#[derive(Debug, Default, Clone)]
pub(crate) struct FormControlState {
    /// The custom validity error message (`setCustomValidity`); empty =
    /// no custom error.
    pub(crate) custom_validity: String,
    /// The text control's value was last changed by a user edit (HTML's
    /// dirty value flag + "last changed by a user edit"): typing, delete,
    /// paste, undo / redo set it; a programmatic value or a form reset
    /// clears it. Only such a value is subject to `maxlength` /
    /// `minlength`.
    pub(crate) value_user_edited: bool,
    /// The user changed the value since it was last committed (HTML
    /// §4.10.5.5: a text control commits on losing focus, firing
    /// `change`). Set with `value_user_edited`; cleared by the commit, a
    /// programmatic value and a reset.
    pub(crate) change_pending: bool,
    /// HTML §4.10.18.1 *user validity*: set when the user commits a
    /// change (wherever a builtin fires `change`) or the form owner's
    /// submission is attempted, cleared by a form reset — what
    /// `:user-valid` / `:user-invalid` read.
    pub(crate) user_validity: bool,
    /// A `<form>`'s "firing submission events" flag (HTML §4.10.21.3
    /// step 6): set while its submission runs interactive validation and
    /// fires `submit`, so a listener's nested submission returns early.
    pub(crate) firing_submission_events: bool,
    /// The compiled `pattern` attribute (`validation::pattern`).
    pub(crate) pattern_cache: PatternCache,
}

/// `id`'s user validity (HTML §4.10.18.1).
pub(crate) fn user_validity(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id)
        .ext()
        .and_then(|e| e.form_state.get())
        .is_some_and(|f| f.user_validity)
}

/// Set `id`'s user validity. Nothing reports it as a mutation, so the
/// App's form-state marks are told a state write happened; clearing a
/// flag never set allocates nothing.
pub(crate) fn set_user_validity(dom: &mut TuiDom, id: NodeId, value: bool) {
    let mut node = dom.node_mut(id);
    let Some(ext) = node.ext_mut() else {
        return;
    };
    let state = if value {
        Some(ext.form_state.get_mut())
    } else {
        ext.form_state.existing_mut()
    };
    if let Some(state) = state
        && state.user_validity != value
    {
        state.user_validity = value;
        crate::runtime::state_writes::note();
    }
}

/// The user committed a change to `id`: set its user validity, then
/// fire a bubbling `change` at it — the order HTML gives every control's
/// commit (§4.10.5.5, the checkbox and radio activation behavior, the
/// select update notifications).
pub(crate) fn fire_change(dom: &mut TuiDom, id: NodeId) {
    set_user_validity(dom, id, true);
    let mut change = crate::TuiEvent::new("change");
    crate::tui_event::dispatch_to_live(dom, id, &mut change);
}

/// A text control is losing focus (HTML §4.10.5.5 "commits"): when the
/// user changed its value since the last commit, fire `change` through
/// [`fire_change`]. Called before `blur`, as browsers order them.
pub(crate) fn commit_pending_change(dom: &mut TuiDom, id: NodeId) {
    let pending = dom
        .node_mut(id)
        .ext_mut()
        .and_then(|e| e.form_state.existing_mut())
        .is_some_and(|f| std::mem::take(&mut f.change_pending));
    if pending {
        fire_change(dom, id);
    }
}

/// Equality is over the state; the pattern cache is left out — it is
/// derived from the `pattern` attribute, which lives in the DOM, not in
/// this state, and two elements with the same attribute may have
/// compiled it or not.
impl PartialEq for FormControlState {
    fn eq(&self, other: &Self) -> bool {
        self.custom_validity == other.custom_validity
            && self.value_user_edited == other.value_user_edited
            && self.change_pending == other.change_pending
            && self.user_validity == other.user_validity
            && self.firing_submission_events == other.firing_submission_events
    }
}

/// `TuiExt`'s form state slot: empty until an element first needs its
/// form state. A `OnceCell` rather than an `Option`, so the pattern
/// cache can be created from the shared borrow validity checks run
/// under. An empty slot equals one holding the default state.
#[derive(Debug, Default, Clone)]
pub(crate) struct FormControlSlot(OnceCell<Box<FormControlState>>);

impl FormControlSlot {
    /// The state, `None` while the element has never needed it (every
    /// field at its default).
    pub(crate) fn get(&self) -> Option<&FormControlState> {
        self.0.get().map(|b| &**b)
    }

    /// The state, created if needed, through a shared borrow.
    pub(crate) fn get_or_init(&self) -> &FormControlState {
        self.0.get_or_init(Box::default)
    }

    /// The state, if it was ever created — for resetting a field to its
    /// default without allocating.
    pub(crate) fn existing_mut(&mut self) -> Option<&mut FormControlState> {
        self.0.get_mut().map(|b| &mut **b)
    }

    /// The state, created if needed.
    pub(crate) fn get_mut(&mut self) -> &mut FormControlState {
        if self.0.get().is_none() {
            let _ = self.0.set(Box::default());
        }
        self.0
            .get_mut()
            .expect("the form state slot was just filled")
    }
}

impl PartialEq for FormControlSlot {
    fn eq(&self, other: &Self) -> bool {
        match (self.get(), other.get()) {
            (Some(a), Some(b)) => a == b,
            (Some(s), None) | (None, Some(s)) => *s == FormControlState::default(),
            (None, None) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_slot_equals_the_default_state_and_differs_from_a_set_one() {
        let empty = FormControlSlot::default();
        let mut touched = FormControlSlot::default();
        touched.get_or_init();
        assert_eq!(empty, touched, "created but untouched");
        touched.get_mut().custom_validity = "bad".into();
        assert_ne!(empty, touched);
        assert_ne!(touched, empty);
    }

    #[test]
    fn clearing_a_field_of_an_element_without_form_state_allocates_nothing() {
        let mut dom: crate::TuiDom = crate::TuiDom::new();
        let input = dom.create_element("input");
        dom.append_child(dom.root(), input).unwrap();
        crate::runtime::builtins::validation::set_custom_validity(&mut dom, input, "");
        crate::runtime::builtins::input::clear_user_edited(&mut dom, input);
        assert!(dom.node(input).ext().unwrap().form_state.get().is_none());
        crate::runtime::builtins::validation::set_custom_validity(&mut dom, input, "bad");
        let state = dom.node(input).ext().unwrap().form_state.get().cloned();
        assert_eq!(state.map(|s| s.custom_validity), Some("bad".to_string()));
    }

    #[test]
    fn equality_ignores_the_pattern_cache() {
        let a = FormControlSlot::default();
        let b = FormControlSlot::default();
        *b.get_or_init().pattern_cache.cell().borrow_mut() = Some(("x".into(), None));
        assert_eq!(a, b);
    }
}
