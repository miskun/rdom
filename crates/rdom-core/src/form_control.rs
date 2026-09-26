//! Form-control state the substrate can answer from names, attributes
//! and tree shape alone — no rendering, no runtime.
//!
//! - [`Dom::is_actually_disabled`] — HTML §4.10.18.5 "disabled" plus the
//!   `<fieldset>` rule (§4.10.15) and the `<option>` / `<optgroup>` rules
//!   (§4.10.9 / §4.10.10): the one predicate behind the `:disabled` /
//!   `:enabled` selectors, focus, activation and form submission.

use crate::dom::Dom;
use crate::node_id::NodeId;

/// Elements HTML calls *form controls* for the purpose of the
/// `disabled` attribute: the ones a `disabled` attribute (own or via a
/// disabled `<fieldset>`) makes actually disabled.
fn is_disableable_control(tag: &str) -> bool {
    matches!(tag, "button" | "input" | "select" | "textarea" | "fieldset")
}

impl<Ext> Dom<Ext> {
    /// Whether `id` is *actually disabled* (HTML §4.16.3, the state
    /// `:disabled` matches):
    ///
    /// - a `<button>`, `<input>`, `<select>`, `<textarea>` or
    ///   `<fieldset>` with a `disabled` attribute (any value);
    /// - one of those that is a descendant of a `<fieldset disabled>`
    ///   and not a descendant of that fieldset's first `<legend>` child
    ///   (§4.10.18.5 form controls, §4.10.15 "disabled fieldset");
    /// - an `<optgroup disabled>`;
    /// - an `<option disabled>`, or an `<option>` whose parent is an
    ///   `<optgroup disabled>` (§4.10.10).
    ///
    /// Every other element — including a `<div disabled>` — is never
    /// disabled: the attribute means nothing there. A dead or
    /// non-element id is not disabled.
    pub fn is_actually_disabled(&self, id: NodeId) -> bool {
        let Some(tag) = self.get_node(id).and_then(|n| n.tag_name()) else {
            return false;
        };
        match tag {
            "option" => {
                self.has_attribute(id, "disabled")
                    || self.parent_element_id(id).is_some_and(|p| {
                        self.tag_is(p, "optgroup") && self.has_attribute(p, "disabled")
                    })
            }
            "optgroup" => self.has_attribute(id, "disabled"),
            t if is_disableable_control(t) => {
                self.has_attribute(id, "disabled") || self.in_disabled_fieldset(id)
            }
            _ => false,
        }
    }

    /// Whether `id` matches `:enabled` (HTML §4.16.3): one of the
    /// elements that *can* be disabled (`<button>`, `<input>`,
    /// `<select>`, `<textarea>`, `<fieldset>`, `<optgroup>`, `<option>`)
    /// that is not [actually disabled](Self::is_actually_disabled).
    pub fn is_enabled_control(&self, id: NodeId) -> bool {
        let Some(tag) = self.get_node(id).and_then(|n| n.tag_name()) else {
            return false;
        };
        (is_disableable_control(tag) || matches!(tag, "optgroup" | "option"))
            && !self.is_actually_disabled(id)
    }

    /// Is `id` a descendant of a `<fieldset disabled>` without being
    /// inside that fieldset's first `<legend>` child? Walks the ancestor
    /// chain once, remembering the child through which each ancestor was
    /// reached.
    fn in_disabled_fieldset(&self, id: NodeId) -> bool {
        let mut via = id;
        let mut cur = self.parent_element_id(id);
        while let Some(anc) = cur {
            if self.tag_is(anc, "fieldset")
                && self.has_attribute(anc, "disabled")
                && self.first_legend_child(anc) != Some(via)
            {
                return true;
            }
            via = anc;
            cur = self.parent_element_id(anc);
        }
        false
    }

    /// The fieldset's first `<legend>` element child, if any.
    fn first_legend_child(&self, fieldset: NodeId) -> Option<NodeId> {
        let mut c = self.get_node(fieldset)?.first_child;
        while let Some(cid) = c {
            let n = self.get_node(cid)?;
            if n.tag_name() == Some("legend") {
                return Some(cid);
            }
            c = n.next_sibling;
        }
        None
    }

    fn parent_element_id(&self, id: NodeId) -> Option<NodeId> {
        self.get_node(id).and_then(|n| n.parent)
    }

