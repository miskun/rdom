//! `NodeRef<'a, Ext>` / `NodeMut<'a, Ext>` — ergonomic wrappers around
//! `(&Dom, NodeId)` / `(&mut Dom, NodeId)` pairs so call sites read DOM-ish:
//!
//! ```ignore
//! dom.node(row).get_attribute("data-sel");
//! dom.node_mut(hero).set_attribute("role", "banner");
//! ```

use crate::dom::Dom;
use crate::dom_string_map::{DomStringMap, DomStringMapMut};
use crate::error::{DomError, Result};
use crate::node::{NodeData, NodeType};
use crate::node_id::NodeId;
use crate::node_list::NodeList;
use crate::node_or_string::NodeOrString;
use crate::token_list::{DomTokenList, DomTokenListMut};
use crate::tree::AdjacentPosition;

// ─────────────────────────────────────────────────────────────────────
//  NodeRef
// ─────────────────────────────────────────────────────────────────────

/// Read-only handle to a node in the arena.
#[derive(Clone, Copy)]
pub struct NodeRef<'a, Ext: 'static = ()> {
    pub(crate) dom: &'a Dom<Ext>,
    pub(crate) id: NodeId,
}

impl<'a, Ext: 'static> NodeRef<'a, Ext> {
    // ── Identity ──────────────────────────────────────────────────

    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Borrow the owning `Dom<Ext>`. Lifetime `'a` matches the
    /// underlying borrow that produced this `NodeRef`, so callers can
    /// hold the returned reference for the same scope.
    ///
    /// Exposed for extension traits (e.g. `rdom-tui::TuiAccessors`)
    /// that need to reach back to dom-level helpers operating on
    /// `(dom, id)` pairs (runtime focus, builtin form helpers, etc.).
    pub fn dom(&self) -> &'a Dom<Ext> {
        self.dom
    }

    /// # Panics
    ///
    /// Panics if the id behind this `NodeRef` is not live (freed, or a
    /// stale handle to a recycled slot). `Dom::node` does not validate
    /// up front; check `Dom::contains` first when the id may be stale.
    pub fn node_type(&self) -> NodeType {
        self.dom
            .get_node(self.id)
            .map(|n| n.node_type())
            .unwrap_or_else(|| panic!("NodeRef::node_type on a node that is not live: {}", self.id))
    }

    /// Canonical `nodeName`: element tag, or `#text` / `#comment` /
    /// `#document-fragment` for non-elements.
    ///
    /// # Panics
    ///
    /// Panics if the id is not live; see [`NodeRef::node_type`].
    pub fn node_name(&self) -> &'a str {
        let n = self.dom.get_node(self.id).unwrap_or_else(|| {
            panic!("NodeRef::node_name on a node that is not live: {}", self.id)
        });
        match &n.data {
            NodeData::Element { tag, .. } => tag,
            NodeData::Text { .. } => "#text",
            NodeData::Comment { .. } => "#comment",
            NodeData::Fragment => "#document-fragment",
        }
    }

    pub fn tag_name(&self) -> Option<&'a str> {
        self.dom.get_node(self.id).and_then(|n| n.tag_name())
    }

    /// Borrow the per-element extension data.
    ///
    /// This is the hook by which rdom-tui (or any downstream crate
    /// parameterizing `Dom<Ext>`) reads presentation / layout / styling
    /// state attached to each Element. Returns `None` for Text / Comment /
    /// Fragment nodes — those don't carry `Ext`.
    pub fn ext(&self) -> Option<&'a Ext> {
        match &self.dom.get_node(self.id)?.data {
            NodeData::Element { ext, .. } => Some(ext),
            _ => None,
        }
    }

    pub fn node_value(&self) -> Option<&'a str> {
        match &self.dom.get_node(self.id)?.data {
            NodeData::Text { data } | NodeData::Comment { data } => Some(data),
            _ => None,
        }
    }

    /// `CharacterData.data` — MDN alias for Text/Comment `nodeValue`. Same
    /// behaviour, different name; exists because the spec treats these as
    /// two different interface members.
    pub fn data(&self) -> Option<&'a str> {
        self.node_value()
    }

    /// `textContent` — concatenate the string content of this node and all
    /// its descendants. Comments are excluded per spec.
    pub fn text_content(&self) -> String {
        self.dom.text_content(self.id)
    }

    // ── Tree: core navigation ─────────────────────────────────────

    pub fn parent_node(&self) -> Option<NodeRef<'a, Ext>> {
        let p = self.dom.get_node(self.id)?.parent?;
        Some(NodeRef {
            dom: self.dom,
            id: p,
        })
    }

    /// Parent that is an element (skips fragment parents).
    pub fn parent_element(&self) -> Option<NodeRef<'a, Ext>> {
        let mut current = self.parent_node();
        while let Some(p) = current {
            if p.node_type() == NodeType::Element {
                return Some(p);
            }
            current = p.parent_node();
        }
        None
    }

    pub fn first_child(&self) -> Option<NodeRef<'a, Ext>> {
        let f = self.dom.get_node(self.id)?.first_child?;
        Some(NodeRef {
            dom: self.dom,
            id: f,
        })
    }

    pub fn last_child(&self) -> Option<NodeRef<'a, Ext>> {
        let l = self.dom.get_node(self.id)?.last_child?;
        Some(NodeRef {
            dom: self.dom,
            id: l,
        })
    }

    pub fn previous_sibling(&self) -> Option<NodeRef<'a, Ext>> {
        let p = self.dom.get_node(self.id)?.prev_sibling?;
        Some(NodeRef {
            dom: self.dom,
            id: p,
        })
    }

    pub fn next_sibling(&self) -> Option<NodeRef<'a, Ext>> {
        let n = self.dom.get_node(self.id)?.next_sibling?;
        Some(NodeRef {
            dom: self.dom,
            id: n,
        })
    }

    pub fn has_child_nodes(&self) -> bool {
        self.dom
            .get_node(self.id)
            .and_then(|n| n.first_child)
            .is_some()
    }

    pub fn child_nodes(&self) -> ChildIter<'a, Ext> {
        ChildIter {
            dom: self.dom,
            next: self.dom.get_node(self.id).and_then(|n| n.first_child),
        }
    }

    // ── Element-only navigation ────────────────────────────────────

    pub fn first_element_child(&self) -> Option<NodeRef<'a, Ext>> {
        let mut c = self.first_child();
        while let Some(n) = c {
            if n.node_type() == NodeType::Element {
                return Some(n);
            }
            c = n.next_sibling();
        }
        None
    }

    pub fn last_element_child(&self) -> Option<NodeRef<'a, Ext>> {
        let mut c = self.last_child();
        while let Some(n) = c {
            if n.node_type() == NodeType::Element {
                return Some(n);
            }
            c = n.previous_sibling();
        }
        None
    }

    pub fn previous_element_sibling(&self) -> Option<NodeRef<'a, Ext>> {
        let mut s = self.previous_sibling();
        while let Some(n) = s {
            if n.node_type() == NodeType::Element {
                return Some(n);
            }
            s = n.previous_sibling();
        }
        None
    }

    pub fn next_element_sibling(&self) -> Option<NodeRef<'a, Ext>> {
        let mut s = self.next_sibling();
        while let Some(n) = s {
            if n.node_type() == NodeType::Element {
                return Some(n);
            }
            s = n.next_sibling();
        }
        None
    }

    pub fn children(&self) -> ElementChildIter<'a, Ext> {
        ElementChildIter {
            inner: self.child_nodes(),
        }
    }

    pub fn child_element_count(&self) -> usize {
        self.children().count()
    }

    // ── Attributes / classes ──────────────────────────────────────

    pub fn id_attr(&self) -> Option<&'a str> {
        self.get_attribute("id")
    }

    pub fn get_attribute(&self, key: &str) -> Option<&'a str> {
        self.dom.get_attribute(self.id, key)
    }

    pub fn has_attribute(&self, key: &str) -> bool {
        self.dom.has_attribute(self.id, key)
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.dom.has_class(self.id, class)
    }

    /// Iterate `(name, value)` pairs in deterministic order.
    pub fn attributes(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.dom.attributes(self.id)
    }

    /// Raw `class` attribute value, or `""` if absent. DOM
    /// `Element.className`.
    ///
    /// This is the unparsed string. For token-level access use
    /// [`Self::class_list`]; for hot-path membership tests use
    /// [`Self::has_class`].
    pub fn class_name(&self) -> &'a str {
        self.get_attribute("class").unwrap_or("")
    }

    /// Snapshot of the element's class tokens. DOM
    /// `Element.classList`.
    ///
    /// **Hot-path footgun.** Each call allocates a fresh `Vec`
    /// snapshot. For per-paint or per-event membership checks,
    /// prefer [`Self::has_class`].
    pub fn class_list(&self) -> DomTokenList {
        DomTokenList::from_tokens(self.dom.class_list(self.id).map(str::to_owned))
    }

    // ── Predicates ────────────────────────────────────────────────

    pub fn contains(&self, other: NodeId) -> bool {
        self.dom.is_ancestor(self.id, other)
    }

    pub fn is_same_node(&self, other: NodeId) -> bool {
        self.id == other
    }

    /// `true` iff this node is reachable from the document root by
    /// walking parent pointers. DOM `Node.isConnected`.
    pub fn is_connected(&self) -> bool {
        self.dom.is_ancestor(self.dom.root(), self.id)
    }

    /// Walk parent pointers until none and return the topmost node.
    /// DOM `Node.getRootNode()` (options ignored — no shadow DOM in
    /// rdom).
    pub fn get_root_node(&self) -> NodeRef<'a, Ext> {
        let mut cur = self.id;
        loop {
            match self.dom.get_node(cur).and_then(|n| n.parent) {
                Some(p) => cur = p,
                None => {
                    return NodeRef {
                        dom: self.dom,
                        id: cur,
                    };
                }
            }
        }
    }

    // ── Selector queries (element-rooted) ─────────────────────────

    /// Does this element match `selector`? DOM `Element.matches`.
    ///
    /// **Divergence from browser:** browser throws `SyntaxError` on
    /// malformed selectors; rdom returns `false`. Authors who need
    /// to surface parser errors can call
    /// [`Dom::matches`](crate::Dom::matches) directly.
    pub fn matches(&self, selector: &str) -> bool {
        self.dom.matches(self.id, selector).unwrap_or(false)
    }

    /// Walk from this node (inclusive) up the ancestor chain and
    /// return the first match. DOM `Element.closest`.
    ///
    /// Same parser-error policy as [`Self::matches`].
    pub fn closest(&self, selector: &str) -> Option<NodeRef<'a, Ext>> {
        self.dom
            .closest(self.id, selector)
            .ok()
            .flatten()
            .map(|id| NodeRef { dom: self.dom, id })
    }

    /// First descendant matching `selector`, in document order. DOM
    /// `Element.querySelector`. The subject element itself is **not**
    /// a candidate (spec).
    ///
    /// Same parser-error policy as [`Self::matches`].
    pub fn query_selector(&self, selector: &str) -> Option<NodeRef<'a, Ext>> {
        self.dom
            .query_selector_in(self.id, selector)
            .ok()
            .flatten()
            .filter(|&id| id != self.id)
            .map(|id| NodeRef { dom: self.dom, id })
    }

    /// All descendants matching `selector`, in document order, as a
    /// snapshot `NodeList`. DOM `Element.querySelectorAll`. The
    /// subject element itself is **not** a candidate.
    ///
    /// Same parser-error policy as [`Self::matches`]; an unparseable
    /// selector yields an empty list.
    pub fn query_selector_all(&self, selector: &str) -> NodeList<'a, Ext> {
        let ids = self
            .dom
            .query_selector_all_in(self.id, selector)
            .unwrap_or_default()
            .into_iter()
            .filter(|&id| id != self.id);
        NodeList::from_ids(self.dom, ids)
    }

    // ── HTMLElement IDL accessors (rdom-core, raw shapes) ─────────

    /// DOM `HTMLElement.dataset` — snapshot view of `data-*`
    /// attributes keyed by their camelCase form.
    pub fn dataset(&self) -> DomStringMap<'a, Ext> {
        DomStringMap::new(NodeRef {
            dom: self.dom,
            id: self.id,
        })
    }

    /// DOM `HTMLElement.tabIndex` — raw `tabindex` attribute
    /// parsed as `i32`. `None` when the attribute is absent or
    /// unparseable.
    ///
    /// **Note:** this is the *raw* value. The TUI-aware effective
    /// tab index (honoring implicit focusability per
    /// `runtime::focus::tabindex`) lives on
    /// `rdom_tui::TuiAccessors::effective_tab_index` per §13 and
    /// ships in M4b step 21.
    pub fn tab_index(&self) -> Option<i32> {
        self.get_attribute("tabindex")?.parse().ok()
    }

    /// DOM `HTMLElement.hidden` — `true` iff the `hidden` boolean
    /// attribute is present.
    pub fn hidden(&self) -> bool {
        self.has_attribute("hidden")
    }

    /// DOM `HTMLElement.contentEditable` — IDL string. Returns the
    /// raw attribute when set, else `"inherit"` per spec.
    pub fn content_editable(&self) -> &'a str {
        self.get_attribute("contenteditable").unwrap_or("inherit")
    }

    /// DOM `Element.innerHTML` getter — markup of this element's
    /// children. Delegates to `Dom::inner_markup`.
    ///
    /// **Hot-path footgun.** Each call re-serializes the entire
    /// subtree; for tight loops, prefer attribute / child walks
    /// against the live tree.
    pub fn inner_html(&self) -> String {
        self.dom.inner_markup(self.id)
    }

    /// DOM `Element.outerHTML` getter — markup of this element
    /// including itself. Delegates to `Dom::outer_markup`.
    pub fn outer_html(&self) -> String {
        self.dom.outer_markup(self.id)
    }
}

