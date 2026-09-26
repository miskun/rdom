//! Constraint-validation candidacy (HTML §4.10.20.1) — which elements
//! take part in constraint validation at all. Names, attributes and tree
//! shape only; the validity *states* need values and patterns and live
//! in the backend (rdom-tui's `validation` builtin).
//!
//! - [`Dom::will_validate`] — a submittable element that is not *barred
//!   from constraint validation*: the `willValidate` IDL attribute.

use crate::dom::Dom;
use crate::input_type::InputTypeState;
use crate::node_id::NodeId;

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
