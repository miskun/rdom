//! [`Dom::descendants`]: a node's descendants in tree order, as an
//! iterator — the walk `TreeWalker` / `NodeIterator` make (DOM §6).

use crate::dom::Dom;
use crate::node_id::NodeId;

/// The descendants of a node in tree order (pre-order, depth-first; DOM
/// §4.2), text and comment nodes included, the node itself not
/// ([`Dom::descendants`]). It follows the child / sibling / parent links
/// with no stack of its own, so any depth is safe. The tree must not
/// change while it is walked (it borrows the `Dom`).
pub struct Descendants<'a, Ext: 'static> {
    dom: &'a Dom<Ext>,
    root: NodeId,
    next: Option<NodeId>,
}

impl<Ext: 'static> Iterator for Descendants<'_, Ext> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let cur = self.next?;
        let node = self.dom.get_node(cur)?;
        self.next = node
            .first_child
            .or_else(|| self.dom.next_in_subtree(cur, self.root));
        Some(cur)
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// `root`'s descendants in tree order ([`Descendants`]): every node a
    /// search over the document's text visits —
    /// `dom.descendants(dom.root()).filter(|&n| dom.node(n).node_type() == NodeType::Text)`.
    pub fn descendants(&self, root: NodeId) -> Descendants<'_, Ext> {
        Descendants {
            dom: self,
            root,
            next: self.get_node(root).and_then(|n| n.first_child),
        }
    }
}
