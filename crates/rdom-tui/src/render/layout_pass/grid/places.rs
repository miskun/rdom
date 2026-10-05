//! A grid's items placed (CSS Grid 2 §8): its explicit grid on each
//! axis — a track list with its named areas (§7.1–§7.3), or a subgridded
//! axis's parent tracks (§9) — each item's lines resolved against it
//! (§8.3) and the auto-placement algorithm run (§8.5); on a subgridded
//! axis every grid area clamped into the explicit grid, which has no
//! implicit tracks there (§9).

use rdom_core::{Dom, NodeId};

use super::Dimension;
use super::placement::{self, Axis, Placed, Placement};
use super::subgrid::{self, Inherit, ParentAxis, SubAxes};
use super::template::{Bounds, Explicit};
use super::track::Span;
use crate::ext::TuiExt;
use crate::layout::{GridLine, GridTemplate, LineNameList, Sides};
use crate::render::layout_pass::items;
use crate::style::ComputedStyle;

/// A sized axis's tracks: each one's start and end offset.
pub(super) type Extents<'a> = &'a [(u32, u32)];

/// A grid's explicit grids and its items placed in them.
pub(super) struct PlacedGrid<'a> {
    pub(super) columns: Explicit<'a>,
    pub(super) rows: Explicit<'a>,
    pub(super) placement: Placement,
    /// The axes each item subgrids (§9), by item.
    pub(super) subgrids: Vec<SubAxes>,
}

impl PlacedGrid<'_> {
    /// What the item `q` inherits from this grid (styled `c`) on the axes
    /// it subgrids, the tracks' extents when this grid has sized them.
    pub(super) fn inherit_for(
        &self,
        dom: &Dom<TuiExt>,
        q: &Placed,
        c: &ComputedStyle,
        extents: (Option<Extents<'_>>, Option<Extents<'_>>),
        cb: u16,
    ) -> Inherit {
        let sub = subgrid::axes(dom, &q.item);
        let rtl = crate::render::layout_pass::margin_trim::inline_reversed(c);
        let axis = |dimension: Dimension| {
            let (explicit, before, extents) = match dimension {
                Dimension::Columns => (&self.columns, self.placement.columns_before, extents.0),
                Dimension::Rows => (&self.rows, self.placement.rows_before, extents.1),
            };
            let parent = ParentAxis {
                names: &explicit.names,
                before,
                rtl,
                extents,
            };
            subgrid::inherited(dom, q, dimension, parent, cb)
        };
        Inherit {
            columns: sub.columns.then(|| axis(Dimension::Columns)),
            rows: sub.rows.then(|| axis(Dimension::Rows)),
        }
    }
}

