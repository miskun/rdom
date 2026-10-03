//! Mouse-drag text selection — `mousedown` begins, `mousemove`
//! extends focus, `mouseup` ends.
//!
//! Called from `runtime/router/mouse/mod.rs` as default actions
//! on the mouse pipeline:
//!
//! - `begin`: tries to start a drag on `mousedown`. Returns true
//!   when the press resolved to selectable text and a drag is now
//!   active — recorded in `router.selection_drag`.
//! - `extend`: on a button-held `mousemove` while a drag is active,
//!   moves the selection's `focus` to the cursor's current position,
//!   wherever the pointer is. Preserves the original anchor so
//!   dragging backward shrinks the selection symmetrically.
//! - `end`: clears router drag state (on `mouseup`, on a
//!   button-less move that shows the `mouseup` was lost, and at the
//!   start of every `mousedown`).
//!
//! ## No pointer capture
//!
//! A browser does not capture the pointer for a text-selection drag:
//! during it `mousemove` / `mouseup` target whatever is under the
//! pointer, and `click` goes to the common ancestor of the mousedown
//! and mouseup targets (UI Events). So the drag lives in router-private
//! state, not in `Dom::pointer_capture` — the router extends the
//! selection on every button-held move regardless of the hit target,
//! and the event targeting stays untouched. (Taking capture here once
//! retargeted the `click` of any widget beside prose to the prose
//! container: P6G-SELECTION-CAPTURE-1.) Edge autoscroll keys on the
//! same router state (`App::note_autoscroll`).
//!
//! ## What "selectable" means here
//!
//! A click is selectable iff `dom.position_at(x, y)` returns
//! `Some(_)`. That function already walks to the innermost IFC
//! block and rejects `user-select: none` subtrees, so this file
//! doesn't duplicate those checks.
//!
//! ## Editing hosts keep their selection
//!
//! A press inside a text control or `contenteditable` host anchors
//! inside that host, never in the prose around it (`press_position`):
//! `position_at`'s empty-space snap to the nearest prose would otherwise
//! put the caret in a `<label>` beside an input whose text has no layout
//! yet, and the next keystroke would be lost (`EDIT-CLICK-IN-CONTROL-1`).
//! The drag that follows stays in the host too: an editing host's used
//! `user-select` is `contain`, so `extend` clamps the focus back into it.

use crossterm::event::MouseEvent;

use rdom_core::{Position, Selection};

use crate::TuiDom;
use crate::node::{
    is_descendant_or_self, last_text_descendant, nearest_editable_ancestor, text_len,
};
use crate::render::inline::inline_flow_for_text;
use crate::runtime::hit_test::{HitTestExt, nearest_inline_target_in_subtree, resolve_in_target};
use crate::runtime::router::Router;
use crate::runtime::selection::user_select;

