//! The highlights' half of the `Dom`: the registry's accessors, and the
//! live-range updates the tree and text mutators call (DOM §5.3).

use super::{HighlightRegistry, HighlightsMut};
use crate::node_id::NodeId;

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
        // or inside the removed subtree. The boundaries share ancestors:
        // each node's answer is found once per removal.
        let mut memo = InsideMemo::new(id);
        let moves = self
            .highlights
            .points()
            .any(|p| p.node == parent || memo.inside(self, p.node));
        if !moves {
            return;
        }
        let index = self.child_index(id);
        let mut registry = std::mem::take(&mut self.highlights);
        registry.removing(parent, index, |n| memo.inside(self, n));
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

/// Whether nodes are the removed node or inside it, each answered once:
/// a walk up from a node stops at the removed node, the root, or a node
/// already answered, and records its answer for every node it passed.
struct InsideMemo {
    removed: NodeId,
    known: std::collections::HashMap<NodeId, bool>,
}

impl InsideMemo {
    fn new(removed: NodeId) -> Self {
        Self {
            removed,
            known: std::collections::HashMap::new(),
        }
    }

    fn inside<Ext>(&mut self, dom: &crate::Dom<Ext>, n: NodeId) -> bool {
        let mut path = Vec::new();
        let mut cur = Some(n);
        let answer = loop {
            let Some(c) = cur else {
                break false;
            };
            #[cfg(test)]
            crate::highlight_tests::INSIDE_HOPS.with(|h| h.set(h.get() + 1));
            if c == self.removed {
                break true;
            }
            if let Some(&known) = self.known.get(&c) {
                break known;
            }
            path.push(c);
            cur = dom.get_node(c).and_then(|x| x.parent);
        };
        for c in path {
            self.known.insert(c, answer);
        }
        answer
    }
}
