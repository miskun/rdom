//! Content distribution in flex layout — CSS Box Alignment 3 §4 / §5:
//! `justify-content` (CSS Flexbox §8.2) places a line's leftover free
//! space, once the flexible lengths and `auto` margins are resolved,
//! before its items or between them; `align-content` (§8.4, §9.4 step
//! 15) does the same with a multi-line container's free cross space and
//! its lines. The keyword mapping and the whole-cell shares are the
//! shared `layout_pass::distribution`'s; this module names the axis's
//! ends in the flex frame.

use crate::layout::{Align, Direction};
use crate::render::layout_pass::distribution::{Distribution, Ends, content_distribution, offsets};
use crate::style::ComputedStyle;

/// The extra main-axis space before each of a line's `n` items: the
/// first entry is the space before the first item (from main-start),
/// each later one is added to the gap before its item. `free` is the
/// line's leftover free space (negative when the items overflow it),
/// `auto_margins` whether any of its items has an `auto` main-axis
/// margin — which took any positive free space already (§8.1).
pub(super) fn justify_offsets(
    container: &ComputedStyle,
    direction: Direction,
    flipped: bool,
    free: i32,
    n: usize,
    auto_margins: bool,
) -> Vec<i32> {
    if n == 0 || (auto_margins && free > 0) {
        return vec![0; n];
    }
    // `start` / `end` are the writing mode's ends of the axis: main-start
    // unless `row-reverse` / `column-reverse` swapped it (CSS Flexbox
    // §5.1); `left` / `right` the physical ones, on the inline axis only
    // (Box Alignment §4.2 — on a column's block axis they are `start`).
    let (start, end) = if container.flex_reverse {
        (Distribution::End, Distribution::Start)
    } else {
        (Distribution::Start, Distribution::End)
    };
    let (left, right) = match direction {
        Direction::Column => (start, start),
        Direction::Row if flipped => (Distribution::End, Distribution::Start),
        Direction::Row => (Distribution::Start, Distribution::End),
    };
    let ends = Ends {
        start,
        end,
        // `normal` behaves as `stretch`, and `stretch` as `flex-start`
        // in flex layout (Box Alignment §5.3, §6.1).
        flex_start: Distribution::Start,
        flex_end: Distribution::End,
        left,
        right,
    };
    offsets(
        content_distribution(container.justify_content, ends, free, n),
        free,
        n,
    )
}

/// `align-content`'s extra cross space before each of a multi-line
/// container's `n` lines (the first from cross-start, each later one
/// added to the gap before its line), or `None` for `normal` /
/// `stretch`, which grow the lines instead (`lines::stretch_lines`).
/// `free` is the container's cross size less the lines and the gaps;
/// `flipped` whether the cross axis runs from its physical end.
pub(super) fn align_content_offsets(
    container: &ComputedStyle,
    direction: Direction,
    flipped: bool,
    free: i32,
    n: usize,
) -> Option<Vec<i32>> {
    let value = container.align_content;
    if n == 0 || matches!(value.keyword, Align::Normal | Align::Stretch) {
        return None;
    }
    // `start` / `end` by the writing mode (§4.2): a row's cross axis is
    // its block axis (top first), a column's its inline axis (right
    // first under `rtl`); in the frame, cross-start is the physical end
    // when the axis is flipped (`wrap-reverse`, a column under `rtl`).
    let rtl = crate::render::layout_pass::margin_trim::inline_reversed(container);
    let physical_start = match direction {
        Direction::Row => true,
        Direction::Column => !rtl,
    };
    let (start, end) = if physical_start != flipped {
        (Distribution::Start, Distribution::End)
    } else {
        (Distribution::End, Distribution::Start)
    };
    let ends = Ends {
        start,
        end,
        flex_start: Distribution::Start,
        flex_end: Distribution::End,
        // Not in `align-content`'s grammar.
        left: Distribution::Start,
        right: Distribution::Start,
    };
    Some(offsets(content_distribution(value, ends, free, n), free, n))
}
