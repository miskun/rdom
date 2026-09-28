//! Per-element form state, kept off the common `TuiExt` path
//! (`P7G-FORM-STATE-BOX-1`): only form controls and `<form>`s use it,
//! so it lives behind a box created on first use and every other
//! element pays one pointer.

use std::cell::OnceCell;

use crate::runtime::builtins::validation::PatternCache;

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
    /// A `<form>`'s "firing submission events" flag (HTML §4.10.21.3
    /// step 6): set while its submission runs interactive validation and
    /// fires `submit`, so a listener's nested submission returns early.
    pub(crate) firing_submission_events: bool,
    /// The compiled `pattern` attribute (`validation::pattern`).
    pub(crate) pattern_cache: PatternCache,
}

/// Equality is over the state; the pattern cache is left out — it is
/// derived from the `pattern` attribute, which lives in the DOM, not in
/// this state, and two elements with the same attribute may have
/// compiled it or not.
impl PartialEq for FormControlState {
    fn eq(&self, other: &Self) -> bool {
        self.custom_validity == other.custom_validity
            && self.value_user_edited == other.value_user_edited
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
