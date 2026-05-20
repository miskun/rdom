//! `HtmlCollection` — snapshot wrapper for named-access
//! collections (`form.elements`, `document.getElementsByTagName`,
//! etc).
//!
//! Differs from [`NodeList`](crate::NodeList) by adding
//! [`HtmlCollection::named_item`] — DOM's "named property getter"
//! that looks up by `name` or `id` attribute. Same snapshot
//! semantics: ids frozen at construction, resolution skips slots
//! whose node has been removed.

use crate::accessor::NodeRef;
use crate::dom::Dom;
use crate::node_id::NodeId;

/// Snapshot of element ids captured at construction with
/// name-based lookup. Used by `form.elements`,
/// `getElementsByTagName`, etc.
pub struct HtmlCollection<'a, Ext: 'static> {
    nodes: Vec<NodeId>,
    dom: &'a Dom<Ext>,
}

/// Alias for `form.elements`'s return type. The web platform
/// distinguishes these at the IDL level but uses the same shape;
/// rdom collapses them to a single struct + a type alias.
pub type FormControlsCollection<'a, Ext> = HtmlCollection<'a, Ext>;

impl<'a, Ext: 'static> HtmlCollection<'a, Ext> {
    /// Construct from an iterator of element ids and a borrowed
    /// Dom. Used by `form.elements()` (M4b step 30) and tests.
    pub fn from_ids(dom: &'a Dom<Ext>, nodes: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
            dom,
        }
    }

    /// Number of element ids in the snapshot. DOM `length`.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// `true` iff the snapshot has no element ids.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Borrow the `i`-th element if (a) the index is in range and
    /// (b) the underlying id still resolves to a live element.
    /// DOM `item(i)`.
    pub fn item(&self, index: usize) -> Option<NodeRef<'a, Ext>> {
        let id = *self.nodes.get(index)?;
        if self.dom.contains(id) {
            Some(self.dom.node(id))
        } else {
            None
        }
    }

    /// Look up the first element whose `name` attribute matches
    /// `name`, falling back to `id` attribute. Matches DOM
    /// `namedItem` (the "named property getter" on
    /// HTMLCollection). Snapshot-resolved — removed nodes are
    /// skipped even if their id was in the original snapshot.
    pub fn named_item(&self, name: &str) -> Option<NodeRef<'a, Ext>> {
        for &id in &self.nodes {
            if !self.dom.contains(id) {
                continue;
            }
            let node = self.dom.node(id);
            if node.get_attribute("name") == Some(name) || node.get_attribute("id") == Some(name) {
                return Some(node);
            }
        }
        None
    }

    /// Iterate over the live members of the snapshot in
    /// snapshot order. Nodes removed since the snapshot was taken
    /// are skipped.
    pub fn iter(&self) -> impl Iterator<Item = NodeRef<'a, Ext>> + '_ {
        self.nodes
            .iter()
            .copied()
            .filter(|id| self.dom.contains(*id))
            .map(move |id| self.dom.node(id))
    }

    /// Raw access to the snapshot's id slice. Escape hatch.
    pub fn ids(&self) -> &[NodeId] {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_len_item() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let col = HtmlCollection::from_ids(&dom, [a, b]);
        assert_eq!(col.len(), 2);
        assert!(!col.is_empty());
        assert_eq!(col.item(0).map(|n| n.id()), Some(a));
        assert_eq!(col.item(1).map(|n| n.id()), Some(b));
        assert_eq!(col.item(2).map(|n| n.id()), None);
    }

    #[test]
    fn named_item_matches_name_attribute_first() {
        let mut dom: Dom = Dom::new();
        let input = dom.create_element("input");
        dom.set_attribute(input, "name", "user").unwrap();
        let col = HtmlCollection::from_ids(&dom, [input]);
        assert_eq!(col.named_item("user").map(|n| n.id()), Some(input));
        assert!(col.named_item("missing").is_none());
    }

    #[test]
    fn named_item_falls_back_to_id_attribute() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_id(el, "main").unwrap();
        let col = HtmlCollection::from_ids(&dom, [el]);
        assert_eq!(col.named_item("main").map(|n| n.id()), Some(el));
    }

    #[test]
    fn named_item_name_wins_over_id_on_other_elements() {
        // If element A has name="foo" and element B has id="foo",
        // a lookup of "foo" returns A (first match in snapshot
        // order, then attribute preference within each candidate).
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("input");
        dom.set_attribute(a, "name", "foo").unwrap();
        let b = dom.create_element("div");
        dom.set_id(b, "foo").unwrap();
        let col = HtmlCollection::from_ids(&dom, [a, b]);
        assert_eq!(col.named_item("foo").map(|n| n.id()), Some(a));
    }

    #[test]
    fn named_item_skips_removed_elements() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("input");
        dom.set_attribute(a, "name", "x").unwrap();
        dom.append_child(root, a).unwrap();
        let b = dom.create_element("input");
        dom.set_attribute(b, "name", "y").unwrap();
        dom.append_child(root, b).unwrap();

        let snapshot = vec![a, b];
        dom.remove_child(root, a).unwrap();
        dom.drop_subtree(a).unwrap();

        let col = HtmlCollection::from_ids(&dom, snapshot);
        // a is dropped → look-up of "x" returns None even though
        // the id is still in the snapshot.
        assert!(col.named_item("x").is_none());
        assert_eq!(col.named_item("y").map(|n| n.id()), Some(b));
    }

    #[test]
    fn iter_yields_live_elements_in_snapshot_order() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let col = HtmlCollection::from_ids(&dom, [a, b]);
        let ids: Vec<NodeId> = col.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec![a, b]);
    }

    #[test]
    fn ids_returns_raw_snapshot_slice() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let col = HtmlCollection::from_ids(&dom, [a, b]);
        assert_eq!(col.ids(), &[a, b][..]);
    }

    #[test]
    fn form_controls_collection_is_alias() {
        // Compile-time check that FormControlsCollection is the
        // same shape as HtmlCollection.
        let mut dom: Dom = Dom::new();
        let input = dom.create_element("input");
        let coll: FormControlsCollection<'_, ()> = HtmlCollection::from_ids(&dom, [input]);
        assert_eq!(coll.len(), 1);
    }
}
