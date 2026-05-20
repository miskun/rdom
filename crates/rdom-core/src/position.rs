//! `compare_document_position` — DOM spec bitflag describing how two nodes
//! relate (precedes / follows / contains / is-contained-by / disconnected).
//!
//! Spec: https://dom.spec.whatwg.org/#dom-node-comparedocumentposition

use crate::bitflags_like;
use crate::dom::Dom;
use crate::node_id::NodeId;

bitflags_like! {
    /// Bitflags matching MDN's `Node.compareDocumentPosition` return value.
    /// Multiple bits can be set — e.g. `CONTAINED_BY | FOLLOWING` when the
    /// other node is a descendant (descendants are considered "following"
    /// in document order).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DocumentPosition(u16) {
        DISCONNECTED            = 0b0000_0001;
        PRECEDING               = 0b0000_0010;
        FOLLOWING               = 0b0000_0100;
        CONTAINS                = 0b0000_1000;
        CONTAINED_BY            = 0b0001_0000;
        IMPLEMENTATION_SPECIFIC = 0b0010_0000;
    }
}

impl<Ext> Dom<Ext> {
    /// Compare `a` against `b` and return a bitmask of their relationship.
    ///
    /// - `a == b` → empty bits (0).
    /// - `a` is ancestor of `b` → `CONTAINS | PRECEDING`.
    /// - `a` is descendant of `b` → `CONTAINED_BY | FOLLOWING`.
    /// - `a` precedes `b` in document order → `PRECEDING`.
    /// - `a` follows `b` → `FOLLOWING`.
    /// - different trees → `DISCONNECTED | IMPLEMENTATION_SPECIFIC | PRECEDING`.
    pub fn compare_document_position(&self, a: NodeId, b: NodeId) -> DocumentPosition {
        if a == b {
            return DocumentPosition::empty();
        }

        // Walk ancestors of each, record paths root → node.
        let a_path = self.ancestor_path(a);
        let b_path = self.ancestor_path(b);

        // Disconnected: one is not reachable from a shared ancestor.
        // For this arena we consider "disconnected" = different roots.
        match (a_path.first(), b_path.first()) {
            (Some(&ra), Some(&rb)) if ra != rb => {
                return DocumentPosition::DISCONNECTED
                    | DocumentPosition::IMPLEMENTATION_SPECIFIC
                    | DocumentPosition::PRECEDING;
            }
            (None, _) | (_, None) => {
                return DocumentPosition::DISCONNECTED
                    | DocumentPosition::IMPLEMENTATION_SPECIFIC
                    | DocumentPosition::PRECEDING;
            }
            _ => {}
        }

        // Find the common prefix length (lowest common ancestor).
        let mut common = 0;
        while common < a_path.len() && common < b_path.len() && a_path[common] == b_path[common] {
            common += 1;
        }

        // If one path is a prefix of the other, it's an ancestor relationship.
        if common == a_path.len() && common < b_path.len() {
            // b is descendant of a.
            return DocumentPosition::CONTAINS | DocumentPosition::PRECEDING;
        }
        if common == b_path.len() && common < a_path.len() {
            // a is descendant of b.
            return DocumentPosition::CONTAINED_BY | DocumentPosition::FOLLOWING;
        }

        // Otherwise we diverged at `common`. Compare child positions under
        // the common ancestor at index `common - 1`. If `common == 0`
        // something is wrong (handled by the disconnected check above).
        debug_assert!(common > 0, "compare_document_position: no LCA found");
        let lca = a_path[common - 1];
        let a_branch = a_path[common];
        let b_branch = b_path[common];

        // Which branch comes first in the child order of `lca`?
        // The returned flags describe b's position relative to a:
        // - if we encounter a_branch first → b comes after a → FOLLOWING
        // - if we encounter b_branch first → b comes before a → PRECEDING
        let mut cur = self.get_node(lca).and_then(|n| n.first_child);
        while let Some(c) = cur {
            if c == a_branch {
                return DocumentPosition::FOLLOWING;
            }
            if c == b_branch {
                return DocumentPosition::PRECEDING;
            }
            cur = self.get_node(c).and_then(|n| n.next_sibling);
        }
        // Shouldn't reach here.
        DocumentPosition::empty()
    }

