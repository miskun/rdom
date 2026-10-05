//! `margin-trim` (CSS Box 4 §3) for the layout passes that drop the
//! trimmed margins, and the horizontal axis's direction they share.
//!
//! The logical sides map as in `horizontal-tb`: block-start / -end are
//! the top / bottom edges; inline-start / -end the left / right ones
//! under `direction: ltr` and the right / left ones under `rtl` (CSS
//! Writing Modes 4 §2.1). A block container trims on the block axis
//! only; a flex container on both (§3.2).

use crate::layout::{Direction, Flow, Sides, TextDirection};
use crate::style::ComputedStyle;

/// Whether `container`'s inline axis runs right to left (`direction:
/// rtl`): its inline-start edge is the right one.
pub(crate) fn inline_reversed(container: &ComputedStyle) -> bool {
    container.text_direction == TextDirection::Rtl
}

/// The physical edges of `container` whose adjoining children's margins
/// are trimmed (`top` = block-start; `left` / `right` = inline-start /
/// -end as the direction maps them).
pub(crate) fn trimmed_edges(container: &ComputedStyle) -> Sides<bool> {
    let t = container.margin_trim;
    let inline_axis = container.flow == Flow::Flex;
    let (start, end) = (inline_axis && t.inline_start, inline_axis && t.inline_end);
    let (left, right) = if inline_reversed(container) {
        (end, start)
    } else {
        (start, end)
    };
    Sides::new(t.block_start, right, t.block_end, left)
}

/// A flex container's trimmed edges in its flex axes (§3.2), logical:
/// the main-axis start / end margins of the first / last item, and the
/// cross-axis margins of every item (rdom's flex lines are single). A
/// `row`'s main axis is the inline axis, a `column`'s cross axis.
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
        let t = container.margin_trim;
        match direction {
            Direction::Row => Self {
                main_start: t.inline_start,
                main_end: t.inline_end,
                cross_start: t.block_start,
                cross_end: t.block_end,
            },
            Direction::Column => Self {
                main_start: t.block_start,
                main_end: t.block_end,
                cross_start: t.inline_start,
                cross_end: t.inline_end,
            },
        }
    }
}
