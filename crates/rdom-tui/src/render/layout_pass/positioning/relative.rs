//! Relative positioning (CSS 2.1 §9.4.3): the shift of a
//! `position: relative` (or sticky) box from its in-flow place.

use crate::layout::{LayoutRect, Length, Position};
use crate::style::ComputedStyle;

/// `position: relative` shifts the element's painted rect by
/// `top` / `left` (or `right` / `bottom` if the corresponding
/// edge is `Auto` and the opposite edge is `Cells`). Returns the
/// shifted rect.
///
/// Applied inside `layout_node` before writing the rect, so the
/// element's own children flow inside the shifted rect. Siblings
/// are unaffected — the parent's flex / IFC distribution had
/// already finalized their positions before this element's
/// `layout_node` runs.
///
/// Per CSS, when both edges of an axis are specified, `top` /
/// `left` win and `bottom` / `right` are ignored. A percentage
/// `top` / `bottom` computes to `auto` unless `parent_height_definite`
/// (CSS 2.1 §9.3.2: the containing block's height must be specified
/// explicitly).
pub(in crate::render::layout_pass) fn apply_relative_shift(
    computed: &ComputedStyle,
    rect: LayoutRect,
    parent: LayoutRect,
    parent_height_definite: bool,
) -> LayoutRect {
    if computed.position != Position::Relative {
        return rect;
    }
    // Relative offsets resolve percentages against the parent's
    // content box on the matching axis (`top`/`bottom` → height,
    // `left`/`right` → width). Per CSS 2.1 §9.4.3.
    let dx =
        resolve_length_offset(&computed.left, parent.width as i32, false).unwrap_or_else(|| {
            resolve_length_offset(&computed.right, parent.width as i32, true).unwrap_or(0)
        });
    let vertical = |len: &Length| -> Length {
        match len {
            Length::Calc(expr) if !parent_height_definite && expr.contains_percent() => {
                Length::Auto
            }
            other => other.clone(),
        }
    };
    let dy = resolve_length_offset(&vertical(&computed.top), parent.height as i32, false)
        .unwrap_or_else(|| {
            resolve_length_offset(&vertical(&computed.bottom), parent.height as i32, true)
                .unwrap_or(0)
        });
    LayoutRect::new(
        rect.x.saturating_add(dx),
        rect.y.saturating_add(dy),
        rect.width,
        rect.height,
    )
}

/// Resolve a `Length` value to a signed integer offset given the
/// axis basis. `negate` flips the sign (used for the `bottom`/
/// `right` insets which point inward from the opposite edge).
/// Returns `None` for `Length::Auto`.
fn resolve_length_offset(len: &Length, basis: i32, negate: bool) -> Option<i32> {
    let cells = len.cells(basis)?;
    Some(if negate {
        cells.saturating_neg()
    } else {
        cells
    })
}
