//! Attribute + classList API on `Dom`.
//!
//! Attributes stored as `BTreeMap<String, String>` for deterministic
//! iteration (markup round-trips, snapshot stability). Class list as
//! `BTreeSet<String>` for set semantics. Both live inside the `Element`
//! variant of `NodeData`.

use crate::dom::Dom;
use crate::error::{DomError, Result};
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::observer::Mutation;

impl<Ext: 'static> Dom<Ext> {
    // ── Attributes ───────────────────────────────────────────────────

    pub fn set_attribute(&mut self, id: NodeId, key: &str, value: &str) -> Result<()> {
        let old_full = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { attrs, .. } => attrs.insert(key.to_string(), value.to_string()),
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if key == "id" {
            if let Some(prev) = &old_full {
                self.indexes.unregister_id(id, prev);
            }
            self.indexes.register_id(id, value);
        }
        // Per WHATWG DOM: setting the "class" attribute MUST update
        // `Element.classList`. The attribute string is just one of
        // three sources that must agree (attrs["class"] / the
        // `classes` BTreeSet / the per-class `indexes` map);
        // `set_attribute` is the WHATWG-canonical entry point for
        // setting `class`, so it owns the sync.
        if key == "class" {
            self.sync_class_list_from_attribute_value(id, value);
        }
        self.fire_mutation(Mutation::AttributeChanged {
            id,
            name: key.to_string(),
            old: old_full,
            new: Some(value.to_string()),
        });
        Ok(())
    }

    /// Rebuild the `classes` BTreeSet + selector indexes from the
    /// whitespace-separated class attribute value. Called by
    /// `set_attribute` whenever the "class" attribute is written so
    /// classList stays in sync with the attribute string.
    ///
    /// Fires one `ClassChanged` record with the net diff (added /
    /// removed) iff the set actually changed — observers see
    /// classList changes whether they came via `add_class` or
    /// `set_attribute("class", _)`.
    fn sync_class_list_from_attribute_value(&mut self, id: NodeId, value: &str) {
        let new_tokens: std::collections::BTreeSet<String> =
            value.split_whitespace().map(String::from).collect();
        let old_tokens: std::collections::BTreeSet<String> = match self.get_node(id) {
            Some(node) => match &node.data {
                NodeData::Element { classes, .. } => classes.clone(),
                _ => return,
            },
            None => return,
        };
        if new_tokens == old_tokens {
            return;
        }
        let added: Vec<String> = new_tokens.difference(&old_tokens).cloned().collect();
        let removed: Vec<String> = old_tokens.difference(&new_tokens).cloned().collect();

        for cls in &removed {
            self.indexes.unregister_class(id, cls);
        }
        for cls in &added {
            self.indexes.register_class(id, cls);
        }
        if let Some(node) = self.node_mut_or_err(id).ok()
            && let NodeData::Element { classes, .. } = &mut node.data
        {
            *classes = new_tokens;
        }
        self.fire_mutation(Mutation::ClassChanged { id, added, removed });
    }

    /// Write the `class` attribute string from the current
    /// `classes` BTreeSet. Called by `add_class`/`remove_class`/
    /// `toggle_class`/`replace_class` to maintain the reverse
    /// half of the round-trip with `set_attribute("class", _)`.
    /// Joins tokens with single spaces — iteration is
    /// alphabetic per the BTreeSet ordering, which is a
    /// pre-existing iteration-order divergence from browsers
    /// (documented in [`crate::token_list::DomTokenList`]).
    fn sync_class_attribute_from_class_list(&mut self, id: NodeId) {
        let new_attr: String = match self.get_node(id) {
            Some(node) => match &node.data {
                NodeData::Element { classes, .. } => {
                    classes.iter().cloned().collect::<Vec<_>>().join(" ")
                }
                _ => return,
            },
            None => return,
        };
        // Write directly to attrs without going back through
        // `set_attribute` — that would loop through
        // sync_class_list_from_attribute_value. The attribute
        // change fires no synthetic `AttributeChanged` record
        // here: the `ClassChanged` record from the calling
        // add/remove/toggle is the canonical signal.
        if let Some(node) = self.node_mut_or_err(id).ok()
            && let NodeData::Element { attrs, .. } = &mut node.data
        {
            if new_attr.is_empty() {
                attrs.remove("class");
            } else {
                attrs.insert("class".to_string(), new_attr);
            }
        }
    }

    pub fn get_attribute(&self, id: NodeId, key: &str) -> Option<&str> {
        match &self.get_node(id)?.data {
            NodeData::Element { attrs, .. } => attrs.get(key).map(String::as_str),
            _ => None,
        }
    }

    pub fn remove_attribute(&mut self, id: NodeId, key: &str) -> Result<bool> {
        let removed = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { attrs, .. } => attrs.remove(key),
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if key == "id"
            && let Some(prev) = &removed
        {
            self.indexes.unregister_id(id, prev);
        }
        if removed.is_some() {
            self.fire_mutation(Mutation::AttributeChanged {
                id,
                name: key.to_string(),
                old: removed.clone(),
                new: None,
            });
        }
        Ok(removed.is_some())
    }

    pub fn has_attribute(&self, id: NodeId, key: &str) -> bool {
        matches!(
            self.get_node(id).map(|n| &n.data),
            Some(NodeData::Element { attrs, .. }) if attrs.contains_key(key)
        )
    }

    /// Toggle: if absent, set to empty string; if present, remove.
    /// Returns the new presence state.
    pub fn toggle_attribute(&mut self, id: NodeId, key: &str) -> Result<bool> {
        let (was_present, prev_value) = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { attrs, .. } => {
                if let Some(prev) = attrs.remove(key) {
                    (true, Some(prev))
                } else {
                    attrs.insert(key.to_string(), String::new());
                    (false, None)
                }
            }
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if key == "id"
            && let Some(prev) = &prev_value
        {
            self.indexes.unregister_id(id, prev);
        }
        // else: added as empty string — empty ids are ignored by indexer.
        let (old, new) = if was_present {
            (prev_value.clone(), None)
        } else {
            (None, Some(String::new()))
        };
        self.fire_mutation(Mutation::AttributeChanged {
            id,
            name: key.to_string(),
            old,
            new,
        });
        Ok(!was_present)
    }

    /// Iterate `(name, value)` pairs in deterministic (alphabetic) order.
    pub fn attributes(&self, id: NodeId) -> impl Iterator<Item = (&str, &str)> {
        let slot = self.get_node(id);

        match slot.map(|n| &n.data) {
            Some(NodeData::Element { attrs, .. }) => {
                Box::new(attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                    as Box<dyn Iterator<Item = (&str, &str)>>
            }
            _ => Box::new(std::iter::empty()) as Box<dyn Iterator<Item = (&str, &str)>>,
        }
    }

    /// Convenience: `id` attribute.
    pub fn set_id(&mut self, id: NodeId, value: &str) -> Result<()> {
        self.set_attribute(id, "id", value)
    }

    pub fn id_attr(&self, id: NodeId) -> Option<&str> {
        self.get_attribute(id, "id")
    }

    // ── classList ────────────────────────────────────────────────────

    pub fn add_class(&mut self, id: NodeId, class: &str) -> Result<()> {
        let inserted = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { classes, .. } => classes.insert(class.to_string()),
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if inserted {
            self.indexes.register_class(id, class);
            self.sync_class_attribute_from_class_list(id);
            self.fire_mutation(Mutation::ClassChanged {
                id,
                added: vec![class.to_string()],
                removed: vec![],
            });
        }
        Ok(())
    }

    pub fn remove_class(&mut self, id: NodeId, class: &str) -> Result<bool> {
        let removed = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { classes, .. } => classes.remove(class),
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if removed {
            self.indexes.unregister_class(id, class);
            self.sync_class_attribute_from_class_list(id);
            self.fire_mutation(Mutation::ClassChanged {
                id,
                added: vec![],
                removed: vec![class.to_string()],
            });
        }
        Ok(removed)
    }

    pub fn toggle_class(&mut self, id: NodeId, class: &str) -> Result<bool> {
        let (removed, added) = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { classes, .. } => {
                if classes.remove(class) {
                    (true, false)
                } else {
                    classes.insert(class.to_string());
                    (false, true)
                }
            }
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if removed {
            self.indexes.unregister_class(id, class);
            self.sync_class_attribute_from_class_list(id);
            self.fire_mutation(Mutation::ClassChanged {
                id,
                added: vec![],
                removed: vec![class.to_string()],
            });
        } else if added {
            self.indexes.register_class(id, class);
            self.sync_class_attribute_from_class_list(id);
            self.fire_mutation(Mutation::ClassChanged {
                id,
                added: vec![class.to_string()],
                removed: vec![],
            });
        }
        Ok(added)
    }

    pub fn has_class(&self, id: NodeId, class: &str) -> bool {
        matches!(
            self.get_node(id).map(|n| &n.data),
            Some(NodeData::Element { classes, .. }) if classes.contains(class)
        )
    }

    pub fn replace_class(&mut self, id: NodeId, old: &str, new: &str) -> Result<bool> {
        let swapped = match &mut self.node_mut_or_err(id)?.data {
            NodeData::Element { classes, .. } => {
                if classes.remove(old) {
                    classes.insert(new.to_string());
                    true
                } else {
                    false
                }
            }
            other => {
                return Err(DomError::WrongNodeType {
                    expected: "Element",
                    got: node_type_of(other),
                });
            }
        };
        if swapped {
            self.indexes.unregister_class(id, old);
            self.indexes.register_class(id, new);
            self.sync_class_attribute_from_class_list(id);
            self.fire_mutation(Mutation::ClassChanged {
                id,
                added: vec![new.to_string()],
                removed: vec![old.to_string()],
            });
        }
        Ok(swapped)
    }

    /// Iterate class tokens in alphabetic order.
    pub fn class_list(&self, id: NodeId) -> impl Iterator<Item = &str> {
        let slot = self.get_node(id);

        match slot.map(|n| &n.data) {
            Some(NodeData::Element { classes, .. }) => {
                Box::new(classes.iter().map(String::as_str)) as Box<dyn Iterator<Item = &str>>
            }
            _ => Box::new(std::iter::empty()) as Box<dyn Iterator<Item = &str>>,
        }
    }
}

