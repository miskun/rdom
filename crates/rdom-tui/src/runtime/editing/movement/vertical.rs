//! Line navigation: Up / Down with sticky-x, and the line edges
//! (Home / End within a wrapped line) that `selection::keyboard`'s
//! extension shares.

use rdom_core::{Position, Selection};

use crate::TuiDom;
use crate::render::inline::caret_cell;
use crate::runtime::hit_test::HitTestExt;

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
pub(super) fn move_vertical(dom: &mut TuiDom, delta_y: i32) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let from = sel.focus;
    let new_pos = vertical_motion(dom, from, delta_y);
    if let Some(to) = new_pos
        && to != from
    {
        dom.set_selection(Some(Selection::caret(to)));
        crate::runtime::scrollbar::reveal_caret(dom);
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
    use crate::render::inline::inline_flow_for_text;

    let from_flow = inline_flow_for_text(dom, from.node)?;
    // Signed: the caret may be above the screen (scrolled) or left of it
    // (an overflowing `rtl` line); the goal column is a screen column.
    let (current_x, current_y) = caret_cell(dom, from)?;
    let current_x = u16::try_from(current_x.max(0)).unwrap_or(u16::MAX);

    let editable = crate::node::nearest_editable_ancestor(dom, from.node);
    let stored_sticky = editable.and_then(|id| {
        dom.node(id)
            .ext()
            .and_then(|e| e.editor_state.as_ref())
            .and_then(|s| s.sticky_x())
    });
    let target_x = stored_sticky.unwrap_or(current_x);

    let new_pos = compute_vertical_target(dom, from_flow, target_x, current_y, delta_y);

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
    let flow = crate::render::inline::inline_flow_for_text(dom, from.node)?;
    let (_, y) = caret_cell(dom, from)?;
    let (layout, content) = crate::render::inline::inline_flow_layout(dom, flow)?;
    let row = u16::try_from(y - content.y).ok()?;
    let target_line = &layout.lines[layout.line_at_row(row)?];
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

/// Resolve the target `Position` for vertical motion `delta_y` lines
/// from the caret row `from_y`, at column `target_x`, inside
/// `from_flow`. Motion goes line box by line box — a line holding a
/// tall inline block spans several rows, its text on one of them (CSS
/// 2.1 §10.8). Handles the in-bounds case via hit-test, clamps to
/// end-of-line for shorter lines, and clamps to line-start / line-end
/// at the edges of content (Up-at-top / Down-at-bottom).
fn compute_vertical_target(
    dom: &TuiDom,
    from_flow: crate::render::inline::InlineFlow,
    target_x: u16,
    from_y: i32,
    delta_y: i32,
) -> Option<Position> {
    use crate::render::inline::{inline_flow_for_text, inline_flow_layout};

    let (layout, content) = inline_flow_layout(dom, from_flow)?;
    // The caret's line; a caret past the last line box (a phantom line
    // after a trailing newline) counts one row per line there.
    let row = from_y - content.y;
    let height = i32::from(layout.height());
    let from_line = match u16::try_from(row).ok().and_then(|r| layout.line_at_row(r)) {
        Some(i) => i as i64,
        None if row >= height => layout.lines.len() as i64 + i64::from(row - height),
        None => i64::from(row),
    };
    let target_line_i = from_line + i64::from(delta_y);
    // The target line's text row, or a row just outside the content.
    let target_y_i32 = match usize::try_from(target_line_i) {
        Ok(i) if i < layout.lines.len() => content.y + i32::from(layout.lines[i].text_row()),
        Ok(_) => content.y + i32::from(layout.height()),
        Err(_) => content.y - 1,
    };

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

    // Down past the last line → clamp to line-end of last line.
    let Some(target_line_idx) = usize::try_from(target_line_i)
        .ok()
        .filter(|&i| i < layout.lines.len())
    else {
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
    };

    let target_y = target_y_i32 as u16;

    // In-bounds: try hit-test at target_x first.
    if let Some(pos) = dom.position_at(target_x, target_y)
        && inline_flow_for_text(dom, pos.node) == Some(from_flow)
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
