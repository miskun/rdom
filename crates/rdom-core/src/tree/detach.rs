//! Taking a node out of its parent: the structural unlink every removal
//! and move shares, with the interaction-state purge and the removing
//! steps it runs first (see the parent module's "Observer record
//! ordering").

use crate::dom::Dom;
use crate::error::Result;
use crate::node_id::NodeId;
use crate::observer::Mutation;

impl<Ext: 'static> Dom<Ext> {
    /// Detach `id` from its parent. Fixes sibling chain + first/last_child
    /// on parent. Safe no-op if the node has no parent.
    ///
    /// Also clears any interaction state (`focused`, `hovered`,
    /// `active`, `pointer_capture`, `selection`) that pointed into the
    /// detached subtree. Without this, a `set_focused`/etc. pointing
    /// at a now-orphaned node leaves the Dom in an internally
    /// inconsistent state — `dom.focused()` returns a NodeId that's
    /// no longer in the tree, and `:focus` keeps matching it.
    ///
    /// Record-emission order is: `PreDetach`, then
    /// `InteractionChanged`/`SelectionChanged` for any cleared
    /// state — both while the subtree is still connected, so an
    /// observer can walk the cleared node's ancestors (whose
    /// `:hover` / `:active` / `:focus-within` flip) — then the
    /// structural pointer update. The `ChildListChanged` record that motivates the
    /// detach is fired by the caller (`remove_child`,
    /// `replace_with`, `clear_children`, ...) AFTER this returns,
    /// so observers see the interaction-state changes before the
    /// tree change that caused them. The simpler causal order
    /// would require each caller to remember a post-detach purge
    /// step — centralizing in this function trades that
    /// observability nuance for a structurally-guaranteed cleanup.
    pub(crate) fn detach_from_parent(&mut self, id: NodeId) -> Result<()> {
        self.node_or_err(id)?;
        // **Pre-detach event window** — fire `Mutation::PreDetach`
        // BEFORE structural unlink, while focused/hovered's
        // ancestor chains are still intact. Observers (notably
        // `rdom-tui`'s App-level observer) use this to dispatch
        // implicit `blur` / `focusout` / `mouseleave` / `mouseout`
        // events with normal bubbling semantics. Only emitted
        // when at least one of focused/hovered is actually inside
        // the subtree being detached — empty PreDetach records
        // would be noise. Membership is an O(depth) ancestor walk
        // from the state's node, not a walk of the subtree.
        let focused_in = self.focused.filter(|&f| self.is_ancestor(id, f));
        let hovered_in = self.hovered.filter(|&h| self.is_ancestor(id, h));
        if focused_in.is_some() || hovered_in.is_some() {
            self.fire_mutation(Mutation::PreDetach {
                detached_root: id,
                focused: focused_in,
                hovered: hovered_in,
            });
        }

        // Clear the interaction state inside the subtree while it is
        // still connected: `:hover`, `:active` and `:focus-within` also
        // match the ancestors, so an observer of the `InteractionChanged`
        // record must be able to walk from `prev` to them. A panicking
        // observer does not stop the unlink (the caller frees a dropped
        // subtree on the way out); whatever the purge had not reached
        // yet is cleared silently before the panic resumes.
        let purged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.purge_interaction_state_for_subtree(id, true);
        }));

        // The live ranges inside it move out (DOM §4.2.3 "remove").
        self.highlights_removing(id);

        let node = self.node_or_err(id)?;
        let parent = node.parent;
        let prev = node.prev_sibling;
        let next = node.next_sibling;

        if let Some(prev) = prev {
            self.get_node_mut(prev).unwrap().next_sibling = next;
        }
        if let Some(next) = next {
            self.get_node_mut(next).unwrap().prev_sibling = prev;
        }
        if let Some(parent) = parent {
            if self.get_node(parent).and_then(|n| n.first_child) == Some(id) {
                self.get_node_mut(parent).unwrap().first_child = next;
            }
            if self.get_node(parent).and_then(|n| n.last_child) == Some(id) {
                self.get_node_mut(parent).unwrap().last_child = prev;
            }
        }

        let n = self.get_node_mut(id).unwrap();
        n.parent = None;
        n.prev_sibling = None;
        n.next_sibling = None;

        if let Err(payload) = purged {
            self.purge_interaction_state_for_subtree(id, false);
            std::panic::resume_unwind(payload);
        }
        Ok(())
    }

    /// Clear any document-level interaction state
    /// (`focused`, `hovered`, `active`, `pointer_capture`, `selection`)
    /// whose referenced node lives inside the subtree rooted at `root`
    /// (inclusive). Called by `detach_from_parent` so detachment
    /// can never leave dangling interaction pointers.
    ///
    /// With `notify`, each cleared field that has a mutation type
    /// (`focused`, `hovered`, `active`, `selection`) goes through its
    /// public setter so the appropriate `InteractionChanged` /
    /// `SelectionChanged` record fires; without it (the cleanup after a
    /// panicking observer) the fields are cleared directly.
    /// `pointer_capture` always clears silently because it has no
    /// associated record type (it's a runtime-routing flag, not a
    /// cascade-affecting state).
    ///
    /// Membership is `is_ancestor(root, node)`: O(depth) per field, no
    /// subtree walk, correct before and after `root` is unlinked (the
    /// links inside the subtree stay).
    fn purge_interaction_state_for_subtree(&mut self, root: NodeId, notify: bool) {
        let inside =
            |dom: &Self, node: Option<NodeId>| node.is_some_and(|n| dom.is_ancestor(root, n));
        if inside(self, self.focused) {
            if notify {
                self.set_focused(None);
            } else {
                self.focused = None;
            }
        }
        if inside(self, self.hovered) {
            if notify {
                self.set_hovered(None);
            } else {
                self.hovered = None;
            }
        }
        if inside(self, self.active) {
            if notify {
                self.set_active(None);
            } else {
                self.active = None;
            }
        }
        if inside(self, self.pointer_capture) {
            self.pointer_capture = None;
        }
        // The removing steps of `<dialog>` (HTML §4.11.4) and of popovers
        // (§6.12): an element leaving the document leaves the top layer.
        let leaving: Vec<NodeId> = self
            .top_layer
            .ids()
            .iter()
            .copied()
            .filter(|&e| inside(self, Some(e)))
            .collect();
        for e in leaving {
            if notify {
                self.remove_from_top_layer(e);
            } else {
                self.top_layer_remove_silently(e);
            }
        }
        let selected = self
            .selection
            .as_ref()
            .map(|sel| (sel.anchor.node, sel.focus.node));
        if let Some((anchor, focus)) = selected
            && (inside(self, Some(anchor)) || inside(self, Some(focus)))
        {
            if notify {
                self.set_selection(None);
            } else {
                self.selection = None;
                self.selection_serial = self.selection_serial.next();
            }
        }
    }
}
