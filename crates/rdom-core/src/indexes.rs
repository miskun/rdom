//! O(1) indexes: id → NodeId, tag → Vec<NodeId>, class → Vec<NodeId>.
//!
//! Every mutation entry point calls a hook that keeps these in sync. The
//! payoff: `get_element_by_id` is a hashmap hit; tag/class getters return
//! pre-filtered candidate lists. On very large trees (10k+ nodes) this is
//! orders of magnitude faster than DFS.
//!
//! ## Invariants
//!
//! For every live Element node `E` with id `I`, tag `T`, classes `Cs`:
//! - `id_index[I]` contains `E` (if `I` is non-empty). When multiple
//!   elements share an id, `id_index[I]` stores each in insertion order;
//!   `get_element_by_id` returns the **first-inserted** element (first
//!   come, first served — diverges slightly from browser's "first in
//!   document order" but stable and easy to reason about).
//! - `tag_index[T]` contains `E`.
//! - For every `c ∈ Cs`, `class_index[c]` contains `E`.
//!
//! When `E` is freed (via `free` or `drop_subtree`), it is removed from
//! every index entry. When `E`'s attrs/classes change, affected entries
//! are updated atomically.

use std::collections::HashMap;

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

#[derive(Debug, Default, Clone)]
pub(crate) struct Indexes {
    pub(crate) by_id: HashMap<String, Vec<NodeId>>,
    pub(crate) by_tag: HashMap<String, Vec<NodeId>>,
    pub(crate) by_class: HashMap<String, Vec<NodeId>>,
}

impl Indexes {
    fn push_unique(vec: &mut Vec<NodeId>, id: NodeId) {
        if !vec.contains(&id) {
            vec.push(id);
        }
    }

    fn remove_from(map: &mut HashMap<String, Vec<NodeId>>, key: &str, id: NodeId) {
        if let Some(vec) = map.get_mut(key) {
            vec.retain(|&x| x != id);
            if vec.is_empty() {
                map.remove(key);
            }
        }
    }

    pub(crate) fn register_id(&mut self, id: NodeId, id_value: &str) {
        if id_value.is_empty() {
            return;
        }
        Self::push_unique(self.by_id.entry(id_value.to_string()).or_default(), id);
    }

    pub(crate) fn unregister_id(&mut self, id: NodeId, id_value: &str) {
        if id_value.is_empty() {
            return;
        }
        Self::remove_from(&mut self.by_id, id_value, id);
    }

    pub(crate) fn register_tag(&mut self, id: NodeId, tag: &str) {
        Self::push_unique(self.by_tag.entry(tag.to_string()).or_default(), id);
    }

    pub(crate) fn unregister_tag(&mut self, id: NodeId, tag: &str) {
        Self::remove_from(&mut self.by_tag, tag, id);
    }

    pub(crate) fn register_class(&mut self, id: NodeId, class: &str) {
        Self::push_unique(self.by_class.entry(class.to_string()).or_default(), id);
    }

    pub(crate) fn unregister_class(&mut self, id: NodeId, class: &str) {
        Self::remove_from(&mut self.by_class, class, id);
    }
}

// ─── Hook helpers (called from dom.rs / attrs.rs / tree.rs) ─────────

impl<Ext> Dom<Ext> {
    /// Register a newly-allocated Element's tag, id, classes. Non-Element
    /// nodes are ignored. Called from `alloc` after the node is inserted.
    pub(crate) fn hook_register(&mut self, id: NodeId) {
        let Some(node) = self.get_node(id) else {
            return;
        };
        let (tag, id_attr, classes) = match &node.data {
            NodeData::Element {
                tag,
                attrs,
                classes,
                ..
            } => {
                let tag = tag.clone();
                let id_attr = attrs.get("id").cloned();
                let classes: Vec<String> = classes.iter().cloned().collect();
                (tag, id_attr, classes)
            }
            _ => return,
        };
        self.indexes.register_tag(id, &tag);
        if let Some(v) = id_attr {
            self.indexes.register_id(id, &v);
        }
        for c in classes {
            self.indexes.register_class(id, &c);
        }
    }

    /// Remove an Element from every index. Non-Element nodes are ignored.
    /// Called from `free` before the slot is wiped.
    pub(crate) fn hook_unregister(&mut self, id: NodeId) {
        let Some(node) = self.get_node(id) else {
            return;
        };
        let (tag, id_attr, classes) = match &node.data {
            NodeData::Element {
                tag,
                attrs,
                classes,
                ..
            } => {
                let tag = tag.clone();
                let id_attr = attrs.get("id").cloned();
                let classes: Vec<String> = classes.iter().cloned().collect();
                (tag, id_attr, classes)
            }
            _ => return,
        };
        self.indexes.unregister_tag(id, &tag);
        if let Some(v) = id_attr {
            self.indexes.unregister_id(id, &v);
        }
        for c in classes {
            self.indexes.unregister_class(id, &c);
        }
    }

    // ─── Public arena-wide lookups ───────────────────────────────────

    /// Return the first element in the arena with the given `id` attribute.
    /// O(1). Returns `None` if no element matches.
    ///
    /// Browser semantics: `document.getElementById`. In the real DOM this
    /// returns the first element in *document order*. Here it returns the
    /// first element that had the id *set* on it, which is almost always
    /// the same node unless the tree is being mutated rapidly.
    pub fn get_element_by_id(&self, id_value: &str) -> Option<NodeId> {
        self.indexes
            .by_id
            .get(id_value)
            .and_then(|v| v.first())
            .copied()
    }

