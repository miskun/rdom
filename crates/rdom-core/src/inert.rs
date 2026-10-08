//! Inertness (HTML §6.3 "Inert subtrees"): the one answer to "is this
//! node inert?" that hit-testing, focus and sequential navigation share.
//!
//! A node is inert when
//!
//! - the document is *blocked by a modal dialog* (§6.3.2) and the node is
//!   neither that dialog nor one of its descendants, or
//! - it or an ancestor has the `inert` attribute (§6.3.1), short of a
//!   modal dialog in between — "descendants which don't otherwise escape
//!   inertness (such as modal dialogs)" (CSS UI 4's mapping:
//!   `[inert] { interactivity: inert } dialog:modal { interactivity: auto }`).
//!
//! What inertness *does* — no hit-testing, no focus, no selection — is the
//! backend's; this module only answers the question.

use crate::dom::Dom;
use crate::node_id::NodeId;
use crate::top_layer::TopLayerKind;

impl<Ext> Dom<Ext> {
    /// The modal dialog the document is *blocked by* (HTML §6.3.2): the
    /// topmost modal dialog in the top layer, or `None`. A popover above
    /// it does not lift the block (HTML names the topmost dialog; the
    /// engines read it as the topmost *modal* one, as rdom does).
    pub fn blocking_modal(&self) -> Option<NodeId> {
        let layer = self.top_layer();
        layer
            .iter()
            .rev()
            .copied()
            .find(|&id| self.top_layer_kind(id) == Some(TopLayerKind::ModalDialog))
    }

    /// Whether `id` is inert (HTML §6.3): outside the blocking modal
    /// dialog ([`blocking_modal`](Self::blocking_modal)), or in an
    /// `inert` subtree no modal dialog escapes. One ancestor walk.
    pub fn is_inert(&self, id: NodeId) -> bool {
        let subject = self.blocking_modal();
        let mut inert_attr = false;
        let mut cur = Some(id);
        while let Some(n) = cur {
            if Some(n) == subject {
                // Inside the subject: only an `inert` below it counts.
                return inert_attr;
            }
            let node = self.node(n);
            inert_attr = inert_attr || node.has_attribute("inert");
            cur = node.parent_node().map(|p| p.id());
        }
        // Reached the root without meeting the subject: outside it when
        // there is one. (A modal dialog other than the subject escapes
        // nothing — it is outside the subject.)
        inert_attr || subject.is_some()
    }
}
