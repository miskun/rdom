//! The explicit grid of one axis (CSS Grid 2 §7.1–§7.3): a
//! `grid-template-*` track list expanded — each `repeat()` written out,
//! an automatic one as many times as fits (§7.2.3.2) — into its tracks
//! and the names of each line, grown to the tracks
//! `grid-template-areas` defines and given the line names its areas
//! imply (§7.3.2).

use std::borrow::Cow;

use super::placement::Lines;
use crate::layout::{
    GridTemplate, GridTemplateAreas, LineNameList, RepeatCount, TrackBreadth, TrackListItem,
    TrackSize,
};

/// The most tracks rdom creates on one axis. CSS Grid 2 §8 lets a UA
/// clamp the grid to a limit of its own (suggesting it hold the lines
/// -10 000 to 10 000); rdom keeps 10 000 tracks an axis, the explicit
/// ones first. A `repeat()` past it is cut short.
pub(super) const MAX_TRACKS: usize = 10_000;

/// The sizes an automatic repetition is counted against (§7.2.3.2), on
/// the axis: the grid container's definite content size, else its
/// definite maximum, else its definite minimum — and the basis its
/// percentages resolve against.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Bounds {
    pub(super) size: Option<u16>,
    pub(super) max: Option<u16>,
    pub(super) min: Option<u16>,
    pub(super) gap: u16,
}

/// One axis's explicit grid.
#[derive(Debug)]
pub(super) struct Explicit<'a> {
    /// The tracks the track list sizes, in order.
    pub(super) sizes: Vec<&'a TrackSize>,
    /// How many tracks the explicit grid has: the sized ones, or more
    /// when `grid-template-areas` defines more — those take their sizes
    /// from `grid-auto-*` (§7.1).
    pub(super) count: usize,
    /// The names of each line, one more than the tracks (`count`).
    pub(super) names: Vec<Vec<Cow<'a, str>>>,
    /// The tracks an `auto-fit` repetition produced, which collapse when
    /// they hold no item (§7.2.3.2).
    pub(super) auto_fit: std::ops::Range<usize>,
    /// A subgridded axis (§9): its tracks are its parent's, so named
    /// areas do not add any.
    pub(super) inherited: bool,
}

impl<'a> Explicit<'a> {
    /// `template` expanded for `bounds`.
    pub(super) fn of(template: &'a GridTemplate, bounds: Bounds) -> Self {
        let mut out = Explicit {
            sizes: Vec::new(),
            count: 0,
            names: vec![Vec::new()],
            auto_fit: 0..0,
            inherited: false,
        };
        let Some(list) = template.tracks() else {
            return out;
        };
        let count = auto_repetitions(template, bounds);
        for (k, item) in list.items.iter().enumerate() {
            out.name_line(&list.line_names[k]);
            match item {
                TrackListItem::Size(size) => out.push(size),
                TrackListItem::Repeat(r) => {
                    let times = match r.count {
                        RepeatCount::Count(n) => n as usize,
                        RepeatCount::AutoFill | RepeatCount::AutoFit => count,
                    };
                    let first = out.sizes.len();
                    for _ in 0..times {
                        if out.sizes.len() + r.sizes.len() > MAX_TRACKS {
                            break;
                        }
                        for (j, size) in r.sizes.iter().enumerate() {
                            out.name_line(&r.line_names[j]);
                            out.push(size);
                        }
                        out.name_line(&r.line_names[r.sizes.len()]);
                    }
                    if r.count == RepeatCount::AutoFit {
                        out.auto_fit = first..out.sizes.len();
                    }
                }
            }
        }
        out.name_line(&list.line_names[list.items.len()]);
        out
    }

    /// The explicit grid grown to the `tracks` `areas` defines on this
    /// axis (§7.1), and each named area's edges named on it (§7.3.2):
    /// "two named `foo-start`, naming the row-start and column-start
    /// lines of the named grid area, and two named `foo-end`" — `rows`
    /// picks the rows' (else the columns') edges.
    pub(super) fn with_areas(mut self, areas: &GridTemplateAreas, rows: bool) -> Self {
        let tracks = if rows {
            areas.row_count()
        } else {
            areas.column_count()
        };
        if !self.inherited {
            self.count = self.count.max(tracks.min(MAX_TRACKS));
        }
        self.names.resize_with(self.count + 1, Vec::new);
        for area in areas.areas() {
            let span = if rows { area.rows } else { area.columns };
            if span.end > self.count {
                continue;
            }
            self.names[span.start].push(Cow::Owned(format!("{}-start", area.name)));
            self.names[span.end].push(Cow::Owned(format!("{}-end", area.name)));
        }
        self
    }

