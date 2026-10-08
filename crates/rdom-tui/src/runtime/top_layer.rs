//! Leaving the top layer (CSS Position 4 §3.3): an element a dialog's
//! close or a popover's hide takes out of the top layer stays there,
//! pending removal, while a transition keeps its `overlay` at `auto` —
//! so it animates out where it was drawn — and leaves once `overlay` is
//! not `auto` at a style update.
//!
//! An element whose `transition-property` does not take `overlay` under
//! `transition-behavior: allow-discrete` leaves at once, as before the
//! request existed: nothing could keep it.

use rdom_core::NodeId;

use crate::TuiDom;

/// "Request an element to be removed from the top layer": pending when a
/// transition of `overlay` could keep it, else removed now.
pub(crate) fn request_removal(dom: &mut TuiDom, id: NodeId) {
    let keeps = dom
        .node(id)
        .ext()
        .and_then(|e| e.cascaded_for(crate::ext::StyleSlot::Host))
        .is_some_and(|style| crate::runtime::animation::transitions_discretely(style, "overlay"));
    if keeps {
        dom.request_remove_from_top_layer(id);
    } else {
        dom.remove_from_top_layer(id);
    }
}

/// At a style update: remove every element pending removal whose
/// computed `overlay` — a running transition's value included — is not
/// `auto`. Returns whether any left.
pub(crate) fn finish_removals(dom: &mut TuiDom) -> bool {
    let mut removed = false;
    for id in dom.pending_top_layer_removals() {
        let kept = dom
            .node(id)
            .ext()
            .and_then(|e| e.computed.as_deref())
            .is_some_and(|c| c.overlay == crate::layout::Overlay::Auto);
        if !kept {
            removed |= dom.remove_from_top_layer(id);
        }
    }
    removed
}
