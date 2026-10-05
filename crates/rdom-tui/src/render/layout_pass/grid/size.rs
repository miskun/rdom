//! The sizing run of a grid (CSS Grid 2 §11.1): its items placed
//! (`places`), each axis's tracks built (`track::tracks_of`) and sized
//! by the track sizing algorithm over the items' contributions
//! ([`run`]) — the columns, then the rows at the columns' widths, then
//! each once more when the rows changed a column contribution (steps
//! 3–4). An axis a subgrid takes from its parent is not sized: its
//! tracks are the parent's (§9); an axis a child subgrids is sized with
//! the child's items in the child's place (§9.5). Both the layout
//! (`layout_grid_children`) and the container's intrinsic measurement
//! (`intrinsic::content_size`) size through [`size_grid`].

use rdom_core::{Dom, NodeId};

use super::lines::{AxisLines, GridLines};
use super::placement::Placed;
use super::places::{PlacedGrid, place_grid};
use super::sizing::{self, Frame};
use super::subgrid::{self, Inherit};
use super::template::{Bounds, Explicit};
use super::track::{Extent, TrackGrid, tracks_of};
use super::{AxisContext, Dimension, Grid, baseline, content_bounds, contribution, margins};
use crate::ext::TuiExt;
use crate::layout::Sides;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::Measure;
use crate::style::ComputedStyle;

