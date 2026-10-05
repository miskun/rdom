//! The sizing run of a grid (CSS Grid 2 §11.1): its items placed (§8),
//! each axis's tracks built ([`tracks_of`]: explicit, implicit, collapsed)
//! and sized by the track sizing algorithm over the items'
//! contributions ([`run`]) — the columns, then the rows at the columns'
//! widths. Both the layout (`layout_grid_children`) and the container's
//! intrinsic measurement (`intrinsic::content_size`) size through
//! [`size_grid`].

use rdom_core::{Dom, NodeId};

use super::lines::{AxisLines, GridLines};
use super::placement::{self, Placed};
use super::sizing::{self, Frame};
use super::template::{Bounds, Explicit};
use super::track::{Track, TrackGrid};
use super::{AxisContext, Dimension, Grid, baseline, content_bounds, contribution, margins};
use crate::ext::TuiExt;
use crate::layout::{Sides, TrackSize};
use crate::render::layout_pass::items;
use crate::style::ComputedStyle;

/// Size the grid of `id` (styled `computed`): its items placed (§8.5),
/// its columns sized under `columns` (§11.3), then — with `rows` — its
/// rows at the columns' widths (§11.1 steps 1–2).
pub(super) fn size_grid(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    columns: AxisContext,
    rows: Option<AxisContext>,
) -> Grid {
    let mut children = items::items_of(dom, id);
    // §8.5: placement takes the items in order-modified document order
    // (CSS Display 3 §3).
    items::sort_by_order(dom, &mut children);
    let row_bounds = rows.map_or_else(Bounds::default, |r| r.bounds);
    let areas = &computed.grid_template_areas;
    let explicit_columns =
        Explicit::of(&computed.grid_template_columns, columns.bounds).with_areas(areas, false);
    let explicit_rows =
        Explicit::of(&computed.grid_template_rows, row_bounds).with_areas(areas, true);
    // §8.3: each item's lines on both axes, against the explicit grid's.
    let column_lines = explicit_columns.lines();
    let row_lines = explicit_rows.lines();
    let areas = children
        .iter()
        .map(|c| {
            let s = c.computed(dom);
            (
                placement::resolve(&s.grid_row_start, &s.grid_row_end, row_lines),
                placement::resolve(&s.grid_column_start, &s.grid_column_end, column_lines),
            )
        })
        .collect();
    let mut placement = placement::place(
        children,
        areas,
        explicit_rows.count,
        explicit_columns.count,
        computed.grid_auto_flow,
    );
    for p in &mut placement.items {
        p.trim = trim(computed, p, placement.columns, placement.rows);
    }
    let mut column_grid = tracks_of(
        &explicit_columns,
        &computed.grid_auto_columns,
        Extent {
            count: placement.columns,
            before: placement.columns_before,
        },
        &placement.items,
        Dimension::Columns,
        columns.bounds,
    );
    // Columns measure their items with no width yet: a percentage of it
    // is cyclic (CSS Sizing 3 §5.2.1), so 0.
    let budgets = vec![(0, 0); placement.items.len()];
    run(
        dom,
        &mut column_grid,
        &placement.items,
        Dimension::Columns,
        Budgets {
            budgets,
            shims: Vec::new(),
        },
        columns,
        computed,
    );
    let mut baselines = Vec::new();
    let row_grid = rows.map(|rows| {
        let mut row_grid = tracks_of(
            &explicit_rows,
            &computed.grid_auto_rows,
            Extent {
                count: placement.rows,
                before: placement.rows_before,
            },
            &placement.items,
            Dimension::Rows,
            rows.bounds,
        );
        // Each item's content wraps to its grid area's width, less its
        // margins (§11.1 step 2).
        let extents = column_grid.extents();
        let areas: Vec<u16> = placement
            .items
            .iter()
            .map(|p| {
                let area = extents[p.columns.end - 1].1 - extents[p.columns.start].0;
                area.min(u32::from(u16::MAX)) as u16
            })
            .collect();
        let budgets = placement
            .items
            .iter()
            .zip(&areas)
            .map(|(p, &area)| {
                let m = margins(dom, p, area);
                let width =
                    (i32::from(area) - m.left - m.right).clamp(0, i32::from(u16::MAX)) as u16;
                (width, area)
            })
            .collect();
        // §11.5 step 1: the baseline-aligned items' shims count toward
        // their rows.
        baselines = baseline::shims(dom, computed, &placement.items, &areas);
        let shims = baselines
            .iter()
            .map(|s| s.map_or(0, |s| s.offset.max(0) as u32))
            .collect();
        run(
            dom,
            &mut row_grid,
            &placement.items,
            Dimension::Rows,
            Budgets { budgets, shims },
            rows,
            computed,
        );
        row_grid
    });
    let axis = |explicit: &Explicit<'_>, before: usize| AxisLines {
        explicit: explicit.count,
        names: explicit
            .names
            .iter()
            .map(|n| n.iter().map(|s| s.to_string()).collect())
            .collect(),
        before,
        edges: Vec::new(),
    };
    Grid {
        columns: column_grid,
        rows: row_grid,
        lines: GridLines {
            columns: axis(&explicit_columns, placement.columns_before),
            rows: axis(&explicit_rows, placement.rows_before),
        },
        placed: placement.items,
        baselines,
    }
}

