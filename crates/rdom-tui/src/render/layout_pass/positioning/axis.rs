//! Shared one-axis resolvers for absolute / fixed elements and
//! positioned pseudo-elements: the size between two insets, the
//! anchored offset, the relative shift.

use crate::layout::Length;

/// Resolve size on one axis when both edges are `Cells`, otherwise
/// return `fallback`. Per CSS, an `auto` width on a positioned box
/// only resolves to `cb_extent - start - end` when both edges are
/// specified; one-sided cases fall back to an intrinsic measure
/// (caller passes `0` for elements, content width for pseudos).
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
/// - `(Auto, Auto)` → `cb_start`. Element placement substitutes the
///   recorded static position before reaching this case; pseudo
///   placement and an element without one land at the containing
///   block's start.
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

/// Resolve start position on one axis using `position: relative`
/// shift semantics (the box stays at its natural anchor and only
/// shifts by `start` or `-end`):
///
/// - `(Cells(s), _)` → `anchor + s`.
/// - `(Auto, Cells(e))` → `anchor - e`.
/// - `(Auto, Auto)` → `anchor`.
///
/// Used by positioned-pseudo placement (`positioned_pseudos.rs`)
/// when the pseudo's cascaded `position` is `Relative`. Element
/// `position: relative` uses a separate path
/// ([`apply_relative_shift`]) because element placement reads `top`
/// / `left` / `right` / `bottom` as a *delta* against the in-flow
/// rect, not against a containing block.
pub(in crate::render::layout_pass) fn axis_position_relative_shift(
    start: &Length,
    end: &Length,
    anchor: i32,
    basis: i32,
) -> i32 {
    let s = start.cells(basis);
    let e = end.cells(basis);
    match (s, e) {
        (Some(s), _) => anchor.saturating_add(s),
        (None, Some(e)) => anchor.saturating_sub(e),
        _ => anchor,
    }
}