    /// All elements with the given tag name across the entire arena, in
    /// registration order (≈ creation order). The wildcard `"*"` returns
    /// every element in the arena.
    pub fn get_elements_by_tag_name_all(&self, tag: &str) -> Vec<NodeId> {
        if tag == "*" {
            let mut out = Vec::new();
            for v in self.indexes.by_tag.values() {
                out.extend(v.iter().copied());
            }
            // Arena-order for determinism.
            out.sort_by_key(|id| id.index());
            out
        } else {
            self.indexes.by_tag.get(tag).cloned().unwrap_or_default()
        }
    }

    /// All elements whose classList contains every class in the whitespace-
    /// separated `names` string, across the entire arena. Empty `names`
    /// returns every element.
    pub fn get_elements_by_class_name_all(&self, names: &str) -> Vec<NodeId> {
        let wanted: Vec<&str> = names.split_ascii_whitespace().collect();
        if wanted.is_empty() {
            return self.get_elements_by_tag_name_all("*");
        }
        // Start with the smallest class bucket to minimize the scan.
        let mut buckets: Vec<&Vec<NodeId>> = wanted
            .iter()
            .filter_map(|w| self.indexes.by_class.get(*w))
            .collect();
        if buckets.len() != wanted.len() {
            return Vec::new(); // one class isn't indexed anywhere
        }
        buckets.sort_by_key(|v| v.len());
        let smallest = buckets[0];
        let mut out: Vec<NodeId> = smallest
            .iter()
            .copied()
            .filter(|id| buckets[1..].iter().all(|b| b.contains(id)))
            .collect();
        out.sort_by_key(|id| id.index());
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    #[test]
    fn id_index_populated_on_set_attribute() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "main").unwrap();
        assert_eq!(dom.get_element_by_id("main"), Some(el));
    }

    #[test]
    fn id_index_unregisters_on_removal() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "main").unwrap();
        dom.remove_attribute(el, "id").unwrap();
        assert_eq!(dom.get_element_by_id("main"), None);
    }

    #[test]
    fn id_index_updates_on_reassignment() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "old").unwrap();
        dom.set_attribute(el, "id", "new").unwrap();
        assert_eq!(dom.get_element_by_id("old"), None);
        assert_eq!(dom.get_element_by_id("new"), Some(el));
    }

    #[test]
    fn id_index_survives_node_drop() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "id", "main").unwrap();
        let root = dom.root();
        dom.append_child(root, el).unwrap();
        dom.drop_subtree(el).unwrap();
        assert_eq!(dom.get_element_by_id("main"), None);
    }

    #[test]
    fn tag_index_finds_elements() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let b = dom.create_element("div");
        let c = dom.create_element("span");
        let divs = dom.get_elements_by_tag_name_all("div");
        assert!(divs.contains(&a));
        assert!(divs.contains(&b));
        assert!(!divs.contains(&c));
    }

    #[test]
    fn tag_index_wildcard_returns_all() {
        let mut dom: Dom = Dom::new();
        let _ = dom.create_element("a");
        let _ = dom.create_element("b");
        // root is a Fragment, not an Element — not in tag index.
        let all = dom.get_elements_by_tag_name_all("*");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn tag_index_clears_on_free() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        assert_eq!(dom.get_elements_by_tag_name_all("a"), vec![a]);
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.drop_subtree(a).unwrap();
        assert!(dom.get_elements_by_tag_name_all("a").is_empty());
    }

    #[test]
    fn class_index_basic() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "foo").unwrap();
        assert_eq!(dom.get_elements_by_class_name_all("foo"), vec![el]);
    }

    #[test]
    fn class_index_intersection() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        dom.add_class(a, "x").unwrap();
        dom.add_class(a, "y").unwrap();
        let b = dom.create_element("div");
        dom.add_class(b, "x").unwrap(); // only x
        let c = dom.create_element("div");
        dom.add_class(c, "y").unwrap(); // only y

        assert_eq!(dom.get_elements_by_class_name_all("x y"), vec![a]);
        let xs = dom.get_elements_by_class_name_all("x");
        assert!(xs.contains(&a) && xs.contains(&b));
    }

    #[test]
    fn class_index_handles_toggle_and_replace() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "old").unwrap();
        assert_eq!(dom.get_elements_by_class_name_all("old"), vec![el]);

        dom.replace_class(el, "old", "new").unwrap();
        assert!(dom.get_elements_by_class_name_all("old").is_empty());
        assert_eq!(dom.get_elements_by_class_name_all("new"), vec![el]);

        dom.toggle_class(el, "new").unwrap(); // removes
        assert!(dom.get_elements_by_class_name_all("new").is_empty());
    }

    #[test]
    fn id_attribute_via_set_id_sugar_indexed() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_id(el, "hero").unwrap();
        assert_eq!(dom.get_element_by_id("hero"), Some(el));
    }

    #[test]
    fn freed_slot_reuse_does_not_leak_old_index_entries() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        dom.set_attribute(a, "id", "x").unwrap();
        dom.add_class(a, "c").unwrap();
        dom.free(a); // drops without structural cleanup — still must unindex

        // Reuse the slot with a new element that has different identity.
        let b = dom.create_element("span");
        assert_eq!(dom.get_element_by_id("x"), None);
        assert!(dom.get_elements_by_class_name_all("c").is_empty());
        assert_eq!(dom.get_elements_by_tag_name_all("span"), vec![b]);
        assert!(dom.get_elements_by_tag_name_all("div").is_empty());
    }

    #[test]
    fn duplicate_ids_first_wins() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        dom.set_attribute(a, "id", "dup").unwrap();
        let b = dom.create_element("span");
        dom.set_attribute(b, "id", "dup").unwrap();
        assert_eq!(dom.get_element_by_id("dup"), Some(a));
        // Remove the first — second takes over.
        dom.remove_attribute(a, "id").unwrap();
        assert_eq!(dom.get_element_by_id("dup"), Some(b));
    }
}
