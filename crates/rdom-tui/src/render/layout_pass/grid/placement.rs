//! Grid item placement (CSS Grid 2 §8): each item's grid area, from its
//! `grid-row-*` / `grid-column-*` lines (§8.3, resolved by [`resolve`]
//! with §8.3.1's conflict handling) and the auto-placement algorithm
//! (§8.5, [`place`]) under `grid-auto-flow`, and the implicit grid that
//! adds — tracks after the explicit grid and before it (§7.5).
//!
//! Lines are numbered here from 0, the explicit grid's first line; a
//! line before the explicit grid is negative. [`place`] turns them into
//! track indices of the implicit grid.

use std::collections::BTreeMap;

use super::template::MAX_TRACKS;
use super::track::Span;
use crate::layout::{Direction, GridAutoFlow, GridLine, Sides};
use crate::render::layout_pass::items::Item;

/// An item and its grid area.
#[derive(Debug, Clone)]
pub(super) struct Placed {
    pub(super) item: Item,
    pub(super) columns: Span,
    pub(super) rows: Span,
    /// Its physical margins `margin-trim` drops (`grid::trim`), set once
    /// the grid's size is known.
    pub(super) trim: Sides<bool>,
}

/// The items placed, and the implicit grid's size: how many tracks it
/// has on each axis, and how many of them come before the explicit grid
/// (§7.5 — the explicit grid's first line is track `before`'s start).
#[derive(Debug, Clone)]
pub(super) struct Placement {
    pub(super) items: Vec<Placed>,
    pub(super) columns: usize,
    pub(super) rows: usize,
    pub(super) columns_before: usize,
    pub(super) rows_before: usize,
}

/// The explicit grid's lines on one axis: how many tracks it has, and
/// each line's names (§7.2, one entry per line). The names a named area
/// gives its edges (`<area>-start` / `-end`, §7.3.2) join them here.
#[derive(Debug, Clone, Copy)]
pub(super) struct Lines<'a> {
    pub(super) tracks: usize,
    pub(super) names: &'a [Vec<&'a str>],
}

/// The farthest a line may lie from the explicit grid's first line (§8:
/// a UA may clamp the implicit grid; rdom keeps lines in ±10 000).
const LIMIT: i32 = MAX_TRACKS as i32;

/// A placement on one axis (§8.3): definite lines, or a span to
/// auto-place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Axis {
    Definite { start: i32, end: i32 },
    Auto { span: i32 },
}

/// Which edge a `<grid-line>` places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Start,
    End,
}

/// A span count, clamped to the grid.
fn count(n: u32) -> i32 {
    n.min(MAX_TRACKS as u32) as i32
}

impl Lines<'_> {
    /// The last explicit line.
    fn last(&self) -> i32 {
        self.tracks as i32
    }

    /// Whether explicit line `x` has `name`.
    fn named(&self, x: i32, name: &str) -> bool {
        usize::try_from(x)
            .ok()
            .and_then(|x| self.names.get(x))
            .is_some_and(|n| n.contains(&name))
    }

    /// The `n`th line named `name` (§8.3 `<integer> && <custom-ident>`):
    /// counted from the start when `n > 0`, from the end when `n < 0`;
    /// when there are fewer, every implicit line counts as one.
    fn nth_named(&self, n: i32, name: &str) -> i32 {
        if n > 0 {
            self.search(-1, name, n, true)
        } else {
            self.search(self.last() + 1, name, n.saturating_neg(), false)
        }
    }

    /// The `count`th line named `name` after (`forwards`) or before line
    /// `from`, every line past the explicit grid on that side counting.
    fn search(&self, from: i32, name: &str, count: i32, forwards: bool) -> i32 {
        let mut left = count;
        if forwards {
            for x in (from + 1).max(0)..=self.last() {
                if self.named(x, name) {
                    left -= 1;
                    if left == 0 {
                        return x;
                    }
                }
            }
            from.max(self.last()).saturating_add(left)
        } else {
            for x in (0..from.min(self.last() + 1)).rev() {
                if self.named(x, name) {
                    left -= 1;
                    if left == 0 {
                        return x;
                    }
                }
            }
            from.min(0).saturating_sub(left)
        }
    }

    /// The line a definite `<grid-line>` names on `edge`, if it is one.
    fn definite(&self, line: &GridLine, edge: Edge) -> Option<i32> {
        let x = match line {
            GridLine::Line { index, name: None } if *index > 0 => index - 1,
            GridLine::Line { index, name: None } => self.last().saturating_add(1 + index),
            GridLine::Line {
                index,
                name: Some(name),
            } => self.nth_named(*index, name),
            // A lone `<custom-ident>` is first the area edge `<ident>-start`
            // / `-end`, else `1 <ident>` (§8.3).
            GridLine::Name(name) => {
                let edge_name = match edge {
                    Edge::Start => format!("{name}-start"),
                    Edge::End => format!("{name}-end"),
                };
                (0..=self.last())
                    .find(|&x| self.named(x, &edge_name))
                    .unwrap_or_else(|| self.nth_named(1, name))
            }
            GridLine::Auto | GridLine::Span { .. } => return None,
        };
        Some(x.clamp(-LIMIT, LIMIT))
    }
}

