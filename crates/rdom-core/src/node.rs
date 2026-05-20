//! `Node<Ext>` — internal arena storage + `NodeData` enum + `NodeType`.
//!
//! A `Node` is a tree cell: linked-list pointers (parent, first_child,
//! last_child, prev_sibling, next_sibling) plus per-type payload in
//! `NodeData`. Nodes carry presentation data (`Ext`) only on the `Element`
//! variant — Text/Comment/Fragment don't need it.

use std::collections::{BTreeMap, BTreeSet};

use crate::NodeId;

/// Per-type payload.
#[derive(Debug, Clone)]
pub enum NodeData<Ext = ()> {
    Element {
        tag: String,
        /// BTreeMap for deterministic iteration (markup round-trips,
        /// snapshot test stability).
        attrs: BTreeMap<String, String>,
        /// classList tokens. Set semantics (no duplicates, membership).
        classes: BTreeSet<String>,
        /// Presentation extension (`()` in core, `TuiExt` in rdom-tui).
        ext: Ext,
    },
    Text {
        data: String,
    },
    Comment {
        data: String,
    },
    /// DocumentFragment — a detachable subtree container. Inserting a
    /// fragment unwraps it: children move to the target, fragment itself
    /// stays empty and reusable.
    Fragment,
}

/// DOM-spec node types with the numeric values the spec assigns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeType {
    Element = 1,
    Text = 3,
    Comment = 8,
    Fragment = 11,
}

impl NodeType {
    /// Canonical node name (mirrors browser `nodeName` for non-elements).
    pub fn canonical_name(self) -> &'static str {
        match self {
            NodeType::Element => "", // element's actual tag; caller handles
            NodeType::Text => "#text",
            NodeType::Comment => "#comment",
            NodeType::Fragment => "#document-fragment",
        }
    }
}

/// Arena node — linked-list pointers + typed payload.
///
/// Never exposed directly; the crate surfaces `NodeRef<'_, Ext>` / `NodeMut<'_, Ext>`
/// wrappers over `(&Dom, NodeId)` pairs.
#[derive(Debug, Clone)]
pub(crate) struct Node<Ext = ()> {
    pub(crate) parent: Option<NodeId>,
    pub(crate) first_child: Option<NodeId>,
    pub(crate) last_child: Option<NodeId>,
    pub(crate) prev_sibling: Option<NodeId>,
    pub(crate) next_sibling: Option<NodeId>,
    pub(crate) data: NodeData<Ext>,
}

impl<Ext> Node<Ext> {
    pub(crate) fn new(data: NodeData<Ext>) -> Self {
        Self {
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            data,
        }
    }

    /// Tag name of an Element, or `None` for other types.
    pub(crate) fn tag_name(&self) -> Option<&str> {
        match &self.data {
            NodeData::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    pub(crate) fn node_type(&self) -> NodeType {
        match &self.data {
            NodeData::Element { .. } => NodeType::Element,
            NodeData::Text { .. } => NodeType::Text,
            NodeData::Comment { .. } => NodeType::Comment,
            NodeData::Fragment => NodeType::Fragment,
        }
    }

    /// Clear pointer fields; used when a node is detached.
    #[allow(dead_code)] // planned helper for Phase 2 (clone_node, etc.)
    pub(crate) fn unlink(&mut self) {
        self.parent = None;
        self.first_child = None;
        self.last_child = None;
        self.prev_sibling = None;
        self.next_sibling = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_numeric_values_match_mdn() {
        assert_eq!(NodeType::Element as u8, 1);
        assert_eq!(NodeType::Text as u8, 3);
        assert_eq!(NodeType::Comment as u8, 8);
        assert_eq!(NodeType::Fragment as u8, 11);
    }

    #[test]
    fn canonical_names_match_browser() {
        assert_eq!(NodeType::Text.canonical_name(), "#text");
        assert_eq!(NodeType::Comment.canonical_name(), "#comment");
        assert_eq!(NodeType::Fragment.canonical_name(), "#document-fragment");
    }

    #[test]
    fn new_element_node_starts_unlinked() {
        let n: Node<()> = Node::new(NodeData::Element {
            tag: "div".into(),
            attrs: BTreeMap::new(),
            classes: BTreeSet::new(),
            ext: (),
        });
        assert!(n.parent.is_none());
        assert!(n.first_child.is_none());
        assert!(n.last_child.is_none());
        assert!(n.prev_sibling.is_none());
        assert!(n.next_sibling.is_none());
        assert_eq!(n.node_type(), NodeType::Element);
        assert_eq!(n.tag_name(), Some("div"));
    }

    #[test]
    fn non_element_tag_name_is_none() {
        let n: Node<()> = Node::new(NodeData::Text { data: "hi".into() });
        assert_eq!(n.tag_name(), None);
        assert_eq!(n.node_type(), NodeType::Text);
    }
}
