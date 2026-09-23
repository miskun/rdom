//! Indexes: id → set<NodeId>, tag → set<NodeId>, class → set<NodeId>.
//!
//! Every mutation entry point calls a hook that keeps these in sync. The
//! payoff: `get_element_by_id` is a hashmap hit; tag/class getters return
//! pre-filtered candidate lists. On very large trees (10k+ nodes) this is
//! orders of magnitude faster than DFS.
//!
//! Buckets are `BTreeSet<NodeId>`: O(log n) register / unregister (a
//! `Vec` made building or tearing down n same-tag nodes O(n²)), and
//! iteration comes out in arena order, which is the deterministic order
//! the bulk getters promise.
//!
//! ## Invariants
//!
//! For every live Element node `E` with id `I`, tag `T`, classes `Cs`:
//! - `id_index[I]` contains `E` (if `I` is non-empty). When multiple
//!   elements share an id, `get_element_by_id` returns the first one in
//!   **document order** among those connected to the root (the web's
//!   answer); detached duplicates are considered only when no connected
//!   element carries the id.
//! - `tag_index[T]` contains `E`.
//! - For every `c ∈ Cs`, `class_index[c]` contains `E`.
//!
//! When `E` is freed (via `free` or `drop_subtree`), it is removed from
//! every index entry. When `E`'s attrs/classes change, affected entries
//! are updated atomically.

use std::collections::{BTreeSet, HashMap};

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

pub(crate) type Bucket = BTreeSet<NodeId>;

#[derive(Debug, Default, Clone)]
pub(crate) struct Indexes {
    pub(crate) by_id: HashMap<String, Bucket>,
    pub(crate) by_tag: HashMap<String, Bucket>,
    pub(crate) by_class: HashMap<String, Bucket>,
}

impl Indexes {
    fn push_unique(bucket: &mut Bucket, id: NodeId) {
        bucket.insert(id);
    }

    fn remove_from(map: &mut HashMap<String, Bucket>, key: &str, id: NodeId) {
        if let Some(bucket) = map.get_mut(key) {
            bucket.remove(&id);
            if bucket.is_empty() {
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

    /// The element carrying this `id` attribute, or `None`.
    ///
    /// `document.getElementById` semantics for the common case: a hash
    /// lookup, O(1) when the id is unique. When several elements share
    /// the id, the first one in **document order** among those connected
    /// to the root wins (the web's answer); that path walks the few
    /// candidates' ancestor chains.
    ///
    /// Divergence (see `DIVERGENCES.md`): the lookup is arena-wide, so
    /// a *detached* element is found when no connected element carries
    /// the id. The web only searches the document tree.
    pub fn get_element_by_id(&self, id_value: &str) -> Option<NodeId> {
        let bucket = self.indexes.by_id.get(id_value)?;
        if bucket.len() == 1 {
            return bucket.iter().next().copied();
        }
        // Duplicate ids: first in document order among the connected
        // candidates (DOM §4.5 `getElementById` walks the tree in order).
        // Candidates are few — this is the exceptional path.
        use crate::position::DocumentPosition;
        let root = self.root();
        let mut best: Option<NodeId> = None;
        for &candidate in bucket {
            let connected = self.ancestor_path(candidate).first() == Some(&root);
            if !connected {
                continue;
            }
            best = Some(match best {
                None => candidate,
                Some(b)
                    if self
                        .compare_document_position(b, candidate)
                        .contains(DocumentPosition::PRECEDING) =>
                {
                    candidate
                }
                Some(b) => b,
            });
        }
        best.or_else(|| bucket.iter().next().copied())
    }

    /// All elements with the given tag name across the entire arena, in
    /// arena order (creation order, except for recycled slots). The
    /// wildcard `"*"` returns every element in the arena.
    pub fn get_elements_by_tag_name_all(&self, tag: &str) -> Vec<NodeId> {
        if tag == "*" {
            // Merge the per-tag buckets; arena order for determinism.
            let mut out: Vec<NodeId> = self
                .indexes
                .by_tag
                .values()
                .flat_map(|b| b.iter().copied())
                .collect();
            out.sort_unstable();
            out
        } else {
            self.indexes
                .by_tag
                .get(tag)
                .map(|b| b.iter().copied().collect())
                .unwrap_or_default()
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
        let mut buckets: Vec<&Bucket> = wanted
            .iter()
            .filter_map(|w| self.indexes.by_class.get(*w))
            .collect();
        if buckets.len() != wanted.len() {
            return Vec::new(); // one class isn't indexed anywhere
        }
        buckets.sort_by_key(|b| b.len());
        let smallest = buckets[0];
        // Iterating a `BTreeSet` yields arena order already.
        smallest
            .iter()
            .copied()
            .filter(|id| buckets[1..].iter().all(|b| b.contains(id)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    /// `getElementById` with duplicate ids returns the first element in
    /// **document order**, not the first one created.
    #[test]
    fn duplicate_ids_resolve_in_document_order() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let later = dom.create_element("p");
        dom.set_attribute(later, "id", "dup").unwrap();
        let earlier = dom.create_element("p");
        dom.set_attribute(earlier, "id", "dup").unwrap();
        // `earlier` was created second but sits first in the tree.
        dom.append_child(root, later).unwrap();
        dom.insert_before(root, earlier, Some(later)).unwrap();
        assert_eq!(dom.get_element_by_id("dup"), Some(earlier));
        // Detaching the first makes the next one in document order win.
        dom.remove_child_dropping(root, earlier).unwrap();
        assert_eq!(dom.get_element_by_id("dup"), Some(later));
    }

    /// Registering and freeing many same-tag nodes keeps the index exact
    /// (the bucket is a set: no duplicates, empty bucket removed). The
    /// log-time cost is structural — `Bucket` is a `BTreeSet` — so no
    /// wall-clock assertion here (it would be flaky under load).
    #[test]
    fn tag_index_stays_exact_across_many_nodes() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let ids: Vec<_> = (0..20_000)
            .map(|_| {
                let d = dom.create_element("div");
                dom.append_child(root, d).unwrap();
                d
            })
            .collect();
        assert_eq!(dom.get_elements_by_tag_name_all("div").len(), 20_000);
        for id in ids {
            dom.remove_child_dropping(root, id).unwrap();
        }
        assert!(dom.get_elements_by_tag_name_all("div").is_empty());
        assert!(
            !dom.indexes.by_tag.contains_key("div"),
            "empty bucket is removed"
        );
        assert!(dom.validate().is_empty());
    }

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