/// Default action for `mousedown`: begin a drag-select if the
/// click landed on selectable text.
///
/// Returns `true` when a drag was started. The "we're dragging"
/// signal for follow-up moves is `router.selection_drag.is_some()`,
/// set by this function.
pub(crate) fn begin(router: &mut Router, dom: &mut TuiDom, mouse: MouseEvent) -> bool {
    let Some(anchor) = press_position(dom, mouse) else {
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
    // text.
    if dom.node(anchor.node).node_type() != rdom_core::NodeType::Text {
        return false;
    }

    // `user-select: all`: a click anywhere inside the host element
    // selects its entire text content as a single unit. The drag
    // still begins, but `extend` becomes a no-op for the
    // duration — the highlight doesn't shrink as the user moves the
    // mouse.
    let initial = match user_select::all_host(dom, anchor.node) {
        Some(host) => {
            user_select::span_all_text(dom, host).unwrap_or_else(|| Selection::caret(anchor))
        }
        None => Selection::caret(anchor),
    };
    dom.set_selection(Some(initial));

    // The drag is router state, keyed on the inline-flow container that
    // holds the anchor (a classic IFC block, or one of a parent's anonymous
    // block boxes — BFC-1 phase 3). No DOM pointer capture: see the module
    // doc. The same state opts the drag into edge autoscroll
    // (DRAG-AUTOSCROLL), like a browser's native selection.
    let anchor_flow = inline_flow_for_text(dom, anchor.node);
    router.selection_drag = anchor_flow;
    true
}

/// Where a press at `mouse` puts the selection anchor.
///
/// A press whose hit target is inside an editing host — a text-family
/// `<input>`, a `<textarea>`, a `contenteditable` host
/// ([`nearest_editable_ancestor`]) — resolves *inside that host*, as a
/// browser's does: clicking a text control always puts the caret in it,
/// and never starts a selection in the page around it
/// (`EDIT-CLICK-IN-CONTROL-1`). In order:
///
/// 1. the fragment-exact position under the pointer, when it is the
///    host's own text;
/// 2. the host's inline flow nearest the pointer's row, resolved like
///    any hit on it (past a line's end → that line's end);
/// 3. with no laid-out text in the host — its text was inserted after
///    the last layout — the end of its text (`0` when empty): with no
///    layout the column cannot be mapped, and the end is where a browser
///    puts the caret for a press past the text.
///
/// `None` (no drag) when the host is `user-select: none` or holds no
/// text node at all. Any other press resolves through
/// [`HitTestExt::position_at`], whose empty-space snap to the nearest
/// prose only applies outside editing hosts.
fn press_position(dom: &TuiDom, mouse: MouseEvent) -> Option<Position> {
    let host = dom
        .hit_test(mouse.column, mouse.row)
        .and_then(|hit| nearest_editable_ancestor(dom, hit));
    let Some(host) = host else {
        return dom.position_at(mouse.column, mouse.row);
    };
    if user_select::is_unselectable(dom, host) {
        return None;
    }
    if let Some(pos) = dom.position_at(mouse.column, mouse.row)
        && is_descendant_or_self(dom, pos.node, host)
    {
        return Some(pos);
    }
    if let Some(target) = nearest_inline_target_in_subtree(dom, host, mouse.row)
        && let Some(pos) = resolve_in_target(dom, target, mouse.column, mouse.row)
    {
        return Some(pos);
    }
    let last = last_text_descendant(dom, host)?;
    Some(Position::new(last, text_len(dom, last)))
}

/// Default action for `mousemove` (while `router.selection_drag` is
/// set): extend the selection's focus to the cursor's current
/// position. Returns `true` when the selection actually changed —
/// caller uses it to request a redraw.
///
/// When the cursor moves outside any selectable text (onto a
/// `user-select: none` bar, into a gap, or past end-of-line) the focus
/// snaps to the nearest selectable position to the pointer via
/// [`HitTestExt::nearest_selectable_position`] — browsers extend the
/// selection past such regions rather than freezing or collapsing it.
pub(crate) fn extend(dom: &mut TuiDom, mouse: MouseEvent) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };

    // `user-select: all`: the host is selected as a unit, so the
    // drag doesn't update focus while the cursor moves.
    if user_select::all_host(dom, sel.anchor.node).is_some() {
        return false;
    }

    // Prefer the hit-tested position — it may land in a DIFFERENT
    // inline-flow container, which is correct for cross-paragraph
    // drag selection (browsers let the selection span multiple
    // paragraphs). When the cursor is over non-selectable space —
    // a `user-select: none` bar, a gap, past end-of-line — `position_at`
    // yields nothing, so snap to the nearest SELECTABLE position to the
    // POINTER (not the anchor flow): dragging up over a `user-select: none`
    // chrome bar must extend to the text beyond it, not collapse the
    // selection back into the anchor block far below the cursor.
    let raw_focus = match dom.position_at(mouse.column, mouse.row) {
        Some(p) => p,
        None => match dom.nearest_selectable_position(mouse.column, mouse.row) {
            Some(p) => p,
            None => return false,
        },
    };

    // `user-select: contain`: the host traps the selection. If
    // `raw_focus` escaped the contain host, clamp it back to the
    // nearest in-host position.
    let focus = match user_select::contain_host(dom, sel.anchor.node) {
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

// `clamp_to_anchor_flow` retired: the no-position fallback now snaps to the
// nearest *selectable* flow to the pointer via
// `HitTestExt::nearest_selectable_position` (so dragging over a
// `user-select: none` bar extends past it instead of collapsing back into the
// anchor flow). The y-overshoot clamp it used lives in `position_at`'s
// `clamp_to_line_layout`, shared by both the contained and nearest paths.

/// Clear router drag state. Call from `mouseup` regardless of
/// whether the up landed on text, from a button-less move (the
/// `mouseup` was lost outside the terminal), and at the start of every
/// `mousedown` (a lost `mouseup` with no button-less motion reported).
pub(crate) fn end(router: &mut Router) {
    router.selection_drag = None;
}
