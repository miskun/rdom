//! `NodeMut<'a, Ext>` — the mutable accessor. Split out of `accessor/mod.rs`
//! so the read surface and the write surface each stay a single-screen
//! module; the `Dom -> accessor` constructors live in the parent.

use super::*;

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
        // `set_attribute("class", …)` is the WHATWG-canonical entry point
        // and owns the attribute ↔ classList ↔ index sync; one call, one
        // `AttributeChanged` record.
        self.dom.set_attribute(self.id, "class", value)
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

    /// DOM `HTMLElement.contentEditable` setter (HTML §6.8.1), matched
    /// ASCII case-insensitively: `"inherit"` removes the attribute;
    /// `"true"`, `"false"` and `"plaintext-only"` write that keyword
    /// in lowercase; anything else is a [`DomError::Syntax`] and leaves
    /// the attribute untouched.
    pub fn set_content_editable(&mut self, value: &str) -> Result<()> {
        if value.eq_ignore_ascii_case("inherit") {
            return self.remove_attribute("contenteditable").map(|_| ());
        }
        match crate::ContentEditableState::from_attribute(value) {
            Some(state) if !value.is_empty() => {
                self.set_attribute("contenteditable", state.keyword())
            }
            _ => Err(DomError::Syntax(
                "contentEditable must be true, false, plaintext-only or inherit",
            )),
        }
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
    /// `ParentNode.append` (§4.2.6): every node is checked before any
    /// moves, so an invalid one inserts nothing.
    pub fn append(&mut self, children: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let items: Vec<NodeOrString> = children.into_iter().collect();
        let parent = self.id;
        self.dom.check_insertable(parent, &items)?;
        self.dom.insert_items(parent, items, None)
    }

    /// Insert each item at the start of this node's child list, in
    /// order — the first item of `children` becomes the new first
    /// child. DOM `ParentNode.prepend` (§4.2.6): before the first child
    /// the list leaves in place; nothing inserted when a node is invalid.
    pub fn prepend(&mut self, children: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let items: Vec<NodeOrString> = children.into_iter().collect();
        let parent = self.id;
        self.dom.check_insertable(parent, &items)?;
        let first = self.dom.get_node(parent).and_then(|n| n.first_child);
        let reference = self.dom.first_not_in(first, &items);
        self.dom.insert_items(parent, items, reference)
    }

    /// Insert each item as a sibling immediately before this node,
    /// in order. DOM `ChildNode.before` (§4.2.6): after the viable
    /// previous sibling — the first preceding sibling not in the list —
    /// so a sibling in the list moves with the rest; nothing inserted
    /// when a node is invalid.
    ///
    /// Silently no-ops when this node has no parent (browser-
    /// faithful). Text items create fresh text nodes only when
    /// insertion actually happens.
    pub fn before(&mut self, siblings: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let id = self.id;
        let Some(parent) = self.dom.get_node(id).and_then(|n| n.parent) else {
            return Ok(());
        };
        let items: Vec<NodeOrString> = siblings.into_iter().collect();
        self.dom.check_insertable(parent, &items)?;
        let listed = |n: NodeId| {
            items
                .iter()
                .any(|i| matches!(i, NodeOrString::Node(m) if *m == n))
        };
        let mut previous = self.dom.get_node(id).and_then(|n| n.prev_sibling);
        while let Some(p) = previous.filter(|&p| listed(p)) {
            previous = self.dom.get_node(p).and_then(|n| n.prev_sibling);
        }
        let from = match previous {
            Some(p) => self.dom.get_node(p).and_then(|n| n.next_sibling),
            None => self.dom.get_node(parent).and_then(|n| n.first_child),
        };
        let reference = self.dom.first_not_in(from, &items);
        self.dom.insert_items(parent, items, reference)
    }

    /// Insert each item as a sibling immediately after this node,
    /// in order. DOM `ChildNode.after` (§4.2.6): before the viable next
    /// sibling — the first following sibling not in the list; nothing
    /// inserted when a node is invalid.
    ///
    /// Silently no-ops when this node has no parent (browser-
    /// faithful).
    pub fn after(&mut self, siblings: impl IntoIterator<Item = NodeOrString>) -> Result<()> {
        let id = self.id;
        let Some(parent) = self.dom.get_node(id).and_then(|n| n.parent) else {
            return Ok(());
        };
        let items: Vec<NodeOrString> = siblings.into_iter().collect();
        self.dom.check_insertable(parent, &items)?;
        let next = self.dom.get_node(id).and_then(|n| n.next_sibling);
        let reference = self.dom.first_not_in(next, &items);
        self.dom.insert_items(parent, items, reference)
    }

    /// Clear this node's children and append the new ones. DOM
    /// `ParentNode.replaceChildren` (§4.2.6): the nodes are checked
    /// before anything is removed, so an invalid one leaves the children
    /// as they were.
    pub fn replace_children(
        &mut self,
        children: impl IntoIterator<Item = NodeOrString>,
    ) -> Result<()> {
        let items: Vec<NodeOrString> = children.into_iter().collect();
        let parent = self.id;
        self.dom.check_insertable(parent, &items)?;
        self.dom.clear_children(parent)?;
        self.dom.insert_items(parent, items, None)
    }

    /// Replace this node with `siblings`, inserted at its position
    /// in the parent, then detach this node. DOM
    /// `ChildNode.replaceWith` (§4.2.6): the nodes go before the viable
    /// next sibling; a node replaced with a list holding itself stays,
    /// in its list position; nothing changes when a node is invalid.
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
        let Some(parent) = dom.get_node(id).and_then(|n| n.parent) else {
            return Ok(());
        };
        let items: Vec<NodeOrString> = siblings.into_iter().collect();
        dom.check_insertable(parent, &items)?;
        let next = dom.get_node(id).and_then(|n| n.next_sibling);
        let reference = dom.first_not_in(next, &items);
        let listed = items
            .iter()
            .any(|i| matches!(i, NodeOrString::Node(n) if *n == id));
        dom.insert_items(parent, items, reference)?;
        if !listed {
            dom.remove_child(parent, id)?;
        }
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
    /// Fires `Mutation::CharacterDataChanged`. The live ranges into it
    /// (registered highlights') treat it as all its data replaced (DOM
    /// §4.10 "replace data" at 0, the old length).
    pub fn set_node_value(&mut self, data: &str) -> Result<()> {
        self.write_data(data, None)
    }

    /// [`set_node_value`](Self::set_node_value), its live-range update
    /// for `replaced` — `(offset, count, len)`: `count` bytes at `offset`
    /// became `len` — or, `None`, for the whole data.
    fn write_data(&mut self, data: &str, replaced: Option<(usize, usize, usize)>) -> Result<()> {
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
        let replaced = replaced.unwrap_or((0, old.len(), data.len()));
        self.dom.highlights_replace_data(id, replaced);
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
    /// Fires `Mutation::CharacterDataChanged` (via `set_node_value`); the
    /// live ranges into it move as DOM §4.10 "replace data" moves them for
    /// exactly this range.
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
        self.write_data(&new_data, Some((start, end - start, replacement.len())))
    }
}

impl<'a, Ext: Default> NodeMut<'a, Ext> {
    /// `textContent` setter — replace all children of this Element/Fragment
    /// with a single Text node. Errors on Text/Comment (use `set_data`).
    pub fn set_text_content(&mut self, text: &str) -> Result<()> {
        self.dom.set_text_content(self.id, text)
    }
}
