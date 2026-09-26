//! Radio button groups — HTML §4.10.5.1.18. Names, form owners and tree
//! shape only; no rendering, no runtime.
//!
//! Two `<input type="radio">` elements *a* and *b* are in the same radio
//! button group when they are in the same tree, have the same form owner
//! (or both have none — [`Dom::form_owner`], which honours `form=`), and
//! both carry a non-empty `name` whose values are equal. The comparison
//! is exact: HTML dropped the old compatibility-caseless match, so
//! `name="G"` and `name="g"` are two groups. A radio without a `name`
//! (or with an empty one) is a group of its own.
//!
//! Exclusivity, arrow-key movement, the single Tab stop and
//! `valueMissing` in `rdom-tui` all use this one definition.

use crate::dom::Dom;
use crate::input_type::InputTypeState;
use crate::node_id::NodeId;

/// What identifies a named radio's group: tree root, form owner, name.
struct GroupKey<'a> {
    root: NodeId,
    owner: Option<NodeId>,
    name: &'a str,
}

impl<Ext> Dom<Ext> {
    /// Whether `a` and `b` are in the same radio button group (HTML
    /// §4.10.5.1.18). Every radio is in its own group, so `a == b` is
    /// `true` for a radio; anything that is not an `<input type="radio">`
    /// is in no group at all.
    pub fn in_same_radio_group(&self, a: NodeId, b: NodeId) -> bool {
        if !self.is_radio(a) || !self.is_radio(b) {
            return false;
        }
        if a == b {
            return true;
        }
        match (self.radio_group_key(a), self.radio_group_key(b)) {
            (Some(ka), Some(kb)) => {
                ka.root == kb.root && ka.owner == kb.owner && ka.name == kb.name
            }
            _ => false,
        }
    }

    /// The radio button group of `id` in tree order, `id` included
    /// (HTML §4.10.5.1.18). A radio without a non-empty `name` is alone
    /// in its group; a non-radio has none (empty). One pass over `id`'s
    /// tree; form owners are resolved only for same-named radios.
    pub fn radio_group(&self, id: NodeId) -> Vec<NodeId> {
        if !self.is_radio(id) {
            return Vec::new();
        }
        let Some(key) = self.radio_group_key(id) else {
            return vec![id];
        };
        let mut out = Vec::new();
        let mut stack = vec![key.root];
        while let Some(cur) = stack.pop() {
            let Some(node) = self.get_node(cur) else {
                continue;
            };
            if cur == id
                || (self.is_radio(cur)
                    && self.get_attribute(cur, "name") == Some(key.name)
                    && self.form_owner(cur) == key.owner)
            {
                out.push(cur);
            }
            // Push children in reverse so they pop in tree order.
            let first = stack.len();
            let mut c = node.first_child;
            while let Some(cid) = c {
                stack.push(cid);
                c = self.get_node(cid).and_then(|n| n.next_sibling);
            }
            stack[first..].reverse();
        }
        out
    }

    fn is_radio(&self, id: NodeId) -> bool {
        self.input_type_state(id) == Some(InputTypeState::Radio)
    }

    /// The group key of a radio with a non-empty `name`; `None` for a
    /// nameless radio.
    fn radio_group_key(&self, id: NodeId) -> Option<GroupKey<'_>> {
        let name = self.get_attribute(id, "name").filter(|n| !n.is_empty())?;
        Some(GroupKey {
            root: self.root_of(id)?,
            owner: self.form_owner(id),
            name,
        })
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

    fn radio(dom: &mut Dom, parent: NodeId, attrs: &[(&str, &str)]) -> NodeId {
        let r = el(dom, parent, "input", &[("type", "radio")]);
        for (k, v) in attrs {
            dom.set_attribute(r, k, v).unwrap();
        }
        r
    }

    #[test]
    fn same_named_radios_in_different_forms_are_different_groups() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let fa = el(&mut dom, root, "form", &[]);
        let fb = el(&mut dom, root, "form", &[]);
        let a1 = radio(&mut dom, fa, &[("name", "g")]);
        let a2 = radio(&mut dom, fa, &[("name", "g")]);
        let b1 = radio(&mut dom, fb, &[("name", "g")]);
        let loose = radio(&mut dom, root, &[("name", "g")]);
        assert_eq!(dom.radio_group(a2), vec![a1, a2]);
        assert_eq!(dom.radio_group(b1), vec![b1]);
        assert_eq!(dom.radio_group(loose), vec![loose]);
        assert!(dom.in_same_radio_group(a1, a2));
        assert!(!dom.in_same_radio_group(a1, b1));
        assert!(!dom.in_same_radio_group(a1, loose));
    }

    #[test]
    fn a_form_attribute_moves_a_radio_into_that_forms_group() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let before = radio(&mut dom, root, &[("name", "g"), ("form", "f")]);
        let f = el(&mut dom, root, "form", &[("id", "f")]);
        let inside = radio(&mut dom, f, &[("name", "g")]);
        let elsewhere = radio(&mut dom, f, &[("name", "g"), ("form", "nope")]);
        let unowned = radio(&mut dom, root, &[("name", "g")]);
        assert_eq!(dom.radio_group(inside), vec![before, inside]);
        assert_eq!(dom.radio_group(elsewhere), vec![elsewhere, unowned]);
    }

    #[test]
    fn names_compare_exactly_and_an_empty_name_is_alone() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let upper = radio(&mut dom, root, &[("name", "G")]);
        let lower = radio(&mut dom, root, &[("name", "g")]);
        let empty1 = radio(&mut dom, root, &[("name", "")]);
        let empty2 = radio(&mut dom, root, &[("name", "")]);
        let none = radio(&mut dom, root, &[]);
        assert!(!dom.in_same_radio_group(upper, lower));
        assert!(!dom.in_same_radio_group(empty1, empty2));
        assert_eq!(dom.radio_group(empty1), vec![empty1]);
        assert_eq!(dom.radio_group(none), vec![none]);
        assert!(dom.in_same_radio_group(none, none));
    }

    #[test]
    fn non_radios_and_other_trees_are_not_in_the_group() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let r = radio(&mut dom, root, &[("name", "g")]);
        let cb = el(
            &mut dom,
            root,
            "input",
            &[("type", "checkbox"), ("name", "g")],
        );
        let upper_type = el(&mut dom, root, "input", &[("type", "RADIO"), ("name", "g")]);
        let detached = dom.create_element("div");
        let other_tree = radio(&mut dom, detached, &[("name", "g")]);
        assert_eq!(dom.radio_group(r), vec![r, upper_type]);
        assert!(dom.radio_group(cb).is_empty());
        assert!(!dom.in_same_radio_group(r, cb));
        assert!(!dom.in_same_radio_group(r, other_tree));
        assert_eq!(dom.radio_group(other_tree), vec![other_tree]);
    }
}
