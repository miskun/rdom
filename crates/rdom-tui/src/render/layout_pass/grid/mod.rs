//! Grid layout — CSS Grid Layout 2 (section numbers are Grid 1's, which
//! Level 2 keeps through §8 and shifts by one after inserting §9
//! "Subgrids").
//!
//! A grid container (§5) lays its items (§6.1: its in-flow elements,
//! pseudo-elements and anonymous items wrapping its runs of text — the
//! item model flex shares, `layout_pass::items`) out in a grid of tracks:
//! the explicit grid of `grid-template-columns` / `-rows` (§7.2,
//! [`template`]), the items placed in it (§8.5, [`placement`]) and the
//! implicit tracks that adds (§7.5), each axis's tracks sized by the
//! track sizing algorithm (§11.3, [`sizing`], over the items'
//! contributions, [`contribution`]) — the columns first, then the rows at
//! the columns' widths (§11.1) — and each item laid out in its grid area
//! ([`arrange`]). A grid container's own min- and max-content sizes are
//! its tracks sized under that constraint (§5.2, [`intrinsic`]).
//!
//! ## Module layout
//!
//! - `mod.rs` — [`layout_grid_children`] and the sizing run both it and
//!   the intrinsic measurement use ([`size_grid`]).
//! - [`template`] — a track list expanded: `repeat()`, `auto-fill` /
//!   `auto-fit` (§7.2.3.2), line names.
//! - [`placement`] — the items' grid areas and the implicit grid.
//! - [`track`] — the tracks being sized, their sizing functions, gutters.
//! - [`sizing`] — the track sizing algorithm (§11.3–§11.8).
//! - [`contribution`] — the items' min-content, max-content and minimum
//!   contributions on an axis.
//! - [`arrange`] — the items laid out in their grid areas.
//! - [`intrinsic`] — the grid container's content size.

mod arrange;
mod contribution;
#[cfg(test)]
mod cost_tests;
mod intrinsic;
mod placement;
mod sizing;
mod template;
mod track;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Align, Direction, LayoutRect, MarginValue, Sides, Size, TrackSize};
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::items;
use crate::style::ComputedStyle;

pub(super) use intrinsic::content_size;
use placement::Placed;
use sizing::{Frame, Space};
use template::{Bounds, Explicit};
use track::{Span, Track, TrackGrid};

/// One of a grid's two sets of tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    /// The columns, sized on the inline (horizontal) axis.
    Columns,
    /// The rows, sized on the block (vertical) axis.
    Rows,
}

impl Dimension {
    /// The layout direction a measurement along these tracks takes.
    fn direction(self) -> Direction {
        match self {
            Dimension::Columns => Direction::Row,
            Dimension::Rows => Direction::Column,
        }
    }

    /// `p`'s span in these tracks.
    fn span(self, p: &Placed) -> Span {
        match self {
            Dimension::Columns => p.columns,
            Dimension::Rows => p.rows,
        }
    }
}

/// How one axis's tracks are sized.
#[derive(Debug, Clone, Copy)]
struct AxisContext {
    /// The space they size into.
    space: Space,
    /// What an automatic repetition is counted against; `bounds.size` is
    /// also the basis of the tracks' percentages.
    bounds: Bounds,
    /// `justify-content` / `align-content` stretches (§11.8).
    stretch: bool,
}

/// A grid container's items placed and its tracks sized.
struct Grid {
    columns: TrackGrid,
    /// `None` when only the columns were asked for.
    rows: Option<TrackGrid>,
    placed: Vec<Placed>,
}