/// Place the items of the grid container `id` (styled `computed`), its
/// automatic repetitions counted against `column_bounds` /
/// `row_bounds`, the axes `inherit` names taken from its parent.
pub(super) fn place_grid<'a>(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &'a ComputedStyle,
    column_bounds: Bounds,
    row_bounds: Bounds,
    inherit: &Inherit,
) -> PlacedGrid<'a> {
    let mut children = items::items_of(dom, id);
    // §8.5: placement takes the items in order-modified document order
    // (CSS Display 3 §3).
    items::sort_by_order(dom, &mut children);
    let areas = &computed.grid_template_areas;
    let explicit = |template: &'a GridTemplate, bounds, dimension| {
        let rows = dimension == Dimension::Rows;
        match (inherit.on(dimension), template.subgrid()) {
            (Some(i), own) => {
                let none = LineNameList::default();
                Explicit::subgrid(i.tracks, i.names.clone(), own.unwrap_or(&none))
            }
            _ => Explicit::of(template, bounds),
        }
        .with_areas(areas, rows)
    };
    let columns = explicit(
        &computed.grid_template_columns,
        column_bounds,
        Dimension::Columns,
    );
    let rows = explicit(&computed.grid_template_rows, row_bounds, Dimension::Rows);
    let subgrids: Vec<SubAxes> = children.iter().map(|c| subgrid::axes(dom, c)).collect();
    // §8.3: each item's lines on both axes, against the explicit grid's;
    // an auto-placed subgrid spans its `<line-name-list>`'s lines less one
    // (§9); a subgridded axis's areas clamped into it (§9).
    let (column_lines, row_lines) = (columns.lines(), rows.lines());
    let areas = children
        .iter()
        .zip(&subgrids)
        .map(|(c, sub)| {
            let s = c.computed(dom);
            let mut row = placement::resolve(&s.grid_row_start, &s.grid_row_end, row_lines);
            let mut column =
                placement::resolve(&s.grid_column_start, &s.grid_column_end, column_lines);
            if sub.rows {
                row = own_span(
                    row,
                    &s.grid_row_start,
                    &s.grid_row_end,
                    &s.grid_template_rows,
                );
            }
            if sub.columns {
                column = own_span(
                    column,
                    &s.grid_column_start,
                    &s.grid_column_end,
                    &s.grid_template_columns,
                );
            }
            if inherit.rows.is_some() {
                row = clamp(row, rows.count);
            }
            if inherit.columns.is_some() {
                column = clamp(column, columns.count);
            }
            (row, column)
        })
        .collect();
    let mut placement = placement::place(
        children,
        areas,
        rows.count,
        columns.count,
        computed.grid_auto_flow,
    );
    // Auto-placement may still have gone past a subgridded axis's end.
    if inherit.columns.is_some() {
        clamp_placed(&mut placement, Dimension::Columns, columns.count);
    }
    if inherit.rows.is_some() {
        clamp_placed(&mut placement, Dimension::Rows, rows.count);
    }
    for p in &mut placement.items {
        p.trim = trim(computed, p, placement.columns, placement.rows);
    }
    PlacedGrid {
        columns,
        rows,
        placement,
        subgrids,
    }
}

/// §9: a subgrid auto-placed on an axis it subgrids spans the lines its
/// `<line-name-list>` names, less one — at least one track.
fn own_span(axis: Axis, start: &GridLine, end: &GridLine, template: &GridTemplate) -> Axis {
    match (axis, start, end, template.subgrid()) {
        (Axis::Auto { .. }, GridLine::Auto, GridLine::Auto, Some(list)) => Axis::Auto {
            span: list
                .explicit_lines()
                .saturating_sub(1)
                .clamp(1, i32::MAX as usize) as i32,
        },
        _ => axis,
    }
}

/// §9: a subgridded axis has no implicit tracks — an area is clamped
/// into its `tracks` "using the same procedure as for clamping placement
/// in an overly-large grid" (§8): its lines into the grid, at least one
/// track; a span no wider than the grid.
fn clamp(axis: Axis, tracks: usize) -> Axis {
    let n = tracks.max(1) as i32;
    match axis {
        Axis::Definite { start, end } => {
            let start = start.clamp(0, n - 1);
            Axis::Definite {
                start,
                end: end.clamp(start + 1, n),
            }
        }
        Axis::Auto { span } => Axis::Auto {
            span: span.clamp(1, n),
        },
    }
}

/// Clamp every placed area into the explicit grid's `tracks` on
/// `dimension`, which then has no implicit tracks.
fn clamp_placed(placement: &mut Placement, dimension: Dimension, tracks: usize) {
    let n = tracks.max(1);
    let before = match dimension {
        Dimension::Columns => placement.columns_before,
        Dimension::Rows => placement.rows_before,
    };
    for p in &mut placement.items {
        let s = dimension.span(p);
        let start = s.start.saturating_sub(before).min(n - 1);
        let end = s.end.saturating_sub(before).clamp(start + 1, n);
        let span = Span::new(start, end);
        match dimension {
            Dimension::Columns => p.columns = span,
            Dimension::Rows => p.rows = span,
        }
    }
    match dimension {
        Dimension::Columns => {
            placement.columns = n;
            placement.columns_before = 0;
        }
        Dimension::Rows => {
            placement.rows = n;
            placement.rows_before = 0;
        }
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
