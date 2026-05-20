//! Tree mutation: `append_child`, `remove_child`, `insert_before`, etc.
//!
//! Every mutation maintains the doubly-linked sibling chain + first_child/
//! last_child + parent invariants. Fragment insertion unwraps the fragment's
//! children. Cycle detection via `is_ancestor`.
//!
//! These are the primitives every user-facing mutation builds on.

use crate::dom::Dom;
use crate::error::{DomError, Result};
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::observer::Mutation;

/// Position relative to a reference node, for `insert_adjacent*`.
///
/// Mirrors HTML `insertAdjacentElement`:
///   - `BeforeBegin` — as previous sibling of reference
///   - `AfterBegin`  — as first child of reference
///   - `BeforeEnd`   — as last child of reference
///   - `AfterEnd`    — as next sibling of reference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjacentPosition {
    BeforeBegin,
    AfterBegin,
    BeforeEnd,
    AfterEnd,
}

impl<Ext: 'static> Dom<Ext> {
    /// Append `child` as the last child of `parent`. If `child` is a
    /// Fragment, its children are appended and the fragment is emptied.
    ///
    /// Returns `Err(HierarchyRequest)` if `child` is an ancestor of
    /// `parent` (would create a cycle), `Err(InvalidNode)` for unknown ids.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        self.validate_insert(parent, child)?;

        // Fragment: splice its children in, leave the fragment empty.
        if matches!(
            self.get_node(child).map(|n| &n.data),
            Some(NodeData::Fragment)
        ) {
            let mut current = self.get_node(child).and_then(|n| n.first_child);
            // Clear fragment's child pointers up front; we re-link below.
            if let Some(n) = self.get_node_mut(child) {
                n.first_child = None;
                n.last_child = None;
            }
            while let Some(c) = current {
                // Capture next sibling before detaching.
                let next = self.get_node(c).and_then(|n| n.next_sibling);
                if let Some(n) = self.get_node_mut(c) {
                    n.prev_sibling = None;
                    n.next_sibling = None;
                    n.parent = None;
                }
                self.append_child(parent, c)?;
                current = next;
            }
            return Ok(());
        }

        // Detach child from current parent if any.
        self.detach_from_parent(child)?;

        // Link child under parent at the end.
        let last = self.get_node(parent).and_then(|n| n.last_child);
        self.get_node_mut(child)
            .ok_or(DomError::InvalidNode(child))?
            .parent = Some(parent);
        self.get_node_mut(child).unwrap().prev_sibling = last;
        self.get_node_mut(child).unwrap().next_sibling = None;

        match last {
            Some(prev) => {
                self.get_node_mut(prev).unwrap().next_sibling = Some(child);
            }
            None => {
                // Empty parent — also becomes first_child.
                self.get_node_mut(parent).unwrap().first_child = Some(child);
            }
        }
        self.get_node_mut(parent).unwrap().last_child = Some(child);
        self.fire_mutation(Mutation::ChildListChanged {
            parent,
            added: vec![child],
            removed: vec![],
        });
        Ok(())
    }

    /// Prepend `child` as the first child of `parent`.
    pub fn prepend_child(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        let first = self.get_node(parent).and_then(|n| n.first_child);
        self.insert_before(parent, child, first)
    }

    /// Insert `new_child` before `reference_child` within `parent`.
    /// If `reference_child` is `None`, appends at the end (matches spec
    /// behavior).
    pub fn insert_before(
        &mut self,
        parent: NodeId,
        new_child: NodeId,
        reference_child: Option<NodeId>,
    ) -> Result<()> {
        // Null reference → append.
        let Some(reference) = reference_child else {
            return self.append_child(parent, new_child);
        };

        // Reference must be an actual child of parent.
        if self.get_node(reference).and_then(|n| n.parent) != Some(parent) {
            return Err(DomError::NotFound);
        }

        self.validate_insert(parent, new_child)?;

        // Fragment unwrap — iterate children and insert each before reference.
        if matches!(
            self.get_node(new_child).map(|n| &n.data),
            Some(NodeData::Fragment)
        ) {
            let mut current = self.get_node(new_child).and_then(|n| n.first_child);
            if let Some(n) = self.get_node_mut(new_child) {
                n.first_child = None;
                n.last_child = None;
            }
            while let Some(c) = current {
                let next = self.get_node(c).and_then(|n| n.next_sibling);
                if let Some(n) = self.get_node_mut(c) {
                    n.prev_sibling = None;
                    n.next_sibling = None;
                    n.parent = None;
                }
                self.insert_before(parent, c, Some(reference))?;
                current = next;
            }
            return Ok(());
        }

        self.detach_from_parent(new_child)?;

        // Link: prev_of_reference <-> new_child <-> reference
        let before = self.get_node(reference).and_then(|n| n.prev_sibling);
        let new_node = self.get_node_mut(new_child).unwrap();
        new_node.parent = Some(parent);
        new_node.prev_sibling = before;
        new_node.next_sibling = Some(reference);

        match before {
            Some(prev) => {
                self.get_node_mut(prev).unwrap().next_sibling = Some(new_child);
            }
            None => {
                self.get_node_mut(parent).unwrap().first_child = Some(new_child);
            }
        }
        self.get_node_mut(reference).unwrap().prev_sibling = Some(new_child);
        self.fire_mutation(Mutation::ChildListChanged {
            parent,
            added: vec![new_child],
            removed: vec![],
        });
        Ok(())
    }

    /// Remove `child` from `parent`. Child is detached (parent + sibling
    /// pointers cleared) but remains in the arena as an orphan — it can
    /// be reattached elsewhere or explicitly dropped via `drop_subtree`.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        if self.get_node(child).and_then(|n| n.parent) != Some(parent) {
            return Err(DomError::NotFound);
        }
        self.detach_from_parent(child)?;
        self.fire_mutation(Mutation::ChildListChanged {
            parent,
            added: vec![],
            removed: vec![child],
        });
        Ok(())
    }

    /// Replace `old_child` with `new_child` under `parent`. `old_child`
    /// is detached and becomes an orphan.
    pub fn replace_child(
        &mut self,
        parent: NodeId,
        old_child: NodeId,
        new_child: NodeId,
    ) -> Result<()> {
        if self.get_node(old_child).and_then(|n| n.parent) != Some(parent) {
            return Err(DomError::NotFound);
        }
        self.validate_insert(parent, new_child)?;

        let next = self.get_node(old_child).and_then(|n| n.next_sibling);
        self.detach_from_parent(old_child)?;
        self.insert_before(parent, new_child, next)
    }

    /// `insertAdjacentElement(position, new_child)`. `reference` is the
    /// node relative to which we insert.
    pub fn insert_adjacent(
        &mut self,
        reference: NodeId,
        position: AdjacentPosition,
        new_child: NodeId,
    ) -> Result<()> {
        match position {
            AdjacentPosition::BeforeBegin => {
                let parent = self
                    .get_node(reference)
                    .and_then(|n| n.parent)
                    .ok_or(DomError::HierarchyRequest)?;
                self.insert_before(parent, new_child, Some(reference))
            }
            AdjacentPosition::AfterBegin => self.prepend_child(reference, new_child),
            AdjacentPosition::BeforeEnd => self.append_child(reference, new_child),
            AdjacentPosition::AfterEnd => {
                let parent = self
                    .get_node(reference)
                    .and_then(|n| n.parent)
                    .ok_or(DomError::HierarchyRequest)?;
                let after = self.get_node(reference).and_then(|n| n.next_sibling);
                self.insert_before(parent, new_child, after)
            }
        }
    }

    /// Remove all children from `parent`. They become orphans in the arena.
    /// Fires a single `ChildListChanged` record with every removed child.
    pub fn clear_children(&mut self, parent: NodeId) -> Result<()> {
        self.node_or_err(parent)?;
        let mut removed: Vec<NodeId> = Vec::new();
        while let Some(first) = self.get_node(parent).and_then(|n| n.first_child) {
            removed.push(first);
            self.detach_from_parent(first)?;
        }
        if !removed.is_empty() {
            self.fire_mutation(Mutation::ChildListChanged {
                parent,
                added: vec![],
                removed,
            });
        }
        Ok(())
    }

    /// Drop `id` and its entire subtree from the arena — frees every slot.
    /// Useful when you know you'll never reattach the nodes.
    pub fn drop_subtree(&mut self, id: NodeId) -> Result<()> {
        self.node_or_err(id)?;
        let parent = self.get_node(id).and_then(|n| n.parent);
        // Detach from parent first.
        let _ = self.detach_from_parent(id);
        // Walk + collect before freeing (can't free during traversal).
        let mut to_free = Vec::new();
        self.collect_descendants(id, &mut to_free);
        for n in to_free {
            self.free(n);
        }
        // Fire one ChildListChanged on the (now-former) parent.
        if let Some(parent) = parent {
            self.fire_mutation(Mutation::ChildListChanged {
                parent,
                added: vec![],
                removed: vec![id],
            });
        }
        Ok(())
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Validate that inserting `child` under `parent` is legal.
    /// Cycle check + id existence.
    fn validate_insert(&self, parent: NodeId, child: NodeId) -> Result<()> {
        self.node_or_err(parent)?;
        self.node_or_err(child)?;
        if self.is_ancestor(child, parent) {
            return Err(DomError::HierarchyRequest);
        }
        Ok(())
    }

    /// Detach `id` from its parent. Fixes sibling chain + first/last_child
    /// on parent. Safe no-op if the node has no parent.
    pub(crate) fn detach_from_parent(&mut self, id: NodeId) -> Result<()> {
        let node = self.node_or_err(id)?;
        let parent = node.parent;
        let prev = node.prev_sibling;
        let next = node.next_sibling;

        if let Some(prev) = prev {
            self.get_node_mut(prev).unwrap().next_sibling = next;
        }
        if let Some(next) = next {
            self.get_node_mut(next).unwrap().prev_sibling = prev;
        }
        if let Some(parent) = parent {
            if self.get_node(parent).and_then(|n| n.first_child) == Some(id) {
                self.get_node_mut(parent).unwrap().first_child = next;
            }
            if self.get_node(parent).and_then(|n| n.last_child) == Some(id) {
                self.get_node_mut(parent).unwrap().last_child = prev;
            }
        }

        let n = self.get_node_mut(id).unwrap();
        n.parent = None;
        n.prev_sibling = None;
        n.next_sibling = None;
        Ok(())
    }

    /// Depth-first descendants including `root`. Used by `drop_subtree`.
    fn collect_descendants(&self, root: NodeId, out: &mut Vec<NodeId>) {
        out.push(root);
        let mut child = self.get_node(root).and_then(|n| n.first_child);
        while let Some(c) = child {
            self.collect_descendants(c, out);
            child = self.get_node(c).and_then(|n| n.next_sibling);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Dom, NodeId, NodeId, NodeId) {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        (dom, a, b, c)
    }

    // ── append_child ─────────────────────────────────────────────────

    #[test]
    fn append_to_empty_parent() {
        let (mut dom, a, _, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        assert_eq!(dom.get_node(root).unwrap().first_child, Some(a));
        assert_eq!(dom.get_node(root).unwrap().last_child, Some(a));
        assert_eq!(dom.get_node(a).unwrap().parent, Some(root));
        assert!(dom.get_node(a).unwrap().prev_sibling.is_none());
        assert!(dom.get_node(a).unwrap().next_sibling.is_none());
    }

    #[test]
    fn append_multiple_maintains_sibling_chain() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();

        assert_eq!(dom.get_node(root).unwrap().first_child, Some(a));
        assert_eq!(dom.get_node(root).unwrap().last_child, Some(c));
        assert_eq!(dom.get_node(a).unwrap().next_sibling, Some(b));
        assert_eq!(dom.get_node(b).unwrap().prev_sibling, Some(a));
        assert_eq!(dom.get_node(b).unwrap().next_sibling, Some(c));
        assert_eq!(dom.get_node(c).unwrap().prev_sibling, Some(b));
    }

    #[test]
    fn append_moves_node_from_old_parent() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(a, b).unwrap();
        dom.append_child(root, b).unwrap(); // re-parent b
        assert_eq!(dom.get_node(b).unwrap().parent, Some(root));
        assert!(dom.get_node(a).unwrap().first_child.is_none());
        assert!(dom.get_node(a).unwrap().last_child.is_none());
    }

    #[test]
    fn append_rejects_cycle() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(a, b).unwrap();
        // Try to append a under b — cycle.
        assert!(matches!(
            dom.append_child(b, a).unwrap_err(),
            DomError::HierarchyRequest
        ));
    }

    #[test]
    fn append_rejects_invalid_parent() {
        let mut dom: Dom = Dom::new();
        // A NodeId that was never allocated — guaranteed invalid.
        let ghost = NodeId::from_index(999);
        let child = dom.create_element("child");
        assert!(matches!(
            dom.append_child(ghost, child).unwrap_err(),
            DomError::InvalidNode(_)
        ));
    }

    // ── insert_before ─────────────────────────────────────────────────

    #[test]
    fn insert_before_first_becomes_new_first() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.insert_before(root, b, Some(a)).unwrap();
        assert_eq!(dom.get_node(root).unwrap().first_child, Some(b));
        assert_eq!(dom.get_node(root).unwrap().last_child, Some(a));
        assert_eq!(dom.get_node(b).unwrap().next_sibling, Some(a));
        assert_eq!(dom.get_node(a).unwrap().prev_sibling, Some(b));
    }

    #[test]
    fn insert_before_middle_updates_chain() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, c).unwrap();
        dom.insert_before(root, b, Some(c)).unwrap();
        // Order should be a, b, c.
        let names: Vec<_> = iter_children(&dom, root)
            .map(|id| dom.get_node(id).unwrap().tag_name().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn insert_before_null_appends() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.insert_before(root, b, None).unwrap();
        assert_eq!(dom.get_node(root).unwrap().last_child, Some(b));
    }

    #[test]
    fn insert_before_rejects_non_child_reference() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        // b is not a child of root.
        assert!(matches!(
            dom.insert_before(root, a, Some(b)).unwrap_err(),
            DomError::NotFound
        ));
    }

    // ── remove_child ─────────────────────────────────────────────────

    #[test]
    fn remove_child_detaches_but_keeps_in_arena() {
        let (mut dom, a, _, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.remove_child(root, a).unwrap();
        assert!(dom.get_node(root).unwrap().first_child.is_none());
        assert!(dom.get_node(a).unwrap().parent.is_none());
        assert!(dom.contains(a)); // still in arena as orphan
    }

    #[test]
    fn remove_middle_child_fixes_siblings() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();
        dom.remove_child(root, b).unwrap();
        assert_eq!(dom.get_node(a).unwrap().next_sibling, Some(c));
        assert_eq!(dom.get_node(c).unwrap().prev_sibling, Some(a));
    }

    #[test]
    fn remove_nonchild_errors() {
        let (mut dom, a, _, _) = sample();
        let root = dom.root();
        assert!(matches!(
            dom.remove_child(root, a).unwrap_err(),
            DomError::NotFound
        ));
    }

    // ── replace_child ────────────────────────────────────────────────

    #[test]
    fn replace_child_preserves_position() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();

        let d = dom.create_element("d");
        dom.replace_child(root, b, d).unwrap();
        let names: Vec<_> = iter_children(&dom, root)
            .map(|id| dom.get_node(id).unwrap().tag_name().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["a", "d", "c"]);
    }

    // ── Fragment unwrap ──────────────────────────────────────────────

    #[test]
    fn fragment_unwraps_on_append() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let frag = dom.create_document_fragment();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(frag, a).unwrap();
        dom.append_child(frag, b).unwrap();

        dom.append_child(root, frag).unwrap();

        // a and b are now direct children of root; frag is empty.
        assert_eq!(dom.get_node(root).unwrap().first_child, Some(a));
        assert_eq!(dom.get_node(root).unwrap().last_child, Some(b));
        assert!(dom.get_node(frag).unwrap().first_child.is_none());
    }

    #[test]
    fn fragment_unwraps_on_insert_before() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let existing = dom.create_element("existing");
        dom.append_child(root, existing).unwrap();

        let frag = dom.create_document_fragment();
        let x = dom.create_element("x");
        let y = dom.create_element("y");
        dom.append_child(frag, x).unwrap();
        dom.append_child(frag, y).unwrap();

        dom.insert_before(root, frag, Some(existing)).unwrap();

        let names: Vec<_> = iter_children(&dom, root)
            .map(|id| dom.get_node(id).unwrap().tag_name().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["x", "y", "existing"]);
    }

    // ── insert_adjacent ──────────────────────────────────────────────

    #[test]
    fn insert_adjacent_before_begin() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.insert_adjacent(a, AdjacentPosition::BeforeBegin, b)
            .unwrap();
        assert_eq!(dom.get_node(root).unwrap().first_child, Some(b));
        assert_eq!(dom.get_node(b).unwrap().next_sibling, Some(a));
    }

    #[test]
    fn insert_adjacent_after_end() {
        let (mut dom, a, b, _) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.insert_adjacent(a, AdjacentPosition::AfterEnd, b)
            .unwrap();
        assert_eq!(dom.get_node(a).unwrap().next_sibling, Some(b));
        assert_eq!(dom.get_node(root).unwrap().last_child, Some(b));
    }

    #[test]
    fn insert_adjacent_after_begin_prepends() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        // c becomes new first child of root.
        dom.insert_adjacent(root, AdjacentPosition::AfterBegin, c)
            .unwrap();
        assert_eq!(dom.get_node(root).unwrap().first_child, Some(c));
    }

    // ── clear + drop ─────────────────────────────────────────────────

    #[test]
    fn clear_children_detaches_all() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();

        dom.clear_children(root).unwrap();
        assert!(dom.get_node(root).unwrap().first_child.is_none());
        // Children are orphans but still in arena.
        assert!(dom.contains(a));
        assert!(dom.contains(b));
        assert!(dom.contains(c));
        assert!(dom.get_node(a).unwrap().parent.is_none());
    }

    #[test]
    fn drop_subtree_frees_everything() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(root, a).unwrap();
        dom.append_child(a, b).unwrap();
        dom.append_child(b, c).unwrap();

        dom.drop_subtree(a).unwrap();
        assert!(!dom.contains(a));
        assert!(!dom.contains(b));
        assert!(!dom.contains(c));
        assert!(dom.get_node(root).unwrap().first_child.is_none());
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn iter_children(dom: &Dom, parent: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let mut cur = dom.get_node(parent).and_then(|n| n.first_child);
        std::iter::from_fn(move || {
            let c = cur?;
            cur = dom.get_node(c).and_then(|n| n.next_sibling);
            Some(c)
        })
    }
}
