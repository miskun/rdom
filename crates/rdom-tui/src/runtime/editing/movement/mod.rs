//! Caret movement — bare arrows, Ctrl+arrows, Home/End, Up/Down
//! line navigation. Plus grapheme-aware Backspace / Delete.
//!
//! All movement functions return the *new* caret `Position`
//! without applying it. The caller (in `App::handle_event`'s
//! editable-keydown path) applies via `dom.set_selection(...)`.
//! Delete operations route through `perform_edit` so the full
//! `beforeinput` → mutate → `input` event cycle fires.
//!
//! ## Behavioral notes (matching browsers)
//!
//! - **Bare Left / Right with a non-collapsed selection**
//!   *collapses* the selection (to the start / end respectively)
//!   rather than moving by a grapheme. Subsequent presses then
//!   move by grapheme. This matches macOS + Windows browsers.
//! - **Word movement** reuses Phase 6.5.3's TR29 boundary helpers
//!   via `pub(crate)` re-export from `selection::keyboard`.
//! - **Up / Down** approximate "same cell x" using the caret's
//!   current cell — no sticky-x tracking in v1. Feels right most
//!   of the time; zig-zag on mixed-width lines can be polished
//!   later.
//!
//! Line navigation (Up / Down, the line edges) lives in `vertical`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::render::inline::caret_cell;
use crate::runtime::editing::perform::{Edit, EditOutcome, perform_edit_as};
mod vertical;

use vertical::move_vertical;
pub(crate) use vertical::{clear_focused_sticky_x, line_edge_position, vertical_motion};

use crate::runtime::selection::keyboard::{
    next_grapheme_byte, next_word_byte, prev_grapheme_byte, prev_word_byte,
};

/// Dispatch an editable-side movement / deletion key. Returns
/// `true` when the key was consumed. Callers still need to gate
/// on "focused is editable" — this just looks at the key code.
pub(crate) fn try_handle_movement_key(dom: &mut TuiDom, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SUPER);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // Shift+arrow is selection extension (handled upstream in
    // `selection::keyboard`). Only bare/Ctrl arrows land here.
    if shift {
        return false;
    }

    // Vertical motions PRESERVE sticky-x; everything else CLEARS it.
    match key.code {
        KeyCode::Up => move_vertical(dom, -1),
        KeyCode::Down => move_vertical(dom, 1),
        KeyCode::Backspace => {
            clear_focused_sticky_x(dom);
            delete_back(dom)
        }
        KeyCode::Delete => {
            clear_focused_sticky_x(dom);
            delete_forward(dom)
        }
        KeyCode::Left if ctrl => {
            clear_focused_sticky_x(dom);
            move_caret(dom, caret_word_left)
        }
        KeyCode::Right if ctrl => {
            clear_focused_sticky_x(dom);
            move_caret(dom, caret_word_right)
        }
        KeyCode::Left => {
            clear_focused_sticky_x(dom);
            move_caret_left_collapse_or_grapheme(dom)
        }
        KeyCode::Right => {
            clear_focused_sticky_x(dom);
            move_caret_right_collapse_or_grapheme(dom)
        }
        KeyCode::Home if ctrl => {
            clear_focused_sticky_x(dom);
            move_caret(dom, caret_doc_start)
        }
        KeyCode::End if ctrl => {
            clear_focused_sticky_x(dom);
            move_caret(dom, caret_doc_end)
        }
        KeyCode::Home => {
            clear_focused_sticky_x(dom);
            move_caret(dom, caret_line_start)
        }
        KeyCode::End => {
            clear_focused_sticky_x(dom);
            move_caret(dom, caret_line_end)
        }
        _ => false,
    }
}

// ── Deletion ────────────────────────────────────────────────────────

