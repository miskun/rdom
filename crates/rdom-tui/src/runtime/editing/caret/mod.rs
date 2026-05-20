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

use rdom_core::{Dom, NodeId, Position};

use crate::ext::TuiExt;
use crate::render::inline::{InlineFragment, InlineLayout};
use crate::render::layout_pass::is_ifc_block;

/// Screen-cell position where a caret at `pos` should paint.
/// Returns `None` when:
///
/// - The text node isn't inside any IFC subtree (editable wasn't
///   laid out, or sits in a non-IFC container).
/// - No fragment in the IFC covers the position (shouldn't happen
///   for a caret set by `position_at`, but handled defensively).
pub fn cell_of_position(dom: &Dom<TuiExt>, pos: Position) -> Option<(u16, u16)> {
    let ifc_id = ifc_block_of(dom, pos.node)?;
    let ifc_ext = dom.node(ifc_id).ext()?;
    let layout = ifc_ext.inline_layout.as_ref()?;
    let content = ifc_ext.content_layout;

    let (line_idx, fragment) = fragment_for_position(layout, pos)?;

    let offset_in_frag = pos.offset.saturating_sub(fragment.source_byte_offset);
    let cell_in_frag = cells_before_byte(&fragment.text, offset_in_frag);

    let x = (content.x + fragment.x as i32 + cell_in_frag as i32).max(0) as u16;
    let y = (content.y + line_idx as i32).max(0) as u16;
    Some((x, y))
}

/// Walk up from `node_id` to the nearest element with an
/// `inline_layout` (i.e., an IFC block). Inclusive — if `node_id`
/// itself is an element and it's an IFC, returns it. The common
/// case is `node_id` is a text node; we start from its parent.
fn ifc_block_of(dom: &Dom<TuiExt>, node_id: NodeId) -> Option<NodeId> {
    let mut cur = Some(node_id);
    while let Some(id) = cur {
        if is_ifc_block(dom, id) {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
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
