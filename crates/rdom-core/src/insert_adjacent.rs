//! HTML-spec sugar over the lower-level `insert_adjacent` primitive:
//! `insert_adjacent_element` and `insert_adjacent_text`. Same semantics as
//! `Element.insertAdjacentElement` / `insertAdjacentText` in browsers.
//!
//! `insert_adjacent_html` is intentionally NOT included here: markup parsing
//! belongs in `rdom-parser`, which will add `insert_adjacent_html` as an
//! extension method once Phase 15 lands.

use crate::dom::Dom;
use crate::error::Result;
use crate::node_id::NodeId;
use crate::tree::AdjacentPosition;

impl<Ext> Dom<Ext> {
    /// Insert `new_element` adjacent to `reference`. Returns the inserted
    /// element's id on success. Errors if `reference` has no parent for
    /// `BeforeBegin`/`AfterEnd`, or if the hierarchy would cycle.
    pub fn insert_adjacent_element(
        &mut self,
        reference: NodeId,
        position: AdjacentPosition,
        new_element: NodeId,
    ) -> Result<NodeId> {
        self.insert_adjacent(reference, position, new_element)?;
        Ok(new_element)
    }
}

impl<Ext: Default> Dom<Ext> {
    /// Insert a freshly-created Text node containing `text` adjacent to
    /// `reference`. Returns the new Text node's id.
    pub fn insert_adjacent_text(
        &mut self,
        reference: NodeId,
        position: AdjacentPosition,
        text: &str,
    ) -> Result<NodeId> {
        let t = self.create_text_node(text);
        self.insert_adjacent(reference, position, t)?;
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use crate::{AdjacentPosition, Dom};

    #[test]
    fn insert_adjacent_element_before_begin() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let ref_el = dom.create_element("mid");
        let before = dom.create_element("before");
        dom.append_child(root, ref_el).unwrap();
        let ret = dom
            .insert_adjacent_element(ref_el, AdjacentPosition::BeforeBegin, before)
            .unwrap();
        assert_eq!(ret, before);
        assert_eq!(dom.node(root).first_child().map(|n| n.id()), Some(before));
    }

    #[test]
    fn insert_adjacent_element_after_end() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let ref_el = dom.create_element("mid");
        let after = dom.create_element("after");
        dom.append_child(root, ref_el).unwrap();
        dom.insert_adjacent_element(ref_el, AdjacentPosition::AfterEnd, after)
            .unwrap();
        assert_eq!(dom.node(root).last_child().map(|n| n.id()), Some(after));
    }

    #[test]
    fn insert_adjacent_element_returns_id() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let parent = dom.create_element("p");
        dom.append_child(root, parent).unwrap();
        let child = dom.create_element("c");
        let id = dom
            .insert_adjacent_element(parent, AdjacentPosition::BeforeEnd, child)
            .unwrap();
        assert_eq!(id, child);
    }

    #[test]
    fn insert_adjacent_text_creates_text_node() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let el = dom.create_element("div");
        dom.append_child(root, el).unwrap();
        let tid = dom
            .insert_adjacent_text(el, AdjacentPosition::AfterBegin, "hello")
            .unwrap();
        assert_eq!(dom.node(tid).node_value(), Some("hello"));
        assert_eq!(dom.node(el).first_child().map(|n| n.id()), Some(tid));
    }

    #[test]
    fn insert_adjacent_text_at_all_positions() {
        for pos in [
            AdjacentPosition::BeforeBegin,
            AdjacentPosition::AfterBegin,
            AdjacentPosition::BeforeEnd,
            AdjacentPosition::AfterEnd,
        ] {
            let mut dom: Dom = Dom::new();
            let root = dom.root();
            let el = dom.create_element("span");
            dom.append_child(root, el).unwrap();
            let t = dom.insert_adjacent_text(el, pos, "t").unwrap();
            assert_eq!(dom.node(t).node_value(), Some("t"));
        }
    }

    #[test]
    fn insert_adjacent_element_errors_on_orphan_before_begin() {
        let mut dom: Dom = Dom::new();
        let orphan = dom.create_element("orphan"); // no parent
        let other = dom.create_element("other");
        assert!(
            dom.insert_adjacent_element(orphan, AdjacentPosition::BeforeBegin, other)
                .is_err()
        );
    }
}
