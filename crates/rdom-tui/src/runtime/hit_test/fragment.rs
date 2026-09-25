//! Fragment-level position resolution — a screen cell inside a known
//! inline-flow target → a `(text_node, byte_offset)` [`Position`].
//!
//! Owns [`resolve_in_target`] and its helpers: the fragment lookup
//! `fragment_at_layout`, the out-of-fragment clamp
//! `clamp_to_line_layout` (past end-of-line, above / below the block),
//! and the grapheme-aware cell → byte walker `cells_to_bytes`. Which
//! target to resolve in is decided upstream, in `nearest.rs`.
//!
//! Generated content (`LineBox::generated`) is not a fragment: it has
//! no DOM position, so a cell it covers falls to the clamp — before the
//! line's text → the text's start, past it → the text's end.

use rdom_core::{Dom, Position};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
            let cell_offset_in_frag = (x as i32 - content.x - fragment.x as i32).max(0) as u16;
            let bytes_into_text = cells_to_bytes(&fragment.text, cell_offset_in_frag);
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
        let raw = (y as i32 - content.y) as usize;
        (raw.min(layout.lines.len() - 1), false, false)
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
                    frag.source_byte_offset + frag.text.len(),
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
            last.source_byte_offset + last.text.len(),
        ));
    }

    // In-bounds y: clamp on x. Past the line's last fragment → end of last
    // fragment. Before the line's first fragment → start of first fragment.
    let line_left = content.x + first.x as i32;
    let line_right = content.x + last.x as i32 + last.width as i32;

    if (x as i32) < line_left {
        Some(Position::new(first.text_node, first.source_byte_offset))
    } else if (x as i32) >= line_right {
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    } else {
        // Somewhere in the middle of the line but no fragment
        // covered the cell (gap between fragments, shouldn't be
        // common). Clamp to the last fragment's end as a fallback.
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
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
    let line_index = y as i32 - content.y;
    if line_index < 0 || line_index as usize >= layout.lines.len() {
        return None;
    }
    let line = &layout.lines[line_index as usize];

    let x_local_i = x as i32 - content.x;
    if x_local_i < 0 {
        return None;
    }
    let x_local = x_local_i as u16;

    line.fragments
        .iter()
        .find(|&fragment| x_local >= fragment.x && x_local < fragment.x + fragment.width)
        .map(|v| v as _)
}

/// Walk graphemes of `text` counting cell widths; return the byte
/// offset of the grapheme whose cell range contains `target_cells`.
///
/// Cell grain is per-grapheme (1 for ASCII, 2 for CJK, etc.), not
/// byte length. If `target_cells` falls inside a wide grapheme, the
/// returned offset is the grapheme's *start* byte — the click snaps
/// to the left edge of the character. If `target_cells` overshoots
/// the text's total cell width, returns `text.len()`.
fn cells_to_bytes(text: &str, target_cells: u16) -> usize {
    let mut consumed_cells: u16 = 0;
    for (idx, g) in text.grapheme_indices(true) {
        let w = UnicodeWidthStr::width(g) as u16;
        if target_cells < consumed_cells.saturating_add(w) {
            return idx;
        }
        consumed_cells = consumed_cells.saturating_add(w);
    }
    text.len()
}
