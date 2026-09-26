//! Constraint-validation candidacy (HTML §4.10.20.1) — which elements
//! take part in constraint validation at all. Names, attributes and tree
//! shape only; the validity *states* need values and patterns and live
//! in the backend (rdom-tui's `validation` builtin).
//!
//! - [`Dom::will_validate`] — a submittable element that is not *barred
//!   from constraint validation*: the `willValidate` IDL attribute.
//! - [`Dom::constraint_validity`] — what `:valid` / `:invalid` match
//!   (HTML §4.16.3), with the per-candidate verdict delegated to the
//!   backend's [`ValidityHook`].
//! - [`Dom::is_required_control`] / [`Dom::is_optional_control`] — what
//!   `:required` / `:optional` match.

use crate::dom::Dom;
use crate::input_type::InputTypeState;
use crate::node_id::NodeId;

/// A backend's constraint check: whether candidate `id` satisfies its
/// constraints. The validity states need values, patterns and state the
/// substrate does not model (rdom-tui's `validation` builtin computes
/// them), so the substrate asks the backend through this hook — a plain
/// `fn`, so matching (`&self`) can call it. Installed with
/// [`Dom::set_validity_hook`].
pub type ValidityHook<Ext> = fn(&Dom<Ext>, NodeId) -> bool;

/// Storage for the hook with a `Debug` impl.
pub(crate) struct ValiditySlot<Ext: 'static>(pub(crate) Option<ValidityHook<Ext>>);

impl<Ext: 'static> std::fmt::Debug for ValiditySlot<Ext> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "ValiditySlot(Some(hook))"
        } else {
            "ValiditySlot(None)"
        })
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// Install (or remove, with `None`) the backend's constraint check
    /// behind `:valid` / `:invalid`. Without one, every candidate is
    /// valid. rdom-tui installs its own in `App` construction (and
    /// `runtime::builtins::validation::install` for a bare `TuiDom`).
    pub fn set_validity_hook(&mut self, hook: Option<ValidityHook<Ext>>) {
        self.validity_hook = ValiditySlot(hook);
    }

    /// What `:valid` / `:invalid` match (HTML §4.16.3):
    ///
    /// - a [candidate](Self::will_validate): `Some(verdict)` of the
    ///   validity hook (`Some(true)` without one);
    /// - a `<form>`: `Some(false)` when a candidate it owns
    ///   ([`form_listed_elements`](Self::form_listed_elements)) is
    ///   invalid, else `Some(true)`;
    /// - a `<fieldset>`: the same over its descendant candidates;
    /// - anything else, barred controls included: `None` (neither).
    pub fn constraint_validity(&self, id: NodeId) -> Option<bool> {
        let tag = self.get_node(id)?.tag_name()?;
        match tag {
            "form" => Some(
                !self
                    .form_listed_elements(id)
                    .into_iter()
                    .any(|c| self.suffers(c)),
            ),
            "fieldset" => Some(!self.any_descendant_suffers(id)),
            _ if self.will_validate(id) => Some(self.satisfies(id)),
            _ => None,
        }
    }

    /// Whether `id` matches `:required` (HTML §4.16.3): an `<input>` in
    /// a state the `required` attribute applies to (not hidden, range,
    /// color or the button types), a `<select>` or a `<textarea>`, with
    /// `required`.
    pub fn is_required_control(&self, id: NodeId) -> bool {
        use InputTypeState as T;
        let applies = match self.get_node(id).and_then(|n| n.tag_name()) {
            Some("select" | "textarea") => true,
            Some("input") => !matches!(
                self.input_type_state(id),
                Some(T::Hidden | T::Range | T::Color | T::Submit | T::Image | T::Reset | T::Button)
            ),
            _ => false,
        };
        applies && self.has_attribute(id, "required")
    }

    /// Whether `id` matches `:optional`: an `<input>`, `<select>` or
    /// `<textarea>` that is not [required](Self::is_required_control).
    pub fn is_optional_control(&self, id: NodeId) -> bool {
        matches!(
            self.get_node(id).and_then(|n| n.tag_name()),
            Some("input" | "select" | "textarea")
        ) && !self.is_required_control(id)
    }

    /// The hook's verdict for candidate `id` (valid without a hook).
    fn satisfies(&self, id: NodeId) -> bool {
        self.validity_hook.0.is_none_or(|hook| hook(self, id))
    }

    /// A candidate that does not satisfy its constraints.
    fn suffers(&self, id: NodeId) -> bool {
        self.will_validate(id) && !self.satisfies(id)
    }

    fn any_descendant_suffers(&self, id: NodeId) -> bool {
        let mut stack: Vec<NodeId> = Vec::new();
        let push_children = |stack: &mut Vec<NodeId>, of: NodeId| {
            let mut c = self.get_node(of).and_then(|n| n.first_child);
            while let Some(cid) = c {
                stack.push(cid);
                c = self.get_node(cid).and_then(|n| n.next_sibling);
            }
        };
        push_children(&mut stack, id);
        while let Some(n) = stack.pop() {
            if self.suffers(n) {
                return true;
            }
            push_children(&mut stack, n);
        }
        false
    }
}

