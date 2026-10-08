//! The axis a box lays its in-flow children out along and the gap
//! between them (CSS Flexbox §5.1, CSS Box Alignment 3 §8.1) — the
//! helpers every formatting context's layout reads.

use crate::layout::{Direction, LayoutRect};
use crate::style::ComputedStyle;

/// The axis `computed`'s in-flow children are laid out along: a flex
/// container's main axis (`flex-direction`, CSS Flexbox §5.1), every
/// other box's block axis — vertical, as every box lays out
/// `horizontal-tb` (DIVERGENCES §1). `flex-direction` applies to flex
/// containers only, so it never turns a block container sideways.
pub(in crate::render) fn flow_axis(computed: &ComputedStyle) -> Direction {
    match computed.flow {
        crate::layout::Flow::Flex => computed.direction,
        // A grid container's items lay out on both axes; its block axis
        // stands for it (it takes no `border-collapse` insets, and its
        // content size is the grid's, `grid::content_size`).
        crate::layout::Flow::Block
        | crate::layout::Flow::FlowRoot
        | crate::layout::Flow::Grid
        | crate::layout::Flow::Table => Direction::Column,
    }
}

/// Resolve the gap between `computed`'s children laid out along `axis`
/// (CSS Box Alignment 3 §8.1): `column-gap` between items placed
/// horizontally, `row-gap` between items stacked vertically; `normal`
/// is 0. Percentages resolve against the container's content size on
/// that axis, and against 0 when that size is indefinite — which for
/// rdom means an `auto`-height container's block axis.
pub(in crate::render) fn resolve_gap(
    computed: &crate::style::ComputedStyle,
    container: LayoutRect,
    axis: Direction,
) -> u16 {
    let basis = match axis {
        Direction::Row => container.width,
        // An `auto` or keyword height (its content height, CSS Sizing 3
        // §3.1) is indefinite.
        Direction::Column
            if matches!(
                computed.height,
                crate::layout::Size::Auto | crate::layout::Size::Intrinsic(_)
            ) =>
        {
            0
        }
        Direction::Column => container.height,
    };
    gap_along(computed, axis).resolve(basis)
}

/// The gap property between children laid out along `axis`:
/// `column-gap` along the horizontal axis, `row-gap` along the vertical.
pub(in crate::render) fn gap_along(
    computed: &crate::style::ComputedStyle,
    axis: Direction,
) -> &crate::layout::GapValue {
    match axis {
        Direction::Row => &computed.column_gap,
        Direction::Column => &computed.row_gap,
    }
}
