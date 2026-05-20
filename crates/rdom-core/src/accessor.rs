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

    pub fn node_type(&self) -> NodeType {
        self.dom.get_node(self.id).map(|n| n.node_type()).unwrap()
    }

    /// Canonical `nodeName`: element tag, or `#text` / `#comment` /
    /// `#document-fragment` for non-elements.
    pub fn node_name(&self) -> &'a str {
        let n = self.dom.get_node(self.id).unwrap();
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

// ─────────────────────────────────────────────────────────────────────
//  NodeMut
// ─────────────────────────────────────────────────────────────────────

/// Mutable handle. All mutations go through this wrapper so future index
/// maintenance hooks (Phase 4) have a single chokepoint.
pub struct NodeMut<'a, Ext: 'static = ()> {
    pub(crate) dom: &'a mut Dom<Ext>,
    pub(crate) id: NodeId,
}

impl<'a, Ext> NodeMut<'a, Ext> {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn as_ref(&self) -> NodeRef<'_, Ext> {
        NodeRef {
            dom: self.dom,
            id: self.id,
        }
    }

    /// Reborrow the inner `&mut Dom<Ext>`. Lets downstream crates
    /// (notably `rdom-parser`'s `NodeMutHtml` extension trait)
    /// reach Dom-level operations that aren't surfaced on
    /// `NodeMut` directly. The borrow shares the receiver's
    /// lifetime.
    pub fn dom_mut(&mut self) -> &mut Dom<Ext> {
        self.dom
    }

    /// Consume this `NodeMut` and return the inner `&'a mut
    /// Dom<Ext>`. Used by extension traits whose methods need to
    /// operate on the Dom past the receiver's logical lifetime
    /// (e.g. `set_outer_html`, which destroys the receiver).
    pub fn into_dom_mut(self) -> &'a mut Dom<Ext> {
        self.dom
    }

    /// Mutable borrow of the per-element extension data. `None` for
    /// Text / Comment / Fragment. Pair of `NodeRef::ext()`.
    pub fn ext_mut(&mut self) -> Option<&mut Ext> {
        match &mut self.dom.get_node_mut(self.id)?.data {
            NodeData::Element { ext, .. } => Some(ext),
            _ => None,
        }
    }
}

