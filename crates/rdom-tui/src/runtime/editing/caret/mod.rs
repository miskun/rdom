//! Caret position math — the reverse of hit-testing.
//!
//! Given a browser-faithful `Position { text_node, byte_offset }`,
//! figure out the `(cell_x, cell_y)` on the terminal grid where
//! the caret should paint. Used by `render::paint_pass` when it
//! encounters a focused editable with a collapsed selection.
//!
//! The forward direction ("cell → position") already exists as
//! `HitTestExt::position_at`. This module's `cell_of_position`
//! closes the loop.
//!
//! ## Algorithm
//!
//! 1. Walk up from the caret's text node to find the enclosing IFC
//!    block (the nearest ancestor with a populated `inline_layout`).
//! 2. Scan its lines; for each line, scan fragments; find the
//!    fragment whose `text_node` and byte range cover the caret
//!    position.
//! 3. Convert the in-fragment byte offset to a cell offset via
//!    unicode-width per grapheme.
//! 4. Return `(ifc.content_rect.x + fragment.x + cell_offset,
//!    ifc.content_rect.y + line_index)`.
//!
//! End-of-text / end-of-line cases: the caret sits *after* the
//! last fragment's last grapheme. `cells_before_byte` with a
//! target past the fragment's own text length returns the full
//! fragment width, putting the caret just past the rendered run.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use rdom_core::{Dom, Position};

use crate::ext::TuiExt;
use crate::render::inline::{
    InlineFragment, InlineLayout, inline_flow_for_text, inline_flow_layout,
};

/// Screen-cell position where a caret at `pos` should paint.
/// Returns `None` when:
///
/// - The text node isn't inside any IFC subtree (editable wasn't
///   laid out, or sits in a non-IFC container).
/// - No fragment in the IFC covers the position (shouldn't happen
///   for a caret set by `position_at`, but handled defensively).
pub fn cell_of_position(dom: &Dom<TuiExt>, pos: Position) -> Option<(u16, u16)> {
    let flow = inline_flow_for_text(dom, pos.node)?;
    let (layout, content) = inline_flow_layout(dom, flow)?;

    // Phantom-position fallback: when `pos` doesn't correspond to
    // any laid-out fragment (empty text, cursor sitting just past a
    // trailing `\n`, etc.), reconstruct (line, column) from the
    // text itself rather than failing. Without this, the caret is
    // invisible at end-of-content after Enter inserts a newline.
    if fragment_for_position(layout, pos).is_none() {
        let text = dom.node(pos.node).node_value()?;
        let (line_idx, col) = phantom_line_and_column(text, pos.offset)?;
        let x = (content.x + col as i32).max(0) as u16;
        let y = (content.y + line_idx as i32).max(0) as u16;
        return Some((x, y));
    }

    let (line_idx, fragment) = fragment_for_position(layout, pos)?;

    let offset_in_frag = pos.offset.saturating_sub(fragment.source_byte_offset);
    let cell_in_frag = cells_before_byte(&fragment.text, offset_in_frag);

    let x = (content.x + fragment.x as i32 + cell_in_frag as i32).max(0) as u16;
    let y = (content.y + line_idx as i32).max(0) as u16;
    Some((x, y))
}

/// Compute (line_index, column) for a position that has no
/// corresponding fragment in the inline layout — counts `\n`s in
/// the text up to `offset` and measures the column as the width of
/// the prefix after the last `\n`. Used by [`cell_of_position`] for
/// caret positions at end-of-content past a trailing newline,
/// inside empty text nodes, etc.
fn phantom_line_and_column(text: &str, offset: usize) -> Option<(usize, u16)> {
    if offset > text.len() {
        return None;
    }
    let prefix = &text[..offset];
    let line_idx = prefix.matches('\n').count();
    let last_break = prefix.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let col_text = &prefix[last_break..];
    let col = UnicodeWidthStr::width(col_text) as u16;
    Some((line_idx, col))
}

/// Find the `(line_idx, fragment)` in `layout` that covers
/// `(pos.node, pos.offset)`. Falls back to the last fragment on
/// the last line when the position sits at the end of the text
/// (caret after the final character).
fn fragment_for_position(layout: &InlineLayout, pos: Position) -> Option<(usize, &InlineFragment)> {
    for (line_idx, line) in layout.lines.iter().enumerate() {
        for fragment in &line.fragments {
            if fragment.text_node != pos.node {
                continue;
            }
            let frag_end = fragment.source_byte_offset + fragment.text.len();
            if fragment.source_byte_offset <= pos.offset && pos.offset <= frag_end {
                return Some((line_idx, fragment));
            }
        }
    }
    None
}

/// Visible cells before byte offset `target` in `text`. Mirrors
/// `paint_pass::inline_paint::cells_before_byte` — extracted here
/// so the caret path doesn't cross into paint internals.
fn cells_before_byte(text: &str, target: usize) -> u16 {
    let mut cells: u16 = 0;
    for (idx, g) in text.grapheme_indices(true) {
        if idx >= target {
            return cells;
        }
        cells = cells.saturating_add(UnicodeWidthStr::width(g) as u16);
    }
    cells
}

#[cfg(test)]
mod tests;
