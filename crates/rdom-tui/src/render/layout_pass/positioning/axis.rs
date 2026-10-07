//! Shared one-axis resolvers for absolute / fixed boxes: the size
//! between two insets and the anchored offset. The relative shift is
//! `relative::relative_offset`.

use crate::layout::Length;

/// Resolve size on one axis when both edges are `Cells`, otherwise
/// return `fallback`. Per CSS, an `auto` width on a positioned box
/// only resolves to `cb_extent - start - end` when both edges are
/// specified; one-sided cases fall back to `fallback`.
pub(super) fn axis_size_from_edges(
    start: &Length,
    end: &Length,
    cb_extent: u16,
    fallback: u16,
) -> u16 {
    // Resolve both edges into Option<i32>. `Auto` → None, others
    // → Some(cells). When both are Some, derive size from the
    // extent minus both insets.
    let basis = cb_extent as i32;
    let s = start.cells(basis);
    let e = end.cells(basis);
    match (s, e) {
        (Some(s), Some(e)) => {
            let span = s.saturating_add(e);
            (basis.saturating_sub(span)).max(0) as u16
        }
        _ => fallback,
    }
}

/// Resolve start position on one axis using the CSS anchored-offset
/// semantics that govern `position: absolute | fixed`:
///
/// - `(Cells(s), _)` → `cb_start + s` (start edge wins per CSS).
/// - `(Auto, Cells(e))` → `cb_start + cb_extent - e - size` (anchor
///   flips to far edge, going inward).
/// - `(Auto, Auto)` → `cb_start`. Placement substitutes the static
///   position before reaching this case; a box without one lands at
///   the containing block's start.
pub(in crate::render::layout_pass) fn axis_position_anchored(
    start: &Length,
    end: &Length,
    cb_start: i32,
    cb_extent: u16,
    size: u16,
) -> i32 {
    let basis = cb_extent as i32;
    let s = start.cells(basis);
    let e = end.cells(basis);
    match (s, e) {
        (Some(s), _) => cb_start.saturating_add(s),
        (None, Some(e)) => cb_start
            .saturating_add(basis)
            .saturating_sub(e)
            .saturating_sub(size as i32),
        _ => cb_start,
    }
}
