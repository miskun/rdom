//! `clone_node(id, deep)` — copy a node (or subtree) into a new orphan in
//! the same arena. Matches MDN semantics: attrs/classes/text preserved,
//! `parent` on the clone is `None`, event listeners **not** copied.

use crate::dom::Dom;
use crate::node::{Node, NodeData};
use crate::node_id::NodeId;

impl<Ext: Clone> Dom<Ext> {
    /// Produce an orphan clone of `id`. If `deep`, recursively clone all
    /// descendants. If not, only the node itself.
    ///
    /// Returns the new orphan's `NodeId`. The caller must attach it with
    /// `append_child` / `insert_before` to make it live in the tree.
    pub fn clone_node(&mut self, id: NodeId, deep: bool) -> NodeId {
        let data = match &self.get_node(id).expect("clone_node: invalid id").data {
            NodeData::Element {
                tag,
                attrs,
                classes,
                ext,
            } => NodeData::Element {
                tag: tag.clone(),
                attrs: attrs.clone(),
                classes: classes.clone(),
                ext: ext.clone(),
            },
            NodeData::Text { data } => NodeData::Text { data: data.clone() },
            NodeData::Comment { data } => NodeData::Comment { data: data.clone() },
            NodeData::Fragment => NodeData::Fragment,
        };
        let new_id = self.alloc(Node::new(data));

        if deep {
            // Clone each child and append to the clone.
            let mut child_id = self.get_node(id).and_then(|n| n.first_child);
            while let Some(c) = child_id {
                let cloned = self.clone_node(c, true);
                self.append_child(new_id, cloned)
                    .expect("clone_node deep: append failed");
                child_id = self.get_node(c).and_then(|n| n.next_sibling);
            }
        }

        new_id
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    #[test]
    fn shallow_clone_copies_tag_and_attrs() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "role", "banner").unwrap();
        dom.add_class(el, "active").unwrap();

        let c = dom.clone_node(el, false);
        assert_eq!(dom.node(c).tag_name(), Some("div"));
        assert_eq!(dom.node(c).get_attribute("role"), Some("banner"));
        assert!(dom.node(c).has_class("active"));
        assert!(dom.node(c).parent_node().is_none());
    }

    #[test]
    fn shallow_clone_has_no_children() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let child = dom.create_element("span");
        dom.append_child(parent, child).unwrap();

        let c = dom.clone_node(parent, false);
        assert!(!dom.node(c).has_child_nodes());
    }

    #[test]
    fn deep_clone_recursively_copies_children() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let child = dom.create_element("span");
        let text = dom.create_text_node("hello");
        dom.append_child(child, text).unwrap();
        dom.append_child(parent, child).unwrap();

        let c = dom.clone_node(parent, true);
        assert_eq!(dom.node(c).child_element_count(), 1);
        let first = dom.node(c).first_element_child().unwrap();
        assert_eq!(first.tag_name(), Some("span"));
        let first_text = first.first_child().unwrap();
        assert_eq!(first_text.node_value(), Some("hello"));
        assert!(dom.is_equal_node(parent, c));
    }

    #[test]
    fn clone_text_node() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hello");
        let c = dom.clone_node(t, true);
        assert_eq!(dom.node(c).node_value(), Some("hello"));
    }

    #[test]
    fn clone_is_independent() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "class", "original").unwrap();

        let c = dom.clone_node(el, false);
        dom.set_attribute(c, "class", "modified").unwrap();

        assert_eq!(dom.node(el).get_attribute("class"), Some("original"));
        assert_eq!(dom.node(c).get_attribute("class"), Some("modified"));
    }

    #[test]
    fn cloned_fragment_unwraps_normally_on_append() {
        let mut dom: Dom = Dom::new();
        let frag = dom.create_document_fragment();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(frag, a).unwrap();
        dom.append_child(frag, b).unwrap();

        let cloned_frag = dom.clone_node(frag, true);
        let root = dom.root();
        dom.append_child(root, cloned_frag).unwrap();
        // Fragment children moved out; cloned_frag is empty.
        assert_eq!(dom.node(root).child_element_count(), 2);
    }
}