/// Backspace — delete the grapheme before the caret, or delete the
/// selection if it's a range. Returns `true` when something was
/// consumed (even if the edit was prevented or the caret was at
/// the start with nothing to delete).
fn delete_back(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    // Range: delete whole range. Caret: delete grapheme before.
    let edit = if !sel.is_collapsed() {
        let Some((node, start, end)) = ordered_range(&sel) else {
            return false;
        };
        Edit {
            node,
            range: start..end,
            text: String::new(),
        }
    } else {
        let node = sel.focus.node;
        let text = match dom.node(node).node_value() {
            Some(s) => s.to_string(),
            None => return false,
        };
        let offset = sel.focus.offset.min(text.len());
        let Some(prev) = prev_grapheme_byte(&text, offset) else {
            // At start of text node — nothing to delete; consume
            // the key so it doesn't bubble to Tab nav.
            return true;
        };
        Edit {
            node,
            range: prev..offset,
            text: String::new(),
        }
    };
    apply_edit(dom, edit, rdom_core::InputType::DeleteContentBackward)
}

/// Delete — delete the grapheme after the caret, or delete the
/// selection if it's a range.
fn delete_forward(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let edit = if !sel.is_collapsed() {
        let Some((node, start, end)) = ordered_range(&sel) else {
            return false;
        };
        Edit {
            node,
            range: start..end,
            text: String::new(),
        }
    } else {
        let node = sel.focus.node;
        let text = match dom.node(node).node_value() {
            Some(s) => s.to_string(),
            None => return false,
        };
        let offset = sel.focus.offset.min(text.len());
        let Some(next) = next_grapheme_byte(&text, offset) else {
            return true;
        };
        Edit {
            node,
            range: offset..next,
            text: String::new(),
        }
    };
    apply_edit(dom, edit, rdom_core::InputType::DeleteContentForward)
}

fn apply_edit(dom: &mut TuiDom, edit: Edit, input_type: rdom_core::InputType) -> bool {
    matches!(
        perform_edit_as(dom, edit, input_type),
        EditOutcome::Applied | EditOutcome::Prevented
    )
}

// ── Caret movement ──────────────────────────────────────────────────

/// Left — collapse selection to its start if non-collapsed, else
/// move by one grapheme.
fn move_caret_left_collapse_or_grapheme(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    if !sel.is_collapsed() {
        // Collapse to range.start. The `Selection`'s directionality
        // is preserved, so sort first.
        let (start, _) = ordered_positions(&sel);
        dom.set_selection(Some(Selection::caret(start)));
        crate::runtime::scrollbar::reveal_caret(dom);
        return true;
    }
    move_caret(dom, caret_grapheme_left)
}

/// Right — collapse selection to its end if non-collapsed, else
/// move by one grapheme.
fn move_caret_right_collapse_or_grapheme(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    if !sel.is_collapsed() {
        let (_, end) = ordered_positions(&sel);
        dom.set_selection(Some(Selection::caret(end)));
        crate::runtime::scrollbar::reveal_caret(dom);
        return true;
    }
    move_caret(dom, caret_grapheme_right)
}

/// Apply a caret-computation function. `compute(dom, from)`
/// returns the new caret position or `None` when no move is
/// possible (at boundary). The key is always consumed — even
/// a bounded arrow press shouldn't trigger Tab focus nav.
fn move_caret<F>(dom: &mut TuiDom, compute: F) -> bool
where
    F: FnOnce(&TuiDom, Position) -> Option<Position>,
{
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let from = sel.focus;
    if let Some(to) = compute(dom, from)
        && to != from
    {
        dom.set_selection(Some(Selection::caret(to)));
        crate::runtime::scrollbar::reveal_caret(dom);
    }
    true
}

// ── Position computations ──────────────────────────────────────────

fn caret_grapheme_left(dom: &TuiDom, from: Position) -> Option<Position> {
    let text = dom.node(from.node).node_value()?;
    let offset = from.offset.min(text.len());
    let new = prev_grapheme_byte(text, offset)?;
    Some(Position::new(from.node, new))
}

fn caret_grapheme_right(dom: &TuiDom, from: Position) -> Option<Position> {
    let text = dom.node(from.node).node_value()?;
    let offset = from.offset.min(text.len());
    let new = next_grapheme_byte(text, offset)?;
    Some(Position::new(from.node, new))
}

