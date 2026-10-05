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
    let (dx, dy) = relative_offset(computed, parent, parent_height_definite);
    LayoutRect::new(
        rect.x.saturating_add(dx),
        rect.y.saturating_add(dy),
        rect.width,
        rect.height,
    )
}

/// The `(dx, dy)` a relatively positioned box with `computed`'s insets
/// moves by, inside a containing block of `cb`'s size (CSS 2.1 §9.4.3).
/// The box only moves — its size is its own. Percentages resolve
/// against the containing block on the matching axis (`top` / `bottom`
/// → height, `left` / `right` → width); a percentage `top` / `bottom`
/// computes to `auto` unless `height_definite` (§9.3.2). With both
/// `left` and `right` set the inline-start one wins — `left` under
/// `ltr`, `right` under `rtl` (rdom reads the box's own `direction`,
/// which it inherits from its containing block unless it sets one —
/// DIVERGENCES) — and with both `top` and `bottom`, `top`. Shared by
/// relative elements and relative pseudo-elements.
pub(in crate::render::layout_pass) fn relative_offset(
    computed: &ComputedStyle,
    cb: LayoutRect,
    height_definite: bool,
) -> (i32, i32) {
    let left = || resolve_length_offset(&computed.left, cb.width as i32, false);
    let right = || resolve_length_offset(&computed.right, cb.width as i32, true);
    let dx = if computed.text_direction == crate::layout::TextDirection::Rtl {
        right().or_else(left).unwrap_or(0)
    } else {
        left().or_else(right).unwrap_or(0)
    };
    let vertical = |len: &Length| -> Length {
        match len {
            Length::Calc(expr) if !height_definite && expr.contains_percent() => Length::Auto,
            other => other.clone(),
        }
    };
    let dy = resolve_length_offset(&vertical(&computed.top), cb.height as i32, false)
        .unwrap_or_else(|| {
            resolve_length_offset(&vertical(&computed.bottom), cb.height as i32, true).unwrap_or(0)
        });
    (dx, dy)
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