fn node_type_of<Ext>(data: &NodeData<Ext>) -> crate::node::NodeType {
    use crate::node::NodeType;
    match data {
        NodeData::Element { .. } => NodeType::Element,
        NodeData::Text { .. } => NodeType::Text,
        NodeData::Comment { .. } => NodeType::Comment,
        NodeData::Fragment => NodeType::Fragment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Attributes ───────────────────────────────────────────────────

    #[test]
    fn set_get_remove_attribute() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(!dom.has_attribute(el, "role"));

        dom.set_attribute(el, "role", "banner").unwrap();
        assert_eq!(dom.get_attribute(el, "role"), Some("banner"));
        assert!(dom.has_attribute(el, "role"));

        assert!(dom.remove_attribute(el, "role").unwrap());
        assert!(!dom.has_attribute(el, "role"));
        assert!(!dom.remove_attribute(el, "role").unwrap());
    }

    #[test]
    fn set_attribute_overwrites() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "role", "banner").unwrap();
        dom.set_attribute(el, "role", "navigation").unwrap();
        assert_eq!(dom.get_attribute(el, "role"), Some("navigation"));
    }

    #[test]
    fn toggle_attribute_flips_presence() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("input");
        assert!(dom.toggle_attribute(el, "disabled").unwrap()); // true (added)
        assert!(dom.has_attribute(el, "disabled"));
        assert_eq!(dom.get_attribute(el, "disabled"), Some(""));

        assert!(!dom.toggle_attribute(el, "disabled").unwrap()); // false (removed)
        assert!(!dom.has_attribute(el, "disabled"));
    }

    #[test]
    fn attributes_iterate_in_alpha_order() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "z", "1").unwrap();
        dom.set_attribute(el, "a", "2").unwrap();
        dom.set_attribute(el, "m", "3").unwrap();
        let names: Vec<&str> = dom.attributes(el).map(|(k, _)| k).collect();
        assert_eq!(names, vec!["a", "m", "z"]);
    }

    #[test]
    fn attribute_on_non_element_errors() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hi");
        assert!(matches!(
            dom.set_attribute(t, "role", "banner").unwrap_err(),
            DomError::WrongNodeType { .. }
        ));
        // Getters gracefully return None.
        assert!(dom.get_attribute(t, "anything").is_none());
    }

    #[test]
    fn id_sugar() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_id(el, "hero").unwrap();
        assert_eq!(dom.id_attr(el), Some("hero"));
    }

    // ── Classes ──────────────────────────────────────────────────────

    #[test]
    fn add_remove_has_class() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "active").unwrap();
        assert!(dom.has_class(el, "active"));
        assert!(dom.remove_class(el, "active").unwrap());
        assert!(!dom.has_class(el, "active"));
        assert!(!dom.remove_class(el, "active").unwrap());
    }

    #[test]
    fn add_class_is_idempotent() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "active").unwrap();
        dom.add_class(el, "active").unwrap();
        let list: Vec<&str> = dom.class_list(el).collect();
        assert_eq!(list, vec!["active"]);
    }

    #[test]
    fn toggle_class() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(dom.toggle_class(el, "on").unwrap()); // added
        assert!(!dom.toggle_class(el, "on").unwrap()); // removed
    }

    #[test]
    fn replace_class_swaps() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "old").unwrap();
        assert!(dom.replace_class(el, "old", "new").unwrap());
        assert!(!dom.has_class(el, "old"));
        assert!(dom.has_class(el, "new"));
    }

    #[test]
    fn replace_class_returns_false_when_old_missing() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(!dom.replace_class(el, "never-there", "new").unwrap());
        assert!(!dom.has_class(el, "new")); // nothing added when old missing
    }

    #[test]
    fn class_list_alpha_order() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "zeta").unwrap();
        dom.add_class(el, "alpha").unwrap();
        dom.add_class(el, "mu").unwrap();
        let list: Vec<&str> = dom.class_list(el).collect();
        assert_eq!(list, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn class_on_non_element_errors() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hi");
        assert!(matches!(
            dom.add_class(t, "x").unwrap_err(),
            DomError::WrongNodeType { .. }
        ));
        assert!(!dom.has_class(t, "anything"));
    }

    // ── class attribute / classList round-trip ────────────────────

    #[test]
    fn set_attribute_class_syncs_class_list() {
        // WHATWG DOM: setting the "class" attribute MUST update
        // `Element.classList`. rdom historically diverged — the
        // attribute string was written but the indexed classList
        // (and selector matching) didn't reflect it. Surfaced by
        // M2's showcase shell: every `.foo` selector silently
        // failed to match. Round-trip fixed in the same patch as
        // this test.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");

        dom.set_attribute(el, "class", "alpha beta").unwrap();

        // class_list now contains the parsed tokens.
        let tokens: Vec<&str> = dom.class_list(el).collect();
        assert!(tokens.contains(&"alpha"));
        assert!(tokens.contains(&"beta"));
        assert_eq!(tokens.len(), 2);

        // has_class reflects the tokens.
        assert!(dom.has_class(el, "alpha"));
        assert!(dom.has_class(el, "beta"));
        assert!(!dom.has_class(el, "gamma"));
    }

    #[test]
    fn set_attribute_class_replaces_existing_classes() {
        // Setting "class" again replaces — the old tokens go away,
        // the new tokens take over.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "old").unwrap();
        assert!(dom.has_class(el, "old"));

        dom.set_attribute(el, "class", "fresh").unwrap();

        assert!(!dom.has_class(el, "old"), "old token cleared");
        assert!(dom.has_class(el, "fresh"), "new token present");
    }

    #[test]
    fn set_attribute_class_empty_clears_class_list() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "x").unwrap();
        dom.add_class(el, "y").unwrap();
        assert_eq!(dom.class_list(el).count(), 2);

        dom.set_attribute(el, "class", "").unwrap();

        assert_eq!(dom.class_list(el).count(), 0);
    }

    #[test]
    fn add_class_syncs_class_attribute() {
        // The reverse direction: `add_class` writes through to
        // `attrs["class"]` so `get_attribute("class")` round-trips
        // with classList membership.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");

        dom.add_class(el, "foo").unwrap();

        let attr = dom.get_attribute(el, "class");
        assert_eq!(attr, Some("foo"), "add_class wrote the attribute as well");
    }

    #[test]
    fn remove_class_syncs_class_attribute() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "a").unwrap();
        dom.add_class(el, "b").unwrap();
        assert!(
            dom.get_attribute(el, "class").unwrap().contains('a')
                && dom.get_attribute(el, "class").unwrap().contains('b')
        );

        dom.remove_class(el, "a").unwrap();

        let attr = dom.get_attribute(el, "class").unwrap_or("");
        assert!(!attr.contains('a'), "removed token gone from attribute");
        assert!(attr.contains('b'), "remaining token still in attribute");
    }

    #[test]
    fn set_attribute_then_class_selector_via_index_round_trips() {
        // The substrate's classList drives selector matching. After
        // `set_attribute(_, "class", "hero")`, queries for class
        // "hero" must return `el`. Without the round-trip sync,
        // every CSS `.hero` selector silently misses — exactly the
        // showcase shell bug surfaced in M2.
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let el = dom.create_element("div");
        dom.append_child(root, el).unwrap();

        dom.set_attribute(el, "class", "hero").unwrap();

        let matches = dom.get_elements_by_class_name(root, "hero");
        assert!(
            matches.contains(&el),
            "el is in the indexed match set for .hero (got {matches:?})"
        );
    }
}