fn caret_word_left(dom: &TuiDom, from: Position) -> Option<Position> {
    let text = dom.node(from.node).node_value()?;
    let offset = from.offset.min(text.len());
    let new = prev_word_byte(text, offset)?;
    Some(Position::new(from.node, new))
}

fn caret_word_right(dom: &TuiDom, from: Position) -> Option<Position> {
    let text = dom.node(from.node).node_value()?;
    let offset = from.offset.min(text.len());
    let new = next_word_byte(text, offset)?;
    Some(Position::new(from.node, new))
}

fn caret_doc_start(dom: &TuiDom, from: Position) -> Option<Position> {
    // MVP: single-text-node per editable, so doc start = offset 0
    // in the caret's text node.
    let _ = dom; // reserved for cross-node traversal in a later revision
    Some(Position::new(from.node, 0))
}

fn caret_doc_end(dom: &TuiDom, from: Position) -> Option<Position> {
    let text = dom.node(from.node).node_value()?;
    Some(Position::new(from.node, text.len()))
}

fn caret_line_start(dom: &TuiDom, from: Position) -> Option<Position> {
    let flow = crate::render::inline::inline_flow_for_text(dom, from.node)?;
    let (_, y) = caret_cell(dom, from)?;
    let (layout, content) = crate::render::inline::inline_flow_layout(dom, flow)?;
    let row = u16::try_from(y - content.y).ok()?;
    let target_line = &layout.lines[layout.line_at_row(row)?];
    // Start of line = position of the first fragment's first byte.
    // Going through `position_at(0, y)` doesn't work because column
    // 0 sits inside the textarea's left padding (no fragment there)
    // and the fallback would resolve to offset 0 of the whole text,
    // not the start of the current line.
    if let Some(first_frag) = target_line.fragments.first() {
        Some(Position::new(
            first_frag.text_node,
            first_frag.source_byte_offset,
        ))
    } else {
        // Empty line — keep caret where it is.
        Some(from)
    }
}

fn caret_line_end(dom: &TuiDom, from: Position) -> Option<Position> {
    let flow = crate::render::inline::inline_flow_for_text(dom, from.node)?;
    let (_, y) = caret_cell(dom, from)?;
    let (layout, content) = crate::render::inline::inline_flow_layout(dom, flow)?;
    let row = u16::try_from(y - content.y).ok()?;
    let target_line = &layout.lines[layout.line_at_row(row)?];
    // End of line = position just past the last fragment on the
    // line. `position_at(u16::MAX, y)` doesn't work because no
    // fragment covers cells past the line's content; the hit-test
    // returns None and we'd fall through to `text.len()` (end of
    // the whole content, not end of THIS line).
    if let Some(last_frag) = target_line.fragments.last() {
        Some(Position::new(
            last_frag.text_node,
            last_frag.source_byte_offset + last_frag.text.len(),
        ))
    } else {
        // Empty line — keep caret where it is (no good "end" to
        // move to within this line).
        Some(from)
    }
}

// ── Shared utilities ───────────────────────────────────────────────

/// Return `(start, end)` of a selection in byte order, narrowed to
/// a single text node. `None` when the selection spans nodes
/// (caller treats as no-op per the MVP restriction).
fn ordered_range(sel: &Selection) -> Option<(NodeId, usize, usize)> {
    if sel.anchor.node != sel.focus.node {
        return None;
    }
    let (start, end) = if sel.anchor.offset <= sel.focus.offset {
        (sel.anchor.offset, sel.focus.offset)
    } else {
        (sel.focus.offset, sel.anchor.offset)
    };
    Some((sel.anchor.node, start, end))
}

fn ordered_positions(sel: &Selection) -> (Position, Position) {
    if sel.anchor.node == sel.focus.node && sel.anchor.offset <= sel.focus.offset {
        (sel.anchor, sel.focus)
    } else if sel.anchor.node == sel.focus.node {
        (sel.focus, sel.anchor)
    } else {
        // Cross-node: use the document-order range from Dom if
        // available. MVP rarely hits this; fall back to
        // `anchor`/`focus` raw.
        (sel.anchor, sel.focus)
    }
}

#[cfg(test)]
mod tests;