impl<Ext> Dom<Ext> {
    /// Whether `id` is a *candidate for constraint validation* (HTML
    /// §4.10.20.1, the `willValidate` IDL attribute): a submittable
    /// element (`<button>`, `<input>`, `<select>`, `<textarea>`) that is
    /// not barred. Barred are:
    ///
    /// - an [actually disabled](Self::is_actually_disabled) control
    ///   (own `disabled`, or a disabled `<fieldset>`);
    /// - a control with a `<datalist>` ancestor;
    /// - a `<button>` that is not a submit button;
    /// - an `<input>` in the Hidden, Reset or Button state;
    /// - an `<input>` with `readonly` in a state `readonly` applies to
    ///   (the text, date / time and number states — not a checkbox), and
    ///   a `<textarea readonly>`.
    ///
    /// `<object>`, `<output>` and `<fieldset>` are never candidates.
    pub fn will_validate(&self, id: NodeId) -> bool {
        let Some(tag) = self.get_node(id).and_then(|n| n.tag_name()) else {
            return false;
        };
        let submittable_and_not_self_barred = match tag {
            "button" => self.is_submit_button(id),
            "input" => {
                use InputTypeState as T;
                match self.input_type_state(id) {
                    Some(T::Hidden | T::Reset | T::Button) | None => false,
                    Some(t) => !(readonly_applies(t) && self.has_attribute(id, "readonly")),
                }
            }
            "select" => true,
            "textarea" => !self.has_attribute(id, "readonly"),
            _ => false,
        };
        submittable_and_not_self_barred
            && !self.is_actually_disabled(id)
            && !self.has_datalist_ancestor(id)
    }

    fn has_datalist_ancestor(&self, id: NodeId) -> bool {
        let mut cur = self.parent_element_id(id);
        while let Some(a) = cur {
            if self.tag_is(a, "datalist") {
                return true;
            }
            cur = self.parent_element_id(a);
        }
        false
    }
}

/// The `<input>` states the `readonly` attribute applies to (HTML
/// §4.10.5, the "readonly" row of the attribute table).
fn readonly_applies(t: InputTypeState) -> bool {
    use InputTypeState as T;
    matches!(
        t,
        T::Text
            | T::Search
            | T::Url
            | T::Tel
            | T::Email
            | T::Password
            | T::Date
            | T::Month
            | T::Week
            | T::Time
            | T::DateTimeLocal
            | T::Number
    )
}

#[cfg(test)]
mod tests {
    use crate::Dom;
    use crate::node_id::NodeId;

    fn el(dom: &mut Dom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
        let e = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(e, k, v).unwrap();
        }
        dom.append_child(parent, e).unwrap();
        e
    }

    #[test]
    fn candidates_are_the_submittable_controls_that_are_not_barred() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        for (tag, attrs, candidate) in [
            ("input", vec![], true),
            ("input", vec![("type", "checkbox")], true),
            ("input", vec![("type", "Submit")], true),
            ("input", vec![("type", "hidden")], false),
            ("input", vec![("type", "reset")], false),
            ("input", vec![("type", "button")], false),
            ("button", vec![], true),
            ("button", vec![("type", "button")], false),
            ("button", vec![("type", "reset")], false),
            ("select", vec![], true),
            ("textarea", vec![], true),
            ("fieldset", vec![], false),
            ("output", vec![], false),
            ("object", vec![], false),
            ("form", vec![], false),
            ("div", vec![], false),
        ] {
            let id = el(&mut dom, root, tag, &attrs);
            assert_eq!(dom.will_validate(id), candidate, "{tag} {attrs:?}");
        }
    }

    #[test]
    fn disabled_readonly_and_datalist_controls_are_barred() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let disabled = el(&mut dom, root, "input", &[("disabled", "")]);
        let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let in_fs = el(&mut dom, fs, "select", &[]);
        let ro_text = el(&mut dom, root, "input", &[("readonly", "")]);
        let ro_area = el(&mut dom, root, "textarea", &[("readonly", "")]);
        let ro_box = el(
            &mut dom,
            root,
            "input",
            &[("type", "checkbox"), ("readonly", "")],
        );
        let list = el(&mut dom, root, "datalist", &[]);
        let in_list = el(&mut dom, list, "input", &[]);
        for id in [disabled, in_fs, ro_text, ro_area, in_list] {
            assert!(!dom.will_validate(id), "{id:?}");
        }
        assert!(
            dom.will_validate(ro_box),
            "readonly does not apply to a checkbox"
        );
    }
}