    /// Is `a` equal to `b` structurally (same tag, attrs, classes, text,
    /// and recursively equal children)? Compares the tree shape — IDs +
    /// parents are not considered.
    pub fn is_equal_node(&self, a: NodeId, b: NodeId) -> bool {
        use crate::node::NodeData;
        let Some(na) = self.get_node(a) else {
            return false;
        };
        let Some(nb) = self.get_node(b) else {
            return false;
        };

        match (&na.data, &nb.data) {
            (
                NodeData::Element {
                    tag: ta,
                    attrs: aa,
                    classes: ca,
                    ..
                },
                NodeData::Element {
                    tag: tb,
                    attrs: ab,
                    classes: cb,
                    ..
                },
            ) => {
                if ta != tb || aa != ab || ca != cb {
                    return false;
                }
            }
            (NodeData::Text { data: da }, NodeData::Text { data: db }) => {
                return da == db;
            }
            (NodeData::Comment { data: da }, NodeData::Comment { data: db }) => {
                return da == db;
            }
            (NodeData::Fragment, NodeData::Fragment) => {}
            _ => return false,
        }

        // Compare children in order.
        let mut ca = na.first_child;
        let mut cb = nb.first_child;
        loop {
            match (ca, cb) {
                (None, None) => return true,
                (Some(ca_id), Some(cb_id)) => {
                    if !self.is_equal_node(ca_id, cb_id) {
                        return false;
                    }
                    ca = self.get_node(ca_id).and_then(|n| n.next_sibling);
                    cb = self.get_node(cb_id).and_then(|n| n.next_sibling);
                }
                _ => return false,
            }
        }
    }

    /// Path from root → this node as `Vec<NodeId>` (inclusive on both ends).
    /// Empty if the node isn't in the arena.
    pub fn ancestor_path(&self, id: NodeId) -> Vec<NodeId> {
        let mut path = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            if self.get_node(c).is_none() {
                return Vec::new();
            }
            path.push(c);
            cur = self.get_node(c).and_then(|n| n.parent);
        }
        path.reverse();
        path
    }

    /// Lowest common ancestor of `a` and `b` — the deepest node
    /// that contains both. Returns `None` if `a` and `b` live in
    /// different arenas or if either node is invalid.
    ///
    /// Used by the runtime for click synthesis: when `mousedown`
    /// fires on one target and `mouseup` on another, the `click`
    /// event dispatches on their common ancestor (HTML semantics).
    ///
    /// When `a == b`, returns `Some(a)`. When one is an ancestor
    /// of the other, returns the ancestor.
    pub fn common_ancestor(&self, a: NodeId, b: NodeId) -> Option<NodeId> {
        if a == b {
            return self.get_node(a).map(|_| a);
        }
        let a_path = self.ancestor_path(a);
        let b_path = self.ancestor_path(b);
        // Different roots → no common ancestor.
        match (a_path.first(), b_path.first()) {
            (Some(ra), Some(rb)) if ra != rb => return None,
            (None, _) | (_, None) => return None,
            _ => {}
        }
        // Walk both paths in lock-step from the root, keeping the
        // last matching node.
        let mut last = None;
        for (x, y) in a_path.iter().zip(b_path.iter()) {
            if x == y {
                last = Some(*x);
            } else {
                break;
            }
        }
        last
    }
}

// ─────────────────────────────────────────────────────────────────────
//  Small bitflags-without-crate helper
// ─────────────────────────────────────────────────────────────────────

