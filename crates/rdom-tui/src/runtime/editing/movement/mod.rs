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

/// Clear sticky-x on the focused editable's `EditorState`. No-op
/// if nothing is focused, the focused element isn't editable, or
/// the editable doesn't yet have an editor state.
pub(crate) fn clear_focused_sticky_x(dom: &mut TuiDom) {
    let Some(focused) = dom.focused() else { return };
    let Some(editable) = crate::node::nearest_editable_ancestor(dom, focused) else {
        return;
    };
    let mut node = dom.node_mut(editable);
    let Some(ext) = node.ext_mut() else { return };
    if let Some(state) = ext.editor_state.as_mut() {
        state.clear_sticky_x();
    }
}

/// Vertical caret motion (Up / Down). Honors sticky-x: the target
/// column is the value last stored in `EditorState.sticky_x`, or
/// the current caret column if no sticky value exists.
///
/// Edge behaviors (matching browser DOM):
///
/// - **Up at top of content** → caret moves to line-start (offset 0
///   of the first line's text).
/// - **Down at bottom of content** → caret moves to line-end (text
///   length of the last line's last fragment).
/// - **Vertical clamp** to a shorter line uses the line's end
///   position; sticky-x is NOT updated (so moving back to a wider
///   line restores the original column).
///
/// Always returns `true` (consumes the key) — even a no-op
/// movement shouldn't fall through to focus navigation.
fn move_vertical(dom: &mut TuiDom, delta_y: i32) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let from = sel.focus;
    let new_pos = vertical_motion(dom, from, delta_y);
    if let Some(to) = new_pos
        && to != from
    {
        dom.set_selection(Some(Selection::caret(to)));
    }
    true
}

/// Compute the target `Position` for vertical motion starting at
/// `from`, honoring sticky-x. Updates `EditorState.sticky_x` on the
/// nearest editable ancestor so subsequent vertical motions reuse
/// the same column even after clamping to shorter lines.
///
/// Returns the resolved target — caller decides what to do with it
/// (move caret to it, or extend selection focus to it). Shared by
/// [`move_vertical`] (bare Up/Down) and `selection::keyboard`'s
/// Shift+Up/Down handlers so the sticky-x state survives across
/// caret-move and selection-extend operations seamlessly.
pub(crate) fn vertical_motion(dom: &mut TuiDom, from: Position, delta_y: i32) -> Option<Position> {
    use crate::render::inline::inline_flow_container;

    let from_ifc = inline_flow_container(dom, from.node)?;
    let (current_x, current_y) = cell_of_position(dom, from)?;

    let editable = crate::node::nearest_editable_ancestor(dom, from.node);
    let stored_sticky = editable.and_then(|id| {
        dom.node(id)
            .ext()
            .and_then(|e| e.editor_state.as_ref())
            .and_then(|s| s.sticky_x())
    });
    let target_x = stored_sticky.unwrap_or(current_x);

    let target_y_i32 = current_y as i32 + delta_y;
    let new_pos = compute_vertical_target(dom, from_ifc, target_x, target_y_i32, delta_y);

    if let Some(id) = editable
        && let Some(ext) = dom.node_mut(id).ext_mut()
    {
        let state = ext.editor_state.get_or_insert_with(|| {
            Box::new(crate::runtime::editing::editor_state::EditorState::new())
        });
        state.set_sticky_x(target_x);
    }

    new_pos
}

