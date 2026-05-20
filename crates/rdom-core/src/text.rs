//! `textContent` — getter concatenates descendant Text data; setter
//! replaces all children with a single Text node.
//!
//! Spec: https://dom.spec.whatwg.org/#dom-node-textcontent

use crate::dom::Dom;
use crate::error::Result;
use crate::node::NodeData;
use crate::node_id::NodeId;

impl<Ext> Dom<Ext> {
    /// Concatenate the string content of `id` and all its descendants.
    ///
    /// - Text nodes: own `data`.
    /// - Element / Fragment: recursive concat of descendants.
    /// - Comment: empty string (matches spec — comments are **not**
    ///   included in textContent).
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.collect_text(id, &mut out);
        out
    }

    fn collect_text(&self, id: NodeId, out: &mut String) {
        let Some(node) = self.get_node(id) else {
            return;
        };
        match &node.data {
            NodeData::Text { data } => out.push_str(data),
            NodeData::Comment { .. } => {}
            NodeData::Element { .. } | NodeData::Fragment => {
                let mut child = node.first_child;
                while let Some(c) = child {
                    self.collect_text(c, out);
                    child = self.get_node(c).and_then(|n| n.next_sibling);
                }
            }
        }
    }
}

impl<Ext: Default> Dom<Ext> {
    /// Replace all children of `id` with a single Text node containing `text`.
    ///
    /// Matches `Node.textContent` setter semantics: any existing children
    /// are detached + dropped; if `text` is empty the node has no
    /// children; otherwise it has exactly one Text child.
    ///
    /// Element-only operation fails with `WrongNodeType` on Text/Comment
    /// (use `set_node_value` on those).
    pub fn set_text_content(&mut self, id: NodeId, text: &str) -> Result<()> {
        use crate::error::DomError;
        use crate::node::NodeType;

        match &self.node_or_err(id)?.data {
            NodeData::Element { .. } | NodeData::Fragment => {}
            NodeData::Text { .. } => {
                return Err(DomError::WrongNodeType {
                    expected: "Element or Fragment",
                    got: NodeType::Text,
                });
            }
            NodeData::Comment { .. } => {
                return Err(DomError::WrongNodeType {
                    expected: "Element or Fragment",
                    got: NodeType::Comment,
                });
            }
        }

        // Drop existing children entirely so we don't leak orphan nodes.
        let existing: Vec<NodeId> = {
            let mut out = Vec::new();
            let mut c = self.get_node(id).and_then(|n| n.first_child);
            while let Some(cid) = c {
                out.push(cid);
                c = self.get_node(cid).and_then(|n| n.next_sibling);
            }
            out
        };
        for cid in existing {
            self.drop_subtree(cid)?;
        }

        if !text.is_empty() {
            let t = self.create_text_node(text);
            self.append_child(id, t)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    #[test]
    fn text_node_returns_own_data() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hello");
        assert_eq!(dom.text_content(t), "hello");
    }

    #[test]
    fn element_concatenates_descendant_text() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let a = dom.create_text_node("hello ");
        let span = dom.create_element("span");
        let b = dom.create_text_node("world");
        dom.append_child(span, b).unwrap();
        dom.append_child(div, a).unwrap();
        dom.append_child(div, span).unwrap();
        assert_eq!(dom.text_content(div), "hello world");
    }

    #[test]
    fn comment_children_not_included() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let a = dom.create_text_node("before ");
        let c = dom.create_comment(" skip me ");
        let b = dom.create_text_node("after");
        dom.append_child(div, a).unwrap();
        dom.append_child(div, c).unwrap();
        dom.append_child(div, b).unwrap();
        assert_eq!(dom.text_content(div), "before after");
    }

    #[test]
    fn empty_element_empty_text_content() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        assert_eq!(dom.text_content(div), "");
    }

    #[test]
    fn set_text_content_replaces_children() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let old_text = dom.create_text_node("old");
        let old_span = dom.create_element("span");
        dom.append_child(div, old_text).unwrap();
        dom.append_child(div, old_span).unwrap();

        dom.set_text_content(div, "new content").unwrap();

        assert_eq!(dom.node(div).child_element_count(), 0);
        assert_eq!(
            dom.node(div).first_child().unwrap().node_value(),
            Some("new content")
        );
        assert_eq!(dom.text_content(div), "new content");
    }

    #[test]
    fn set_text_content_empty_clears_children() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let t = dom.create_text_node("existing");
        dom.append_child(div, t).unwrap();

        dom.set_text_content(div, "").unwrap();

        assert!(!dom.node(div).has_child_nodes());
    }

    #[test]
    fn set_text_content_on_text_errors() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hi");
        assert!(dom.set_text_content(t, "bye").is_err());
    }

    #[test]
    fn set_text_content_on_fragment_works() {
        let mut dom: Dom = Dom::new();
        let frag = dom.create_document_fragment();
        let old = dom.create_element("span");
        dom.append_child(frag, old).unwrap();

        dom.set_text_content(frag, "flat text").unwrap();
        assert_eq!(dom.text_content(frag), "flat text");
    }
}
