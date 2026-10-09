//! Size containment (CSS Containment 2 §3.1, CSS Containment 3's
//! inline-size containment): a box's intrinsic size on a contained axis
//! is computed as if it had no content — its `contain-intrinsic-size`
//! (CSS Sizing 4 §6.1) where given, else nothing — so laying it out never
//! depends on what it holds. Which axes are contained is
//! `style::containment`'s: `contain: size | inline-size`, and a query
//! container on the axes it answers size queries on (`container-type`,
//! CSS Conditional 5 §6.1).

use crate::layout::Direction;
use crate::style::ComputedStyle;

/// Whether `computed` has size containment on `direction`'s axis (Row:
/// the inline axis — horizontal-tb, the width; Column: the block axis).
pub(crate) fn contains(computed: &ComputedStyle, direction: Direction) -> bool {
    match direction {
        Direction::Row => crate::style::containment::size_inline(computed),
        Direction::Column => crate::style::containment::size_block(computed),
    }
}

/// The content-box size a size-contained box has on `direction`'s axis
/// in place of its content's: `contain-intrinsic-*`'s length, else 0.
pub(crate) fn contained_size(computed: &ComputedStyle, direction: Direction) -> u16 {
    let size = match direction {
        Direction::Row => &computed.contain_intrinsic_width,
        Direction::Column => &computed.contain_intrinsic_height,
    };
    // A computed length: cells, its viewport units absolute.
    size.length.as_ref().map_or(0, |l| {
        let cells =
            rdom_style::calc::to_cells(l.resolve_f64(&rdom_style::calc::ResolveCtx::new(0)));
        cells.clamp(0, i32::from(u16::MAX)) as u16
    })
}
