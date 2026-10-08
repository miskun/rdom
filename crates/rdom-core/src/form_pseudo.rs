//! What the input pseudo-classes match (Selectors 4 §14, with HTML
//! §4.16.3's definitions): the predicates the substrate answers from
//! names, attributes and tree shape. The validity verdicts behind
//! `:valid` / `:invalid` live in `constraint`.

use crate::control_state::ControlState;
use crate::dom::Dom;
use crate::input_type::InputTypeState;
use crate::node_id::NodeId;
use crate::query_selector::caches::SelectorCaches;

impl<Ext> Dom<Ext> {
    /// Whether `id` matches `:read-write` (HTML §4.16.3):
    ///
    /// - an `<input>` that the `readonly` attribute applies to (the
    ///   text, date / time and number states) and that is *mutable* —
    ///   no `readonly` attribute, not [actually
    ///   disabled](Self::is_actually_disabled);
    /// - a `<textarea>` without `readonly` that is not actually disabled;
    /// - any other element that is an [editing host or
    ///   editable](Self::is_editable_or_editing_host).
    ///
    /// `:read-only` matches every other element.
    pub fn is_read_write(&self, id: NodeId) -> bool {
        match self.get_node(id).and_then(|n| n.tag_name()) {
            None => false,
            Some("input") => {
                self.input_type_state(id)
                    .is_some_and(crate::constraint::readonly_applies)
                    && self.is_mutable(id)
            }
            Some("textarea") => self.is_mutable(id),
            Some(_) => self.is_editable_or_editing_host(id),
        }
    }

    /// Whether `id` matches `:indeterminate` (HTML §4.16.3):
    ///
    /// - an `<input type=checkbox>` whose indeterminate flag is set — the
    ///   `indeterminate` IDL attribute, which rdom reflects into an
    ///   `indeterminate` content attribute as it reflects checkedness into
    ///   `checked` (DIVERGENCES §2);
    /// - an `<input type=radio>` whose [radio button
    ///   group](Self::radio_group) has no checked member;
    /// - a `<progress>` without a `value` attribute.
    pub fn is_indeterminate(&self, id: NodeId) -> bool {
        self.indeterminate_with(id, &mut SelectorCaches::new())
    }

    /// [`is_indeterminate`](Self::is_indeterminate), the radio groups
    /// answered once per pass through `caches`.
    pub(crate) fn indeterminate_with(&self, id: NodeId, caches: &mut SelectorCaches) -> bool {
        match self.get_node(id).and_then(|n| n.tag_name()) {
            Some("progress") => !self.has_attribute(id, "value"),
            Some("input") => match self.input_type_state(id) {
                Some(InputTypeState::Checkbox) => self.has_attribute(id, "indeterminate"),
                Some(InputTypeState::Radio) => {
                    if let Some(&unchecked) = caches.radio_unchecked.get(&id) {
                        return unchecked;
                    }
                    caches.count_radio_group_walk();
                    let group = self.radio_group(id);
                    let unchecked = !group.iter().any(|&r| self.has_attribute(r, "checked"));
                    for r in group {
                        caches.radio_unchecked.insert(r, unchecked);
                    }
                    unchecked
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether `id` matches `:default` (HTML §4.16.3):
    ///
    /// - a `<button>` or an `<input type=submit|image>` that is the
    ///   *default button* of its form owner — the form's first submit
    ///   button in tree order (HTML §4.10.21.2), disabled or not;
    /// - an `<input type=checkbox|radio>` checked by default
    ///   ([`ControlState::DefaultChecked`]);
    /// - an `<option>` selected by default
    ///   ([`ControlState::DefaultSelected`]).
    pub fn is_default(&self, id: NodeId) -> bool {
        self.default_with(id, &mut SelectorCaches::new())
    }

    /// [`is_default`](Self::is_default), each form's default button found
    /// once per pass through `caches`.
    pub(crate) fn default_with(&self, id: NodeId, caches: &mut SelectorCaches) -> bool {
        match self.get_node(id).and_then(|n| n.tag_name()) {
            Some("option") => self.control_state(id, ControlState::DefaultSelected),
            Some("input")
                if matches!(
                    self.input_type_state(id),
                    Some(InputTypeState::Checkbox | InputTypeState::Radio)
                ) =>
            {
                self.control_state(id, ControlState::DefaultChecked)
            }
            Some("button" | "input") if self.is_submit_button(id) => {
                let Some(form) = self.form_owner(id) else {
                    return false;
                };
                let default = match caches.default_buttons.get(&form) {
                    Some(&found) => found,
                    None => {
                        caches.count_default_button_walk();
                        let found = self
                            .form_listed_elements(form)
                            .into_iter()
                            .find(|&c| self.is_submit_button(c));
                        caches.default_buttons.insert(form, found);
                        found
                    }
                };
                default == Some(id)
            }
            _ => false,
        }
    }

    /// `:in-range` / `:out-of-range` (HTML §4.16.3): for a [candidate for
    /// constraint validation](Self::will_validate) that has range
    /// limitations ([`ControlState::RangeLimited`]), `Some(true)` when its
    /// value suffers from neither an underflow nor an overflow
    /// ([`ControlState::OutOfRange`]), `Some(false)` when it does; `None`
    /// — neither pseudo-class — for every other element.
    pub fn range_state(&self, id: NodeId) -> Option<bool> {
        (self.will_validate(id) && self.control_state(id, ControlState::RangeLimited))
            .then(|| !self.control_state(id, ControlState::OutOfRange))
    }

    /// `:user-valid` / `:user-invalid` (HTML §4.16.3): for an `<input>`,
    /// `<textarea>` or `<select>` whose user validity is set
    /// ([`ControlState::UserValidity`]) and that is a [candidate for
    /// constraint validation](Self::will_validate), `Some(true)` when it
    /// satisfies its constraints, `Some(false)` when it does not; `None`
    /// — neither pseudo-class — for every other element.
    pub fn user_validity_state(&self, id: NodeId) -> Option<bool>
    where
        Ext: 'static,
    {
        if !matches!(
            self.get_node(id).and_then(|n| n.tag_name()),
            Some("input" | "textarea" | "select")
        ) || !self.control_state(id, ControlState::UserValidity)
        {
            return None;
        }
        self.constraint_validity(id)
    }

    /// No `readonly` attribute and not actually disabled.
    fn is_mutable(&self, id: NodeId) -> bool {
        !self.has_attribute(id, "readonly") && !self.is_actually_disabled(id)
    }
}
