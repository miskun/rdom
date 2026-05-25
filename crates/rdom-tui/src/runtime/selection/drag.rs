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

use rdom_core::Selection;

use crate::TuiDom;
use crate::layout::UserSelect;
use crate::node::is_descendant_or_self;
use crate::render::inline::{InlineFlow, inline_flow_for_text, inline_flow_layout};
use crate::runtime::hit_test::HitTestExt;
use crate::runtime::router::Router;
use crate::runtime::selection::user_select;

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

    // Anchor must point at a real text node, not at an atomic
    // inline-block sentinel. When an IFC packer emits a fragment
    // for `Display::InlineBlock` (BFC-1 phase 3.5b), it sets
    // `text_node = node` so the fragment carries the inline-block
    // element id as its source. `position_at` happily returns a
    // Position whose `node` is that element. Engaging drag-select
    // there is wrong — atomic inline-blocks are interactive
    // widgets (`<button>`, `<input type=submit>`), not selectable
    // text. The click event must route to the widget itself, which
    // pointer capture would clobber by retargeting to the IFC
    // owner. Bail out so the click flow stays intact.
    if dom.node(anchor.node).node_type() != rdom_core::NodeType::Text {
        return false;
    }

    // `user-select: all`: a click anywhere inside the host element
    // selects its entire text content as a single unit. The drag
    // still engages capture, but `extend` becomes a no-op for the
    // duration — the highlight doesn't shrink as the user moves the
    // mouse.
    let initial = match user_select::ancestor_with(dom, anchor.node, UserSelect::All) {
        Some(host) => {
            user_select::span_all_text(dom, host).unwrap_or_else(|| Selection::caret(anchor))
        }
        None => Selection::caret(anchor),
    };
    dom.set_selection(Some(initial));

    // Hold pointer capture on the inline-flow container that holds
    // the anchor. For a classic IFC, the holder is the IFC block;
    // for an anonymous block box (BFC-1 phase 3), it's the parent
    // container — pointer capture is a per-NodeId concept, and the
    // container is the closest real node. The follow-up mousemove /
    // mouseup events route through that holder.
    let anchor_flow = inline_flow_for_text(dom, anchor.node);
    let capture_holder = anchor_flow.map(|f| f.owner()).unwrap_or(anchor.node);
    let _ = dom.set_pointer_capture(capture_holder);

    router.selection_drag = anchor_flow;
    true
}

/// Default action for `mousemove` (while `router.selection_drag` is
/// set): extend the selection's focus to the cursor's current
/// position. Returns `true` when the selection actually changed —
/// caller uses it to request a redraw.
///
/// `anchor_flow` is the inline-flow container the drag started in
/// (kept by the router in `selection_drag`). When the cursor moves
/// outside ANY element (or onto a non-text element), we still want
/// to extend the selection within `anchor_flow` — browsers do this
/// so dragging past the end of a line / past the bottom of a
/// paragraph still selects up to the line's end / paragraph's end.
pub(crate) fn extend(dom: &mut TuiDom, mouse: MouseEvent, anchor_flow: InlineFlow) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };

    // `user-select: all`: the host is selected as a unit, so the
    // drag doesn't update focus while the cursor moves.
    if user_select::ancestor_with(dom, sel.anchor.node, UserSelect::All).is_some() {
        return false;
    }

    // Prefer the hit-tested position — it may land in a DIFFERENT
    // inline-flow container, which is correct for cross-paragraph
    // drag selection (browsers let the selection span multiple
    // paragraphs). Only when no valid position exists anywhere on
    // screen (e.g. cursor is outside any IFC) do we clamp to the
    // anchor's nearest position so the user sees feedback for
    // dragging past end-of-line.
    let raw_focus = match dom.position_at(mouse.column, mouse.row) {
        Some(p) => p,
        None => match clamp_to_anchor_flow(dom, anchor_flow, mouse.column, mouse.row) {
            Some(p) => p,
            None => return false,
        },
    };

    // `user-select: contain`: the host traps the selection. If
    // `raw_focus` escaped the contain host, clamp it back to the
    // nearest in-host position.
    let focus = match user_select::ancestor_with(dom, sel.anchor.node, UserSelect::Contain) {
        Some(host) if !is_descendant_or_self(dom, raw_focus.node, host) => {
            match user_select::clamp_to_contain_host(dom, host, mouse) {
                Some(p) => p,
                None => return false,
            }
        }
        _ => raw_focus,
    };

    if sel.focus == focus {
        return false;
    }
    dom.set_selection(Some(Selection::new(sel.anchor, focus)));
    true
}

/// Compute the position inside `anchor_flow` nearest to `(x, y)`.
/// Used by drag-extend when the cursor moves out of the anchor's
/// inline-flow container (past end of line, off the bottom, etc.).
fn clamp_to_anchor_flow(
    dom: &TuiDom,
    anchor_flow: InlineFlow,
    x: u16,
    y: u16,
) -> Option<rdom_core::Position> {
    let (layout, content) = inline_flow_layout(dom, anchor_flow)?;
    if layout.lines.is_empty() {
        return None;
    }

    // Decide the target line AND whether the y was clamped. A
    // y-clamp dominates the x logic: dragging past the bottom of a
    // multi-line block should anchor at the LAST line's END
    // regardless of where x is on that line. Same for top.
    let (line_idx, y_overshoot_down, y_overshoot_up) = if (y as i32) < content.y {
        (0, false, true)
    } else if (y as i32) >= content.y + content.height as i32 {
        (layout.lines.len() - 1, true, false)
    } else {
        let raw = (y as i32 - content.y) as usize;
        (raw.min(layout.lines.len() - 1), false, false)
    };

    let target_line = &layout.lines[line_idx];

    // Empty target line → walk to the nearest non-empty line.
    if target_line.fragments.is_empty() {
        for line in layout.lines.iter().rev() {
            if let Some(frag) = line.fragments.last() {
                return Some(rdom_core::Position::new(
                    frag.text_node,
                    frag.source_byte_offset + frag.text.len(),
                ));
            }
        }
        return None;
    }

    let first = target_line.fragments.first().unwrap();
    let last = target_line.fragments.last().unwrap();

    // Y overshoot dominates: down → last line end, up → first line start.
    if y_overshoot_down {
        return Some(rdom_core::Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ));
    }
    if y_overshoot_up {
        return Some(rdom_core::Position::new(
            first.text_node,
            first.source_byte_offset,
        ));
    }

    // In-bounds y: clamp on x. Past line end → end. Before line
    // start → start. Middle gap (rare) → end.
    let line_right = content.x + last.x as i32 + last.width as i32;
    let line_left = content.x + first.x as i32;
    if (x as i32) >= line_right {
        Some(rdom_core::Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    } else if (x as i32) < line_left {
        Some(rdom_core::Position::new(
            first.text_node,
            first.source_byte_offset,
        ))
    } else {
        Some(rdom_core::Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    }
}

/// Clear router drag state. Call from `mouseup` regardless of
/// whether the up landed on text — the pointer capture is what
/// kept the drag alive, and it's auto-released by the router.
pub(crate) fn end(router: &mut Router) {
    router.selection_drag = None;
}

// `ifc_block_of` retired — drag now stores `InlineFlow` directly,
// resolved via `inline_flow_for_text`. The pointer-capture holder
// falls back to `InlineFlow::owner()` (the container node id).
