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
        self.fire_mutation(Mutation::AttributeChanged {
            id,
            name: key.to_string(),
            old: old_full,
            new: Some(value.to_string()),
        });
        Ok(())
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
            self.fire_mutation(Mutation::ClassChanged {
                id,
                added: vec![],
                removed: vec![class.to_string()],
            });
        } else if added {
            self.indexes.register_class(id, class);
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
}
