//! Row heights (CSS 2.1 §17.5.3): a row is as tall as its tallest cell
//! that spans it alone, and at least its own `height`; a cell spanning
//! several rows that needs more than they and the lines between them give
//! spreads the excess over them by their heights (equally when they have
//! none — CSS Tables 3); a `visibility: collapse` row is 0 tall (§17.5.5).
//! A cell's height is its border box's at its width — its content's, at
//! least its own `height` — less, in the collapsing model, its borders on
//! the lines.

use rdom_core::Dom;

use super::Model;
use super::anonymous;
use super::grid::{Grid, GridCell};
use super::lines::{Lines, cell_border};
use super::structure::Cell;
use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::content_max_size;
use crate::render::layout_pass::shares::Rolling;

/// `cell`'s border-box width given the column widths: its columns and
/// the lines between them, plus — in the collapsing model — the outer
/// lines its own borders sit on.
pub(super) fn cell_width(
    dom: &Dom<TuiExt>,
    cell: &GridCell,
    lines: &Lines,
    model: Model,
    columns: &[u16],
) -> u16 {
    let inside = columns[cell.column..cell.column_end().min(columns.len())]
        .iter()
        .fold(0u16, |a, &w| a.saturating_add(w))
        .saturating_add(lines.inner_vertical(cell.column, cell.column_end()));
    match model {
        Model::Separate { .. } => inside,
        Model::Collapse => {
            let b = cell_border(dom, cell);
            let on = |side: crate::layout::BorderStyle, line: usize| {
                if side.is_none() {
                    0
                } else {
                    lines.vertical.get(line).copied().unwrap_or(0)
                }
            };
            inside
                .saturating_add(on(b.left, cell.column))
                .saturating_add(on(b.right, cell.column_end()))
        }
    }
}

/// `cell`'s height between the lines, at its border-box width `width`.
pub(super) fn cell_height(
    dom: &Dom<TuiExt>,
    cell: &GridCell,
    model: Model,
    width: u16,
    cb: u16,
) -> u16 {
    let id = match &cell.cell {
        Cell::Element(id) => *id,
        Cell::Anonymous(a) => return anonymous::height(dom, a, width, cb),
    };
    let Some(c) = dom.node(id).computed_rc() else {
        return 0;
    };
    let content = content_max_size(dom, id, Direction::Column, width, cb);
    let own = match &c.height {
        Size::Fixed(h) => Sizer::vertical(&c, cb).outer(*h),
        _ => 0,
    };
    let borders = match model {
        Model::Collapse => c.border.top.cells().saturating_add(c.border.bottom.cells()),
        Model::Separate { .. } => 0,
    };
    content.max(own).saturating_sub(borders)
}

/// The rows' heights and baselines.
pub(super) struct Rows {
    pub(super) heights: Vec<u16>,
    /// Each row's baseline from its track's top (`align::row_baselines`).
    pub(super) baselines: Vec<Option<u16>>,
}

/// Every row's height (see the module docs), its `baseline` cells aligned
/// (`align`, §17.5.3).
pub(super) fn heights(
    dom: &Dom<TuiExt>,
    grid: &Grid,
    lines: &Lines,
    model: Model,
    columns: &[u16],
    cb: u16,
) -> Rows {
    let mut rows: Vec<u16> = grid
        .rows
        .iter()
        .map(|r| {
            r.element
                .and_then(|id| dom.node(id).computed().map(|c| c.height.clone()))
                .and_then(|h| match h {
                    Size::Fixed(n) => Some(n),
                    _ => None,
                })
                .unwrap_or(0)
        })
        .collect();
    let widths: Vec<u16> = grid
        .cells
        .iter()
        .map(|cell| cell_width(dom, cell, lines, model, columns))
        .collect();
    let baselines = super::align::row_baselines(dom, grid, lines, model, &widths, cb);
    let mut spanning = Vec::new();
    for (cell, &width) in grid.cells.iter().zip(&widths) {
        let h = cell_height(dom, cell, model, width, cb);
        if cell.rows > 1 {
            spanning.push((cell, h));
            continue;
        }
        // A `baseline` cell needs its row deep enough to hold it moved
        // down to the row's baseline.
        let shift = match baselines[cell.row] {
            Some(b) if super::align::of(dom, cell) == super::align::CellAlign::Baseline => {
                b.saturating_sub(super::align::baseline(dom, cell, lines, model, width, cb))
            }
            _ => 0,
        };
        rows[cell.row] = rows[cell.row].max(h.saturating_add(shift));
    }
    spanning.sort_by_key(|(cell, _)| cell.rows);
    for (cell, h) in spanning {
        let range = cell.row..cell.row_end().min(rows.len());
        let need = h.saturating_sub(lines.inner_horizontal(cell.row, cell.row_end()));
        let live: Vec<usize> = range.filter(|&r| !grid.rows[r].collapsed).collect();
        spread(&mut rows, &live, need);
    }
    for (r, row) in grid.rows.iter().enumerate() {
        if row.collapsed {
            rows[r] = 0;
        }
    }
    Rows {
        heights: rows,
        baselines,
    }
}

/// Grow the rows `members` so they sum to at least `need`, the shortfall
/// spread by their heights (equally when they are all 0), in whole cells.
pub(super) fn spread(rows: &mut [u16], members: &[usize], need: u16) {
    let have: u32 = members.iter().map(|&r| u32::from(rows[r])).sum();
    let short = u32::from(need).saturating_sub(have);
    if short == 0 || members.is_empty() {
        return;
    }
    let equal = have == 0;
    let total = if equal {
        members.len() as f64
    } else {
        f64::from(have)
    };
    let mut shares = Rolling::new(f64::from(short), total);
    for &r in members {
        let w = if equal { 1.0 } else { f64::from(rows[r]) };
        rows[r] = rows[r].saturating_add(shares.share(w).min(u32::from(u16::MAX)) as u16);
    }
}