/// Compute the position at the start (`forward == false`) or end
/// (`forward == true`) of the line containing `from`. Mirrors the
/// internal `caret_line_start` / `caret_line_end` but exposed so
/// `selection::keyboard` can implement Shift+Home / Shift+End by
/// extending selection focus to the same target.
pub(crate) fn line_edge_position(dom: &TuiDom, from: Position, forward: bool) -> Option<Position> {
    let from_ifc = crate::render::inline::inline_flow_container(dom, from.node)?;
    let (_, y) = cell_of_position(dom, from)?;
    let ext = dom.node(from_ifc).ext()?;
    let layout = ext.inline_layout.as_ref()?;
    let content = ext.content_layout;
    let line_idx = (y as i32 - content.y) as usize;
    let target_line = layout.lines.get(line_idx)?;
    if forward {
        target_line
            .fragments
            .last()
            .map(|f| Position::new(f.text_node, f.source_byte_offset + f.text.len()))
    } else {
        target_line
            .fragments
            .first()
            .map(|f| Position::new(f.text_node, f.source_byte_offset))
    }
}

/// Resolve the target `Position` for vertical motion given a
/// desired `(target_x, target_y)` inside `from_ifc`. Handles the
/// in-bounds case via hit-test, clamps to end-of-line for shorter
/// lines, and clamps to line-start / line-end at the edges of
/// content (Up-at-top / Down-at-bottom).
fn compute_vertical_target(
    dom: &TuiDom,
    from_ifc: NodeId,
    target_x: u16,
    target_y_i32: i32,
    delta_y: i32,
) -> Option<Position> {
    use crate::render::inline::inline_flow_container;

    let ext = dom.node(from_ifc).ext()?;
    let layout = ext.inline_layout.as_ref()?;
    let content = ext.content_layout;

    // Up past the first line → clamp to line-start of first line.
    if target_y_i32 < content.y {
        if delta_y < 0
            && let Some(first_line) = layout.lines.first()
            && let Some(first_frag) = first_line.fragments.first()
        {
            return Some(Position {
                node: first_frag.text_node,
                offset: first_frag.source_byte_offset,
            });
        }
        return None;
    }

    let target_line_idx = (target_y_i32 - content.y) as usize;

    // Down past the last line → clamp to line-end of last line.
    if target_line_idx >= layout.lines.len() {
        if delta_y > 0
            && let Some(last_line) = layout.lines.last()
            && let Some(last_frag) = last_line.fragments.last()
        {
            return Some(Position {
                node: last_frag.text_node,
                offset: last_frag.source_byte_offset + last_frag.text.len(),
            });
        }
        return None;
    }

    let target_y = target_y_i32 as u16;

    // In-bounds: try hit-test at target_x first.
    if let Some(pos) = dom.position_at(target_x, target_y)
        && inline_flow_container(dom, pos.node) == Some(from_ifc)
    {
        return Some(pos);
    }

    // Target line exists but target_x is past its content — clamp to
    // the line's last-fragment-end. Sticky-x stays at target_x (caller
    // doesn't update it); next vertical motion may restore the column
    // if it falls back into a wider line.
    let target_line = &layout.lines[target_line_idx];
    if let Some(last_frag) = target_line.fragments.last() {
        return Some(Position {
            node: last_frag.text_node,
            offset: last_frag.source_byte_offset + last_frag.text.len(),
        });
    }

    // Empty line in the middle of content (e.g. blank line between
    // two non-empty ones via `\n\n`). Position the caret at the
    // start of that line — the phantom path in `cell_of_position`
    // handles rendering. Returning None preserves the previous
    // caret position which is acceptable for v1.
    None
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
    let from_ifc = crate::render::inline::inline_flow_container(dom, from.node)?;
    let (_, y) = cell_of_position(dom, from)?;
    let ext = dom.node(from_ifc).ext()?;
    let layout = ext.inline_layout.as_ref()?;
    let content = ext.content_layout;
    let line_idx = (y as i32 - content.y) as usize;
    let target_line = layout.lines.get(line_idx)?;
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
    let from_ifc = crate::render::inline::inline_flow_container(dom, from.node)?;
    let (_, y) = cell_of_position(dom, from)?;
    let ext = dom.node(from_ifc).ext()?;
    let layout = ext.inline_layout.as_ref()?;
    let content = ext.content_layout;
    let line_idx = (y as i32 - content.y) as usize;
    let target_line = layout.lines.get(line_idx)?;
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
