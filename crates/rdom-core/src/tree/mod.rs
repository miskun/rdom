//! Tree mutation: `append_child`, `remove_child`, `insert_before`, etc.
//!
//! ## Observer record ordering
//!
//! Every detach path (`remove_child`, `replace_child`, `replace_with`,
//! `clear_children`, `drop_subtree`) routes through
//! `detach_from_parent`, which clears any interaction state
//! (`focused` / `hovered` / `pointer_capture` / `selection`) that
//! pointed into the subtree being removed. The
//! `InteractionChanged` / `SelectionChanged` records for those
//! cleanups fire *during* the detach, before the caller's
//! `ChildListChanged` record. Observers therefore see "effect"
//! records before the "cause" record. The alternative —
//! post-detach purge in every caller — would centralize the
//! ordering at the cost of forgetting it on a future tree-mutation
//! API. Document, don't decentralize.
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

mod detach;

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
    /// A `child` that already has a parent is moved: it is removed from
    /// that parent first, with its own `ChildListChanged` record, then
    /// inserted (DOM §4.2.3 "insert" → "adopt" → "remove") — an observer
    /// sees the old parent lose it before the new parent gains it, and
    /// everything keyed on removal (focus, the top layer, live ranges, a
    /// renderer's animations) treats the move as a removal.
    ///
    /// Returns `Err(HierarchyRequest)` if `child` is an ancestor of
    /// `parent` (would create a cycle), `Err(InvalidNode)` for unknown ids.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        self.validate_insert(parent, child)?;
        for node in self.take_for_insertion(child)? {
            self.link(parent, node, None);
            self.fire_mutation(Mutation::ChildListChanged {
                parent,
                added: vec![node],
                removed: vec![],
            });
        }
        Ok(())
    }

    /// Prepend `child` as the first child of `parent`.
    pub fn prepend_child(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        let first = self.get_node(parent).and_then(|n| n.first_child);
        self.insert_before(parent, child, first)
    }

    /// Insert `new_child` before `reference_child` within `parent`.
    /// If `reference_child` is `None`, appends at the end (matches spec
    /// behavior); if it is `new_child` itself, `new_child`'s next
    /// sibling is the reference (DOM §4.2.3 "pre-insert" step 3). A
    /// `new_child` with a parent is moved, as
    /// [`append_child`](Self::append_child) describes.
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
        if reference == new_child {
            let next = self.get_node(reference).and_then(|n| n.next_sibling);
            return self.insert_before(parent, new_child, next);
        }

        self.validate_insert(parent, new_child)?;
        for node in self.take_for_insertion(new_child)? {
            self.link(parent, node, Some(reference));
            self.fire_mutation(Mutation::ChildListChanged {
                parent,
                added: vec![node],
                removed: vec![],
            });
        }
        Ok(())
    }

    /// Remove `child` from `parent`. Child is detached (parent + sibling
    /// pointers cleared) but **remains in the arena as an orphan** — it
    /// can be reattached elsewhere, or explicitly freed via
    /// [`drop_subtree`](Self::drop_subtree).
    ///
    /// The arena has no GC: a detached node is never reclaimed on its
    /// own. Code that removes nodes it will **not** reattach — especially
    /// high-churn UIs (a virtualized list/table re-materializing rows on
    /// every scroll) — must free them, or arena slots leak. Use
    /// [`remove_child_dropping`](Self::remove_child_dropping) to remove
    /// and free in one call.
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
    ///
    /// DOM §4.2.3 "replace": `new_child` is first removed from its own
    /// parent (its own record — a Fragment is emptied with one record
    /// for it), then one `ChildListChanged` record for `parent` names
    /// what arrived and `old_child` as removed. Replacing a child with
    /// its next sibling leaves that sibling in the child's place.
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

        let next_of = |dom: &Self, id: NodeId| dom.get_node(id).and_then(|n| n.next_sibling);
        let mut reference = next_of(self, old_child);
        if reference == Some(new_child) {
            reference = next_of(self, new_child);
        }
        let added = self.take_for_insertion(new_child)?;
        // `new_child` may have been `old_child` itself, removed above.
        let removed = if self.get_node(old_child).and_then(|n| n.parent) == Some(parent) {
            self.detach_from_parent(old_child)?;
            vec![old_child]
        } else {
            Vec::new()
        };
        for &node in &added {
            self.link(parent, node, reference);
        }
        if !added.is_empty() || !removed.is_empty() {
            self.fire_mutation(Mutation::ChildListChanged {
                parent,
                added,
                removed,
            });
        }
        Ok(())
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

    /// Remove all children from `parent`. They become **orphans in the
    /// arena** (not freed — see [`remove_child`](Self::remove_child) on
    /// the no-GC contract). Fires a single `ChildListChanged` record with
    /// every removed child. To remove and free in one call, use
    /// [`clear_children_dropping`](Self::clear_children_dropping).
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
    ///
    /// The root cannot be dropped (`HierarchyRequest`): `Dom::root` must
    /// stay live for the lifetime of the arena. Use
    /// [`clear_children_dropping`](Self::clear_children_dropping) to
    /// empty it.
    pub fn drop_subtree(&mut self, id: NodeId) -> Result<()> {
        self.node_or_err(id)?;
        if id == self.root {
            return Err(DomError::HierarchyRequest);
        }
        let parent = self.get_node(id).and_then(|n| n.parent);
        // Snapshot the subtree to free WHILE it's still alive and before
        // anything can panic.
        let mut to_free = Vec::new();
        self.collect_descendants(id, &mut to_free);
        // Detach, then fire the mutation BEFORE freeing, so observers (the
        // dirty tracker, implicit blur/focusout-on-detach) can still read
        // the removed nodes in their callback — same contract as
        // `remove_child`, and what the MutationObserver spec requires
        // (`removedNodes` are inspectable). Both steps fire records
        // (detach purges focus / hover / selection inside the subtree); a
        // panicking observer is re-raised by `fire_mutation`, so the whole
        // window runs under one guard and the slots are reclaimed on the
        // way out (`CORE-DROP-PANIC-LEAK-1`).
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = self.detach_from_parent(id);
            if let Some(parent) = parent {
                self.fire_mutation(Mutation::ChildListChanged {
                    parent,
                    added: vec![],
                    removed: vec![id],
                });
            }
        }));
        self.free_if_detached(id, to_free);
        if let Err(payload) = outcome {
            std::panic::resume_unwind(payload);
        }
        Ok(())
    }

    /// Reclaim `to_free` (a subtree rooted at `root`) — unless `root` is
    /// still attached, which happens when an observer panicked in the
    /// `PreDetach` window, before the unlink: then the tree is intact and
    /// freeing it would leave the parent pointing at dead slots.
    fn free_if_detached(&mut self, root: NodeId, to_free: Vec<NodeId>) {
        if self.get_node(root).is_some_and(|n| n.parent.is_some()) {
            return;
        }
        for n in to_free {
            self.free(n);
        }
    }

    /// Remove `child` from `parent` **and free** its subtree from the
    /// arena (the non-leaking [`remove_child`](Self::remove_child)).
    /// Use when you won't reattach the removed node. Fires the same
    /// single `ChildListChanged` record `remove_child` does (the
    /// follow-up free runs on the already-detached orphan, so it adds no
    /// extra record). Observers still see the removed node alive in their
    /// synchronous callback — it's freed only after dispatch returns.
    pub fn remove_child_dropping(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        if self.get_node(child).and_then(|n| n.parent) != Some(parent) {
            return Err(DomError::NotFound);
        }
        let mut to_free = Vec::new();
        self.collect_descendants(child, &mut to_free);
        // Same guard as `drop_subtree`: the record fires inside
        // `remove_child`; a panicking observer must not leak the orphan.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.remove_child(parent, child)
        }));
        self.free_if_detached(child, to_free);
        match outcome {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    /// Remove all children from `parent` **and free** their subtrees
    /// from the arena (the non-leaking [`clear_children`](Self::clear_children)).
    /// Fires the same single `ChildListChanged` record `clear_children`
    /// does; the frees run on the already-detached orphans.
    pub fn clear_children_dropping(&mut self, parent: NodeId) -> Result<()> {
        self.node_or_err(parent)?;
        // Snapshot each child's subtree before detaching anything.
        let mut subtrees: Vec<(NodeId, Vec<NodeId>)> = Vec::new();
        let mut cur = self.get_node(parent).and_then(|n| n.first_child);
        while let Some(id) = cur {
            let mut to_free = Vec::new();
            self.collect_descendants(id, &mut to_free);
            subtrees.push((id, to_free));
            cur = self.get_node(id).and_then(|n| n.next_sibling);
        }
        // Detaches all, one batch record; guarded like `drop_subtree`.
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.clear_children(parent)));
        for (child, to_free) in subtrees {
            self.free_if_detached(child, to_free);
        }
        match outcome {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// The nodes inserting `node` inserts, each out of its old parent
    /// (DOM §4.2.3 "insert" steps 4 and 7.1): a Fragment's children,
    /// removed from it with one record for the fragment (step 4 removes
    /// them with observers suppressed and queues that one record); else
    /// `node`, removed from its parent with that parent's record if it
    /// has one ("adopt" step 2 runs "remove").
    fn take_for_insertion(&mut self, node: NodeId) -> Result<Vec<NodeId>> {
        let data = &self.node_or_err(node)?.data;
        if matches!(data, NodeData::Fragment) {
            let mut kids = Vec::new();
            while let Some(c) = self.get_node(node).and_then(|n| n.first_child) {
                self.detach_from_parent(c)?;
                kids.push(c);
            }
            if !kids.is_empty() {
                self.fire_mutation(Mutation::ChildListChanged {
                    parent: node,
                    added: vec![],
                    removed: kids.clone(),
                });
            }
            return Ok(kids);
        }
        if let Some(old) = self.get_node(node).and_then(|n| n.parent) {
            self.detach_from_parent(node)?;
            self.fire_mutation(Mutation::ChildListChanged {
                parent: old,
                added: vec![],
                removed: vec![node],
            });
        }
        Ok(vec![node])
    }

    /// Link the parentless `child` under `parent`, before `reference`
    /// (a child of `parent`) or last — the structural half of an
    /// insertion; the caller fires its record.
    fn link(&mut self, parent: NodeId, child: NodeId, reference: Option<NodeId>) {
        let before = match reference {
            Some(r) => self.get_node(r).and_then(|n| n.prev_sibling),
            None => self.get_node(parent).and_then(|n| n.last_child),
        };
        let node = self.get_node_mut(child).expect("validated");
        node.parent = Some(parent);
        node.prev_sibling = before;
        node.next_sibling = reference;
        match before {
            Some(prev) => self.get_node_mut(prev).unwrap().next_sibling = Some(child),
            None => self.get_node_mut(parent).unwrap().first_child = Some(child),
        }
        match reference {
            Some(r) => self.get_node_mut(r).unwrap().prev_sibling = Some(child),
            None => self.get_node_mut(parent).unwrap().last_child = Some(child),
        }
        self.highlights_inserted(parent, child);
    }

    /// Validate that inserting `child` under `parent` is legal (DOM §4.2.3
    /// "ensure pre-insertion validity"): both exist, the parent can have
    /// children, and no cycle. Step 3 (the reference child is the
    /// parent's) is the callers'; steps 4–6 restrict kinds rdom has no
    /// counterpart for (a Document, a DocumentType).
    pub(crate) fn validate_insert(&self, parent: NodeId, child: NodeId) -> Result<()> {
        let parent_node = self.node_or_err(parent)?;
        self.node_or_err(child)?;
        // DOM §4.2.3 "ensure pre-insertion validity" step 1: a parent is
        // a Document, a DocumentFragment or an Element — a Text or a
        // Comment takes no child.
        if !matches!(
            parent_node.data,
            crate::node::NodeData::Element { .. } | crate::node::NodeData::Fragment
        ) {
            return Err(DomError::HierarchyRequest);
        }
        // Step 2: no node goes under itself or its descendant.
        if self.is_ancestor(child, parent) {
            return Err(DomError::HierarchyRequest);
        }
        Ok(())
    }

    /// Depth-first descendants including `root` (iterative). Used by
    /// `drop_subtree`.
    fn collect_descendants(&self, root: NodeId, out: &mut Vec<NodeId>) {
        self.walk_subtree(root, &mut |id, _| out.push(id));
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

    // ── Pre-insertion validity (DOM §4.2.3, C14G-CORE-GAPS) ──────────

    /// Step 1: "If parent is not a Document, DocumentFragment, or Element
    /// node, then throw a HierarchyRequestError" — a Text or Comment node
    /// takes no child (it did).
    #[test]
    fn a_text_or_comment_parent_takes_no_child() {
        let mut dom: Dom = Dom::new();
        let text = dom.create_text_node("t");
        let comment = dom.create_comment("c");
        let x = dom.create_text_node("x");
        assert_eq!(dom.append_child(text, x), Err(DomError::HierarchyRequest));
        assert_eq!(
            dom.append_child(comment, x),
            Err(DomError::HierarchyRequest)
        );
        assert_eq!(
            dom.insert_before(text, x, None),
            Err(DomError::HierarchyRequest)
        );
        assert!(dom.node(text).first_child().is_none());
    }

    /// Step 2: "If node is a host-including inclusive ancestor of parent,
    /// then throw a HierarchyRequestError".
    #[test]
    fn an_ancestor_cannot_go_under_its_descendant() {
        let (mut dom, a, b, _) = sample();
        dom.append_child(a, b).unwrap();
        assert_eq!(dom.append_child(b, a), Err(DomError::HierarchyRequest));
        assert_eq!(dom.append_child(a, a), Err(DomError::HierarchyRequest));
    }

    /// Step 3: "If child is non-null and its parent is not parent, then
    /// throw a NotFoundError".
    #[test]
    fn a_reference_child_must_be_the_parents() {
        let (mut dom, a, b, c) = sample();
        assert_eq!(dom.insert_before(a, b, Some(c)), Err(DomError::NotFound));
    }

    /// Steps 4–6 restrict node kinds rdom has no counterpart for (a
    /// Document, a DocumentType, a Document parent): every rdom node kind —
    /// Element, Text, Comment, DocumentFragment — may be inserted under an
    /// Element or a DocumentFragment.
    #[test]
    fn every_node_kind_goes_under_an_element_or_a_fragment() {
        let mut dom: Dom = Dom::new();
        let e = dom.create_element("e");
        let f = dom.create_document_fragment();
        for kid in [
            dom.create_element("k"),
            dom.create_text_node("t"),
            dom.create_comment("c"),
        ] {
            dom.append_child(f, kid).unwrap();
        }
        dom.append_child(e, f).unwrap();
        assert_eq!(dom.node(e).child_nodes().count(), 3);
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

    /// `CORE-DROP-PANIC-LEAK-1`: the `ChildListChanged` record fires
    /// before the slots are freed (observers may inspect the removed
    /// subtree). A panicking observer must not turn that ordering into
    /// a leak — the subtree is freed on the way out, then the panic
    /// continues.
    #[test]
    fn drop_subtree_frees_even_when_an_observer_panics() {
        struct Bomb;
        impl crate::MutationObserver<()> for Bomb {
            fn observe(&mut self, _dom: &mut Dom, _record: &crate::Mutation) {
                panic!("observer bomb");
            }
        }
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        let span = dom.create_element("span");
        dom.append_child(root, div).unwrap();
        dom.append_child(div, span).unwrap();
        dom.add_mutation_observer(Box::new(Bomb));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.drop_subtree(div).unwrap();
        }));
        assert!(result.is_err(), "the bomb must actually fire");
        assert!(!dom.contains(div), "subtree root was freed");
        assert!(!dom.contains(span), "subtree descendant was freed");
        assert_eq!(dom.len(), 1, "only the root remains");
        assert!(dom.validate().is_empty());
    }

    /// A panicking observer on the *purge* records that `detach_from_parent`
    /// fires (`InteractionChanged` for a focused descendant, `SelectionChanged`
    /// for a selection anchored inside) must not leak either: those fire while
    /// the subtree is still connected, but the unlink still happens (the rest
    /// of the purge runs silently), so the subtree is freed on the way out.
    #[test]
    fn drop_subtree_frees_when_the_focus_purge_observer_panics() {
        struct Bomb;
        impl crate::MutationObserver<()> for Bomb {
            fn observe(&mut self, _dom: &mut Dom, record: &crate::Mutation) {
                if matches!(record, crate::Mutation::InteractionChanged { .. }) {
                    panic!("observer bomb");
                }
            }
        }
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        let span = dom.create_element("span");
        dom.append_child(root, div).unwrap();
        dom.append_child(div, span).unwrap();
        dom.set_focused(Some(span));
        dom.add_mutation_observer(Box::new(Bomb));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.drop_subtree(div).unwrap();
        }));
        assert!(result.is_err(), "the bomb must actually fire");
        assert!(
            !dom.contains(div) && !dom.contains(span),
            "subtree was freed"
        );
        assert_eq!(dom.len(), 1);
        assert_eq!(dom.focused(), None);
        assert!(dom.validate().is_empty());
    }

    #[test]
    fn drop_subtree_frees_when_the_selection_purge_observer_panics() {
        struct Bomb;
        impl crate::MutationObserver<()> for Bomb {
            fn observe(&mut self, _dom: &mut Dom, record: &crate::Mutation) {
                if matches!(record, crate::Mutation::SelectionChanged { .. }) {
                    panic!("observer bomb");
                }
            }
        }
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        let text = dom.create_text_node("hello");
        dom.append_child(root, div).unwrap();
        dom.append_child(div, text).unwrap();
        let at = crate::Position {
            node: text,
            offset: 1,
        };
        dom.set_selection(Some(crate::Selection {
            anchor: at,
            focus: at,
        }));
        dom.add_mutation_observer(Box::new(Bomb));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.drop_subtree(div).unwrap();
        }));
        assert!(result.is_err(), "the bomb must actually fire");
        assert!(
            !dom.contains(div) && !dom.contains(text),
            "subtree was freed"
        );
        assert_eq!(dom.len(), 1);
        assert!(dom.validate().is_empty());
    }

    /// `remove_child_dropping` / `clear_children_dropping` fire their
    /// `ChildListChanged` before the orphan reaches `drop_subtree`; the
    /// same guarantee holds there.
    #[test]
    fn dropping_wrappers_free_when_an_observer_panics() {
        struct Bomb;
        impl crate::MutationObserver<()> for Bomb {
            fn observe(&mut self, _dom: &mut Dom, record: &crate::Mutation) {
                if matches!(record, crate::Mutation::ChildListChanged { .. }) {
                    panic!("observer bomb");
                }
            }
        }
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let a_kid = dom.create_element("kid");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(root, a).unwrap();
        dom.append_child(a, a_kid).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();
        dom.add_mutation_observer(Box::new(Bomb));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.remove_child_dropping(root, a).unwrap();
        }));
        assert!(result.is_err());
        assert!(
            !dom.contains(a) && !dom.contains(a_kid),
            "removed subtree was freed"
        );
        assert_eq!(dom.len(), 3, "root, b, c");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.clear_children_dropping(root).unwrap();
        }));
        assert!(result.is_err());
        assert!(
            !dom.contains(b) && !dom.contains(c),
            "cleared children were freed"
        );
        assert_eq!(dom.len(), 1);
        assert!(dom.validate().is_empty());
    }

    /// The other side of the rule: a panic in the `PreDetach` window
    /// happens *before* the unlink, so the subtree is still attached and
    /// must not be freed out from under its parent.
    #[test]
    fn drop_subtree_keeps_an_attached_subtree_when_pre_detach_panics() {
        struct Bomb;
        impl crate::MutationObserver<()> for Bomb {
            fn observe(&mut self, _dom: &mut Dom, record: &crate::Mutation) {
                if matches!(record, crate::Mutation::PreDetach { .. }) {
                    panic!("observer bomb");
                }
            }
        }
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        let span = dom.create_element("span");
        dom.append_child(root, div).unwrap();
        dom.append_child(div, span).unwrap();
        dom.set_focused(Some(span));
        dom.add_mutation_observer(Box::new(Bomb));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dom.drop_subtree(div).unwrap();
        }));
        assert!(result.is_err());
        assert!(
            dom.contains(div) && dom.contains(span),
            "still attached, not freed"
        );
        assert_eq!(dom.node(root).first_child().map(|n| n.id()), Some(div));
        assert!(dom.validate().is_empty());
    }

    /// The root is the arena's anchor; dropping it would leave `Dom::root`
    /// pointing at a dead slot and break every later call.
    #[test]
    fn drop_subtree_rejects_the_root() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let child = dom.create_element("div");
        dom.append_child(root, child).unwrap();
        assert!(matches!(
            dom.drop_subtree(root).unwrap_err(),
            DomError::HierarchyRequest
        ));
        assert!(dom.contains(root));
        assert!(dom.contains(child), "nothing was freed");
        assert!(dom.validate().is_empty());
    }

    #[test]
    fn append_rejects_invalid_parent() {
        let mut dom: Dom = Dom::new();
        // A NodeId that was never allocated — guaranteed invalid.
        let ghost = NodeId::from_parts(999, std::num::NonZeroU32::MIN);
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
    fn remove_child_dropping_frees_the_node() {
        let (mut dom, a, b, _c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(a, b).unwrap();

        dom.remove_child_dropping(root, a).unwrap();
        // Detached AND freed — the whole subtree is gone from the arena.
        assert!(!dom.contains(a));
        assert!(!dom.contains(b));
        assert!(dom.get_node(root).unwrap().first_child.is_none());
    }

    #[test]
    fn clear_children_dropping_frees_all() {
        let (mut dom, a, b, c) = sample();
        let root = dom.root();
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();

        dom.clear_children_dropping(root).unwrap();
        assert!(dom.get_node(root).unwrap().first_child.is_none());
        // Unlike clear_children, every child is freed, not orphaned.
        assert!(!dom.contains(a));
        assert!(!dom.contains(b));
        assert!(!dom.contains(c));
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