    /// A subgridded axis's explicit grid (§9): the `tracks` its parent's
    /// grid area spans, each line named by the parent (`parent`) and by
    /// the subgrid's own `<line-name-list>` (`own`).
    pub(super) fn subgrid(tracks: usize, parent: Vec<Vec<String>>, own: &LineNameList) -> Self {
        let mut names: Vec<Vec<Cow<'a, str>>> = parent
            .into_iter()
            .map(|n| n.into_iter().map(Cow::Owned).collect())
            .collect();
        names.resize_with(tracks + 1, Vec::new);
        for (line, own) in names.iter_mut().zip(own.expand(tracks + 1)) {
            line.extend(own.into_iter().map(Cow::Owned));
        }
        Explicit {
            sizes: Vec::new(),
            count: tracks,
            names,
            auto_fit: 0..0,
            inherited: true,
        }
    }

    /// The explicit grid's lines, for placement.
    pub(super) fn lines(&self) -> Lines<'_> {
        Lines {
            tracks: self.count,
            names: &self.names,
        }
    }

    fn push(&mut self, size: &'a TrackSize) {
        if self.sizes.len() < MAX_TRACKS {
            self.sizes.push(size);
            self.count = self.sizes.len();
            self.names.push(Vec::new());
        }
    }

    /// Add `names` to the current last line.
    fn name_line(&mut self, names: &'a [String]) {
        if let Some(line) = self.names.last_mut() {
            line.extend(names.iter().map(|n| Cow::Borrowed(n.as_str())));
        }
    }
}

/// How many times an automatic repetition repeats (§7.2.3.2): with a
/// definite size (or maximum), the most repetitions that do not overflow
/// it — at least one; else with a definite minimum the fewest that reach
/// it; else one. Each track counts as its max track sizing function when
/// that is definite — floored by a definite min one, as a track's growth
/// limit is never below its base size (§11.4), so `minmax(5, 2)` counts
/// as 5 — else its min one; gaps count between every track.
fn auto_repetitions(template: &GridTemplate, bounds: Bounds) -> usize {
    let Some(list) = template.tracks() else {
        return 0;
    };
    let basis = bounds.size;
    let definite = |s: &TrackSize| -> u32 {
        let pick = |b: &TrackBreadth| b.cells(basis).map(u32::from);
        let min = pick(s.min_sizing());
        match pick(s.max_sizing()) {
            Some(max) => max.max(min.unwrap_or(0)),
            None => min.unwrap_or(0),
        }
    };
    let (mut others, mut other_tracks) = (0u32, 0u32);
    let mut repeat = None;
    for item in &list.items {
        match item {
            TrackListItem::Size(s) => {
                others = others.saturating_add(definite(s));
                other_tracks = other_tracks.saturating_add(1);
            }
            TrackListItem::Repeat(r) if r.count.is_auto() => repeat = Some(r),
            TrackListItem::Repeat(r) => {
                let RepeatCount::Count(n) = r.count else {
                    continue;
                };
                let n = n.min(MAX_TRACKS as u32);
                let per = r.sizes.iter().map(definite).fold(0u32, u32::saturating_add);
                others = others.saturating_add(n.saturating_mul(per));
                other_tracks = other_tracks.saturating_add(n.saturating_mul(r.sizes.len() as u32));
            }
        }
    }
    let Some(r) = repeat else {
        return 0;
    };
    let per = i64::from(r.sizes.iter().map(definite).fold(0u32, u32::saturating_add));
    let k = r.sizes.len() as i64;
    let gap = i64::from(bounds.gap);
    // With `n ≥ 1` repetitions the grid is `fixed + n × step` cells: the
    // other tracks, their gaps and the repetitions' gaps.
    let fixed = i64::from(others) + gap * i64::from(other_tracks) - gap;
    let step = per + gap * k;
    let most = (MAX_TRACKS as i64 / k.max(1)).max(1);
    if step == 0 {
        return 1;
    }
    let n = if let Some(limit) = bounds.size.or(bounds.max) {
        // The most that do not overflow it.
        (i64::from(limit) - fixed).div_euclid(step)
    } else if let Some(min) = bounds.min {
        // The fewest that reach it.
        (i64::from(min) - fixed + step - 1).div_euclid(step)
    } else {
        1
    };
    n.clamp(1, most) as usize
}