/// Minimal bitflag macro so we don't pull in the `bitflags` crate for
/// a single use. Generates impls for `|`, `&`, `contains`, `empty`,
/// `bits`, `all`, `from_bits_truncate`, etc.
#[macro_export]
#[doc(hidden)]
macro_rules! bitflags_like {
    (
        $(#[$outer:meta])*
        $vis:vis struct $name:ident ( $repr:ty ) {
            $( $flag:ident = $value:expr; )+
        }
    ) => {
        $(#[$outer])*
        $vis struct $name($repr);

        impl $name {
            $( pub const $flag: Self = Self($value); )+

            #[inline] pub const fn empty() -> Self { Self(0) }
            #[inline] pub const fn all() -> Self { Self( $( $value )|+ ) }
            #[inline] pub const fn bits(self) -> $repr { self.0 }
            #[inline] pub const fn from_bits_truncate(bits: $repr) -> Self {
                Self(bits & Self::all().0)
            }
            #[inline] pub const fn contains(self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }
            #[inline] pub const fn is_empty(self) -> bool { self.0 == 0 }
            /// Clear the bits of `other` from `self`. Equivalent to
            /// `self & !other` but doesn't need a `Not` impl.
            #[inline] pub const fn without(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;
            #[inline] fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
        }
        impl std::ops::BitOrAssign for $name {
            #[inline] fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
        }
        impl std::ops::BitAnd for $name {
            type Output = Self;
            #[inline] fn bitand(self, rhs: Self) -> Self { Self(self.0 & rhs.0) }
        }
        impl std::ops::BitAndAssign for $name {
            #[inline] fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dom;

    fn build() -> (Dom, NodeId, NodeId, NodeId, NodeId) {
        // root
        //   ├─ a
        //   │    └─ grandchild
        //   └─ b
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let grandchild = dom.create_element("g");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(a, grandchild).unwrap();
        (dom, a, b, grandchild, root)
    }

    #[test]
    fn self_is_empty() {
        let (dom, a, _, _, _) = build();
        assert_eq!(
            dom.compare_document_position(a, a),
            DocumentPosition::empty()
        );
    }

    #[test]
    fn ancestor_contains_descendant() {
        let (dom, a, _, g, _) = build();
        let r = dom.compare_document_position(a, g);
        assert!(r.contains(DocumentPosition::CONTAINS));
        assert!(r.contains(DocumentPosition::PRECEDING));
    }

    #[test]
    fn descendant_contained_by_ancestor() {
        let (dom, a, _, g, _) = build();
        let r = dom.compare_document_position(g, a);
        assert!(r.contains(DocumentPosition::CONTAINED_BY));
        assert!(r.contains(DocumentPosition::FOLLOWING));
    }

    #[test]
    fn siblings_ordered_by_position() {
        let (dom, a, b, _, _) = build();
        assert!(
            dom.compare_document_position(a, b)
                .contains(DocumentPosition::FOLLOWING)
        );
        assert!(
            dom.compare_document_position(b, a)
                .contains(DocumentPosition::PRECEDING)
        );
    }

    #[test]
    fn disconnected_nodes_flagged() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a"); // orphan
        let b = dom.create_element("b"); // orphan
        let r = dom.compare_document_position(a, b);
        assert!(r.contains(DocumentPosition::DISCONNECTED));
    }

    // ── common_ancestor ──────────────────────────────────────────────

    #[test]
    fn common_ancestor_self_is_self() {
        let (dom, a, _, _, _) = build();
        assert_eq!(dom.common_ancestor(a, a), Some(a));
    }

    #[test]
    fn common_ancestor_siblings_is_parent() {
        let (dom, a, b, _, root) = build();
        assert_eq!(dom.common_ancestor(a, b), Some(root));
    }

    #[test]
    fn common_ancestor_nested_is_ancestor() {
        // g is descendant of a → common ancestor is a itself.
        let (dom, a, _, g, _) = build();
        assert_eq!(dom.common_ancestor(a, g), Some(a));
        assert_eq!(dom.common_ancestor(g, a), Some(a));
    }

    #[test]
    fn common_ancestor_cousins_is_lca() {
        // root → a → g, root → b. g and b share root.
        let (dom, _, b, g, root) = build();
        assert_eq!(dom.common_ancestor(g, b), Some(root));
    }

    #[test]
    fn common_ancestor_disconnected_returns_none() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("a"); // orphan
        let b = dom.create_element("b"); // orphan
        assert_eq!(dom.common_ancestor(a, b), None);
    }

    // ── is_equal_node ────────────────────────────────────────────────

    #[test]
    fn equal_node_same_tag_and_attrs() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let b = dom.create_element("div");
        dom.set_attribute(a, "role", "banner").unwrap();
        dom.set_attribute(b, "role", "banner").unwrap();
        assert!(dom.is_equal_node(a, b));
    }

    #[test]
    fn unequal_different_tag() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let b = dom.create_element("span");
        assert!(!dom.is_equal_node(a, b));
    }

    #[test]
    fn unequal_different_attr() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let b = dom.create_element("div");
        dom.set_attribute(a, "role", "banner").unwrap();
        dom.set_attribute(b, "role", "navigation").unwrap();
        assert!(!dom.is_equal_node(a, b));
    }

    #[test]
    fn equal_text_nodes_same_data() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_text_node("hi");
        let b = dom.create_text_node("hi");
        let c = dom.create_text_node("bye");
        assert!(dom.is_equal_node(a, b));
        assert!(!dom.is_equal_node(a, c));
    }

    #[test]
    fn equal_with_children() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let a1 = dom.create_text_node("hello");
        dom.append_child(a, a1).unwrap();

        let b = dom.create_element("div");
        let b1 = dom.create_text_node("hello");
        dom.append_child(b, b1).unwrap();

        assert!(dom.is_equal_node(a, b));
    }

    #[test]
    fn unequal_different_child_count() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let a1 = dom.create_text_node("x");
        dom.append_child(a, a1).unwrap();

        let b = dom.create_element("div");
        // no children

        assert!(!dom.is_equal_node(a, b));
    }

    #[test]
    fn unequal_different_node_types() {
        let mut dom: Dom = Dom::new();
        let a = dom.create_element("div");
        let b = dom.create_text_node("div");
        assert!(!dom.is_equal_node(a, b));
    }
}
