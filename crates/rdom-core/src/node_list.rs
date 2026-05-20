//! `NodeList` — snapshot wrapper for query-selector-all-style
//! collections.
//!
//! ## Live vs snapshot
//!
//! Per the M4 scope lock (parity ledger §25 #2), wrapper **shape**
//! ships but **liveness** does not. The `NodeId`s in `nodes` are
//! frozen at construction; if a node gets removed from the tree
//! between the snapshot and an `item(i)` / `iter` call, the
//! resolver returns `None` for that slot.
//!
//! ## API shape
//!
//! Matches the JS `NodeList` interface: `length` / `item(i)` /
//! iteration. `ids()` is an rdom-side escape hatch that hands back
//! the raw `&[NodeId]` for callers who want to do their own
//! resolution.

use crate::accessor::NodeRef;
use crate::dom::Dom;
use crate::node_id::NodeId;

/// Snapshot of node ids captured at construction. `item(i)` and
/// `iter()` resolve against the borrowed `&Dom`; if a node has
/// been removed since the snapshot, the resolution returns `None`
/// for that slot (filtered out by `iter()`).
pub struct NodeList<'a, Ext: 'static> {
    nodes: Vec<NodeId>,
    dom: &'a Dom<Ext>,
}

impl<'a, Ext: 'static> NodeList<'a, Ext> {
    /// Construct from an iterator of node ids and a borrowed Dom.
    /// Used by `Dom::query_selector_all` (M4b step 18) and tests.
    pub fn from_ids(dom: &'a Dom<Ext>, nodes: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
            dom,
        }
    }

    /// Number of node ids in the snapshot. DOM `length`.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// `true` iff the snapshot has no node ids.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Borrow the `i`-th node if (a) the index is in range and
    /// (b) the underlying id still resolves to a live node. DOM
    /// `item(i)`.
    pub fn item(&self, index: usize) -> Option<NodeRef<'a, Ext>> {
        let id = *self.nodes.get(index)?;
        if self.dom.contains(id) {
            Some(self.dom.node(id))
        } else {
            None
        }
    }

    /// Iterate over the live members of the snapshot in
    /// snapshot order. Nodes that have been removed from the tree
    /// since the snapshot was taken are skipped.
    pub fn iter(&self) -> impl Iterator<Item = NodeRef<'a, Ext>> + '_ {
        self.nodes
            .iter()
            .copied()
            .filter(|id| self.dom.contains(*id))
            .map(move |id| self.dom.node(id))
    }

    /// Raw access to the snapshot's id slice. Escape hatch for
    /// callers who want to do their own resolution (e.g. compare
    /// snapshots, build their own indices).
    pub fn ids(&self) -> &[NodeId] {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_len_item_ids() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        let list = NodeList::from_ids(&dom, [a, b, c]);

        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());
        assert_eq!(list.ids(), &[a, b, c][..]);

        assert_eq!(list.item(0).map(|n| n.id()), Some(a));
        assert_eq!(list.item(1).map(|n| n.id()), Some(b));
        assert_eq!(list.item(2).map(|n| n.id()), Some(c));
        assert_eq!(list.item(3).map(|n| n.id()), None);
    }

    #[test]
    fn empty_snapshot_is_empty() {
        let dom: Dom = Dom::new();
        let list = NodeList::from_ids(&dom, Vec::<NodeId>::new());
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.item(0).map(|n| n.id()), None);
    }

    #[test]
    fn iter_yields_snapshot_order() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let list = NodeList::from_ids(&dom, [a, b]);
        let ids: Vec<NodeId> = list.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec![a, b]);
    }

    #[test]
    fn removed_node_returns_none_from_item() {
        // Snapshot before removal; after removal, item() returns
        // None for the removed slot, matching JS behavior for a
        // live NodeList post-removal.
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();

        let snapshot_ids = vec![a, b];
        dom.remove_child(root, a).unwrap();
        dom.drop_subtree(a).unwrap();

        let list = NodeList::from_ids(&dom, snapshot_ids);
        // Slot 0 (the removed `a`) resolves to None…
        assert!(list.item(0).is_none());
        // …but slot 1 (`b`) is still live.
        assert_eq!(list.item(1).map(|n| n.id()), Some(b));
    }

    #[test]
    fn iter_skips_removed_nodes() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();

        let snapshot_ids = vec![a, b, c];
        dom.remove_child(root, b).unwrap();
        dom.drop_subtree(b).unwrap();

        let list = NodeList::from_ids(&dom, snapshot_ids);
        let live: Vec<NodeId> = list.iter().map(|n| n.id()).collect();
        assert_eq!(live, vec![a, c]);
    }
}
