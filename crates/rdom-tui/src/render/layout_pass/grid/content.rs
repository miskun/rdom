//! Aligning the grid (CSS Grid 2 §10.5): once the tracks are sized,
//! `justify-content` / `align-content` distribute the content box's free
//! space on each axis around them — each track that is not collapsed an
//! alignment subject (Box Alignment 3 §5.1) — through the content
//! distribution flex uses too (`layout_pass::distribution`). The space
//! between two tracks widens the gutter, and with it any grid area that
//! spans it.

use super::track::TrackGrid;
use crate::layout::Alignment;
use crate::render::layout_pass::distribution::{Distribution, Ends, content_distribution, offsets};

/// The ends of the inline axis in the grid's frame (its inline-start
/// edge the origin): `left` / `right` are physical, so they swap under
/// `rtl`; `flex-start` / `flex-end` are `start` / `end` (§5.3).
pub(super) fn inline_ends(rtl: bool) -> Ends {
    let (left, right) = if rtl {
        (Distribution::End, Distribution::Start)
    } else {
        (Distribution::Start, Distribution::End)
    };
    Ends {
        left,
        right,
        ..BLOCK_ENDS
    }
}

/// The ends of the block axis (top to bottom); `left` / `right` are not
/// in `align-content`'s grammar.
pub(super) const BLOCK_ENDS: Ends = Ends {
    start: Distribution::Start,
    end: Distribution::End,
    flex_start: Distribution::Start,
    flex_end: Distribution::End,
    left: Distribution::Start,
    right: Distribution::Start,
};

/// Each of `grid`'s tracks' start and end offsets from the content box's
/// start edge, `size` cells long, once `value` has distributed the free
/// space (`size` less the tracks and gutters; negative when they
/// overflow it): the extra space before each subject shifts it and every
/// track after it, a collapsed track keeping its predecessor's shift.
pub(super) fn distribute(
    grid: &TrackGrid,
    size: u16,
    value: Alignment,
    ends: Ends,
) -> Vec<(i32, i32)> {
    let extents = grid.extents();
    let total = extents.last().map_or(0, |e| e.1);
    let free = i32::from(size) - i32::try_from(total).unwrap_or(i32::MAX);
    let subjects = grid.collapsed.iter().filter(|c| !**c).count();
    let mut extra = offsets(
        content_distribution(value, ends, free, subjects),
        free,
        subjects,
    )
    .into_iter();
    let mut shift = 0;
    extents
        .iter()
        .zip(&grid.collapsed)
        .map(|(&(a, b), &collapsed)| {
            if !collapsed {
                shift += extra.next().unwrap_or(0);
            }
            let at = |x: u32| i32::try_from(x).unwrap_or(i32::MAX).saturating_add(shift);
            (at(a), at(b))
        })
        .collect()
}
