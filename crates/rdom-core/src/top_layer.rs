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
}

impl<Ext> Dom<Ext> {
    /// The top layer, bottom to top: the order the elements render in.
    pub fn top_layer(&self) -> &[NodeId] {
        &self.top_layer.ids
    }

    /// Why `id` is in the top layer, or `None` when it is not.
    pub fn top_layer_kind(&self, id: NodeId) -> Option<TopLayerKind> {
        let i = self.top_layer.ids.iter().position(|&e| e == id)?;
        Some(self.top_layer.kinds[i])
    }

    /// Whether `id` is in the top layer.
    pub fn is_in_top_layer(&self, id: NodeId) -> bool {
        self.top_layer.ids.contains(&id)
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
        self.notify_top_layer(id);
        Ok(())
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
}

impl TopLayer {
    /// Take `id` out; whether it was in.
    fn remove(&mut self, id: NodeId) -> bool {
        match self.ids.iter().position(|&e| e == id) {
            Some(i) => {
                self.ids.remove(i);
                self.kinds.remove(i);
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