/// Size the grid of `id` (styled `computed`): its items placed (§8.5),
/// its columns sized under `columns` (§11.3), then — with `rows` — its
/// rows at the columns' widths (§11.1 steps 1–2).
fn size_grid(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    columns: AxisContext,
    rows: Option<AxisContext>,
) -> Grid {
    let mut children = items::items_of(dom, id);
    // §8.5: auto-placement takes the items in order-modified document
    // order (CSS Display 3 §3).
    items::sort_by_order(dom, &mut children);
    let row_bounds = rows.map_or_else(Bounds::default, |r| r.bounds);
    let explicit_columns = Explicit::of(&computed.grid_template_columns, columns.bounds);
    let explicit_rows = Explicit::of(&computed.grid_template_rows, row_bounds);
    let mut placement = placement::place(
        children,
        explicit_columns.sizes.len(),
        explicit_rows.sizes.len(),
    );
    for p in &mut placement.items {
        p.trim = trim(computed, p, placement.columns, placement.rows);
    }
    let mut column_grid = tracks_of(
        &explicit_columns,
        placement.columns,
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
        budgets,
        columns,
        computed,
    );
    let row_grid = rows.map(|rows| {
        let mut row_grid = tracks_of(
            &explicit_rows,
            placement.rows,
            &placement.items,
            Dimension::Rows,
            rows.bounds,
        );
        // Each item's content wraps to its grid area's width, less its
        // margins (§11.1 step 2).
        let extents = column_grid.extents();
        let budgets = placement
            .items
            .iter()
            .map(|p| {
                let area = extents[p.columns.end - 1].1 - extents[p.columns.start].0;
                let area = area.min(u32::from(u16::MAX)) as u16;
                let m = margins(dom, p, area);
                let width =
                    (i32::from(area) - m.left - m.right).clamp(0, i32::from(u16::MAX)) as u16;
                (width, area)
            })
            .collect();
        run(
            dom,
            &mut row_grid,
            &placement.items,
            Dimension::Rows,
            budgets,
            rows,
            computed,
        );
        row_grid
    });
    Grid {
        columns: column_grid,
        rows: row_grid,
        placed: placement.items,
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

/// `p`'s margins, their percentages against `cb` (its grid area's
/// width), either sign: an `auto` one 0, a trimmed one 0.
fn margins(dom: &Dom<TuiExt>, p: &Placed, cb: u16) -> Sides<i32> {
    let c = p.item.computed(dom);
    let side = |m: &MarginValue, trimmed: bool| if trimmed { 0 } else { i32::from(m.resolve(cb)) };
    Sides {
        top: side(&c.margin.top, p.trim.top),
        right: side(&c.margin.right, p.trim.right),
        bottom: side(&c.margin.bottom, p.trim.bottom),
        left: side(&c.margin.left, p.trim.left),
    }
}

/// One axis's tracks: the explicit ones, then implicit `auto` ones up to
/// `count` (§7.6), each initialized against the percentage basis
/// `bounds.size` (§11.4); an `auto-fit` repetition's tracks that no item
/// spans are collapsed (§7.2.3.2).
fn tracks_of(
    explicit: &Explicit<'_>,
    count: usize,
    placed: &[Placed],
    dimension: Dimension,
    bounds: Bounds,
) -> TrackGrid {
    const IMPLICIT: TrackSize = TrackSize::AUTO;
    let mut occupied = vec![false; count];
    for p in placed {
        for t in dimension.span(p).tracks() {
            occupied[t] = true;
        }
    }
    let collapsed: Vec<bool> = (0..count)
        .map(|t| explicit.auto_fit.contains(&t) && !occupied[t])
        .collect();
    let tracks = (0..count)
        .map(|t| {
            if collapsed[t] {
                return Track::collapsed();
            }
            let size = explicit.sizes.get(t).copied().unwrap_or(&IMPLICIT);
            Track::new(size, bounds.size)
        })
        .collect();
    TrackGrid::new(tracks, &collapsed, u32::from(bounds.gap))
}

/// Size `grid` (§11.3) for `placed`'s items along `dimension`, each
/// measured against `budgets[i]`.
fn run(
    dom: &Dom<TuiExt>,
    grid: &mut TrackGrid,
    placed: &[Placed],
    dimension: Dimension,
    budgets: Vec<(u16, u16)>,
    axis: AxisContext,
    computed: &ComputedStyle,
) {
    let spans = contribution::spans(placed, dimension);
    let mut measured = contribution::Measured::new(dom, placed, dimension, grid, budgets);
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

/// The grid container's definite minimum and maximum content-box size on
/// `dimension`'s axis: its `min-*` / `max-*` in cells (a percentage has
/// no basis here), less its padding and border under `border-box`.
fn content_bounds(computed: &ComputedStyle, dimension: Dimension) -> (Option<u16>, Option<u16>) {
    let sizer = Sizer::along(computed, dimension.direction(), 0);
    let (min, max) = match dimension {
        Dimension::Columns => (&computed.min_width, &computed.max_width),
        Dimension::Rows => (&computed.min_height, &computed.max_height),
    };
    (
        sizer.inner_opt(min.cells(None)).filter(|&n| n > 0),
        sizer.inner_opt(max.cells(None)),
    )
}

/// Whether `justify-content` (`Columns`) / `align-content` (`Rows`) lets
/// §11.8 stretch the `auto` tracks: `normal` or `stretch`.
fn stretches(computed: &ComputedStyle, dimension: Dimension) -> bool {
    let a = match dimension {
        Dimension::Columns => computed.justify_content,
        Dimension::Rows => computed.align_content,
    };
    matches!(a.keyword, Align::Normal | Align::Stretch)
}

/// The axis context of a laid-out grid container's content box, `size`
/// cells on `dimension`'s axis when definite.
fn laid_out_axis(computed: &ComputedStyle, dimension: Dimension, size: Option<u16>) -> AxisContext {
    let (min, max) = content_bounds(computed, dimension);
    let gap_basis = size.unwrap_or(0);
    let gap =
        crate::render::layout_pass::gap_along(computed, dimension.direction()).resolve(gap_basis);
    AxisContext {
        space: size.map_or(Space::MaxContent, |s| Space::Definite(u32::from(s))),
        bounds: Bounds {
            size,
            max,
            min,
            gap,
        },
        stretch: stretches(computed, dimension),
    }
}

/// Lay out the grid container `id`'s items inside `container`, its
/// content box (CSS Grid 2 §11.1): the columns sized into its width, the
/// rows into its height when that is definite — else sized to their
/// content, which the container's own height already measured — and each
/// item laid out in its grid area. Returns the anonymous items' boxes.
pub(super) fn layout_grid_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    computed: &ComputedStyle,
) -> Vec<AnonymousIfc> {
    // A positioned child's static position: the content box's start
    // (DIVERGENCES §2, as for a flex container), scrolled.
    let (static_x, static_y) = dom.node(id).ext().map_or((container.x, container.y), |e| {
        (container.x - e.scroll_x, container.y - e.scroll_y)
    });
    for n in super::positioning::out_of_flow_positioned_children(dom, id) {
        super::positioning::record_static_position(dom, n, static_x, static_y);
    }
    let rows_definite = rows_are_definite(dom, id, computed);
    let columns = laid_out_axis(computed, Dimension::Columns, Some(container.width));
    let rows = laid_out_axis(
        computed,
        Dimension::Rows,
        rows_definite.then_some(container.height),
    );
    let grid = size_grid(dom, id, computed, columns, Some(rows));
    arrange::arrange(dom, id, computed, grid, container)
}

/// Whether the grid container `id`'s height is definite (CSS Grid 2
/// §11.1, CSS Sizing 3 §4.1): a length, a percentage of a definite
/// height, or — its height `auto` — the size its flex or grid container
/// gave it (CSS Flexbox §9.8). An `auto` height in block flow is its
/// content's: the rows size to their content.
fn rows_are_definite(dom: &Dom<TuiExt>, id: NodeId, computed: &ComputedStyle) -> bool {
    match &computed.height {
        Size::Fixed(_) => true,
        Size::Percent(_) | Size::Calc(_) => {
            crate::render::layout_pass::block::nearest_block_ancestor_height_is_definite(dom, id)
        }
        Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => {
            crate::render::box_tree::box_parent(dom, id)
                .is_some_and(|p| crate::render::box_tree::is_flex_or_grid_container(dom, p))
        }
    }
}
