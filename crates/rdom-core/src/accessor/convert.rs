//! DOM §4.2.6's shared steps for the `ParentNode` / `ChildNode`
//! convenience methods (C13G-DOM-CONVENIENCE).
//!
//! The spec converts its nodes into one node — strings become `Text`
//! nodes, several nodes a `DocumentFragment` each is moved into — and
//! inserts it once, before a *viable* sibling: the first sibling, from the
//! one it would insert before, that is not itself in the list. rdom keeps
//! the result without the fragment: every node is checked first (nothing
//! moves when one cannot be inserted), the viable reference is found among
//! the siblings the list leaves in place, and the nodes are inserted
//! before it in list order — each moved out of its old place as it goes,
//! where the spec moves them all into the fragment first. The tree ends
//! the same, and no temporary fragment's records reach the observers.

use super::*;

impl<Ext: 'static> Dom<Ext> {
    /// DOM §4.2.3 "ensure pre-insertion validity", for every node of
    /// `items` under `parent`, before any of them moves: each exists and
    /// is no inclusive ancestor of `parent` (`HierarchyRequest`).
    pub(super) fn check_insertable(&self, parent: NodeId, items: &[NodeOrString]) -> Result<()> {
        for item in items {
            if let NodeOrString::Node(n) = item {
                self.validate_insert(parent, *n)?;
            }
        }
        Ok(())
    }

    /// The first of `from` and its following siblings that is not a node
    /// of `items` — where the converted node goes (DOM §4.2.6's viable
    /// sibling, once the list's nodes left their places).
    pub(super) fn first_not_in(
        &self,
        from: Option<NodeId>,
        items: &[NodeOrString],
    ) -> Option<NodeId> {
        // One set, not a scan of the list per sibling (O(items + siblings)).
        let listed: std::collections::HashSet<NodeId> = items
            .iter()
            .filter_map(|i| match i {
                NodeOrString::Node(n) => Some(*n),
                _ => None,
            })
            .collect();
        let listed = |id: NodeId| listed.contains(&id);
        let mut cur = from;
        while let Some(id) = cur {
            if !listed(id) {
                return Some(id);
            }
            cur = self.get_node(id).and_then(|n| n.next_sibling);
        }
        None
    }

    /// Insert `items` under `parent` before `reference` (a child of
    /// `parent` that is no node of `items`, or `None` for the end), in
    /// order, strings as new `Text` nodes.
    pub(super) fn insert_items(
        &mut self,
        parent: NodeId,
        items: Vec<NodeOrString>,
        reference: Option<NodeId>,
    ) -> Result<()> {
        for item in items {
            let node = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => self.create_text_node(&s),
            };
            self.insert_before(parent, node, reference)?;
        }
        Ok(())
    }
}