impl<'a, Ext: 'static> NodeMut<'a, Ext> {
    // ── Attributes ────────────────────────────────────────────────

    pub fn set_attribute(&mut self, key: &str, value: &str) -> Result<()> {
        self.dom.set_attribute(self.id, key, value)
    }

    pub fn remove_attribute(&mut self, key: &str) -> Result<bool> {
        self.dom.remove_attribute(self.id, key)
    }

    pub fn toggle_attribute(&mut self, key: &str) -> Result<bool> {
        self.dom.toggle_attribute(self.id, key)
    }

    pub fn set_id(&mut self, value: &str) -> Result<()> {
        self.dom.set_id(self.id, value)
    }

    pub fn add_class(&mut self, class: &str) -> Result<()> {
        self.dom.add_class(self.id, class)
    }

    pub fn remove_class(&mut self, class: &str) -> Result<bool> {
        self.dom.remove_class(self.id, class)
    }

    pub fn toggle_class(&mut self, class: &str) -> Result<bool> {
        self.dom.toggle_class(self.id, class)
    }

    pub fn replace_class(&mut self, old: &str, new: &str) -> Result<bool> {
        self.dom.replace_class(self.id, old, new)
    }

    /// Replace the entire `class` attribute. DOM
    /// `Element.className` setter.
    ///
    /// Writes the raw attribute string AND rebuilds the canonical
    /// classList from whitespace-separated tokens — both reads
    /// ([`NodeRef::class_name`] and [`NodeRef::class_list`]) reflect
    /// the new value after this call. Empty `value` clears the
    /// classList entirely.
    pub fn set_class_name(&mut self, value: &str) -> Result<()> {
        let existing: Vec<String> = self.dom.class_list(self.id).map(str::to_owned).collect();
        for cls in &existing {
            self.dom.remove_class(self.id, cls)?;
        }
        self.dom.set_attribute(self.id, "class", value)?;
        for tok in value.split_whitespace() {
            self.dom.add_class(self.id, tok)?;
        }
        Ok(())
    }

    /// Mutating handle for the element's class tokens. DOM
    /// `Element.classList`.
    ///
    /// The returned wrapper holds a reborrowed `NodeMut`; drop the
    /// wrapper to release the borrow before mutating other fields
    /// on this element.
    pub fn class_list_mut(&mut self) -> DomTokenListMut<'_, Ext> {
        DomTokenListMut::new(NodeMut {
            dom: &mut *self.dom,
            id: self.id,
        })
    }

    /// DOM `HTMLElement.dataset` mutator — write-side handle for
    /// `data-*` attributes keyed by their camelCase form. The
    /// returned wrapper holds a reborrowed `NodeMut`; drop it
    /// before mutating other fields on this element.
    pub fn dataset_mut(&mut self) -> DomStringMapMut<'_, Ext> {
        DomStringMapMut::new(NodeMut {
            dom: &mut *self.dom,
            id: self.id,
        })
    }

    /// DOM `HTMLElement.tabIndex` setter. Writes the integer value
    /// to the `tabindex` attribute as its decimal string form.
    pub fn set_tab_index(&mut self, value: i32) -> Result<()> {
        self.set_attribute("tabindex", &value.to_string())
    }

    /// DOM `HTMLElement.hidden` setter. `true` writes the boolean
    /// attribute (empty value); `false` removes it.
    pub fn set_hidden(&mut self, value: bool) -> Result<()> {
        if value {
            self.set_attribute("hidden", "")
        } else {
            self.remove_attribute("hidden").map(|_| ())
        }
    }

    /// DOM `HTMLElement.contentEditable` setter. Writes the
    /// `contenteditable` attribute literally; the spec-recognized
    /// values are `"true"`, `"false"`, `"plaintext-only"`,
    /// `"inherit"`, but any string is accepted (browser-faithful —
    /// the IDL doesn't validate at assignment time).
    pub fn set_content_editable(&mut self, value: &str) -> Result<()> {
        self.set_attribute("contenteditable", value)
    }

    /// Toggle an attribute with optional force. DOM
    /// `Element.toggleAttribute(qualifiedName, force?)`.
    ///
    /// - `force = Some(true)` → ensure present (empty string value
    ///   if newly added); returns `true`.
    /// - `force = Some(false)` → ensure absent; returns `false`.
    /// - `force = None` → flip; returns the post-flip presence.
    pub fn toggle_attribute_force(&mut self, name: &str, force: Option<bool>) -> Result<bool> {
        match force {
            Some(true) => {
                if !self.as_ref().has_attribute(name) {
                    self.set_attribute(name, "")?;
                }
                Ok(true)
            }
            Some(false) => {
                self.remove_attribute(name)?;
                Ok(false)
            }
            None => self.toggle_attribute(name),
        }
    }

    // ── Tree mutation ─────────────────────────────────────────────

    pub fn append_child(&mut self, child: NodeId) -> Result<()> {
        self.dom.append_child(self.id, child)
    }

    pub fn prepend_child(&mut self, child: NodeId) -> Result<()> {
        self.dom.prepend_child(self.id, child)
    }

    pub fn remove_child(&mut self, child: NodeId) -> Result<()> {
        self.dom.remove_child(self.id, child)
    }

    pub fn replace_child(&mut self, old: NodeId, new: NodeId) -> Result<()> {
        self.dom.replace_child(self.id, old, new)
    }

    pub fn insert_before(&mut self, new: NodeId, reference: Option<NodeId>) -> Result<()> {
        self.dom.insert_before(self.id, new, reference)
    }

    pub fn insert_adjacent(&mut self, position: AdjacentPosition, new: NodeId) -> Result<()> {
        self.dom.insert_adjacent(self.id, position, new)
    }

    pub fn clear_children(&mut self) -> Result<()> {
        self.dom.clear_children(self.id)
    }

    // ── Variadic tree helpers (DOM `ChildNode` / `ParentNode`) ────

    /// Append each item to the end of this node's child list, in
    /// order. Text items create fresh text nodes. DOM
    /// `ParentNode.append`.
    pub fn append(&mut self, children: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let parent = self.id;
        for item in children {
            let new_id = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => self.dom.create_text_node(&s),
            };
            self.dom.append_child(parent, new_id)?;
        }
        Ok(())
    }

    /// Insert each item at the start of this node's child list, in
    /// order — the first item of `children` becomes the new first
    /// child. DOM `ParentNode.prepend`.
    pub fn prepend(&mut self, children: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let parent = self.id;
        let reference = self.dom.get_node(parent).and_then(|n| n.first_child);
        for item in children {
            let new_id = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => self.dom.create_text_node(&s),
            };
            self.dom.insert_before(parent, new_id, reference)?;
        }
        Ok(())
    }

    /// Insert each item as a sibling immediately before this node,
    /// in order. DOM `ChildNode.before`.
    ///
    /// Silently no-ops when this node has no parent (browser-
    /// faithful). Text items create fresh text nodes only when
    /// insertion actually happens.
    pub fn before(&mut self, siblings: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let id = self.id;
        let parent = match self.dom.get_node(id).and_then(|n| n.parent) {
            Some(p) => p,
            None => return Ok(()),
        };
        for item in siblings {
            let new_id = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => self.dom.create_text_node(&s),
            };
            self.dom.insert_before(parent, new_id, Some(id))?;
        }
        Ok(())
    }

    /// Insert each item as a sibling immediately after this node,
    /// in order. DOM `ChildNode.after`.
    ///
    /// Silently no-ops when this node has no parent (browser-
    /// faithful).
    pub fn after(&mut self, siblings: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let id = self.id;
        if self.dom.get_node(id).and_then(|n| n.parent).is_none() {
            return Ok(());
        }
        let mut cursor = id;
        for item in siblings {
            let new_id = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => self.dom.create_text_node(&s),
            };
            self.dom
                .insert_adjacent(cursor, AdjacentPosition::AfterEnd, new_id)?;
            cursor = new_id;
        }
        Ok(())
    }

    /// Clear this node's children and append the new ones. DOM
    /// `ParentNode.replaceChildren`.
    pub fn replace_children(
        &mut self,
        children: impl IntoIterator<Item = NodeOrString>,
    ) -> Result<()> {
        let parent = self.id;
        self.dom.clear_children(parent)?;
        for item in children {
            let new_id = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => self.dom.create_text_node(&s),
            };
            self.dom.append_child(parent, new_id)?;
        }
        Ok(())
    }

    /// Replace this node with `siblings`, inserted at its position
    /// in the parent, then detach this node. DOM
    /// `ChildNode.replaceWith`.
    ///
    /// **Consumes `self`** — the receiver is detached from the
    /// tree, so the handle is no longer usable. Silently no-ops
    /// when this node has no parent.
    ///
    /// ```compile_fail
    /// use rdom_core::Dom;
    /// let mut dom: Dom = Dom::new();
    /// let parent = dom.create_element("div");
    /// let el = dom.create_element("span");
    /// dom.append_child(parent, el).unwrap();
    /// let nm = dom.node_mut(el);
    /// nm.replace_with([]).unwrap();
    /// let _ = nm.id();  // ERROR: nm was consumed
    /// ```
    pub fn replace_with(self, siblings: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let NodeMut { dom, id } = self;
        let parent = match dom.get_node(id).and_then(|n| n.parent) {
            Some(p) => p,
            None => return Ok(()),
        };
        for item in siblings {
            let new_id = match item {
                NodeOrString::Node(n) => n,
                NodeOrString::Text(s) => dom.create_text_node(&s),
            };
            dom.insert_before(parent, new_id, Some(id))?;
        }
        dom.remove_child(parent, id)?;
        Ok(())
    }

    /// Detach this node from its parent. DOM `ChildNode.remove`.
    ///
    /// **Consumes `self`**. Silently no-ops on parentless nodes.
    /// The node remains in the arena — it's just orphaned.
    ///
    /// ```compile_fail
    /// use rdom_core::Dom;
    /// let mut dom: Dom = Dom::new();
    /// let parent = dom.create_element("div");
    /// let el = dom.create_element("span");
    /// dom.append_child(parent, el).unwrap();
    /// let nm = dom.node_mut(el);
    /// nm.remove_self().unwrap();
    /// let _ = nm.id();  // ERROR: nm was consumed
    /// ```
    pub fn remove_self(self) -> Result<()> {
        let NodeMut { dom, id } = self;
        if let Some(parent) = dom.get_node(id).and_then(|n| n.parent) {
            dom.remove_child(parent, id)?;
        }
        Ok(())
    }

    /// Set Text/Comment node's own data. Errors on Element/Fragment.
    /// Fires `Mutation::CharacterDataChanged`.
    pub fn set_node_value(&mut self, data: &str) -> Result<()> {
        let id = self.id;
        let old = match &self.dom.node_or_err(id)?.data {
            NodeData::Text { data: d } | NodeData::Comment { data: d } => d.clone(),
            NodeData::Element { .. } => {
                return Err(DomError::WrongNodeType {
                    expected: "Text or Comment",
                    got: NodeType::Element,
                });
            }
            NodeData::Fragment => {
                return Err(DomError::WrongNodeType {
                    expected: "Text or Comment",
                    got: NodeType::Fragment,
                });
            }
        };
        if old == data {
            return Ok(());
        }
        match &mut self.dom.node_mut_or_err(id)?.data {
            NodeData::Text { data: d } | NodeData::Comment { data: d } => {
                *d = data.to_string();
            }
            _ => unreachable!("type-checked above"),
        }
        self.dom
            .fire_mutation(crate::Mutation::CharacterDataChanged {
                id,
                old,
                new: data.to_string(),
            });
        Ok(())
    }

    /// `CharacterData.data` setter — alias for `set_node_value` on Text/
    /// Comment.
    pub fn set_data(&mut self, data: &str) -> Result<()> {
        self.set_node_value(data)
    }

    /// Replace the byte range `[start..end)` of a Text/Comment node's
    /// data with `replacement`. Convenience over `set_node_value` for
    /// editors that want byte-precise mutations (insert, delete,
    /// replace a range) without assembling the full new string.
    ///
    /// Errors:
    /// - `WrongNodeType` — node isn't Text/Comment.
    /// - `InvalidOffset` — `start` or `end` overshoot the data length
    ///   or land mid-UTF-8-codepoint. Editors that derive offsets
    ///   from `Position` / grapheme walks won't hit this.
    ///
    /// Fires `Mutation::CharacterDataChanged` (via `set_node_value`).
    pub fn edit_text(&mut self, start: usize, end: usize, replacement: &str) -> Result<()> {
        let id = self.id;
        let data = match &self.dom.node_or_err(id)?.data {
            NodeData::Text { data: d } | NodeData::Comment { data: d } => d.clone(),
            NodeData::Element { .. } => {
                return Err(DomError::WrongNodeType {
                    expected: "Text or Comment",
                    got: NodeType::Element,
                });
            }
            NodeData::Fragment => {
                return Err(DomError::WrongNodeType {
                    expected: "Text or Comment",
                    got: NodeType::Fragment,
                });
            }
        };
        if start > data.len() || !data.is_char_boundary(start) {
            return Err(DomError::InvalidOffset {
                node: id,
                offset: start,
            });
        }
        let end = end.max(start);
        if end > data.len() || !data.is_char_boundary(end) {
            return Err(DomError::InvalidOffset {
                node: id,
                offset: end,
            });
        }
        let mut new_data = String::with_capacity(data.len() - (end - start) + replacement.len());
        new_data.push_str(&data[..start]);
        new_data.push_str(replacement);
        new_data.push_str(&data[end..]);
        self.set_node_value(&new_data)
    }
}

