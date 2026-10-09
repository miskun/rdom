//! Fragment-level position resolution — a screen cell inside a known
//! inline-flow target → a `(text_node, byte_offset)` [`Position`].
//!
//! Owns [`resolve_in_target`] and its helpers: the fragment lookup
//! `fragment_at_layout`, the out-of-fragment clamp
//! `clamp_to_line_layout` (past end-of-line, above / below the block),
//! and the cell → source byte walk (`InlineFragment::source_at_cell`). Which
//! target to resolve in is decided upstream, in `nearest.rs`.
//!
//! Generated content (`LineBox::generated`) is not a fragment: it has
//! no DOM position, so a cell it covers falls to the clamp — before the
//! line's text → the text's start, past it → the text's end.

use rdom_core::{Dom, Position};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::inline::InlineFragment;

use super::nearest::InlineTarget;

/// Resolve a screen cell to a [`Position`] within a known inline-flow
/// `target`. Returns the fragment-exact position when a fragment covers
/// `(x, y)`, else clamps to the nearest valid position on the target's
/// lines (drag past end-of-line / past last-line bottom — see
/// [`clamp_to_line_layout`]).
pub(crate) fn resolve_in_target(
    dom: &Dom<TuiExt>,
    target: InlineTarget,
    x: u16,
    y: u16,
) -> Option<Position> {
    let (inline_layout, content) = target.layout_and_rect(dom)?;
    match fragment_at_layout(inline_layout, content, x, y) {
        Some(fragment) => {
            let cell_offset_in_frag = (x as i32 - content.x - fragment.x).max(0) as u16;
            let bytes_into_text = fragment.source_at_cell(cell_offset_in_frag);
            Some(Position::new(
                fragment.text_node,
                fragment.source_byte_offset + bytes_into_text,
            ))
        }
        None => clamp_to_line_layout(inline_layout, content, x, y),
    }
}

/// Clamp `(x, y)` to the nearest valid position on the inline layout
/// of `ifc_id`. Used when the hit cell isn't covered by a fragment —
/// drag past end-of-line, click past last-line bottom, etc.
///
/// Rules:
/// - `y < content.y` → first line's start position.
/// - `y >= content.y + content.height` → last line's end position.
/// - In-bounds y, x past line's content → that line's end position.
/// - In-bounds y, line is empty → walk to the nearest non-empty line.
fn clamp_to_line_layout(
    layout: &crate::render::inline::InlineLayout,
    content: crate::layout::LayoutRect,
    x: u16,
    y: u16,
) -> Option<Position> {
    if layout.lines.is_empty() {
        return None;
    }

    // A y-overshoot dominates the x logic: a point above the block snaps to
    // the FIRST line's start, below it to the LAST line's end — regardless of
    // x (matching the drag-extend clamp in `selection::drag`). Only an
    // in-bounds y consults x to pick the position along the line.
    let (line_idx, overshoot_up, overshoot_down) = if (y as i32) < content.y {
        (0, true, false)
    } else if (y as i32) >= content.y + content.height as i32 {
        (layout.lines.len() - 1, false, true)
    } else {
        let row = (y as i32 - content.y) as u16;
        let line = layout
            .line_at_point(x as i32 - content.x, row)
            .unwrap_or(layout.lines.len() - 1);
        (line, false, false)
    };

    let target_line = &layout.lines[line_idx];

    // Empty line — try walking out to find a non-empty fragment.
    // Falls back to the last line's last fragment if everything's
    // empty (shouldn't happen for a populated IFC, but defensive).
    if target_line.fragments.is_empty() {
        for line in layout.lines.iter().rev() {
            if let Some(frag) = line.fragments.last() {
                return Some(Position::new(
                    frag.text_node,
                    frag.source_byte_offset + frag.source_len(),
                ));
            }
        }
        return None;
    }

    let first = target_line.fragments.first().unwrap();
    let last = target_line.fragments.last().unwrap();

    // Y overshoot dominates: up → first line start, down → last line end.
    if overshoot_up {
        return Some(Position::new(first.text_node, first.source_byte_offset));
    }
    if overshoot_down {
        return Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.source_len(),
        ));
    }

    // In-bounds y: clamp on x. Past the line's last fragment → end of last
    // fragment. Before the line's first fragment → start of first fragment.
    let line_left = content.x + first.x;
    let line_right = content.x + last.x + i32::from(last.width);

    if (x as i32) < line_left {
        Some(Position::new(first.text_node, first.source_byte_offset))
    } else if (x as i32) >= line_right {
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.source_len(),
        ))
    } else {
        // Somewhere in the middle of the line but no fragment
        // covered the cell (gap between fragments, shouldn't be
        // common). Clamp to the last fragment's end as a fallback.
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.source_len(),
        ))
    }
}

/// Look up the `InlineFragment` under `(x, y)` inside an IFC
/// block's content area. Returns a reference into the block's
/// stored `InlineLayout` — the caller extracts whatever info it
/// needs (owner, text_node, source offset) without cloning.
fn fragment_at_layout(
    layout: &crate::render::inline::InlineLayout,
    content: LayoutRect,
    x: u16,
    y: u16,
) -> Option<&InlineFragment> {
    let row = u16::try_from(y as i32 - content.y).ok()?;
    // Negative left of the content box (an overflowing `rtl` line).
    let x_local = x as i32 - content.x;
    let line = &layout.lines[layout.line_at_point(x_local, row)?];

    // A text fragment's inline box spans its line's rows — the leading
    // above and below its glyph row (CSS 2.1 §10.8.1) — so a cell on any
    // of them hits it; an atom only on its own rows.
    line.fragments
        .iter()
        .find(|&fragment| {
            x_local >= fragment.x
                && x_local < fragment.x + i32::from(fragment.width)
                && (!fragment.atomic || line.covers(fragment, row))
        })
        .map(|v| v as _)
}