/// An item's placement on one axis from its `start` and `end` lines
/// (§8.3), conflicts handled per §8.3.1: lines in the wrong order swap,
/// an end equal to the start drops, the end's span drops beside a
/// start's, and a span to a name alone is a span of one.
pub(super) fn resolve(start: &GridLine, end: &GridLine, lines: Lines<'_>) -> Axis {
    let span = |line: &GridLine| match line {
        GridLine::Span { count: n, .. } => count(*n),
        _ => 1,
    };
    match (
        lines.definite(start, Edge::Start),
        lines.definite(end, Edge::End),
    ) {
        (Some(s), Some(e)) if s == e => Axis::Definite {
            start: s,
            end: s + 1,
        },
        (Some(s), Some(e)) => Axis::Definite {
            start: s.min(e),
            end: s.max(e),
        },
        (Some(s), None) => {
            let e = match end {
                GridLine::Span {
                    count: n,
                    name: Some(name),
                } => lines.search(s, name, count(*n), true),
                other => s + span(other),
            };
            Axis::Definite {
                start: s,
                end: e.clamp(s + 1, LIMIT + 1),
            }
        }
        (None, Some(e)) => {
            let s = match start {
                GridLine::Span {
                    count: n,
                    name: Some(name),
                } => lines.search(e, name, count(*n), false),
                other => e - span(other),
            };
            Axis::Definite {
                start: s.clamp(-LIMIT - 1, e - 1),
                end: e,
            }
        }
        // Both automatic: the start's span, else the end's; a span to a
        // name is a span of one.
        (None, None) => {
            let unnamed = |line: &GridLine| match line {
                GridLine::Span {
                    count: n,
                    name: None,
                } => Some(count(*n)),
                GridLine::Span { name: Some(_), .. } => Some(1),
                _ => None,
            };
            Axis::Auto {
                span: unnamed(start).or_else(|| unnamed(end)).unwrap_or(1),
            }
        }
    }
}

/// The cells already taken, by major-axis line: the minor-axis ranges
/// occupied there.
#[derive(Default)]
struct Occupied(BTreeMap<i32, Vec<(i32, i32)>>);

impl Occupied {
    fn fits(&self, major: (i32, i32), minor: (i32, i32)) -> bool {
        self.0
            .range(major.0..major.1)
            .all(|(_, ranges)| ranges.iter().all(|&(a, b)| b <= minor.0 || minor.1 <= a))
    }

    fn take(&mut self, major: (i32, i32), minor: (i32, i32)) {
        for m in major.0..major.1 {
            self.0.entry(m).or_default().push(minor);
        }
    }
}

/// One item's placement on the flow's two axes: `major` is the axis
/// auto-placement steps along a track at a time (the rows of `row`
/// flow), `minor` the one it fills first.
#[derive(Debug, Clone, Copy)]
struct Area {
    major: Axis,
    minor: Axis,
}

/// The line range of `a`, auto-placed at `at`.
fn range(a: Axis, at: i32) -> (i32, i32) {
    match a {
        Axis::Definite { start, end } => (start, end),
        Axis::Auto { span } => (at, at + span),
    }
}

/// Place `items` (in order-modified document order), whose row- and
/// column-axis placements are `areas`, in a grid of `explicit_rows` ×
/// `explicit_columns` explicit tracks, per the grid item placement
/// algorithm (§8.5) under `flow`.
pub(super) fn place(
    items: Vec<Item>,
    areas: Vec<(Axis, Axis)>,
    explicit_rows: usize,
    explicit_columns: usize,
    flow: GridAutoFlow,
) -> Placement {
    let row_flow = flow.direction == Direction::Row;
    let (explicit_major, explicit_minor) = if row_flow {
        (explicit_rows, explicit_columns)
    } else {
        (explicit_columns, explicit_rows)
    };
    let mut slots: Vec<Area> = areas
        .into_iter()
        .map(|(rows, columns)| match row_flow {
            true => Area {
                major: rows,
                minor: columns,
            },
            false => Area {
                major: columns,
                minor: rows,
            },
        })
        .collect();
    let mut occupied = Occupied::default();
    // 1. Items with definite positions on both axes.
    for s in &slots {
        if let (Axis::Definite { .. }, Axis::Definite { .. }) = (s.major, s.minor) {
            occupied.take(range(s.major, 0), range(s.minor, 0));
        }
    }
    lock_to_major_tracks(&mut slots, &mut occupied, flow.dense);
    // 3. The implicit grid's minor extent: the explicit grid, every
    //    definite minor position, and the widest auto span.
    let (minor_min, mut minor_max) = extent(&slots, explicit_minor, |s| s.minor);
    let widest = slots
        .iter()
        .filter_map(|s| match s.minor {
            Axis::Auto { span } => Some(span),
            Axis::Definite { .. } => None,
        })
        .max()
        .unwrap_or(0);
    minor_max = minor_max.max(minor_min + widest);
    auto_place(
        &mut slots,
        &mut occupied,
        flow.dense,
        (minor_min, minor_max),
    );

    // The implicit grid: every track from the first line used (at most
    // the explicit grid's first) to the last (at least its last).
    let (major_lo, major_hi) = extent(&slots, explicit_major, |s| s.major);
    let (minor_lo, minor_hi) = extent(&slots, explicit_minor, |s| s.minor);
    let span_of = |a: Axis, lo: i32| {
        let (start, end) = range(a, 0);
        Span::new((start - lo) as usize, (end - lo) as usize)
    };
    let placed = items
        .into_iter()
        .zip(&slots)
        .map(|(item, s)| {
            let (major, minor) = (span_of(s.major, major_lo), span_of(s.minor, minor_lo));
            let (rows, columns) = if row_flow {
                (major, minor)
            } else {
                (minor, major)
            };
            Placed {
                item,
                columns,
                rows,
                trim: Sides::default(),
            }
        })
        .collect();
    let major = ((major_hi - major_lo) as usize, (-major_lo) as usize);
    let minor = ((minor_hi - minor_lo) as usize, (-minor_lo) as usize);
    let ((rows, rows_before), (columns, columns_before)) = if row_flow {
        (major, minor)
    } else {
        (minor, major)
    };
    Placement {
        items: placed,
        columns,
        rows,
        columns_before,
        rows_before,
    }
}

