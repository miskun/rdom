//! The `::selection` highlight overlay: restyles the cells of a
//! painted fragment that fall inside the current (non-collapsed)
//! selection range, keeping the fragment's symbols intact so a
//! re-paint without selection restores the original appearance.
//!
//! Owns the byte-range → cell-range mapping for a fragment
//! (`selection_byte_range_in`, `cells_before_byte`) and the walk to
//! the nearest ancestor with a cascaded `::selection` style. The
//! runtime's `user-select: none` rule decides what is excluded from
//! the highlight; paint only asks.

use rdom_core::{Dom, NodeId, Position, Range};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::render::inline::InlineFragment;
use crate::render::paint_pass::text::style_from_computed;
use crate::render::{Buffer, Rect, Style};
use crate::style::ComputedStyle;

/// Overlay the selection style on cells of `fragment` whose source
/// bytes fall within `range`. Preserves the underlying symbols.
///
/// Author `::selection` styling: walks up from the fragment's text
/// node to the nearest ancestor element with `computed_selection`
/// set and uses that style (fg/bg/modifiers). When no ancestor has
/// a cascaded selection style, falls back to the v1 default
/// transparent overlay (no visual change). The UA's
/// `*::selection { bg: #394B7E; fg: white }` rule ensures the
/// fallback rarely fires.
pub(super) fn apply_selection_overlay(
    dom: &Dom<TuiExt>,
    buf: &mut Buffer,
    line_y: u16,
    frag_x: i32,
    clip: Rect,
    fragment: &InlineFragment,
    range: &Range,
) {
    // `user-select: none` content is excluded from a selection's highlight
    // (and from copy) even when the selection spans across it — e.g. dragging
    // from a title down through a `user-select: none` chrome bar into the body
    // must not paint the bar. Browsers skip such content; so do we.
    if crate::runtime::selection::user_select::has_none_ancestor(dom, fragment.text_node) {
        return;
    }

    let Some((byte_start, byte_end)) = selection_byte_range_in(dom, range, fragment.text_node)
    else {
        return;
    };

    // Intersect with the fragment's source byte window.
    let frag_start = fragment.source_byte_offset;
    let frag_end = fragment.source_byte_offset + fragment.text.len();
    let local_start = byte_start.max(frag_start);
    let local_end = byte_end.min(frag_end);
    if local_start >= local_end {
        return;
    }

    // Byte offsets within the fragment's own text.
    let off_start = local_start - frag_start;
    let off_end = local_end - frag_start;

    // Map byte offsets → visible cell offsets inside the fragment.
    let cell_start = cells_before_byte(&fragment.text, off_start);
    let cell_end = cells_before_byte(&fragment.text, off_end);
    if cell_start >= cell_end {
        return;
    }

    // Author `::selection` cascade overrides the UA default.
    // The UA `*::selection { bg: #394B7E; fg: white }` rule means
    // every focusable always has a computed_selection style — so
    // this fallback only fires if an author *explicitly* removes
    // the UA rule via `*::selection { background-color: initial; }`
    // or similar. In that case we paint nothing (Style::new()).
    let overlay = match nearest_selection_style(dom, fragment.text_node) {
        Some(c) => style_from_computed(c),
        None => Style::new(),
    };
    for c in cell_start..cell_end {
        let x = (frag_x + c as i32) as u16;
        if x < clip.x || x >= clip.right() {
            continue;
        }
        buf.set_style(x, line_y, overlay);
    }
}

/// Walk up from `text_node` to the nearest ancestor element whose
/// cascade produced a `::selection` computed style. Returns `None`
/// if no ancestor has one.
fn nearest_selection_style(dom: &Dom<TuiExt>, text_node: NodeId) -> Option<&ComputedStyle> {
    let mut cur = dom.node(text_node).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if let Some(ext) = dom.node(id).ext()
            && let Some(sel) = ext.computed_selection.as_ref()
        {
            return Some(sel);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// Byte range within `text_node`'s data that falls inside the
/// document-ordered selection `range`. `None` when `text_node` sits
/// fully outside the range. Handles:
///
/// - range entirely within one text node (start == end == text_node)
/// - range starts in `text_node` (end is elsewhere in the tree)
/// - range ends in `text_node` (start is elsewhere)
/// - `text_node` is strictly between start and end in document order —
///   entire text node is selected
fn selection_byte_range_in(
    dom: &Dom<TuiExt>,
    range: &Range,
    text_node: NodeId,
) -> Option<(usize, usize)> {
    let is_start = range.start.node == text_node;
    let is_end = range.end.node == text_node;

    if is_start && is_end {
        return Some((range.start.offset, range.end.offset));
    }
    let text_len = dom
        .node(text_node)
        .node_value()
        .map(|s| s.len())
        .unwrap_or(0);
    if is_start {
        return Some((range.start.offset, text_len));
    }
    if is_end {
        return Some((0, range.end.offset));
    }

    // Whole-node membership is a boundary-point comparison (DOM §5.2):
    // the text is selected when `start <= (text, 0)` and
    // `(text, len) <= end`. This orders an element position `(el, k)`
    // by its offset against the child index, so a range that ends at
    // `(parent, 0)` does not swallow the text under `parent`'s children.
    use std::cmp::Ordering;
    let after_start = matches!(
        dom.compare_boundary_points(range.start, Position::new(text_node, 0)),
        Some(Ordering::Less | Ordering::Equal)
    );
    let before_end = matches!(
        dom.compare_boundary_points(Position::new(text_node, text_len), range.end),
        Some(Ordering::Less | Ordering::Equal)
    );
    if after_start && before_end {
        Some((0, text_len))
    } else {
        None
    }
}

/// Count visible cells before byte offset `target` in `text`. If
/// `target` falls between graphemes, returns the cells up to that
/// boundary. Mid-grapheme targets round up to the next boundary
/// (shouldn't happen — selection offsets land on grapheme edges).
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
