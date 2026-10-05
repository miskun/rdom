//! `justify-content` in flex layout — CSS Flexbox §8.2 with CSS Box
//! Alignment 3 §4 / §5.2: a line's leftover free space, once the
//! flexible lengths and `auto` margins are resolved, placed before the
//! items or spread between them.
//!
//! Whole cells, deterministically: `center` gives the leading space the
//! free space halved and rounded down (toward main-start, also when it is
//! negative); the distributions place each item at a rolling position
//! rounded up, so a remainder cell goes to each of the first spaces.

use crate::layout::{Align, Alignment, Direction, OverflowAlign};
use crate::style::ComputedStyle;

/// Where the line's items go, in the frame whose origin is main-start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement {
    Start,
    End,
    Center,
    Between,
    Around,
    Evenly,
}

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
    let mut extra = vec![0; n];
    if n == 0 || (auto_margins && free > 0) {
        return extra;
    }
    let placement = placement(
        container.justify_content,
        container,
        direction,
        flipped,
        free,
        n,
    );
    let at = |numerator: i64, denominator: i64| -> i32 {
        // Rounded up: a remainder cell lands in the earliest spaces.
        -(-(i64::from(free) * numerator)).div_euclid(denominator) as i32
    };
    let positions: Vec<i32> = match placement {
        Placement::Start => return extra,
        Placement::End => {
            extra[0] = free;
            return extra;
        }
        Placement::Center => {
            extra[0] = free.div_euclid(2);
            return extra;
        }
        Placement::Between => (0..n as i64).map(|i| at(i, n as i64 - 1)).collect(),
        Placement::Around => (0..n as i64).map(|i| at(2 * i + 1, 2 * n as i64)).collect(),
        Placement::Evenly => (0..n as i64).map(|i| at(i + 1, n as i64 + 1)).collect(),
    };
    let mut previous = 0;
    for (e, p) in extra.iter_mut().zip(positions) {
        *e = p - previous;
        previous = p;
    }
    extra
}

/// The placement `justify-content: value` makes of the line's free space
/// in the main-start frame (`flipped`: the main axis runs from its
/// physical end).
fn placement(
    value: Alignment,
    container: &ComputedStyle,
    direction: Direction,
    flipped: bool,
    free: i32,
    n: usize,
) -> Placement {
    // `start` / `end` are the writing mode's ends of the axis: main-start
    // unless `row-reverse` / `column-reverse` swapped it (CSS Flexbox
    // §5.1); `left` / `right` the physical ones, on the inline axis only
    // (Box Alignment §4.2 — on a column's block axis they are `start`).
    let start = if container.flex_reverse {
        Placement::End
    } else {
        Placement::Start
    };
    let end = if container.flex_reverse {
        Placement::Start
    } else {
        Placement::End
    };
    let overflows = free < 0;
    // §4.4: a `safe` alignment that would overflow aligns as `start`.
    if overflows && value.overflow == OverflowAlign::Safe {
        return start;
    }
    match value.keyword {
        // `normal` behaves as `stretch`, and `stretch` as `flex-start`
        // in flex layout (Box Alignment §5.3, §6.1).
        Align::Normal | Align::Stretch | Align::FlexStart => Placement::Start,
        Align::FlexEnd => Placement::End,
        Align::Start => start,
        Align::End => end,
        Align::Left | Align::Right if direction == Direction::Column => start,
        Align::Left if flipped => Placement::End,
        Align::Left => Placement::Start,
        Align::Right if flipped => Placement::Start,
        Align::Right => Placement::End,
        Align::Center => Placement::Center,
        // Flexbox §8.2: negative free space or a single item falls back
        // to `safe flex-start` (`space-between`) or `safe center`.
        Align::SpaceBetween if overflows => start,
        Align::SpaceBetween if n == 1 => Placement::Start,
        Align::SpaceBetween => Placement::Between,
        Align::SpaceAround | Align::SpaceEvenly if overflows => start,
        Align::SpaceAround | Align::SpaceEvenly if n == 1 => Placement::Center,
        Align::SpaceAround => Placement::Around,
        Align::SpaceEvenly => Placement::Evenly,
        // Not in `justify-content`'s grammar.
        Align::Auto | Align::SelfStart | Align::SelfEnd | Align::Baseline | Align::LastBaseline => {
            Placement::Start
        }
    }
}
