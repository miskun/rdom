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

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::runtime::editing::caret::cell_of_position;
use crate::runtime::editing::perform::{Edit, EditOutcome, perform_edit};
use crate::runtime::hit_test::HitTestExt;
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

    match key.code {
        KeyCode::Backspace => delete_back(dom),
        KeyCode::Delete => delete_forward(dom),
        KeyCode::Left if ctrl => move_caret(dom, caret_word_left),
        KeyCode::Right if ctrl => move_caret(dom, caret_word_right),
        KeyCode::Left => move_caret_left_collapse_or_grapheme(dom),
        KeyCode::Right => move_caret_right_collapse_or_grapheme(dom),
        KeyCode::Home if ctrl => move_caret(dom, caret_doc_start),
        KeyCode::End if ctrl => move_caret(dom, caret_doc_end),
        KeyCode::Home => move_caret(dom, caret_line_start),
        KeyCode::End => move_caret(dom, caret_line_end),
        KeyCode::Up => move_caret(dom, caret_up),
        KeyCode::Down => move_caret(dom, caret_down),
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
    apply_edit(dom, edit)
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
    apply_edit(dom, edit)
}

fn apply_edit(dom: &mut TuiDom, edit: Edit) -> bool {
    matches!(
        perform_edit(dom, edit),
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
    let (_, y) = cell_of_position(dom, from)?;
    // Find the first fragment on this line by hit-testing x=0
    // within the IFC's content rect. `HitTestExt::position_at`
    // snaps to the nearest valid position; with x=0 it lands at
    // the first cell of the row, which maps to the start of the
    // first fragment — i.e. line start.
    //
    // Fall back to a 0-offset position in the caret's node if the
    // hit-test somehow fails (shouldn't in practice).
    dom.position_at(0, y)
        .or_else(|| Some(Position::new(from.node, 0)))
}

fn caret_line_end(dom: &TuiDom, from: Position) -> Option<Position> {
    let (_, y) = cell_of_position(dom, from)?;
    // `position_at(u16::MAX, y)` snaps to the last cell of the
    // row — the end of the last fragment on the line. `position_at`
    // already clamps x to valid bounds for us.
    dom.position_at(u16::MAX, y).or_else(|| {
        let text = dom.node(from.node).node_value()?;
        Some(Position::new(from.node, text.len()))
    })
}

fn caret_up(dom: &TuiDom, from: Position) -> Option<Position> {
    let (x, y) = cell_of_position(dom, from)?;
    if y == 0 {
        return None;
    }
    dom.position_at(x, y - 1)
}

fn caret_down(dom: &TuiDom, from: Position) -> Option<Position> {
    let (x, y) = cell_of_position(dom, from)?;
    dom.position_at(x, y.saturating_add(1))
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