    fn tag_is(&self, id: NodeId, tag: &str) -> bool {
        self.get_node(id).and_then(|n| n.tag_name()) == Some(tag)
    }
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
    fn own_disabled_attribute_disables_a_control_but_not_a_div() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let input = el(&mut dom, root, "input", &[("disabled", "")]);
        let div = el(&mut dom, root, "div", &[("disabled", "")]);
        let plain = el(&mut dom, root, "button", &[]);
        assert!(dom.is_actually_disabled(input));
        assert!(!dom.is_actually_disabled(div));
        assert!(!dom.is_actually_disabled(plain));
    }

    /// HTML §4.10.18.5: a descendant of a disabled fieldset is disabled.
    #[test]
    fn control_inside_disabled_fieldset_is_disabled() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let div = el(&mut dom, fs, "div", &[]);
        let input = el(&mut dom, div, "input", &[]);
        let button = el(&mut dom, fs, "button", &[]);
        let select = el(&mut dom, fs, "select", &[]);
        let textarea = el(&mut dom, fs, "textarea", &[]);
        for c in [input, button, select, textarea] {
            assert!(dom.is_actually_disabled(c));
        }
        assert!(!dom.is_actually_disabled(div), "a div is never disabled");
    }

    /// The first `<legend>` child is exempt; a later legend is not.
    #[test]
    fn control_in_first_legend_stays_enabled_but_not_in_a_later_one() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let legend = el(&mut dom, fs, "legend", &[]);
        let in_legend = el(&mut dom, legend, "input", &[("type", "checkbox")]);
        let legend2 = el(&mut dom, fs, "legend", &[]);
        let in_legend2 = el(&mut dom, legend2, "input", &[]);
        assert!(!dom.is_actually_disabled(in_legend));
        assert!(dom.is_actually_disabled(in_legend2));
    }

    /// A legend that is not a direct child of the fieldset is no exemption.
    #[test]
    fn nested_legend_that_is_not_a_child_is_no_exemption() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let div = el(&mut dom, fs, "div", &[]);
        let legend = el(&mut dom, div, "legend", &[]);
        let input = el(&mut dom, legend, "input", &[]);
        assert!(dom.is_actually_disabled(input));
    }

    /// §4.10.15: a fieldset inside a disabled fieldset is itself
    /// disabled, and so is its content — even its own first legend,
    /// which only exempts from *its* `disabled` attribute.
    #[test]
    fn nested_fieldset_in_a_disabled_fieldset_disables_its_content() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let outer = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let inner = el(&mut dom, outer, "fieldset", &[]);
        let inner_legend = el(&mut dom, inner, "legend", &[]);
        let in_inner_legend = el(&mut dom, inner_legend, "input", &[]);
        let input = el(&mut dom, inner, "input", &[]);
        assert!(dom.is_actually_disabled(outer));
        assert!(dom.is_actually_disabled(inner));
        assert!(dom.is_actually_disabled(input));
        assert!(dom.is_actually_disabled(in_inner_legend));
    }

    /// An inner disabled fieldset disables its content even when the
    /// outer one is enabled; a fieldset inside the outer's first legend
    /// is exempt from the outer's attribute.
    #[test]
    fn inner_disabled_fieldset_and_legend_exempt_fieldset() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let outer = el(&mut dom, root, "fieldset", &[]);
        let inner = el(&mut dom, outer, "fieldset", &[("disabled", "")]);
        let input = el(&mut dom, inner, "input", &[]);
        assert!(!dom.is_actually_disabled(outer));
        assert!(dom.is_actually_disabled(input));

        let off = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let legend = el(&mut dom, off, "legend", &[]);
        let fs_in_legend = el(&mut dom, legend, "fieldset", &[]);
        let deep = el(&mut dom, fs_in_legend, "input", &[]);
        assert!(!dom.is_actually_disabled(fs_in_legend));
        assert!(!dom.is_actually_disabled(deep));
    }

    /// §4.10.10: an option is disabled by its own attribute or a disabled
    /// optgroup parent — not by a disabled fieldset (it is not a form
    /// control; its `<select>` is).
    #[test]
    fn option_and_optgroup_rules() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let select = el(&mut dom, fs, "select", &[]);
        let og = el(&mut dom, select, "optgroup", &[("disabled", "")]);
        let in_og = el(&mut dom, og, "option", &[]);
        let own = el(&mut dom, select, "option", &[("disabled", "")]);
        let plain = el(&mut dom, select, "option", &[]);
        assert!(dom.is_actually_disabled(select));
        assert!(dom.is_actually_disabled(og));
        assert!(dom.is_actually_disabled(in_og));
        assert!(dom.is_actually_disabled(own));
        assert!(!dom.is_actually_disabled(plain));
    }

    #[test]
    fn disabled_and_enabled_selectors_follow_the_predicate() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let fs = el(&mut dom, root, "fieldset", &[("disabled", "")]);
        let legend = el(&mut dom, fs, "legend", &[]);
        let in_legend = el(&mut dom, legend, "input", &[]);
        let input = el(&mut dom, fs, "input", &[]);
        let div = el(&mut dom, root, "div", &[("disabled", "")]);

        assert!(dom.matches(fs, ":disabled").unwrap());
        assert!(dom.matches(input, ":disabled").unwrap());
        assert!(dom.matches(input, "input:disabled").unwrap());
        assert!(!dom.matches(input, ":enabled").unwrap());
        assert!(!dom.matches(in_legend, ":disabled").unwrap());
        assert!(dom.matches(in_legend, ":enabled").unwrap());
        // `:disabled` / `:enabled` apply only to disableable elements.
        assert!(!dom.matches(div, ":disabled").unwrap());
        assert!(!dom.matches(div, ":enabled").unwrap());
        assert!(!dom.matches(legend, ":enabled").unwrap());
        assert_eq!(
            dom.query_selector_all_in(root, ":disabled").unwrap(),
            vec![fs, input]
        );
    }
}
