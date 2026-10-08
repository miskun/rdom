//! The highlights' half of the `Dom`: the registry's accessors, and the
//! live-range updates the tree and text mutators call (DOM §5.3).

use super::{HighlightRegistry, HighlightsMut};
use crate::node_id::NodeId;
use crate::selection::Position;

impl<Ext: 'static> crate::Dom<Ext> {
    /// The document's highlight registry (CSS Custom Highlight API 1 §4,
    /// `CSS.highlights`).
    pub fn highlights(&self) -> &HighlightRegistry {
        &self.highlights
    }

    /// The registry, to register, change or remove highlights: a guard
    /// that reports a change as one `Mutation::HighlightsChanged` when it
    /// goes, after the change, so a renderer observing mutations repaints
    /// the highlights ([`HighlightsMut`]).
    pub fn highlights_mut(&mut self) -> HighlightsMut<'_, Ext> {
        HighlightsMut::new(self)
    }

    /// The index of `id` among its parent's children.
    pub(crate) fn child_index(&self, id: NodeId) -> usize {
        let mut index = 0;
        let mut cur = self.get_node(id).and_then(|n| n.prev_sibling);
        while let Some(prev) = cur {
            #[cfg(test)]
            crate::highlight_tests::INDEX_HOPS.with(|c| c.set(c.get() + 1));
            index += 1;
            cur = self.get_node(prev).and_then(|n| n.prev_sibling);
        }
        index
    }

    /// The live ranges after `child` was inserted under `parent` (DOM
    /// §4.2.3 "insert" step 6: a boundary in `parent` past the insertion
    /// index moves right). Free with no highlight registered, and for an
    /// append — the index is the old child count, which no boundary
    /// offset exceeds; elsewhere the index is walked only when a boundary
    /// sits in `parent`.
    pub(crate) fn highlights_inserted(&mut self, parent: NodeId, child: NodeId) {
        if self.highlights.is_empty()
            || self.get_node(child).and_then(|n| n.next_sibling).is_none()
            || !self.highlights.has_point_in(parent)
        {
            return;
        }
        let index = self.child_index(child);
        self.highlights.inserted(parent, index);
    }

    /// The live ranges before `id` is removed from its parent (DOM
    /// §4.2.3 "remove" steps 4–7). Free with no highlight registered.
    pub(crate) fn highlights_removing(&mut self, id: NodeId) {
        if self.highlights.is_empty() {
            return;
        }
        let Some(parent) = self.get_node(id).and_then(|n| n.parent) else {
            return;
        };
        // The index is walked only when a boundary moves: one in `parent`
        // or inside the removed subtree.
        let moves = |p: &Position| p.node == parent || self.is_ancestor(id, p.node);
        if !self.highlights.points().any(moves) {
            return;
        }
        let index = self.child_index(id);
        let mut registry = std::mem::take(&mut self.highlights);
        registry.removing(parent, index, |n| self.is_ancestor(id, n));
        self.highlights = registry;
    }

    /// The live ranges after `count` bytes at `offset` of the text `node`
    /// were replaced by `len` bytes (DOM §4.10 "replace data").
    pub(crate) fn highlights_replace_data(
        &mut self,
        node: NodeId,
        (offset, count, len): (usize, usize, usize),
    ) {
        if !self.highlights.is_empty() {
            self.highlights.replace_data(node, offset, count, len);
        }
    }
}
