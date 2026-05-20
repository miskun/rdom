//! `NodeOrString` — sum type for variadic tree-mutation helpers.
//!
//! JS's variadic `el.append(node1, "text", node2)` doesn't map
//! cleanly to Rust (no varargs, no string coercion). The rdom
//! shape takes a slice or iterator of [`NodeOrString`] — built
//! with `From` impls so authors can write:
//!
//! ```
//! use rdom_core::{Dom, NodeOrString};
//!
//! let mut dom: Dom = Dom::new();
//! let p = dom.create_element("p");
//! let strong = dom.create_element("strong");
//!
//! let items: Vec<NodeOrString> = vec![
//!     "hello, ".into(),
//!     strong.into(),
//!     "!".into(),
//! ];
//! # let _ = (p, items);
//! ```
//!
//! The actual `append` / `prepend` / `before` / `after` /
//! `replace_children` / `replace_with` methods on `NodeMut` come
//! in M4b step 14. This module ships the carrier type ahead of
//! time so step 14 has a stable substrate to build on.

use crate::node_id::NodeId;

/// Either an existing node (by id) or a string to be wrapped in a
/// fresh text node. Built via `From` impls so call sites read
/// naturally — see module docs for an example.
///
/// The variant is named `Text` (not `String`) to keep the carrier
/// semantics explicit: a `Text` value becomes a `Text` node in
/// the DOM, never inline markup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeOrString {
    /// An existing node — will be linked into the destination by
    /// id. The caller is responsible for ensuring the node id is
    /// valid for the target Dom.
    Node(NodeId),
    /// Text content — the receiver will create a fresh `Text`
    /// node carrying this string.
    Text(String),
}

impl From<NodeId> for NodeOrString {
    fn from(id: NodeId) -> Self {
        NodeOrString::Node(id)
    }
}

impl From<String> for NodeOrString {
    fn from(s: String) -> Self {
        NodeOrString::Text(s)
    }
}

impl From<&str> for NodeOrString {
    fn from(s: &str) -> Self {
        NodeOrString::Text(s.to_string())
    }
}

impl From<&String> for NodeOrString {
    fn from(s: &String) -> Self {
        NodeOrString::Text(s.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dom;

    #[test]
    fn from_node_id_produces_node_variant() {
        let mut dom: Dom = Dom::new();
        let n = dom.create_element("p");
        let v: NodeOrString = n.into();
        assert_eq!(v, NodeOrString::Node(n));
    }

    #[test]
    fn from_str_produces_text_variant() {
        let v: NodeOrString = "hello".into();
        assert_eq!(v, NodeOrString::Text("hello".into()));
    }

    #[test]
    fn from_string_produces_text_variant() {
        let v: NodeOrString = String::from("hello").into();
        assert_eq!(v, NodeOrString::Text("hello".into()));
    }

    #[test]
    fn from_string_ref_clones_and_produces_text_variant() {
        let s = String::from("hello");
        let v: NodeOrString = (&s).into();
        assert_eq!(v, NodeOrString::Text("hello".into()));
        // Original String unaffected — &String → NodeOrString clones.
        assert_eq!(s, "hello");
    }

    #[test]
    fn variadic_ergonomics_via_into_iterator() {
        // The shape M4b step 14's `append` will take:
        // `impl IntoIterator<Item = NodeOrString>`. Authors build
        // a Vec<NodeOrString> via Into::into per element. This
        // test demonstrates the ergonomic that future helpers will
        // expose; it's also the canonical step-9 failing test
        // before the From impls existed.
        let mut dom: Dom = Dom::new();
        let strong = dom.create_element("strong");
        let em = dom.create_element("em");

        let items: Vec<NodeOrString> = vec![
            "before ".into(),
            strong.into(),
            " mid ".into(),
            em.into(),
            " after".into(),
        ];

        assert_eq!(items.len(), 5);
        match &items[0] {
            NodeOrString::Text(s) => assert_eq!(s, "before "),
            _ => panic!("expected Text variant"),
        }
        match &items[1] {
            NodeOrString::Node(n) => assert_eq!(*n, strong),
            _ => panic!("expected Node variant"),
        }
        match &items[4] {
            NodeOrString::Text(s) => assert_eq!(s, " after"),
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn distinct_variants_compare_not_equal() {
        let mut dom: Dom = Dom::new();
        let n = dom.create_element("p");
        let from_node = NodeOrString::Node(n);
        let from_text = NodeOrString::Text("p".into());
        assert_ne!(from_node, from_text);
    }
}