/// Size the grid of `id` (styled `computed`), which takes the axes
/// `inherit` names from its parent: its items placed (§8.5), its columns
/// sized under `columns` (§11.3), then — with `rows` — its rows at the
/// columns' widths (§11.1 steps 1–2), and the columns and rows once more
/// each when the rows changed what an item contributes to the columns
/// (steps 3–4).
pub(super) fn size_grid(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    columns: AxisContext,
    rows: Option<AxisContext>,
    inherit: &Inherit,
) -> Grid {
    #[cfg(test)]
    RUNS.with(|r| r.borrow_mut().push(None));
    let row_bounds = rows.map_or_else(Bounds::default, |r| r.bounds);
    let grid = place_grid(dom, id, computed, columns.bounds, row_bounds, inherit);
    let placed = &grid.placement.items;
    let fixed = |dimension: Dimension| {
        inherit
            .on(dimension)
            .and_then(|i| i.extents.as_deref())
            .map(TrackGrid::fixed)
    };
    // The columns sized for items that transfer `transfers` (their
    // ratio widths) to them. Columns measure their items with no width
    // yet: a percentage of it is cyclic (CSS Sizing 3 §5.2.1), so 0.
    let size_columns = |transfers: &[Option<u16>]| {
        if let Some(fixed) = fixed(Dimension::Columns) {
            return fixed;
        }
        let mut tracks = tracks_of(
            &grid.columns,
            &computed.grid_auto_columns,
            Extent {
                count: grid.placement.columns,
                before: grid.placement.columns_before,
            },
            placed,
            Dimension::Columns,
            columns.bounds,
        );
        let list = run_items(dom, computed, &grid, Dimension::Columns, None);
        let mut transfers = transfers.to_vec();
        transfers.resize(list.len(), None);
        let budgets = Budgets {
            budgets: vec![(0, 0); list.len()],
            shims: Vec::new(),
            transfers,
        };
        run(
            dom,
            &mut tracks,
            &list,
            Dimension::Columns,
            budgets,
            columns,
            computed,
        );
        tracks
    };
    // The rows sized at `column_grid`'s widths, and the baseline shims.
    let size_rows = |column_grid: &TrackGrid, rows: AxisContext| {
        let extents = column_grid.extents();
        // Each item's content wraps to its grid area's width, less its
        // margins (§11.1 step 2); a subgrid's item to its width there.
        let areas: Vec<u16> = placed
            .iter()
            .map(|p| span_size(&extents, p.columns.start, p.columns.end))
            .collect();
        // §11.5 step 1: the baseline-aligned items' shims count toward
        // their rows — a subgrid is stretched, so not one of them.
        let baselines: Vec<_> = baseline::shims(dom, computed, placed, &areas)
            .into_iter()
            .zip(&grid.subgrids)
            .map(|(s, sub)| s.filter(|_| !sub.any()))
            .collect();
        if let Some(fixed) = fixed(Dimension::Rows) {
            return (fixed, baselines);
        }
        let mut tracks = tracks_of(
            &grid.rows,
            &computed.grid_auto_rows,
            Extent {
                count: grid.placement.rows,
                before: grid.placement.rows_before,
            },
            placed,
            Dimension::Rows,
            rows.bounds,
        );
        let list = run_items(dom, computed, &grid, Dimension::Rows, Some(&extents));
        let budgets = list
            .iter()
            .map(|p| {
                let area = p
                    .width
                    .unwrap_or_else(|| span_size(&extents, p.columns.start, p.columns.end));
                let m = margins(dom, p, area);
                let width =
                    (i32::from(area) - m.left - m.right).clamp(0, i32::from(u16::MAX)) as u16;
                (width, area)
            })
            .collect();
        let shims = baselines
            .iter()
            .map(|s| s.map_or(0, |s| s.offset.max(0) as u32))
            .collect();
        let budgets = Budgets {
            budgets,
            shims,
            transfers: Vec::new(),
        };
        run(
            dom,
            &mut tracks,
            &list,
            Dimension::Rows,
            budgets,
            rows,
            computed,
        );
        (tracks, baselines)
    };
    // §11.1 step 1: the columns, the ratio items' widths transferred from
    // the heights known without the rows.
    let transfers = ratio_widths(dom, computed, placed, None);
    let mut column_grid = size_columns(&transfers);
    let mut baselines = Vec::new();
    let row_grid = rows.map(|rows| {
        // Step 2: the rows at the columns' widths.
        let (mut row_grid, mut shims) = size_rows(&column_grid, rows);
        // Step 3: an item whose min-content contribution to the columns
        // changed with the rows — a ratio item whose height is now
        // definite — has the columns sized again, once.
        let again = ratio_widths(dom, computed, placed, Some(&row_grid.extents()));
        if again != transfers {
            let before = column_grid.extents();
            column_grid = size_columns(&again);
            // Step 4: the rows again, once, if the columns moved.
            if column_grid.extents() != before {
                (row_grid, shims) = size_rows(&column_grid, rows);
            }
        }
        baselines = shims;
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
    let lines = GridLines {
        columns: axis(&grid.columns, grid.placement.columns_before),
        rows: axis(&grid.rows, grid.placement.rows_before),
        rtl: false,
        subgrids: Vec::new(),
    };
    let inherited = |dimension: Dimension| inherit.on(dimension).and_then(|i| i.extents.clone());
    Grid {
        columns: column_grid,
        rows: row_grid,
        lines,
        inherited_columns: inherited(Dimension::Columns),
        inherited_rows: inherited(Dimension::Rows),
        subgrids: grid.subgrids,
        placed: grid.placement.items,
        baselines,
    }
}

/// The items that size `grid`'s tracks on `dimension`, its own items
/// first and in order (so their indices are `placed`'s): an item that
/// subgrids the axis contributes nothing itself — its items, flattened
/// into these tracks, follow the grid's own (§9.5); one that subgrids
/// only the other axis is measured as a grid sized with that axis
/// inherited. `columns`, for the rows, are the columns' extents.
fn run_items(
    dom: &Dom<TuiExt>,
    computed: &ComputedStyle,
    grid: &PlacedGrid<'_>,
    dimension: Dimension,
    columns: Option<&[(u32, u32)]>,
) -> Vec<Placed> {
    let placed = &grid.placement.items;
    let mut list = placed.clone();
    let rtl = crate::render::layout_pass::margin_trim::inline_reversed(computed);
    let gap = match dimension {
        Dimension::Columns => computed.column_gap.resolve(0),
        Dimension::Rows => computed.row_gap.resolve(0),
    };
    let mut flattened = Vec::new();
    for (k, p) in placed.iter().enumerate() {
        let sub = grid.subgrids[k];
        if !sub.any() {
            continue;
        }
        let area = columns.map_or(0, |e| span_size(e, p.columns.start, p.columns.end));
        let inherit = grid.inherit_for(dom, p, computed, (columns, None), area);
        if sub.on(dimension) {
            list[k].size = Some((0, 0));
            list[k].trim = Sides {
                top: true,
                right: true,
                bottom: true,
                left: true,
            };
            let c = p.item.computed(dom);
            let e = subgrid::edges(&c, area);
            let content = i32::from(area) - e.left - e.right;
            flattened.extend(subgrid::flatten(
                dom,
                p,
                dimension,
                &inherit,
                rtl,
                gap,
                Some(content.clamp(0, i32::from(u16::MAX)) as u16),
            ));
        } else {
            list[k].size = Some(measure_subgrid(dom, p, dimension, &inherit, area));
        }
    }
    list.extend(flattened);
    list
}

/// The border-box min- and max-content size on `dimension` of the
/// subgrid `p`, which subgrids only the other axis (`inherit`): its
/// tracks on `dimension` sized as its content size is (§5.2) with the
/// other axis its parent's, plus its padding and border. `area` is its
/// grid area's width.
fn measure_subgrid(
    dom: &Dom<TuiExt>,
    p: &Placed,
    dimension: Dimension,
    inherit: &Inherit,
    area: u16,
) -> (u16, u16) {
    let crate::render::layout_pass::items::Item::Element(id) = p.item else {
        return (0, 0);
    };
    // Once a pass for each subgrid, axis, area and inherited axis (its
    // own measurement nests the measurements of its subgrids): a chain of
    // nested subgrids costs its length, not its square.
    let key = (id, format!("{dimension:?} {area} {inherit:?}"));
    if let Some(size) = crate::render::layout_pass::intrinsic::get_subgrid(dom, &key) {
        return size;
    }
    let c = p.item.computed(dom);
    let content = |measure| {
        super::intrinsic::content_size_with(
            dom,
            id,
            &c,
            dimension.direction(),
            area,
            area,
            measure,
            inherit,
        )
    };
    let chrome = Sizer::along(&c, dimension.direction(), area).chrome();
    let size = |n: u16| n.saturating_add(chrome);
    let sizes = match dimension {
        Dimension::Columns => (
            size(content(Measure::MinContent)),
            size(content(Measure::MaxContent)),
        ),
        Dimension::Rows => {
            let h = size(content(Measure::MaxContent));
            (h, h)
        }
    };
    crate::render::layout_pass::intrinsic::put_subgrid(dom, key, sizes);
    sizes
}

/// What one axis's items are measured against: each item's budget on
/// the other axis and its containing block's width
/// (`contribution::Measured`), and the baseline shim added to its
/// contributions (none for an item past the list's end).
struct Budgets {
    budgets: Vec<(u16, u16)>,
    shims: Vec<u32>,
    /// Per item on the columns: the width its aspect ratio transfers
    /// from a definite height, its contribution then (CSS Sizing 4 §5.1).
    transfers: Vec<Option<u16>>,
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
        .with_shims(budgets.shims)
        .with_transfers(budgets.transfers);
    #[cfg(test)]
    RUNS.with(|r| r.borrow_mut().push(Some(dimension)));
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

/// The extent of tracks `start..end` of `extents` (each track's start
/// and end offset), gutters included.
pub(super) fn span_size(extents: &[(u32, u32)], start: usize, end: usize) -> u16 {
    let size = extents[end - 1].1 - extents[start].0;
    size.min(u32::from(u16::MAX)) as u16
}

/// Each item's width transferred through its aspect ratio from a definite
/// height (`arrange::ratio_width`), the rows' extents `rows` giving its
/// grid area's height once they are sized.
fn ratio_widths(
    dom: &Dom<TuiExt>,
    container: &ComputedStyle,
    placed: &[Placed],
    rows: Option<&[(u32, u32)]>,
) -> Vec<Option<u16>> {
    placed
        .iter()
        .map(|p| {
            let area = rows.map(|r| span_size(r, p.rows.start, p.rows.end));
            super::arrange::ratio_width(dom, p, container, area)
        })
        .collect()
}

#[cfg(test)]
thread_local! {
    /// The sizing runs made, by axis, in order, each `size_grid` call
    /// opening with a `None` (tests only).
    pub(super) static RUNS: std::cell::RefCell<Vec<Option<Dimension>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}
