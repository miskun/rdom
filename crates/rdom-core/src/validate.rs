//! Debug-only arena validator.
//!
//! `Dom::validate()` walks every occupied slot, checks every invariant from
//! the design RFC, and returns the list of violations (not just a bool, so
//! failures are actionable). Used by fuzz / property tests in Phase 4; the
//! current (Phase 1) test suite also calls it at the end of non-trivial
//! scenarios as a second check.
//!
//! Never enabled in release builds — it's O(n) and walks every pointer.

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantViolation {
    /// A `parent` pointer on some node doesn't appear in that parent's
    /// children (chain from first_child via next_sibling).
    ParentNotContainingChild { parent: NodeId, child: NodeId },
    /// Sibling chain is not doubly linked: `a.next_sibling == Some(b)` but
    /// `b.prev_sibling != Some(a)`.
    SiblingChainBroken { a: NodeId, b: NodeId },
    /// Parent's `first_child` doesn't point to a node whose `prev_sibling` is None.
    FirstChildNotLeader { parent: NodeId },
    /// Parent's `last_child` doesn't point to a node whose `next_sibling` is None.
    LastChildNotTail { parent: NodeId },
    /// A node appears in a parent's child chain but its own `parent` is wrong.
    WrongParentPointer { child: NodeId, expected: NodeId },
    /// Freed slot referenced from a live node's pointer.
    DanglingPointer { from: NodeId, to: NodeId },
    /// An index entry references a node that no longer has the indexed
    /// attribute (id/tag/class). Index out-of-sync with node state.
    IndexOrphan {
        kind: &'static str,
        key: String,
        id: NodeId,
    },
    /// A live Element has an attribute/class that should be indexed but
    /// is not present in the index.
    IndexMissing {
        kind: &'static str,
        key: String,
        id: NodeId,
    },
}

