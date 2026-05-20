//! Mouse-drag text selection — `mousedown` begins, `mousemove`
//! extends focus, `mouseup` ends.
//!
//! Called from `runtime/router/mouse/mod.rs` as default actions
//! on the mouse pipeline:
//!
//! - `begin`: tries to start a drag on `mousedown`. Returns true
//!   when the click landed on selectable text and a drag is now
//!   active — caller relies on pointer capture (which `begin` sets)
//!   to route subsequent `mousemove`/`mouseup` back here.
//! - `extend`: on `mousemove` while a drag is active, moves the
//!   selection's `focus` to the cursor's current position. Preserves
//!   the original anchor so dragging backward shrinks the selection
//!   symmetrically.
//! - `end`: clears router drag state. Pointer capture is released
//!   by the router's own `handle_up` (browser-faithful auto-release).
//!
//! ## Why pointer capture
//!
//! Without capture, dragging off the original paragraph routes
//! subsequent moves to whatever chrome happens to sit underneath —
//! selection would "jump" or freeze. Holding capture on the IFC
//! block means every move comes back to us while the button is
//! down. Matches browser `setPointerCapture` semantics.
//!
//! ## What "selectable" means here
//!
//! A click is selectable iff `dom.position_at(x, y)` returns
//! `Some(_)`. That function already walks to the innermost IFC
//! block and rejects `user-select: none` subtrees, so this file
//! doesn't duplicate those checks.

use crossterm::event::MouseEvent;

use rdom_core::{NodeId, Selection};

use crate::TuiDom;
use crate::render::layout_pass::is_ifc_block;
use crate::runtime::hit_test::HitTestExt;
use crate::runtime::router::Router;

/// Default action for `mousedown`: begin a drag-select if the
/// click landed on selectable text.
///
/// Returns `true` when a drag was started. The router uses this
/// only to keep symmetry with other default actions — the real
/// "we're dragging" signal for follow-up moves is
/// `router.selection_drag.is_some()`, set by this function.
pub(crate) fn begin(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> bool {
    let Some(anchor) = dom.position_at(mouse.column, mouse.row) else {
        return false;
    };

    dom.set_selection(Some(Selection::caret(anchor)));

    // Hold pointer capture on the IFC block containing the anchor
    // (or, if we somehow can't find one, the text node itself). The
    // holder node receives every follow-up `mousemove` / `mouseup`
    // until auto-release on the next up.
    let capture_holder = ifc_block_of(dom, anchor.node).unwrap_or(anchor.node);
    let _ = dom.set_pointer_capture(capture_holder);

    router.selection_drag = Some(capture_holder);
    true
}

/// Default action for `mousemove` (while `router.selection_drag` is
/// set): extend the selection's focus to the cursor's current
/// position. Returns `true` when the selection actually changed —
/// caller uses it to request a redraw.
pub(crate) fn extend(dom: &mut TuiDom, mouse: MouseEvent) -> bool {
    // Out-of-bounds moves (cursor leaves any IFC block) are
    // ignored — selection's focus stays at its last valid
    // position. Clamping to first/last position of the IFC on
    // overshoot is a polish pass (§7.6 note).
    let Some(focus) = dom.position_at(mouse.column, mouse.row) else {
        return false;
    };
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    if sel.focus == focus {
        return false;
    }
    dom.set_selection(Some(Selection::new(sel.anchor, focus)));
    true
}

/// Clear router drag state. Call from `mouseup` regardless of
/// whether the up landed on text — the pointer capture is what
/// kept the drag alive, and it's auto-released by the router.
pub(crate) fn end(router: &mut Router) {
    router.selection_drag = None;
}

/// Walk up from `node_id` to the nearest ancestor that establishes
/// an inline formatting context (has an `inline_layout` on its
/// `TuiExt`). Returns `None` if no ancestor qualifies, which in
/// practice only happens for orphan text nodes and is a signal
/// that selection wouldn't behave sensibly anyway.
fn ifc_block_of(dom: &TuiDom, node_id: NodeId) -> Option<NodeId> {
    let mut cur = dom.node(node_id).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if is_ifc_block(dom, id) {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}
