//! §11.7 Expand Flexible Tracks and §11.7.1 Find the Size of an `fr`:
//! the flex fraction — from the leftover space when it is definite, from
//! the tracks' base sizes and the items' max-content contributions when
//! it is not — and each flexible track grown to its share of it, the
//! shares rolled into whole cells.

use super::{Contributions, Frame, Space};
use crate::render::layout_pass::grid::track::{Span, TrackGrid};
use crate::render::layout_pass::shares::{Rolling, sums_below_one};

/// §11.7 over `grid`, whose tracks `items` span.
pub(super) fn expand(
    grid: &mut TrackGrid,
    items: &[Span],
    c: &mut dyn Contributions,
    frame: Frame,
) {
    if grid.tracks.iter().all(|t| t.flex().is_none()) {
        return;
    }
    let fraction = match frame.space {
        // "If the free space is zero or if sizing the grid container
        // under a min-content constraint: the used flex fraction is zero."
        Space::MinContent => return,
        Space::Definite(available) => {
            if available <= grid.total() {
                return;
            }
            fr_size(grid, 0..grid.tracks.len(), available)
        }
        Space::MaxContent => {
            let fraction = indefinite_fraction(grid, items, c);
            // "If using this flex fraction would cause the grid to be
            // smaller than the grid container's min-width/height (or
            // larger than its max-width/height), then redo this step,
            // treating the free space as definite."
            let total = grown_total(grid, fraction);
            match (frame.min, frame.max) {
                (Some(min), _) if total < min => fr_size(grid, 0..grid.tracks.len(), min),
                (_, Some(max)) if total > max => fr_size(grid, 0..grid.tracks.len(), max),
                _ => fraction,
            }
        }
    };
    grow(grid, fraction);
}

/// The flex fraction for an indefinite free space: the largest of each
/// flexible track's base size per fr (its base size when its factor is
/// at most one) and, for each item crossing a flexible track, the size
/// of an fr its spanned tracks need to hold its max-content contribution.
fn indefinite_fraction(grid: &TrackGrid, items: &[Span], c: &mut dyn Contributions) -> f64 {
    let mut fraction: f64 = 0.0;
    for t in &grid.tracks {
        if let Some(f) = t.flex() {
            let f = f64::from(f);
            let base = f64::from(t.base);
            fraction = fraction.max(if f > 1.0 { base / f } else { base });
        }
    }
    for (i, &span) in items.iter().enumerate() {
        if grid.tracks[span.tracks()]
            .iter()
            .any(|t| t.flex().is_some())
        {
            fraction = fraction.max(fr_size(grid, span.tracks(), c.max_content(i)));
        }
    }
    fraction
}

/// §11.7.1 Find the Size of an `fr` among the tracks `range`, filling
/// `space`: the leftover space after the inflexible tracks (and the
/// gutters between the tracks) over the flex factors' sum (at least
/// one), restarted with every flexible track whose share would fall
/// below its base size treated as inflexible. Never negative.
fn fr_size(grid: &TrackGrid, range: std::ops::Range<usize>, space: u32) -> f64 {
    let tracks = &grid.tracks[range.clone()];
    let gutters: u32 = if range.is_empty() {
        0
    } else {
        grid.inner_gutters(Span::new(range.start, range.end))
    };
    let mut inflexible: Vec<bool> = tracks.iter().map(|t| t.flex().is_none()).collect();
    loop {
        let fixed: u32 = tracks
            .iter()
            .zip(&inflexible)
            .filter(|(_, f)| **f)
            .map(|(t, _)| t.base)
            .sum();
        let leftover = f64::from(space) - f64::from(fixed) - f64::from(gutters);
        let sum: f64 = tracks
            .iter()
            .zip(&inflexible)
            .filter(|(_, f)| !**f)
            .map(|(t, _)| f64::from(t.flex().unwrap_or(0.0)))
            .sum();
        let fraction = leftover / if sums_below_one(sum) { 1.0 } else { sum };
        let mut restart = false;
        for (t, f) in tracks.iter().zip(inflexible.iter_mut()) {
            if !*f && fraction * f64::from(t.flex().unwrap_or(0.0)) < f64::from(t.base) {
                *f = true;
                restart = true;
            }
        }
        if !restart {
            return fraction.max(0.0);
        }
    }
}

/// The flexible tracks whose share of `fraction` exceeds their base
/// size, with that share.
fn growing(grid: &TrackGrid, fraction: f64) -> impl Iterator<Item = (usize, f64)> + '_ {
    grid.tracks.iter().enumerate().filter_map(move |(i, t)| {
        let share = fraction * f64::from(t.flex()?);
        (share > f64::from(t.base)).then_some((i, share))
    })
}

/// The grid's total were the flexible tracks grown to `fraction`.
fn grown_total(grid: &TrackGrid, fraction: f64) -> u32 {
    let extra: f64 = growing(grid, fraction)
        .map(|(i, share)| share - f64::from(grid.tracks[i].base))
        .sum();
    grid.total() + crate::render::layout_pass::shares::floor_cells(extra)
}

/// Set each flexible track whose share of `fraction` exceeds its base
/// size to that share, rolled in whole cells across those tracks so
/// their sizes sum to the floor of their shares' sum.
fn grow(grid: &mut TrackGrid, fraction: f64) {
    let shares: Vec<(usize, f64)> = growing(grid, fraction).collect();
    let total: f64 = shares.iter().map(|(_, s)| s).sum();
    let mut rolling = Rolling::new(total, total);
    for (i, share) in shares {
        let t = &mut grid.tracks[i];
        t.base = rolling.share(share).max(t.base);
    }
}
