//! The lines between and around a table's columns and rows, each a
//! whole number of cells wide (CSS 2.1 §17.6):
//!
//! - **separated borders** (§17.6.1): `border-spacing` before each column
//!   and after the last, and the same between rows — the cells' own
//!   borders inside their boxes;
//! - **collapsing borders** (§17.6.2): one border line where neighbours
//!   meet, one cell wide when any box on it — the table, a row group, a
//!   row, a column (group) or a cell — has a border there, none otherwise.
//!   A cell with a border on a line covers it (its border box reaches over
//!   the line, sharing the cell with its neighbour's border, and paint's
//!   junction pass resolves the conflict, CSS Tables 3 §11.5); a cell
//!   without one stops beside it.
//!
//! A `visibility: collapse` row or column (§17.5.5) takes no space: the
//! lines on its two sides become one.

use rdom_core::{Dom, NodeId};

use super::Model;
use super::grid::{Grid, GridCell};
use super::structure::Structure;
use crate::ext::TuiExt;
use crate::layout::Border;
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

/// The widths of a table's lines.
#[derive(Debug, Clone)]
pub(super) struct Lines {
    /// Before column `j`, and after the last (`columns + 1` lines).
    pub(super) vertical: Vec<u16>,
    /// Before row `i`, and after the last (`rows + 1` lines).
    pub(super) horizontal: Vec<u16>,
}

/// `id`'s used border sides (a missing style is none).
fn border(dom: &Dom<TuiExt>, id: NodeId) -> Border {
    dom.node(id)
        .computed()
        .map_or_else(Border::none, |c| c.border)
}

impl Lines {
    /// The lines of `grid` in a table styled `computed` laid out by
    /// `model`.
    pub(super) fn of(
        dom: &Dom<TuiExt>,
        computed: &ComputedStyle,
        structure: &Structure,
        grid: &Grid,
        model: Model,
    ) -> Self {
        let (columns, rows) = (grid.columns, grid.rows.len());
        let mut lines = match model {
            Model::Separate { h, v } => Lines {
                vertical: vec![if columns == 0 { 0 } else { h }; columns + 1],
                horizontal: vec![if rows == 0 { 0 } else { v }; rows + 1],
            },
            Model::Collapse => Lines {
                vertical: vec![0; columns + 1],
                horizontal: vec![0; rows + 1],
            },
        };
        if model == Model::Collapse {
            lines.collapse_borders(dom, computed, structure, grid);
        }
        for (k, _) in grid
            .collapsed_columns
            .iter()
            .enumerate()
            .filter(|(_, c)| **c)
        {
            merge(&mut lines.vertical, k);
        }
        for (k, _) in grid.rows.iter().enumerate().filter(|(_, r)| r.collapsed) {
            merge(&mut lines.horizontal, k);
        }
        lines
    }

    /// §17.6.2: a line is one cell wide where any box on it has a border
    /// there (`hidden` too — it is the conflict resolution's kill-switch,
    /// which paint resolves on the line, DIVERGENCES §2).
    fn collapse_borders(
        &mut self,
        dom: &Dom<TuiExt>,
        computed: &ComputedStyle,
        structure: &Structure,
        grid: &Grid,
    ) {
        let (n, m) = (grid.columns, grid.rows.len());
        let mut mark =
            |b: Border, l: usize, r: usize, t: usize, bt: usize| self.mark(b, l, r, t, bt);
        mark(computed.border, 0, n, 0, m);
        for (r, row) in grid.rows.iter().enumerate() {
            if let Some(id) = row.element {
                mark(border(dom, id), 0, n, r, r + 1);
            }
        }
        // A row group's top and bottom are its first and last rows'.
        for group in structure.groups.iter() {
            let Some(id) = group.element else { continue };
            let Some(first) = grid.rows.iter().position(|r| r.group == Some(id)) else {
                continue;
            };
            let last = grid
                .rows
                .iter()
                .rposition(|r| r.group == Some(id))
                .unwrap_or(first);
            mark(border(dom, id), 0, n, first, last + 1);
        }
        for col in &structure.column_boxes {
            super::count_column_scan();
            if col.start < col.end {
                mark(border(dom, col.id), col.start, col.end, 0, m);
            }
        }
        for cell in &grid.cells {
            let b = cell_border(dom, cell);
            mark(b, cell.column, cell.column_end(), cell.row, cell.row_end());
        }
    }

    /// Mark the sides of `b` that have a border on the lines `left`,
    /// `right` (vertical) and `top`, `bottom` (horizontal).
    fn mark(&mut self, b: Border, left: usize, right: usize, top: usize, bottom: usize) {
        let set = |list: &mut Vec<u16>, line: usize, side: crate::layout::BorderStyle| {
            if !side.is_none()
                && let Some(w) = list.get_mut(line)
            {
                *w = (*w).max(1);
            }
        };
        set(&mut self.vertical, left, b.left);
        set(&mut self.vertical, right, b.right);
        set(&mut self.horizontal, top, b.top);
        set(&mut self.horizontal, bottom, b.bottom);
    }

    /// The width of every vertical line together.
    pub(super) fn total_columns(&self, _grid: &Grid) -> u16 {
        self.vertical.iter().fold(0u16, |a, &w| a.saturating_add(w))
    }

    /// The grid's height: every horizontal line and the rows.
    pub(super) fn total_rows(&self, _grid: &Grid, rows: &[u16]) -> u16 {
        self.horizontal
            .iter()
            .chain(rows)
            .fold(0u16, |a, &w| a.saturating_add(w))
    }

    /// The lines strictly inside columns `from..to` (a spanning cell's).
    pub(super) fn inner_vertical(&self, from: usize, to: usize) -> u16 {
        self.vertical
            .get(from + 1..to)
            .map_or(0, |l| l.iter().fold(0u16, |a, &w| a.saturating_add(w)))
    }

    /// The lines strictly inside rows `from..to`.
    pub(super) fn inner_horizontal(&self, from: usize, to: usize) -> u16 {
        self.horizontal
            .get(from + 1..to)
            .map_or(0, |l| l.iter().fold(0u16, |a, &w| a.saturating_add(w)))
    }
}

/// A cell's border: a `table-cell` element's, none for an anonymous one.
pub(super) fn cell_border(dom: &Dom<TuiExt>, cell: &GridCell) -> Border {
    cell.element()
        .map_or_else(Border::none, |id| border(dom, id))
}

/// The lines on both sides of a collapsed track `k` become one, after it.
fn merge(lines: &mut [u16], k: usize) {
    if k + 1 < lines.len() {
        lines[k + 1] = lines[k + 1].max(lines[k]);
        lines[k] = 0;
    }
}
