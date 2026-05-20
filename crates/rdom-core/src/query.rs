//! Tree-walking getters: `get_elements_by_tag_name`, `get_elements_by_class_name`.
//!
//! DOM spec getters that return *live* collections in the browser; here we
//! return a `Vec<NodeId>` snapshot — the caller owns the result and re-queries
//! if the tree has changed. Document order = pre-order DFS.
//!
//! Phase 2 ships the DFS implementation. Phase 4 will layer O(1) indexes on
//! top of the same public API (`get_elements_by_tag_name` becomes an index
//! probe; same function signature, same results).

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

impl<Ext> Dom<Ext> {
    /// Return descendants of `root_id` whose tag matches `tag`, in document
    /// order. The special wildcard `"*"` matches every element.
    ///
    /// `root_id` itself is NOT included in the result (matches browser
    /// behaviour: `element.getElementsByTagName("div")` returns descendants
    /// only). If you also want `root_id` to be considered, wrap it in a
    /// parent or use `matches` yourself.
    pub fn get_elements_by_tag_name(&self, root_id: NodeId, tag: &str) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.walk_descendants(root_id, &mut |id, data| {
            if let NodeData::Element { tag: t, .. } = data
                && (tag == "*" || t == tag)
            {
                out.push(id);
            }
        });
        out
    }

    /// Return descendants of `root_id` whose classList contains *all* of
    /// the given space-separated class names. Empty `names` returns all
    /// elements (matches DOM spec for `getElementsByClassName("")`).
    pub fn get_elements_by_class_name(&self, root_id: NodeId, names: &str) -> Vec<NodeId> {
        let wanted: Vec<&str> = names.split_ascii_whitespace().collect();
        let mut out = Vec::new();
        self.walk_descendants(root_id, &mut |id, data| {
            if let NodeData::Element { classes, .. } = data
                && wanted.iter().all(|w| classes.contains(*w))
            {
                out.push(id);
            }
        });
        out
    }

    /// Return the first descendant of `root_id` with the given id attribute.
    /// Subtree-scoped; for the arena-wide O(1) lookup use
    /// `Dom::get_element_by_id` (defined in `indexes.rs`).
    pub fn get_element_by_id_within(&self, root_id: NodeId, id_value: &str) -> Option<NodeId> {
        let mut found = None;
        self.walk_descendants(root_id, &mut |id, data| {
            if found.is_some() {
                return;
            }
            if let NodeData::Element { attrs, .. } = data
                && attrs.get("id").map(String::as_str) == Some(id_value)
            {
                found = Some(id);
            }
        });
        found
    }

    /// Pre-order DFS of descendants (excluding `root_id` itself). Calls
    /// `f(id, &data)` for each descendant Element/Text/Comment/Fragment.
    /// The closure CANNOT mutate the arena (shared borrow).
    pub(crate) fn walk_descendants<F>(&self, root_id: NodeId, f: &mut F)
    where
        F: FnMut(NodeId, &NodeData<Ext>),
    {
        let Some(root) = self.get_node(root_id) else {
            return;
        };
        let mut child = root.first_child;
        while let Some(c) = child {
            self.walk_subtree(c, f);
            child = self.get_node(c).and_then(|n| n.next_sibling);
        }
    }

    /// Pre-order DFS *including* `id` itself.
    pub(crate) fn walk_subtree<F>(&self, id: NodeId, f: &mut F)
    where
        F: FnMut(NodeId, &NodeData<Ext>),
    {
        let Some(node) = self.get_node(id) else {
            return;
        };
        f(id, &node.data);

        let mut child = node.first_child;
        while let Some(c) = child {
            self.walk_subtree(c, f);
            child = self.get_node(c).and_then(|n| n.next_sibling);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    // Build:
    //   root
    //     div#outer.alpha
    //       span.alpha.beta      "hello"
    //       span.beta
    //       section
    //         span.alpha
    //         p#target.beta
    fn build() -> (Dom, [crate::NodeId; 6]) {
        let mut dom: Dom = Dom::new();
        let root = dom.root();

        let outer = dom.create_element("div");
        dom.set_attribute(outer, "id", "outer").unwrap();
        dom.add_class(outer, "alpha").unwrap();

        let s1 = dom.create_element("span");
        dom.add_class(s1, "alpha").unwrap();
        dom.add_class(s1, "beta").unwrap();
        let t = dom.create_text_node("hello");
        dom.append_child(s1, t).unwrap();

        let s2 = dom.create_element("span");
        dom.add_class(s2, "beta").unwrap();

        let section = dom.create_element("section");
        let s3 = dom.create_element("span");
        dom.add_class(s3, "alpha").unwrap();
        let p = dom.create_element("p");
        dom.set_attribute(p, "id", "target").unwrap();
        dom.add_class(p, "beta").unwrap();

        dom.append_child(section, s3).unwrap();
        dom.append_child(section, p).unwrap();

        dom.append_child(outer, s1).unwrap();
        dom.append_child(outer, s2).unwrap();
        dom.append_child(outer, section).unwrap();

        dom.append_child(root, outer).unwrap();

        (dom, [outer, s1, s2, s3, p, section])
    }

    #[test]
    fn tag_name_returns_in_document_order() {
        let (dom, [_, s1, s2, s3, _, _]) = build();
        let root = dom.root();
        let spans = dom.get_elements_by_tag_name(root, "span");
        assert_eq!(spans, vec![s1, s2, s3]);
    }

    #[test]
    fn tag_name_wildcard_matches_every_element() {
        let (dom, _) = build();
        let root = dom.root();
        let all = dom.get_elements_by_tag_name(root, "*");
        // outer, s1, s2, section, s3, p  =  6 elements (text nodes excluded).
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn tag_name_excludes_root_itself() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        // Asking for "div" on this orphan returns nothing — root not included.
        assert!(dom.get_elements_by_tag_name(div, "div").is_empty());
    }

    #[test]
    fn class_name_single_class_matches() {
        let (dom, [outer, s1, _, s3, _, _]) = build();
        let root = dom.root();
        let alphas = dom.get_elements_by_class_name(root, "alpha");
        assert_eq!(alphas, vec![outer, s1, s3]);
    }

    #[test]
    fn class_name_multiple_classes_requires_all() {
        let (dom, [_, s1, _, _, _, _]) = build();
        let root = dom.root();
        let ab = dom.get_elements_by_class_name(root, "alpha beta");
        // Only s1 has both.
        assert_eq!(ab, vec![s1]);
    }

    #[test]
    fn class_name_empty_returns_all_elements() {
        let (dom, _) = build();
        let root = dom.root();
        let any = dom.get_elements_by_class_name(root, "");
        assert_eq!(any.len(), 6);
    }

    #[test]
    fn class_name_whitespace_is_tolerated() {
        let (dom, [_, s1, _, _, _, _]) = build();
        let root = dom.root();
        let ab = dom.get_elements_by_class_name(root, "  alpha   beta  ");
        assert_eq!(ab, vec![s1]);
    }

    #[test]
    fn element_by_id_finds_match() {
        let (dom, [outer, _, _, _, p, _]) = build();
        let root = dom.root();
        assert_eq!(dom.get_element_by_id_within(root, "outer"), Some(outer));
        assert_eq!(dom.get_element_by_id_within(root, "target"), Some(p));
    }

    #[test]
    fn element_by_id_missing_returns_none() {
        let (dom, _) = build();
        let root = dom.root();
        assert!(dom.get_element_by_id_within(root, "nope").is_none());
    }

    #[test]
    fn element_by_id_scoped_to_subtree() {
        let (dom, [_, _, _, _, p, section]) = build();
        // section contains p#target. From section we can find it.
        assert_eq!(dom.get_element_by_id_within(section, "target"), Some(p));
        // From p itself (no descendants) we can't find itself.
        assert!(dom.get_element_by_id_within(p, "target").is_none());
    }
}
