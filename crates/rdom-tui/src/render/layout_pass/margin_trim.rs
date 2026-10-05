//! `margin-trim` (CSS Box 4 §3) mapped onto a container's physical
//! content edges, for the layout passes that drop the trimmed margins.
//!
//! The logical sides map as in `horizontal-tb`: block-start / -end are
//! the top / bottom edges, inline-start / -end the left / right ones.
//! A block container trims on the block axis only; a flex container on
//! both (§3.2).

use crate::layout::{Direction, Flow, Sides};
use crate::style::ComputedStyle;

/// The edges of `container` whose adjoining children's margins are
/// trimmed (`top` = block-start, …).
pub(crate) fn trimmed_edges(container: &ComputedStyle) -> Sides<bool> {
    let t = container.margin_trim;
    let inline_axis = container.flow == Flow::Flex;
    Sides::new(
        t.block_start,
        inline_axis && t.inline_end,
        t.block_end,
        inline_axis && t.inline_start,
    )
}

/// A flex container's trimmed edges in its flex axes (§3.2): the
/// main-axis start / end margins of the first / last item, and the
/// cross-axis margins of every item (rdom's flex lines are single).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FlexTrim {
    pub(crate) main_start: bool,
    pub(crate) main_end: bool,
    pub(crate) cross_start: bool,
    pub(crate) cross_end: bool,
}

impl FlexTrim {
    /// `container`'s trim, laid out along `direction`.
    pub(crate) fn of(container: &ComputedStyle, direction: Direction) -> Self {
        let e = trimmed_edges(container);
        match direction {
            Direction::Row => Self {
                main_start: e.left,
                main_end: e.right,
                cross_start: e.top,
                cross_end: e.bottom,
            },
            Direction::Column => Self {
                main_start: e.top,
                main_end: e.bottom,
                cross_start: e.left,
                cross_end: e.right,
            },
        }
    }
}