impl<Ext> Dom<Ext> {
    /// Walk the arena, check every invariant, return all violations.
    /// Intended for tests and debug builds. Release builds of callers can
    /// simply skip this call.
    pub fn validate(&self) -> Vec<InvariantViolation> {
        let mut out = Vec::new();

        for (idx, slot) in self.nodes.iter().enumerate() {
            let Some(node) = slot else { continue };
            let id = NodeId::from_index(idx);

            // Pointer reachability — freed slots not referenced.
            for (label, target) in [
                ("parent", node.parent),
                ("first_child", node.first_child),
                ("last_child", node.last_child),
                ("prev_sibling", node.prev_sibling),
                ("next_sibling", node.next_sibling),
            ] {
                if let Some(t) = target
                    && self.get_node(t).is_none()
                {
                    let _ = label; // informational
                    out.push(InvariantViolation::DanglingPointer { from: id, to: t });
                }
            }

            // Sibling doubly-linked.
            if let Some(next) = node.next_sibling
                && let Some(next_node) = self.get_node(next)
                && next_node.prev_sibling != Some(id)
            {
                out.push(InvariantViolation::SiblingChainBroken { a: id, b: next });
            }
            if let Some(prev) = node.prev_sibling
                && let Some(prev_node) = self.get_node(prev)
                && prev_node.next_sibling != Some(id)
            {
                out.push(InvariantViolation::SiblingChainBroken { a: prev, b: id });
            }

            // If this node has a parent, parent's child chain must contain it.
            if let Some(parent) = node.parent {
                let mut found = false;
                let mut cur = self.get_node(parent).and_then(|n| n.first_child);
                while let Some(c) = cur {
                    if c == id {
                        found = true;
                        break;
                    }
                    cur = self.get_node(c).and_then(|n| n.next_sibling);
                }
                if !found {
                    out.push(InvariantViolation::ParentNotContainingChild { parent, child: id });
                }
            }

            // First-child: its prev_sibling should be None.
            if let Some(first) = node.first_child
                && let Some(fn_) = self.get_node(first)
            {
                if fn_.prev_sibling.is_some() {
                    out.push(InvariantViolation::FirstChildNotLeader { parent: id });
                }
                if fn_.parent != Some(id) {
                    out.push(InvariantViolation::WrongParentPointer {
                        child: first,
                        expected: id,
                    });
                }
            }
            if let Some(last) = node.last_child
                && let Some(ln) = self.get_node(last)
            {
                if ln.next_sibling.is_some() {
                    out.push(InvariantViolation::LastChildNotTail { parent: id });
                }
                if ln.parent != Some(id) {
                    out.push(InvariantViolation::WrongParentPointer {
                        child: last,
                        expected: id,
                    });
                }
            }

            // Index invariants: for every live Element, its tag / id / classes
            // must be present in the corresponding index.
            if let NodeData::Element {
                tag,
                attrs,
                classes,
                ..
            } = &node.data
            {
                if !self
                    .indexes
                    .by_tag
                    .get(tag)
                    .is_some_and(|v| v.contains(&id))
                {
                    out.push(InvariantViolation::IndexMissing {
                        kind: "tag",
                        key: tag.clone(),
                        id,
                    });
                }
                if let Some(id_attr) = attrs.get("id")
                    && !id_attr.is_empty()
                    && !self
                        .indexes
                        .by_id
                        .get(id_attr)
                        .is_some_and(|v| v.contains(&id))
                {
                    out.push(InvariantViolation::IndexMissing {
                        kind: "id",
                        key: id_attr.clone(),
                        id,
                    });
                }
                for c in classes {
                    if !self
                        .indexes
                        .by_class
                        .get(c)
                        .is_some_and(|v| v.contains(&id))
                    {
                        out.push(InvariantViolation::IndexMissing {
                            kind: "class",
                            key: c.clone(),
                            id,
                        });
                    }
                }
            }
        }

        // Reverse direction: every index entry must reference a live
        // Element that still carries the attribute/class/tag.
        for (tag, ids) in &self.indexes.by_tag {
            for &iid in ids {
                match self.get_node(iid).map(|n| &n.data) {
                    Some(NodeData::Element { tag: t, .. }) if t == tag => {}
                    _ => out.push(InvariantViolation::IndexOrphan {
                        kind: "tag",
                        key: tag.clone(),
                        id: iid,
                    }),
                }
            }
        }
        for (key, ids) in &self.indexes.by_id {
            for &iid in ids {
                let has_id = matches!(
                    self.get_node(iid).map(|n| &n.data),
                    Some(NodeData::Element { attrs, .. }) if attrs.get("id") == Some(key)
                );
                if !has_id {
                    out.push(InvariantViolation::IndexOrphan {
                        kind: "id",
                        key: key.clone(),
                        id: iid,
                    });
                }
            }
        }
        for (cls, ids) in &self.indexes.by_class {
            for &iid in ids {
                let has_cls = matches!(
                    self.get_node(iid).map(|n| &n.data),
                    Some(NodeData::Element { classes, .. }) if classes.contains(cls)
                );
                if !has_cls {
                    out.push(InvariantViolation::IndexOrphan {
                        kind: "class",
                        key: cls.clone(),
                        id: iid,
                    });
                }
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    #[test]
    fn empty_dom_validates() {
        let dom: Dom = Dom::new();
        assert!(dom.validate().is_empty());
    }

    #[test]
    fn simple_tree_validates() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(a, c).unwrap();
        assert!(dom.validate().is_empty());
    }

    #[test]
    fn deep_tree_validates() {
        let mut dom: Dom = Dom::new();
        let mut cur = dom.root();
        for _ in 0..100 {
            let el = dom.create_element("div");
            dom.append_child(cur, el).unwrap();
            cur = el;
        }
        assert!(dom.validate().is_empty());
    }

    #[test]
    fn mutations_preserve_invariants() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();

        dom.remove_child(root, b).unwrap();
        assert!(dom.validate().is_empty());

        let d = dom.create_element("d");
        dom.insert_before(root, d, Some(c)).unwrap();
        assert!(dom.validate().is_empty());

        dom.replace_child(root, a, b).unwrap();
        assert!(dom.validate().is_empty());
    }
}