mod node_mut;
pub use node_mut::NodeMut;

// ─────────────────────────────────────────────────────────────────────
//  Iterators
// ─────────────────────────────────────────────────────────────────────

/// Iterator over all direct children (any node type), in document order.
pub struct ChildIter<'a, Ext: 'static> {
    dom: &'a Dom<Ext>,
    next: Option<NodeId>,
}

impl<'a, Ext: 'static> Iterator for ChildIter<'a, Ext> {
    type Item = NodeRef<'a, Ext>;
    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = self.dom.get_node(current).and_then(|n| n.next_sibling);
        Some(NodeRef {
            dom: self.dom,
            id: current,
        })
    }
}

/// Iterator over element children only.
pub struct ElementChildIter<'a, Ext: 'static> {
    inner: ChildIter<'a, Ext>,
}

impl<'a, Ext: 'static> Iterator for ElementChildIter<'a, Ext> {
    type Item = NodeRef<'a, Ext>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .by_ref()
            .find(|n| n.node_type() == NodeType::Element)
    }
}

// ─────────────────────────────────────────────────────────────────────
//  Dom -> accessor helpers
// ─────────────────────────────────────────────────────────────────────

impl<Ext> Dom<Ext> {
    /// Read accessor for `id`. Construction does not validate the id:
    /// most `NodeRef` methods return `None` / empty for a dead id, but
    /// the ones that have no empty value (`node_type`, `node_name`)
    /// panic. Check [`Dom::contains`] first when the id may be stale.
    pub fn node(&self, id: NodeId) -> NodeRef<'_, Ext> {
        NodeRef { dom: self, id }
    }

    /// Mutable accessor for `id`. Same validation contract as
    /// [`Dom::node`]: mutation methods return `Err(InvalidNode)` for a
    /// dead id.
    pub fn node_mut(&mut self, id: NodeId) -> NodeMut<'_, Ext> {
        NodeMut { dom: self, id }
    }

    /// Convenience: `NodeRef` for the root.
    pub fn root_ref(&self) -> NodeRef<'_, Ext> {
        self.node(self.root())
    }
}

#[cfg(test)]
mod tests;