/// Which of `p`'s physical margins `margin-trim` drops on its grid
/// container (CSS Box 4 §3): those adjoining a trimmed edge of the grid
/// — `block-start` / `block-end` the items in its first / last row,
/// `inline-start` / `inline-end` those in its first / last column (the
/// right / left one under `direction: rtl`), of a grid `columns` ×
/// `rows` tracks.
fn trim(container: &ComputedStyle, p: &Placed, columns: usize, rows: usize) -> Sides<bool> {
    let t = container.margin_trim;
    let start = t.inline_start && p.columns.start == 0;
    let end = t.inline_end && p.columns.end == columns;
    let (left, right) = if crate::render::layout_pass::margin_trim::inline_reversed(container) {
        (end, start)
    } else {
        (start, end)
    };
    Sides {
        top: t.block_start && p.rows.start == 0,
        right,
        bottom: t.block_end && p.rows.end == rows,
        left,
    }
}

/// The size of one axis's implicit grid.
#[derive(Debug, Clone, Copy)]
struct Extent {
    /// Tracks in all.
    count: usize,
    /// Implicit tracks before the explicit grid.
    before: usize,
}

/// One axis's tracks (`extent` of them): the explicit ones, and around
/// them implicit ones sized by the `implicit` pattern (§7.6:
/// `grid-auto-columns` / `-rows` — the first after the explicit grid
/// takes its first size and so on forwards, the last before it its last
/// size and so on backwards), each initialized against the percentage
/// basis `bounds.size` (§11.4); an `auto-fit` repetition's tracks that
/// no item spans are collapsed (§7.2.3.2).
fn tracks_of(
    explicit: &Explicit<'_>,
    implicit: &[TrackSize],
    extent: Extent,
    placed: &[Placed],
    dimension: Dimension,
    bounds: Bounds,
) -> TrackGrid {
    const AUTO: TrackSize = TrackSize::AUTO;
    let Extent { count, before } = extent;
    let mut occupied = vec![false; count];
    for p in placed {
        for t in dimension.span(p).tracks() {
            occupied[t] = true;
        }
    }
    let collapsed: Vec<bool> = (0..count)
        .map(|t| t >= before && explicit.auto_fit.contains(&(t - before)) && !occupied[t])
        .collect();
    let pattern = |k: usize, forwards: bool| -> &TrackSize {
        let n = implicit.len();
        match n {
            0 => &AUTO,
            _ if forwards => &implicit[k % n],
            _ => &implicit[n - 1 - k % n],
        }
    };
    let tracks = (0..count)
        .map(|t| {
            if collapsed[t] {
                return Track::collapsed();
            }
            let size = if t < before {
                pattern(before - 1 - t, false)
            } else {
                let k = t - before;
                explicit
                    .sizes
                    .get(k)
                    .copied()
                    .unwrap_or_else(|| pattern(k - explicit.sizes.len(), true))
            };
            Track::new(size, bounds.size)
        })
        .collect();
    TrackGrid::new(tracks, &collapsed, u32::from(bounds.gap))
}

/// What one axis's items are measured against: each item's budget on
/// the other axis and its containing block's width
/// (`contribution::Measured`), and the baseline shim added to its
/// contributions (none for an item past the list's end).
struct Budgets {
    budgets: Vec<(u16, u16)>,
    shims: Vec<u32>,
}

/// Size `grid` (§11.3) for `placed`'s items along `dimension`, each
/// measured against its budget.
fn run(
    dom: &Dom<TuiExt>,
    grid: &mut TrackGrid,
    placed: &[Placed],
    dimension: Dimension,
    budgets: Budgets,
    axis: AxisContext,
    computed: &ComputedStyle,
) {
    let spans = contribution::spans(placed, dimension);
    let mut measured = contribution::Measured::new(dom, placed, dimension, grid, budgets.budgets)
        .with_shims(budgets.shims);
    let (min, max) = content_bounds(computed, dimension);
    sizing::size_tracks(
        grid,
        &spans,
        &mut measured,
        Frame {
            space: axis.space,
            stretch: axis.stretch,
            min: min.map(u32::from),
            max: max.map(u32::from),
        },
    );
}
