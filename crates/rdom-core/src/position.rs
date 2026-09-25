//! `compare_document_position` — DOM spec bitflag describing how two nodes
//! relate (precedes / follows / contains / is-contained-by / disconnected).
//!
//! Spec: <https://dom.spec.whatwg.org/#dom-node-comparedocumentposition>

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
    /// `a.compareDocumentPosition(b)`: a bitmask describing **`b` relative
    /// to `a`** (DOM §4.4). Same orientation as the web: the bits say where
    /// the *argument* sits.
    ///
    /// - `a == b` → empty bits (0).
    /// - `b` is a descendant of `a` → `CONTAINED_BY | FOLLOWING` (20).
    /// - `b` is an ancestor of `a` → `CONTAINS | PRECEDING` (10).
    /// - `b` comes later in tree order → `FOLLOWING`.
    /// - `b` comes earlier in tree order → `PRECEDING`.
    /// - different trees → `DISCONNECTED | IMPLEMENTATION_SPECIFIC | PRECEDING`.
    ///
    /// `FOLLOWING` therefore always means "`b` is later in tree order",
    /// whether `b` is a sibling's subtree or `a`'s own descendant.
    pub fn compare_document_position(&self, a: NodeId, b: NodeId) -> DocumentPosition {
        if a == b {
            return DocumentPosition::empty();
        }
        // DOM §4.4 step 6: different trees.
        const IN_DIFFERENT_TREES: DocumentPosition = DocumentPosition::from_bits_truncate(
            DocumentPosition::DISCONNECTED.bits()
                | DocumentPosition::IMPLEMENTATION_SPECIFIC.bits()
                | DocumentPosition::PRECEDING.bits(),
        );
        let (Some(da), Some(db)) = (self.depth_of(a), self.depth_of(b)) else {
            return IN_DIFFERENT_TREES;
        };
        // Bring both cursors to the same depth. If they meet, one node is
        // an ancestor of the other. No paths are materialized: this runs
        // per text node per frame while a selection is painted
        // (`CORE-DOCPOS-ALLOC-1`).
        let (mut x, mut y) = (a, b);
        for _ in db..da {
            x = self.parent_of(x);
        }
        for _ in da..db {
            y = self.parent_of(y);
        }
        if x == y {
            return if da < db {
                // b is a descendant of a: contained by a and, in tree
                // order, after it.
                DocumentPosition::CONTAINED_BY | DocumentPosition::FOLLOWING
            } else {
                // b is an ancestor of a: contains a and precedes it.
                DocumentPosition::CONTAINS | DocumentPosition::PRECEDING
            };
        }
        // Walk up in lockstep until the parents coincide; `x` / `y` are
        // then the two branches under the lowest common ancestor.
        let lca = loop {
            match (
                self.get_node(x).and_then(|n| n.parent),
                self.get_node(y).and_then(|n| n.parent),
            ) {
                (Some(px), Some(py)) if px == py => break px,
                (Some(px), Some(py)) => {
                    x = px;
                    y = py;
                }
                // Different roots.
                _ => return IN_DIFFERENT_TREES,
            }
        };
        // Which branch comes first among `lca`'s children? The flags
        // describe b relative to a: a's branch first → b FOLLOWING.
        let mut cur = self.get_node(lca).and_then(|n| n.first_child);
        while let Some(c) = cur {
            if c == x {
                return DocumentPosition::FOLLOWING;
            }
            if c == y {
                return DocumentPosition::PRECEDING;
            }
            cur = self.get_node(c).and_then(|n| n.next_sibling);
        }
        debug_assert!(
            false,
            "compare_document_position: branches not under their LCA"
        );
        DocumentPosition::empty()
    }

    /// Depth of `id` below its root (root = 0); `None` if `id` is not in
    /// the arena.
    fn depth_of(&self, id: NodeId) -> Option<usize> {
        let mut node = self.get_node(id)?;
        let mut depth = 0;
        while let Some(p) = node.parent {
            node = self.get_node(p)?;
            depth += 1;
        }
        Some(depth)
    }

    /// Parent of a node known to be live and non-root (callers have
    /// measured its depth).
    fn parent_of(&self, id: NodeId) -> NodeId {
        self.get_node(id)
            .and_then(|n| n.parent)
            .expect("parent_of: node has a parent (depth checked by caller)")
    }

    /// The root of the tree `id` is in; `None` if `id` is not live.
    pub(crate) fn root_of(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = id;
        let mut node = self.get_node(cur)?;
        while let Some(p) = node.parent {
            cur = p;
            node = self.get_node(cur)?;
        }
        Some(cur)
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

    /// Order two boundary points per DOM §5.2 ("position of a boundary
    /// point relative to another"). `Less` when `a` comes before `b`,
    /// `Equal` when they are the same point, `None` when the nodes are in
    /// different trees.
    ///
    /// An element position `(el, k)` sits between `el`'s children `k-1`
    /// and `k`, so it orders against a point inside child `j` by comparing
    /// `j` with `k` — not by which node contains the other.
    pub fn compare_boundary_points(
        &self,
        a: crate::Position,
        b: crate::Position,
    ) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        if a.node == b.node {
            return Some(a.offset.cmp(&b.offset));
        }
        let pos = self.compare_document_position(a.node, b.node);
        if pos.contains(DocumentPosition::DISCONNECTED) {
            return None;
        }
        // `a.node` follows `b.node` (including `a.node` inside `b.node`):
        // answer from the other side and flip.
        if pos.contains(DocumentPosition::PRECEDING) {
            return self.compare_boundary_points(b, a).map(Ordering::reverse);
        }
        // `a.node` is an ancestor of `b.node`: find `a.node`'s child on
        // the way down to `b.node` and compare its index with `a.offset`.
        if pos.contains(DocumentPosition::CONTAINED_BY) {
            // Climb from `b.node` to the child of `a.node` on its path.
            let mut child = b.node;
            loop {
                let parent = self.get_node(child)?.parent?;
                if parent == a.node {
                    break;
                }
                child = parent;
            }
            let index = self.child_index_of(child)?;
            return Some(if index < a.offset {
                Ordering::Greater
            } else {
                Ordering::Less
            });
        }
        // Plain tree order: `b.node` follows `a.node`.
        Some(Ordering::Less)
    }

    /// Index of `id` among its parent's children, `None` for a root or a
    /// freed node.
    fn child_index_of(&self, id: NodeId) -> Option<usize> {
        let parent = self.get_node(id)?.parent?;
        let mut cur = self.get_node(parent)?.first_child;
        let mut index = 0;
        while let Some(c) = cur {
            if c == id {
                return Some(index);
            }
            index += 1;
            cur = self.get_node(c)?.next_sibling;
        }
        None
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
        let (da, db) = (self.depth_of(a)?, self.depth_of(b)?);
        let (mut x, mut y) = (a, b);
        for _ in db..da {
            x = self.parent_of(x);
        }
        for _ in da..db {
            y = self.parent_of(y);
        }
        while x != y {
            // Same depth: both have parents or neither does. Neither →
            // two roots → disconnected.
            x = self.get_node(x)?.parent?;
            y = self.get_node(y)?.parent?;
        }
        Some(x)
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

    /// The original path-materializing algorithm, kept as the oracle for
    /// the allocation-free depth walk.
    fn reference_compare(dom: &Dom, a: NodeId, b: NodeId) -> DocumentPosition {
        if a == b {
            return DocumentPosition::empty();
        }
        let a_path = dom.ancestor_path(a);
        let b_path = dom.ancestor_path(b);
        if a_path.first() != b_path.first() || a_path.is_empty() {
            return DocumentPosition::DISCONNECTED
                | DocumentPosition::IMPLEMENTATION_SPECIFIC
                | DocumentPosition::PRECEDING;
        }
        let mut common = 0;
        while common < a_path.len() && common < b_path.len() && a_path[common] == b_path[common] {
            common += 1;
        }
        if common == a_path.len() {
            return DocumentPosition::CONTAINED_BY | DocumentPosition::FOLLOWING;
        }
        if common == b_path.len() {
            return DocumentPosition::CONTAINS | DocumentPosition::PRECEDING;
        }
        let lca = a_path[common - 1];
        let mut cur = dom.get_node(lca).and_then(|n| n.first_child);
        while let Some(c) = cur {
            if c == a_path[common] {
                return DocumentPosition::FOLLOWING;
            }
            if c == b_path[common] {
                return DocumentPosition::PRECEDING;
            }
            cur = dom.get_node(c).and_then(|n| n.next_sibling);
        }
        unreachable!()
    }

    /// `CORE-DOCPOS-ALLOC-1`: the depth walk agrees with the path oracle
    /// on every ordered pair of a bushy tree plus a detached subtree.
    #[test]
    fn depth_walk_matches_the_path_oracle_on_every_pair() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let mut all = vec![root];
        // Three levels, three children each: 1 + 3 + 9 + 27 nodes.
        let mut frontier = vec![root];
        for _ in 0..3 {
            let mut next = Vec::new();
            for &p in &frontier {
                for _ in 0..3 {
                    let c = dom.create_element("n");
                    dom.append_child(p, c).unwrap();
                    all.push(c);
                    next.push(c);
                }
            }
            frontier = next;
        }
        // A detached subtree with its own root.
        let loose = dom.create_element("loose");
        let loose_child = dom.create_element("lc");
        dom.append_child(loose, loose_child).unwrap();
        all.push(loose);
        all.push(loose_child);
        // A freed node.
        let gone = dom.create_element("gone");
        dom.drop_subtree(gone).unwrap();
        all.push(gone);

        for &a in &all {
            for &b in &all {
                assert_eq!(
                    dom.compare_document_position(a, b),
                    reference_compare(&dom, a, b),
                    "pair ({a:?}, {b:?})"
                );
                let expected_lca = {
                    let pa = dom.ancestor_path(a);
                    let pb = dom.ancestor_path(b);
                    pa.iter()
                        .zip(pb.iter())
                        .take_while(|(x, y)| x == y)
                        .last()
                        .map(|(x, _)| *x)
                };
                assert_eq!(
                    dom.common_ancestor(a, b),
                    expected_lca,
                    "lca ({a:?}, {b:?})"
                );
            }
        }
    }

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

    /// DOM §4.4 `compareDocumentPosition`: the bits describe `other`
    /// relative to `this`. `parent.compareDocumentPosition(child)` is
    /// `CONTAINED_BY | FOLLOWING` (20) in every browser.
    #[test]
    fn descendant_argument_is_contained_by_and_following() {
        let (dom, a, _, g, _) = build();
        let r = dom.compare_document_position(a, g);
        assert_eq!(
            r,
            DocumentPosition::CONTAINED_BY | DocumentPosition::FOLLOWING
        );
    }

    /// `child.compareDocumentPosition(parent)` is `CONTAINS | PRECEDING` (10).
    #[test]
    fn ancestor_argument_contains_and_precedes() {
        let (dom, a, _, g, _) = build();
        let r = dom.compare_document_position(g, a);
        assert_eq!(r, DocumentPosition::CONTAINS | DocumentPosition::PRECEDING);
    }

    /// The containment branch and the sibling branch must agree on what
    /// FOLLOWING means: "the argument comes later in tree order".
    #[test]
    fn following_bit_is_consistent_across_containment_and_siblings() {
        let (dom, a, b, g, _) = build();
        // g is inside a, and a precedes b — so g precedes b too.
        assert!(
            dom.compare_document_position(a, g)
                .contains(DocumentPosition::FOLLOWING)
        );
        assert!(
            dom.compare_document_position(a, b)
                .contains(DocumentPosition::FOLLOWING)
        );
        assert!(
            dom.compare_document_position(g, b)
                .contains(DocumentPosition::FOLLOWING)
        );
    }

    // ── Boundary points (DOM §5.2) ─────────────────────────────────────

    #[test]
    fn boundary_points_same_node_order_by_offset() {
        use crate::Position;
        use std::cmp::Ordering;
        let (dom, a, _, _, _) = build();
        assert_eq!(
            dom.compare_boundary_points(Position::new(a, 0), Position::new(a, 1)),
            Some(Ordering::Less)
        );
        assert_eq!(
            dom.compare_boundary_points(Position::new(a, 1), Position::new(a, 1)),
            Some(Ordering::Equal)
        );
    }

    /// (ancestor, offset) vs a point inside child `j`: the ancestor point
    /// is before iff `j >= offset`.
    #[test]
    fn boundary_points_ancestor_offset_splits_around_child_index() {
        use crate::Position;
        use std::cmp::Ordering;
        let (dom, a, _, g, root) = build();
        // root children: [a, b]; g is inside a (child index 0 of root).
        let in_g = Position::new(g, 0);
        assert_eq!(
            dom.compare_boundary_points(Position::new(root, 0), in_g),
            Some(Ordering::Less)
        );
        assert_eq!(
            dom.compare_boundary_points(Position::new(root, 1), in_g),
            Some(Ordering::Greater)
        );
        // Symmetric.
        assert_eq!(
            dom.compare_boundary_points(in_g, Position::new(root, 1)),
            Some(Ordering::Less)
        );
        assert_eq!(
            dom.compare_boundary_points(in_g, Position::new(a, 0)),
            Some(Ordering::Greater)
        );
    }

    /// An offset past the last child is "after every child" (a Range end
    /// of `(parent, childCount)`), not an error.
    #[test]
    fn boundary_point_offset_beyond_child_count_orders_after_all_children() {
        use crate::Position;
        use std::cmp::Ordering;
        let (dom, a, b, g, root) = build();
        let end = Position::new(root, 99);
        for inside in [
            Position::new(a, 0),
            Position::new(g, 0),
            Position::new(b, 0),
        ] {
            assert_eq!(
                dom.compare_boundary_points(end, inside),
                Some(Ordering::Greater)
            );
            assert_eq!(
                dom.compare_boundary_points(inside, end),
                Some(Ordering::Less)
            );
        }
    }

    #[test]
    fn boundary_points_disconnected_is_none() {
        use crate::Position;
        let (mut dom, a, _, _, _) = build();
        let loose = dom.create_element("x");
        assert_eq!(
            dom.compare_boundary_points(Position::new(a, 0), Position::new(loose, 0)),
            None
        );
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