/// The lines `axis` of `slots` reaches: from the explicit grid's first
/// line or an earlier one used, to its last (`explicit` tracks on) or a
/// later one used.
fn extent(slots: &[Area], explicit: usize, axis: impl Fn(&Area) -> Axis) -> (i32, i32) {
    slots
        .iter()
        .fold((0, explicit as i32), |(lo, hi), s| match axis(s) {
            Axis::Definite { start, end } => (lo.min(start), hi.max(end)),
            Axis::Auto { .. } => (lo, hi),
        })
}

/// §8.5 step 2: each item with a definite major position and an
/// automatic minor one takes the earliest minor line from the explicit
/// grid's first where it overlaps nothing — and, unless `dense`, past the
/// items this step placed in the same major track before it.
fn lock_to_major_tracks(slots: &mut [Area], occupied: &mut Occupied, dense: bool) {
    let mut past: BTreeMap<i32, i32> = BTreeMap::new();
    for s in slots.iter_mut() {
        let (Axis::Definite { start, end }, Axis::Auto { span }) = (s.major, s.minor) else {
            continue;
        };
        let mut minor = if dense {
            0
        } else {
            past.get(&start).copied().unwrap_or(0)
        };
        while !occupied.fits((start, end), (minor, minor + span)) && minor < LIMIT {
            minor += 1;
        }
        occupied.take((start, end), (minor, minor + span));
        past.insert(start, minor + span);
        s.minor = Axis::Definite {
            start: minor,
            end: minor + span,
        };
    }
}

/// §8.5 step 4: the items not yet placed — an automatic major position —
/// with the auto-placement cursor, from the implicit grid's first major
/// line and `minor`'s first: an item with a definite minor position
/// moves the cursor to it (a new major track when that is behind it) and
/// down to where it fits; an item automatic on both axes takes the next
/// position from the cursor where it fits within `minor`, a new major
/// track when none does. `dense` starts each search from the grid's
/// start.
fn auto_place(slots: &mut [Area], occupied: &mut Occupied, dense: bool, minor: (i32, i32)) {
    let (minor_min, minor_max) = minor;
    let major_start = slots
        .iter()
        .filter_map(|s| match s.major {
            Axis::Definite { start, .. } => Some(start),
            Axis::Auto { .. } => None,
        })
        .fold(0, i32::min);
    let mut cursor = (major_start, minor_min);
    for s in slots.iter_mut() {
        let Axis::Auto { span: major_span } = s.major else {
            // Steps 1 and 2 placed every item with a definite major
            // position.
            continue;
        };
        let fits = |occupied: &Occupied, major: i32, (a, b): (i32, i32)| {
            occupied.fits((major, major + major_span), (a, b))
        };
        match s.minor {
            Axis::Definite { start, end } => {
                if dense {
                    cursor.0 = major_start;
                } else if start < cursor.1 {
                    cursor.0 += 1;
                }
                cursor.1 = start;
                while !fits(occupied, cursor.0, (start, end)) {
                    cursor.0 += 1;
                }
            }
            Axis::Auto { span } => {
                if dense {
                    cursor = (major_start, minor_min);
                }
                loop {
                    let found = (cursor.1..=minor_max - span)
                        .find(|&m| fits(occupied, cursor.0, (m, m + span)));
                    if let Some(m) = found {
                        cursor.1 = m;
                        break;
                    }
                    cursor = (cursor.0 + 1, minor_min);
                }
                s.minor = Axis::Definite {
                    start: cursor.1,
                    end: cursor.1 + span,
                };
            }
        }
        s.major = Axis::Definite {
            start: cursor.0,
            end: cursor.0 + major_span,
        };
        occupied.take(range(s.major, 0), range(s.minor, 0));
    }
}
