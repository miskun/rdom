//! The track sizing algorithm — CSS Grid 2 §11.3 (Grid 1 numbering),
//! for one axis: initialize the tracks (§11.4, `Track::new`), resolve
//! the intrinsic track sizes (§11.5, [`intrinsic`]), maximize the tracks
//! (§11.6), expand the flexible tracks (§11.7, [`flexible`]) and stretch
//! the `auto` tracks (§11.8).
//!
//! Whole cells throughout (DIVERGENCES §1): a free space shared equally
//! gives its remainder cells to the first tracks that can take them, an
//! `fr` share and a share by flex factor roll (`shares::Rolling`), so
//! the shares always sum to the space.
//!
//! The items are known here only by the tracks they span and their
//! contributions on the axis ([`Contributions`]), which the caller
//! measures — lazily, each at most once per run.

mod flexible;
mod intrinsic;
#[cfg(test)]
mod tests;

use super::track::{MaxFn, Span, TrackGrid};

/// The space an axis's tracks are sized into (§11.1, §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Space {
    /// A definite available grid space: the content box's size.
    Definite(u32),
    /// Sizing the grid container under a min-content constraint (§5.2):
    /// the free space is zero.
    MinContent,
    /// Under a max-content constraint — and an indefinite size, which
    /// rdom sizes as the content's (an `auto` height): the free space is
    /// infinite (§11.6) and indefinite (§11.7).
    MaxContent,
}

impl Space {
    /// Sized under a min- or max-content constraint.
    fn is_constrained(self) -> bool {
        !matches!(self, Space::Definite(_))
    }
}

/// An item's contributions on the axis being sized (§11.5): outer sizes,
/// margins included, in cells.
pub(super) trait Contributions {
    /// The min-content contribution (CSS Sizing 3 §5.2).
    fn min_content(&mut self, item: usize) -> u32;
    /// The max-content contribution.
    fn max_content(&mut self, item: usize) -> u32;
    /// The minimum contribution (§11.5 / §6.6): the outer size its used
    /// minimum size gives, where its preferred size is `auto`.
    fn minimum(&mut self, item: usize) -> u32;
}

/// What the algorithm needs beyond the tracks and the items.
#[derive(Debug, Clone, Copy)]
pub(super) struct Frame {
    pub(super) space: Space,
    /// `justify-content` / `align-content` is `normal` or `stretch`
    /// (§11.8 stretches the `auto` tracks only then).
    pub(super) stretch: bool,
    /// The grid container's definite minimum and maximum content size on
    /// the axis, which an indefinite space falls back to (§11.7, §11.8).
    pub(super) min: Option<u32>,
    pub(super) max: Option<u32>,
}

/// Run the track sizing algorithm (§11.3) over `grid`'s tracks, which
/// `items` span: each track's base size is its final size.
pub(super) fn size_tracks(
    grid: &mut TrackGrid,
    items: &[Span],
    contributions: &mut dyn Contributions,
    frame: Frame,
) {
    intrinsic::resolve(grid, items, contributions, frame.space);
    maximize(grid, frame);
    flexible::expand(grid, items, contributions, frame);
    if frame.stretch {
        stretch_auto(grid, frame);
    }
}

/// §11.6 Maximize Tracks: a positive free space goes to the base sizes,
/// equally, each track freezing at its growth limit; under a max-content
/// constraint the free space is infinite, so every base size becomes its
/// growth limit — unless that makes the grid larger than the container's
/// definite maximum, when the step is redone with that maximum as the
/// available space; under a min-content constraint it is zero.
fn maximize(grid: &mut TrackGrid, frame: Frame) {
    match frame.space {
        Space::MinContent => {}
        Space::MaxContent => {
            let bases: Vec<u32> = grid.tracks.iter().map(|t| t.base).collect();
            for t in &mut grid.tracks {
                t.base = t.limit_or_base();
            }
            if let Some(max) = frame.max
                && grid.total() > max
            {
                for (t, base) in grid.tracks.iter_mut().zip(bases) {
                    t.base = base;
                }
                maximize_into(grid, max);
            }
        }
        Space::Definite(available) => maximize_into(grid, available),
    }
}

/// §11.6 into a definite `available` space.
fn maximize_into(grid: &mut TrackGrid, available: u32) {
    let free = available.saturating_sub(grid.total());
    let headroom: Vec<Option<u32>> = grid
        .tracks
        .iter()
        .map(|t| Some(t.limit_or_base().saturating_sub(t.base)))
        .collect();
    let (increase, _) = fill_equally(free, &headroom);
    for (t, inc) in grid.tracks.iter_mut().zip(increase) {
        t.base += inc;
    }
}

/// §11.8 Expand Stretched `auto` Tracks: a positive, definite free space
/// goes equally to the tracks with an `auto` max track sizing function;
/// an indefinite one is measured against the container's definite
/// minimum size, if any.
fn stretch_auto(grid: &mut TrackGrid, frame: Frame) {
    let available = match frame.space {
        Space::Definite(n) => n,
        Space::MaxContent => match frame.min {
            Some(n) => n,
            None => return,
        },
        Space::MinContent => return,
    };
    let free = available.saturating_sub(grid.total());
    let autos: Vec<usize> = (0..grid.tracks.len())
        .filter(|&i| grid.tracks[i].max == MaxFn::Auto)
        .collect();
    if free == 0 || autos.is_empty() {
        return;
    }
    let (increase, _) = fill_equally(free, &vec![None; autos.len()]);
    for (&i, inc) in autos.iter().zip(increase) {
        grid.tracks[i].base += inc;
    }
}

/// Share `space` equally among tracks whose room to grow is `headroom`
/// (`None`: unlimited), freezing each as it fills (§11.5.1 / §11.6 "and
/// continuing to grow the unfrozen tracks as needed"). Whole cells: an
/// equal share each, the remainder one cell each to the first open
/// tracks. Returns each track's increase and the space left over once
/// every track is frozen.
pub(super) fn fill_equally(space: u32, headroom: &[Option<u32>]) -> (Vec<u32>, u32) {
    let mut increase = vec![0u32; headroom.len()];
    let room = |i: usize, inc: &[u32]| headroom[i].map(|h| h - inc[i]);
    let mut open: Vec<usize> = (0..headroom.len())
        .filter(|&i| headroom[i] != Some(0))
        .collect();
    let mut left = space;
    while left > 0 && !open.is_empty() {
        let k = open.len() as u32;
        let share = left / k;
        let least = open.iter().filter_map(|&i| room(i, &increase)).min();
        match least {
            Some(c) if c <= share => {
                for &i in &open {
                    increase[i] += c;
                }
                left -= c * k;
                open.retain(|&i| room(i, &increase) != Some(0));
            }
            _ => {
                // Every open track has room for one more than the share.
                for &i in &open {
                    increase[i] += share;
                }
                for &i in open.iter().take((left % k) as usize) {
                    increase[i] += 1;
                }
                left = 0;
            }
        }
    }
    (increase, left)
}
