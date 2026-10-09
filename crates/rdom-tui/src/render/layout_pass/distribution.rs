//! Content distribution — CSS Box Alignment 3 §5: how a container's free
//! space on one axis is placed around its alignment subjects (a flex
//! line's items, a flex container's lines, a grid's tracks) by
//! `justify-content` / `align-content`. Shared by flex (`flex::content`)
//! and grid (`grid::arrange`); each maps the axis's ends into the frame
//! it lays its subjects out in ([`Ends`]).
//!
//! Whole cells, deterministically (DIVERGENCES §1): `center` gives the
//! leading space the free space halved and rounded down (toward the
//! start, also when it is negative); the distributions place each subject
//! at a rolling position rounded up, so a remainder cell goes to each of
//! the first spaces.

use crate::layout::{Align, Alignment, OverflowAlign};

/// Where the subjects go, in the frame whose origin is the axis's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) enum Distribution {
    Start,
    End,
    Center,
    Between,
    Around,
    Evenly,
}

/// The frame placements the positional keywords name on one axis.
#[derive(Debug, Clone, Copy)]
pub(in crate::render::layout_pass) struct Ends {
    /// `start` / `end`: the writing mode's ends of the axis (§4.2) —
    /// also the `safe` fallback and the distributions' overflow fallback.
    pub(in crate::render::layout_pass) start: Distribution,
    pub(in crate::render::layout_pass) end: Distribution,
    /// `flex-start` / `flex-end` (and `normal` / `stretch`, which pack to
    /// `flex-start` once nothing stretches, §5.3): the flex container's
    /// ends, `start` / `end` outside flex.
    pub(in crate::render::layout_pass) flex_start: Distribution,
    pub(in crate::render::layout_pass) flex_end: Distribution,
    /// `left` / `right`: the physical ends of an inline axis (on a block
    /// axis they are `start`, §4.2).
    pub(in crate::render::layout_pass) left: Distribution,
    pub(in crate::render::layout_pass) right: Distribution,
}

/// The placement `value` makes of `free` cells around `n` subjects
/// (§5.1, §5.2, §4.4): a `safe` value that would overflow is `start`;
/// `space-between` falls back to `flex-start` for one subject and to
/// `start` on overflow, `space-around` / `space-evenly` to `center` and
/// `start` (Box Alignment §5.3, CSS Flexbox §8.2 / §8.4); the baseline
/// values are `start` / `end` (§9.3).
pub(in crate::render::layout_pass) fn content_distribution(
    value: Alignment,
    ends: Ends,
    free: i32,
    n: usize,
) -> Distribution {
    let overflows = free < 0;
    if overflows && value.overflow == OverflowAlign::Safe {
        return ends.start;
    }
    match value.keyword {
        Align::Normal | Align::Stretch | Align::FlexStart => ends.flex_start,
        Align::FlexEnd => ends.flex_end,
        Align::Start | Align::Baseline => ends.start,
        Align::End | Align::LastBaseline => ends.end,
        Align::Left => ends.left,
        Align::Right => ends.right,
        Align::Center => Distribution::Center,
        Align::SpaceBetween if overflows => ends.start,
        Align::SpaceBetween if n == 1 => ends.flex_start,
        Align::SpaceBetween => Distribution::Between,
        Align::SpaceAround | Align::SpaceEvenly if overflows => ends.start,
        Align::SpaceAround | Align::SpaceEvenly if n == 1 => Distribution::Center,
        Align::SpaceAround => Distribution::Around,
        Align::SpaceEvenly => Distribution::Evenly,
        // Not in the content-distribution grammars.
        Align::Auto | Align::SelfStart | Align::SelfEnd | Align::AnchorCenter => ends.flex_start,
        // `Align` is non-exhaustive (DESIGN): a keyword added to it must be
        // mapped here — the workspace's tests catch one that is not.
        _ => {
            debug_assert!(false, "unmapped `Align` keyword {:?}", value.keyword);
            ends.flex_start
        }
    }
}

/// The extra space before each of `n` subjects that `placement` makes of
/// `free`: the first entry from the axis's start, each later one added to
/// the gap before its subject.
pub(in crate::render::layout_pass) fn offsets(
    placement: Distribution,
    free: i32,
    n: usize,
) -> Vec<i32> {
    let mut extra = vec![0; n];
    if n == 0 {
        return extra;
    }
    let at = |numerator: i64, denominator: i64| -> i32 {
        // Rounded up: a remainder cell lands in the earliest spaces.
        -(-(i64::from(free) * numerator)).div_euclid(denominator) as i32
    };
    let positions: Vec<i32> = match placement {
        Distribution::Start => return extra,
        Distribution::End => {
            extra[0] = free;
            return extra;
        }
        Distribution::Center => {
            extra[0] = free.div_euclid(2);
            return extra;
        }
        Distribution::Between => (0..n as i64).map(|i| at(i, n as i64 - 1)).collect(),
        Distribution::Around => (0..n as i64).map(|i| at(2 * i + 1, 2 * n as i64)).collect(),
        Distribution::Evenly => (0..n as i64).map(|i| at(i + 1, n as i64 + 1)).collect(),
    };
    let mut previous = 0;
    for (e, p) in extra.iter_mut().zip(positions) {
        *e = p - previous;
        previous = p;
    }
    extra
}
