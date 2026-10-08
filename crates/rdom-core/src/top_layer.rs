//! The document's *top layer* (CSS Position 4 "top layer", the set HTML
//! adds modal dialogs (§4.11.4) and showing popovers (§6.12) to): an
//! ordered set of elements a backend renders above everything else, in
//! order, each with its `::backdrop`. Document state like focus — what
//! `:modal` reads — so it lives on the `Dom`; how its members render and
//! what puts them there is the backend's.
//!
//! Every change fires `Mutation::InteractionChanged` naming the element
//! as both `prev` and `next`, with `InteractionKind::TopLayer`. An
//! element removed from the document leaves the top layer (the removing
//! steps of `<dialog>` and of popovers), reported while it is still
//! connected.

use crate::dom::Dom;
use crate::error::{DomError, Result};
use crate::node_id::NodeId;
use crate::observer::{InteractionKind, Mutation};

/// Why an element is in the top layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TopLayerKind {
    /// A `<dialog>` shown with `showModal()` — its *is modal* flag
    /// (HTML §4.11.4): `:modal`, and the rest of the document inert.
    ModalDialog,
    /// An element whose popover is showing (HTML §6.12):
    /// `:popover-open`.
    Popover,
    /// An open drop-down `<select>`: its picker (the option list,
    /// HTML's `::picker(select)`) renders in the top layer while the
    /// select's own box stays in flow. Neither `:modal` nor
    /// `:popover-open`.
    Picker,
}

impl<Ext> Dom<Ext> {
    /// The top layer, bottom to top: the order the elements render in.
    pub fn top_layer(&self) -> &[NodeId] {
        &self.top_layer.ids
    }

    /// Why `id` is in the top layer, or `None` when it is not — or is
    /// only waiting to leave it
    /// ([`request_remove_from_top_layer`](Self::request_remove_from_top_layer)):
    /// such an element is no modal dialog or showing popover any more.
    pub fn top_layer_kind(&self, id: NodeId) -> Option<TopLayerKind> {
        let i = self.top_layer.ids.iter().position(|&e| e == id)?;
        (!self.top_layer.pending[i]).then(|| self.top_layer.kinds[i])
    }

    /// Whether `id` is in the top layer — where it renders — pending
    /// removal or not.
    pub fn is_in_top_layer(&self, id: NodeId) -> bool {
        self.top_layer.ids.contains(&id)
    }

    /// Whether `id` is in the top layer waiting to leave it (CSS Position
    /// 4 §3.3, "pending top layer removals").
    pub fn is_pending_top_layer_removal(&self, id: NodeId) -> bool {
        self.top_layer
            .ids
            .iter()
            .position(|&e| e == id)
            .is_some_and(|i| self.top_layer.pending[i])
    }

    /// The elements waiting to leave the top layer, bottom to top.
    pub fn pending_top_layer_removals(&self) -> Vec<NodeId> {
        self.top_layer
            .ids
            .iter()
            .zip(&self.top_layer.pending)
            .filter(|(_, pending)| **pending)
            .map(|(id, _)| *id)
            .collect()
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// "Add an element to the top layer": `id` goes on top — removed
    /// first when it is already in it — as `kind`. Errors with
    /// `DomError::WrongNodeType` for a node that is not an element and
    /// `DomError::InvalidState` for one not connected to the document.
    pub fn add_to_top_layer(&mut self, id: NodeId, kind: TopLayerKind) -> Result<()> {
        let node = self.node_or_err(id)?;
        if node.tag_name().is_none() {
            return Err(DomError::WrongNodeType {
                expected: "element",
                got: node.node_type(),
            });
        }
        if !self.node(id).is_connected() {
            return Err(DomError::InvalidState(
                "add_to_top_layer: the element is not connected",
            ));
        }
        self.top_layer.remove(id);
        self.top_layer.ids.push(id);
        self.top_layer.kinds.push(kind);
        self.top_layer.pending.push(false);
        self.notify_top_layer(id);
        Ok(())
    }

    /// CSS Position 4 §3.3, "request an element to be removed from the top
    /// layer": `id` stays in the top layer, in its place — it still
    /// renders there — but pending removal: it is no modal dialog or
    /// showing popover any more ([`top_layer_kind`](Self::top_layer_kind)
    /// is `None`). The backend removes it
    /// ([`remove_from_top_layer`](Self::remove_from_top_layer)) once its
    /// `overlay` is not `auto` — at once, unless a transition keeps it.
    /// Returns whether the request took (`id` was in the top layer and not
    /// already pending).
    pub fn request_remove_from_top_layer(&mut self, id: NodeId) -> bool {
        let Some(i) = self.top_layer.ids.iter().position(|&e| e == id) else {
            return false;
        };
        if self.top_layer.pending[i] {
            return false;
        }
        self.top_layer.pending[i] = true;
        self.notify_top_layer(id);
        true
    }

    /// Take `id` out of the top layer. Returns whether it was in it.
    pub fn remove_from_top_layer(&mut self, id: NodeId) -> bool {
        let removed = self.top_layer.remove(id);
        if removed {
            self.notify_top_layer(id);
        }
        removed
    }

    /// Take `id` out without a record — the cleanup after a panicking
    /// observer, as the other interaction state's.
    pub(crate) fn top_layer_remove_silently(&mut self, id: NodeId) {
        self.top_layer.remove(id);
    }

    fn notify_top_layer(&mut self, id: NodeId) {
        self.fire_mutation(Mutation::InteractionChanged {
            prev: Some(id),
            next: Some(id),
            kind: InteractionKind::TopLayer,
        });
    }
}

/// The top layer's storage: the elements, bottom to top, and beside
/// each the reason it is there.
#[derive(Debug, Default)]
pub(crate) struct TopLayer {
    ids: Vec<NodeId>,
    kinds: Vec<TopLayerKind>,
    /// Beside each, whether it waits to leave (CSS Position 4 §3.3).
    pending: Vec<bool>,
}

impl TopLayer {
    /// Take `id` out; whether it was in.
    fn remove(&mut self, id: NodeId) -> bool {
        match self.ids.iter().position(|&e| e == id) {
            Some(i) => {
                self.ids.remove(i);
                self.kinds.remove(i);
                self.pending.remove(i);
                true
            }
            None => false,
        }
    }

    /// The members, bottom to top.
    pub(crate) fn ids(&self) -> &[NodeId] {
        &self.ids
    }
}