impl<'a, Ext: Default> NodeMut<'a, Ext> {
    /// `textContent` setter — replace all children of this Element/Fragment
    /// with a single Text node. Errors on Text/Comment (use `set_data`).
    pub fn set_text_content(&mut self, text: &str) -> Result<()> {
        self.dom.set_text_content(self.id, text)
    }
}

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
    pub fn node(&self, id: NodeId) -> NodeRef<'_, Ext> {
        NodeRef { dom: self, id }
    }

    pub fn node_mut(&mut self, id: NodeId) -> NodeMut<'_, Ext> {
        NodeMut { dom: self, id }
    }

    /// Convenience: `NodeRef` for the root.
    pub fn root_ref(&self) -> NodeRef<'_, Ext> {
        self.node(self.root())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(dom: &mut Dom) -> (NodeId, NodeId, NodeId, NodeId) {
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();
        (root, a, b, c)
    }

    #[test]
    fn dom_accessor_returns_owning_dom() {
        let mut dom: Dom = Dom::new();
        let (root, a, _b, _c) = build(&mut dom);
        let node = dom.node(a);
        // dom() returns the same arena `node()` was built from —
        // tree shape observed through it matches.
        let via_dom = node.dom();
        assert_eq!(via_dom.root(), root);
        assert_eq!(via_dom.node(a).parent_node().map(|p| p.id()), Some(root));
    }

    #[test]
    fn navigation_via_noderef() {
        let mut dom: Dom = Dom::new();
        let (root, a, b, c) = build(&mut dom);

        let r = dom.node(root);
        assert_eq!(r.first_child().unwrap().id(), a);
        assert_eq!(r.last_child().unwrap().id(), c);
        assert!(r.has_child_nodes());

        let br = dom.node(b);
        assert_eq!(br.previous_sibling().unwrap().id(), a);
        assert_eq!(br.next_sibling().unwrap().id(), c);
        assert_eq!(br.parent_node().unwrap().id(), root);
    }

    #[test]
    fn child_nodes_iterator_yields_all_in_order() {
        let mut dom: Dom = Dom::new();
        let (root, a, b, c) = build(&mut dom);
        let ids: Vec<NodeId> = dom.node(root).child_nodes().map(|n| n.id()).collect();
        assert_eq!(ids, vec![a, b, c]);
    }

    #[test]
    fn element_child_iter_skips_text_and_comment() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let t = dom.create_text_node("hi");
        let c = dom.create_comment("note");
        let b = dom.create_element("b");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, t).unwrap();
        dom.append_child(root, c).unwrap();
        dom.append_child(root, b).unwrap();

        let ids: Vec<NodeId> = dom.node(root).children().map(|n| n.id()).collect();
        assert_eq!(ids, vec![a, b]);

        assert_eq!(dom.node(root).first_element_child().unwrap().id(), a);
        assert_eq!(dom.node(root).last_element_child().unwrap().id(), b);
        assert_eq!(dom.node(root).child_element_count(), 2);
    }

    #[test]
    fn node_name_matches_spec() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let t = dom.create_text_node("hi");
        let cm = dom.create_comment("note");
        let frag = dom.create_document_fragment();
        assert_eq!(dom.node(el).node_name(), "div");
        assert_eq!(dom.node(t).node_name(), "#text");
        assert_eq!(dom.node(cm).node_name(), "#comment");
        assert_eq!(dom.node(frag).node_name(), "#document-fragment");
    }

    #[test]
    fn node_value_for_text_and_comment_only() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let t = dom.create_text_node("hi");
        let cm = dom.create_comment("note");
        assert_eq!(dom.node(t).node_value(), Some("hi"));
        assert_eq!(dom.node(cm).node_value(), Some("note"));
        assert_eq!(dom.node(el).node_value(), None);
    }

    #[test]
    fn set_node_value_updates_text() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("old");
        dom.node_mut(t).set_node_value("new").unwrap();
        assert_eq!(dom.node(t).node_value(), Some("new"));
    }

    #[test]
    fn set_node_value_errors_on_element() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(matches!(
            dom.node_mut(el).set_node_value("x").unwrap_err(),
            DomError::WrongNodeType { .. }
        ));
    }

    #[test]
    fn contains_includes_self_and_descendants() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(root, a).unwrap();
        dom.append_child(a, b).unwrap();

        assert!(dom.node(root).contains(a));
        assert!(dom.node(root).contains(b));
        assert!(dom.node(a).contains(b));
        assert!(!dom.node(b).contains(a));
    }

    #[test]
    fn noderef_mutation_via_nodemut() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.node_mut(el).set_attribute("class", "hero").unwrap();
        dom.node_mut(el).add_class("active").unwrap();
        assert_eq!(dom.node(el).get_attribute("class"), Some("hero"));
        assert!(dom.node(el).has_class("active"));
    }

    // ── M4b step 14: NodeRef accessor additions ───────────────────────

    #[test]
    fn is_connected_true_for_root_and_attached_nodes() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let el = dom.create_element("div");
        dom.append_child(root, el).unwrap();
        assert!(dom.node(root).is_connected());
        assert!(dom.node(el).is_connected());
    }

    #[test]
    fn is_connected_false_for_detached_subtree() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let child = dom.create_element("span");
        dom.append_child(detached, child).unwrap();
        assert!(!dom.node(detached).is_connected());
        assert!(!dom.node(child).is_connected());
    }

    #[test]
    fn get_root_node_returns_doc_root_for_connected() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let el = dom.create_element("div");
        let leaf = dom.create_element("span");
        dom.append_child(el, leaf).unwrap();
        dom.append_child(root, el).unwrap();
        assert_eq!(dom.node(leaf).get_root_node().id(), root);
        assert_eq!(dom.node(root).get_root_node().id(), root);
    }

    #[test]
    fn get_root_node_returns_detached_subtree_root() {
        let mut dom: Dom = Dom::new();
        let outer = dom.create_element("div");
        let inner = dom.create_element("span");
        dom.append_child(outer, inner).unwrap();
        assert_eq!(dom.node(inner).get_root_node().id(), outer);
        assert_eq!(dom.node(outer).get_root_node().id(), outer);
    }

    #[test]
    fn class_name_returns_raw_attribute_or_empty_string() {
        let mut dom: Dom = Dom::new();
        let bare = dom.create_element("div");
        let styled = dom.create_element("div");
        dom.node_mut(styled)
            .set_attribute("class", "hero  active")
            .unwrap();
        assert_eq!(dom.node(bare).class_name(), "");
        assert_eq!(dom.node(styled).class_name(), "hero  active");
    }

    #[test]
    fn class_list_returns_dom_token_list_snapshot() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.node_mut(el).add_class("foo").unwrap();
        dom.node_mut(el).add_class("bar").unwrap();
        let list: DomTokenList = dom.node(el).class_list();
        assert_eq!(list.len(), 2);
        assert!(list.contains("foo"));
        assert!(list.contains("bar"));
        assert!(!list.contains("baz"));
        let collected: Vec<&str> = list.iter().collect();
        assert_eq!(collected, ["bar", "foo"]); // BTreeSet ordering (documented divergence)
    }

    #[test]
    fn matches_simple_selectors() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("button");
        dom.node_mut(el).set_id("go").unwrap();
        dom.node_mut(el).add_class("primary").unwrap();
        assert!(dom.node(el).matches("button"));
        assert!(dom.node(el).matches("#go"));
        assert!(dom.node(el).matches(".primary"));
        assert!(dom.node(el).matches("button.primary#go"));
        assert!(!dom.node(el).matches("div"));
    }

    #[test]
    fn matches_returns_false_on_invalid_selector() {
        // Spec divergence: browser throws SyntaxError; rdom swallows
        // and returns false. Documented on the method.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(!dom.node(el).matches("!!!not a selector"));
    }

    #[test]
    fn closest_returns_self_when_self_matches() {
        let mut dom: Dom = Dom::new();
        let outer = dom.create_element("section");
        let inner = dom.create_element("div");
        dom.node_mut(inner).add_class("target").unwrap();
        dom.append_child(outer, inner).unwrap();
        let hit = dom.node(inner).closest(".target").unwrap();
        assert_eq!(hit.id(), inner);
    }

    #[test]
    fn closest_walks_up_ancestors() {
        let mut dom: Dom = Dom::new();
        let form = dom.create_element("form");
        let label = dom.create_element("label");
        let input = dom.create_element("input");
        dom.append_child(label, input).unwrap();
        dom.append_child(form, label).unwrap();
        let hit = dom.node(input).closest("form").unwrap();
        assert_eq!(hit.id(), form);
    }

    #[test]
    fn closest_returns_none_in_detached_subtree_with_no_match() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let child = dom.create_element("span");
        dom.append_child(detached, child).unwrap();
        // No ancestor matches "form" and the subtree is detached.
        assert!(dom.node(child).closest("form").is_none());
    }

    #[test]
    fn closest_returns_none_on_invalid_selector() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(dom.node(el).closest("!!!").is_none());
    }

    #[test]
    fn query_selector_finds_first_descendant() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let section = dom.create_element("section");
        let a = dom.create_element("p");
        dom.node_mut(a).add_class("hit").unwrap();
        let b = dom.create_element("p");
        dom.node_mut(b).add_class("hit").unwrap();
        dom.append_child(section, a).unwrap();
        dom.append_child(section, b).unwrap();
        dom.append_child(root, section).unwrap();
        let hit = dom.node(section).query_selector(".hit").unwrap();
        assert_eq!(hit.id(), a);
    }

    #[test]
    fn query_selector_returns_none_when_no_descendant_matches() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(dom.node(el).query_selector(".missing").is_none());
    }

    #[test]
    fn query_selector_excludes_self_per_spec() {
        // DOM spec: querySelector is element-rooted but searches
        // descendants only. The subject element itself is not a
        // candidate.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.node_mut(el).add_class("foo").unwrap();
        assert!(dom.node(el).query_selector(".foo").is_none());
    }

    #[test]
    fn query_selector_all_returns_node_list_in_document_order() {
        let mut dom: Dom = Dom::new();
        let root = dom.create_element("section");
        let a = dom.create_element("p");
        let b = dom.create_element("p");
        let c = dom.create_element("p");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        dom.append_child(root, c).unwrap();
        let list = dom.node(root).query_selector_all("p");
        assert_eq!(list.len(), 3);
        let ids: Vec<NodeId> = list.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec![a, b, c]);
    }

    #[test]
    fn query_selector_all_returns_empty_on_invalid_selector() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let list = dom.node(el).query_selector_all("!!!");
        assert_eq!(list.len(), 0);
    }

    // ── M4b step 15: NodeMut accessor additions ───────────────────────

    #[test]
    fn set_class_name_replaces_full_class_attribute() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.node_mut(el).add_class("old").unwrap();
        dom.node_mut(el).set_class_name("hero  active").unwrap();
        assert_eq!(dom.node(el).class_name(), "hero  active");
        // classList is rebuilt: old is gone, both new tokens present.
        let list = dom.node(el).class_list();
        assert!(!list.contains("old"));
        assert!(list.contains("hero"));
        assert!(list.contains("active"));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn set_class_name_empty_clears_classes() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.node_mut(el).add_class("foo").unwrap();
        dom.node_mut(el).set_class_name("").unwrap();
        assert_eq!(dom.node(el).class_name(), "");
        assert_eq!(dom.node(el).class_list().len(), 0);
    }

    #[test]
    fn class_list_mut_returns_mutating_handle() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        {
            let mut nm = dom.node_mut(el);
            let mut list = nm.class_list_mut();
            list.add("foo").unwrap();
            list.toggle("bar", Some(true)).unwrap();
        }
        assert!(dom.node(el).has_class("foo"));
        assert!(dom.node(el).has_class("bar"));
    }

    #[test]
    fn toggle_attribute_force_true_is_force_add() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("input");
        // Absent: force-add adds with empty string, returns true.
        assert!(
            dom.node_mut(el)
                .toggle_attribute_force("disabled", Some(true))
                .unwrap()
        );
        assert!(dom.node(el).has_attribute("disabled"));
        // Already present: idempotent, returns true, no overwrite.
        dom.node_mut(el).set_attribute("disabled", "1").unwrap();
        assert!(
            dom.node_mut(el)
                .toggle_attribute_force("disabled", Some(true))
                .unwrap()
        );
        assert_eq!(dom.node(el).get_attribute("disabled"), Some("1"));
    }

    #[test]
    fn toggle_attribute_force_false_is_force_remove() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("input");
        dom.node_mut(el).set_attribute("disabled", "").unwrap();
        // Present: force-remove returns false, attribute gone.
        assert!(
            !dom.node_mut(el)
                .toggle_attribute_force("disabled", Some(false))
                .unwrap()
        );
        assert!(!dom.node(el).has_attribute("disabled"));
        // Already absent: idempotent, returns false.
        assert!(
            !dom.node_mut(el)
                .toggle_attribute_force("disabled", Some(false))
                .unwrap()
        );
    }

    #[test]
    fn toggle_attribute_force_none_flips_state() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("input");
        // Absent → present.
        assert!(
            dom.node_mut(el)
                .toggle_attribute_force("disabled", None)
                .unwrap()
        );
        assert!(dom.node(el).has_attribute("disabled"));
        // Present → absent.
        assert!(
            !dom.node_mut(el)
                .toggle_attribute_force("disabled", None)
                .unwrap()
        );
        assert!(!dom.node(el).has_attribute("disabled"));
    }

    #[test]
    fn toggle_attribute_force_errors_on_non_element() {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hi");
        assert!(
            dom.node_mut(t)
                .toggle_attribute_force("disabled", Some(true))
                .is_err()
        );
    }

    // ── M4b step 16: variadic tree helpers ────────────────────────────

    use crate::NodeOrString;

    fn child_ids<Ext: 'static>(dom: &Dom<Ext>, parent: NodeId) -> Vec<NodeId> {
        dom.node(parent).child_nodes().map(|n| n.id()).collect()
    }

    #[test]
    fn append_variadic_inserts_in_order_with_text_coercion() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("p");
        let strong = dom.create_element("strong");
        let em = dom.create_element("em");
        dom.node_mut(parent)
            .append([
                "hello ".into(),
                strong.into(),
                " mid ".into(),
                em.into(),
                NodeOrString::Text("!".into()),
            ])
            .unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids.len(), 5);
        // Middle node-typed slots are preserved by id.
        assert_eq!(ids[1], strong);
        assert_eq!(ids[3], em);
        // Text slots produced fresh text nodes.
        assert_eq!(dom.node(ids[0]).node_value(), Some("hello "));
        assert_eq!(dom.node(ids[2]).node_value(), Some(" mid "));
        assert_eq!(dom.node(ids[4]).node_value(), Some("!"));
    }

    #[test]
    fn prepend_variadic_inserts_in_order_before_existing_children() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("ul");
        let existing = dom.create_element("li");
        dom.append_child(parent, existing).unwrap();
        let a = dom.create_element("li");
        let b = dom.create_element("li");
        dom.node_mut(parent)
            .prepend([a.into(), "x".into(), b.into()])
            .unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], a);
        assert_eq!(dom.node(ids[1]).node_value(), Some("x"));
        assert_eq!(ids[2], b);
        assert_eq!(ids[3], existing);
    }

    #[test]
    fn before_inserts_siblings_in_order_preceding_self() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let target = dom.create_element("span");
        let after = dom.create_element("span");
        dom.append_child(parent, target).unwrap();
        dom.append_child(parent, after).unwrap();
        let a = dom.create_element("p");
        let b = dom.create_element("p");
        dom.node_mut(target)
            .before([a.into(), "hi".into(), b.into()])
            .unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids.len(), 5);
        assert_eq!(ids[0], a);
        assert_eq!(dom.node(ids[1]).node_value(), Some("hi"));
        assert_eq!(ids[2], b);
        assert_eq!(ids[3], target);
        assert_eq!(ids[4], after);
    }

    #[test]
    fn after_inserts_siblings_in_order_following_self() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let before = dom.create_element("span");
        let target = dom.create_element("span");
        let tail = dom.create_element("span");
        dom.append_child(parent, before).unwrap();
        dom.append_child(parent, target).unwrap();
        dom.append_child(parent, tail).unwrap();
        let a = dom.create_element("p");
        let b = dom.create_element("p");
        dom.node_mut(target)
            .after([a.into(), "hi".into(), b.into()])
            .unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids.len(), 6);
        assert_eq!(ids[0], before);
        assert_eq!(ids[1], target);
        assert_eq!(ids[2], a);
        assert_eq!(dom.node(ids[3]).node_value(), Some("hi"));
        assert_eq!(ids[4], b);
        assert_eq!(ids[5], tail);
    }

    #[test]
    fn before_after_silently_noop_on_parentless_node() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let extra = dom.create_element("p");
        // Detached has no parent: before/after silently no-op.
        dom.node_mut(detached).before([extra.into()]).unwrap();
        dom.node_mut(detached).after([extra.into()]).unwrap();
        // Nothing inserted around detached; it's still parentless.
        assert!(dom.node(detached).parent_node().is_none());
    }

    #[test]
    fn replace_children_clears_existing_and_appends_new() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let old_a = dom.create_element("a");
        let old_b = dom.create_element("b");
        dom.append_child(parent, old_a).unwrap();
        dom.append_child(parent, old_b).unwrap();
        let new_a = dom.create_element("i");
        let new_b = dom.create_element("u");
        dom.node_mut(parent)
            .replace_children([new_a.into(), "mid".into(), new_b.into()])
            .unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0], new_a);
        assert_eq!(dom.node(ids[1]).node_value(), Some("mid"));
        assert_eq!(ids[2], new_b);
    }

    #[test]
    fn replace_with_inserts_siblings_then_detaches_self() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let target = dom.create_element("span");
        let tail = dom.create_element("span");
        dom.append_child(parent, target).unwrap();
        dom.append_child(parent, tail).unwrap();
        let r1 = dom.create_element("p");
        let r2 = dom.create_element("p");
        dom.node_mut(target)
            .replace_with([r1.into(), "mid".into(), r2.into()])
            .unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], r1);
        assert_eq!(dom.node(ids[1]).node_value(), Some("mid"));
        assert_eq!(ids[2], r2);
        assert_eq!(ids[3], tail);
        // target is still alive in the arena but parentless.
        assert!(dom.node(target).parent_node().is_none());
    }

    #[test]
    fn replace_with_on_parentless_node_is_silent_noop() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        let extra = dom.create_element("p");
        dom.node_mut(detached).replace_with([extra.into()]).unwrap();
        // No parent, no operation. extra is still detached too.
        assert!(dom.node(detached).parent_node().is_none());
        assert!(dom.node(extra).parent_node().is_none());
    }

    #[test]
    fn remove_self_detaches_from_parent() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        let target = dom.create_element("span");
        let sibling = dom.create_element("span");
        dom.append_child(parent, target).unwrap();
        dom.append_child(parent, sibling).unwrap();
        dom.node_mut(target).remove_self().unwrap();
        let ids = child_ids(&dom, parent);
        assert_eq!(ids, vec![sibling]);
        assert!(dom.node(target).parent_node().is_none());
    }

    #[test]
    fn remove_self_on_parentless_node_is_silent_noop() {
        let mut dom: Dom = Dom::new();
        let detached = dom.create_element("div");
        // Already parentless: silent no-op.
        dom.node_mut(detached).remove_self().unwrap();
        assert!(dom.contains(detached));
    }

    // ── M4b step 19: HTMLElement IDL accessors ────────────────────────

    #[test]
    fn dataset_round_trips_camelcase_to_kebab() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        {
            let mut nm = dom.node_mut(el);
            let mut ds = nm.dataset_mut();
            ds.set("fooBar", "yes").unwrap();
            ds.set("x", "1").unwrap();
        }
        // Raw attribute names use kebab-case.
        assert_eq!(dom.node(el).get_attribute("data-foo-bar"), Some("yes"));
        assert_eq!(dom.node(el).get_attribute("data-x"), Some("1"));
        // Dataset getter reads back via camelCase.
        let ds = dom.node(el).dataset();
        assert_eq!(ds.get("fooBar"), Some("yes"));
        assert_eq!(ds.get("x"), Some("1"));
        assert_eq!(ds.len(), 2);
    }

    #[test]
    fn tab_index_returns_parsed_attribute_or_none() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("input");
        assert_eq!(dom.node(el).tab_index(), None);
        dom.node_mut(el).set_tab_index(0).unwrap();
        assert_eq!(dom.node(el).tab_index(), Some(0));
        dom.node_mut(el).set_tab_index(-1).unwrap();
        assert_eq!(dom.node(el).tab_index(), Some(-1));
        // Manually written garbage attribute returns None.
        dom.node_mut(el)
            .set_attribute("tabindex", "not a number")
            .unwrap();
        assert_eq!(dom.node(el).tab_index(), None);
    }

    #[test]
    fn hidden_reads_attribute_presence() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert!(!dom.node(el).hidden());
        dom.node_mut(el).set_hidden(true).unwrap();
        assert!(dom.node(el).hidden());
        assert!(dom.node(el).has_attribute("hidden"));
        dom.node_mut(el).set_hidden(false).unwrap();
        assert!(!dom.node(el).hidden());
        assert!(!dom.node(el).has_attribute("hidden"));
    }

    #[test]
    fn content_editable_defaults_to_inherit() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        // No attribute: IDL returns "inherit".
        assert_eq!(dom.node(el).content_editable(), "inherit");
        dom.node_mut(el).set_content_editable("true").unwrap();
        assert_eq!(dom.node(el).content_editable(), "true");
        dom.node_mut(el)
            .set_content_editable("plaintext-only")
            .unwrap();
        assert_eq!(dom.node(el).content_editable(), "plaintext-only");
    }

    #[test]
    fn inner_html_and_outer_html_getters_delegate_to_markup() {
        let mut dom: Dom = Dom::new();
        let parent = dom.create_element("div");
        dom.node_mut(parent).set_id("hero").unwrap();
        let child = dom.create_element("span");
        let text = dom.create_text_node("hi");
        dom.append_child(child, text).unwrap();
        dom.append_child(parent, child).unwrap();
        // inner_html: just the children's markup.
        assert_eq!(dom.node(parent).inner_html(), "<span>hi</span>");
        // outer_html: includes self.
        assert_eq!(
            dom.node(parent).outer_html(),
            r#"<div id="hero"><span>hi</span></div>"#
        );
    }
}
