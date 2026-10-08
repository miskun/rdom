//! The table grid (CSS 2.1 §17.5, CSS Tables 3 §3.3): each cell in its
//! slots, placed by `rdom_core::table::assign_slots` — the HTML table
//! model's row processing, which the column combinator shares — and the
//! rows and columns that `visibility: collapse` removes (§17.5.5).

use rdom_core::table::{CellSpan, assign_slots};
use rdom_core::{Dom, NodeId};

use super::structure::{Cell, Structure};
use crate::ext::TuiExt;
use crate::layout::Visibility;
use crate::node::TuiNodeExt;

/// A table's grid.
#[derive(Debug)]
pub(super) struct Grid {
    /// Columns: the widest row's slots, or the column boxes' count.
    pub(super) columns: usize,
    /// Every row, in display order.
    pub(super) rows: Vec<GridRow>,
    /// Every cell placed.
    pub(super) cells: Vec<GridCell>,
    /// By column: removed by `visibility: collapse` on its column or
    /// column group box (§17.5.5).
    pub(super) collapsed_columns: Vec<bool>,
}

/// One row of the grid.
#[derive(Debug, Clone, Copy)]
pub(super) struct GridRow {
    pub(super) element: Option<NodeId>,
    /// Its row group's box.
    pub(super) group: Option<NodeId>,
    /// `visibility: collapse` on the row (or its group): no height.
    pub(super) collapsed: bool,
}

/// A cell in its slots: rows `row..row + rows`, columns `column..column
/// + columns`.
#[derive(Debug, Clone)]
pub(super) struct GridCell {
    pub(super) cell: Cell,
    pub(super) row: usize,
    pub(super) column: usize,
    pub(super) rows: usize,
    pub(super) columns: usize,
}

impl GridCell {
    /// The element, for a `table-cell` element.
    pub(super) fn element(&self) -> Option<NodeId> {
        match &self.cell {
            Cell::Element(id) => Some(*id),
            Cell::Anonymous(_) => None,
        }
    }

    /// The column after its last.
    pub(super) fn column_end(&self) -> usize {
        self.column + self.columns
    }

    /// The row after its last.
    pub(super) fn row_end(&self) -> usize {
        self.row + self.rows
    }
}

/// A cell's spans: a `<td>` / `<th>`'s `colspan` and `rowspan` (HTML
/// §4.9.11); one slot for any other cell — CSS has no span property.
fn span_of(dom: &Dom<TuiExt>, cell: &Cell) -> CellSpan {
    match cell {
        Cell::Element(id) => {
            let node = dom.node(*id);
            if matches!(node.tag_name(), Some("td" | "th")) {
                CellSpan::from_attributes(
                    node.get_attribute("colspan"),
                    node.get_attribute("rowspan"),
                )
            } else {
                CellSpan::new(1, 1)
            }
        }
        Cell::Anonymous(_) => CellSpan::new(1, 1),
    }
}

fn collapsed(dom: &Dom<TuiExt>, id: Option<NodeId>) -> bool {
    id.and_then(|id| dom.node(id).computed().map(|c| c.visibility)) == Some(Visibility::Collapse)
}

impl Grid {
    /// The grid of `structure`.
    pub(super) fn of(dom: &Dom<TuiExt>, structure: &Structure) -> Self {
        let spans: Vec<Vec<Vec<CellSpan>>> = structure
            .groups
            .iter()
            .map(|g| {
                g.rows
                    .iter()
                    .map(|r| r.cells.iter().map(|c| span_of(dom, c)).collect())
                    .collect()
            })
            .collect();
        let slots = assign_slots(&spans);
        let mut rows = Vec::with_capacity(slots.rows);
        let mut cells = Vec::new();
        let mut placed = slots.cells.into_iter();
        for group in &structure.groups {
            let group_collapsed = collapsed(dom, group.element);
            for row in &group.rows {
                rows.push(GridRow {
                    element: row.element,
                    group: group.element,
                    collapsed: group_collapsed || collapsed(dom, row.element),
                });
                let row_slots = placed.next().unwrap_or_default();
                for (cell, slot) in row.cells.iter().zip(row_slots) {
                    cells.push(GridCell {
                        cell: cell.clone(),
                        row: slot.row,
                        column: slot.column,
                        rows: slot.rows,
                        columns: slot.columns,
                    });
                }
            }
        }
        let columns = slots.columns.max(structure.columns.len());
        let collapsed_columns = (0..columns)
            .map(|c| {
                structure
                    .columns
                    .get(c)
                    .is_some_and(|s| collapsed(dom, s.column) || collapsed(dom, s.group))
            })
            .collect();
        Grid {
            columns,
            rows,
            cells,
            collapsed_columns,
        }
    }
}
