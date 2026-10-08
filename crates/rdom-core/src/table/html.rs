//! HTML's table model (HTML §4.9.12.1 "forming a table") read from the
//! DOM alone: which columns each `<td>` / `<th>` of a `<table>` spans and
//! which columns each `<col>` / `<colgroup>` represents — the column model
//! the column combinator and `:nth-col()` match against (Selectors 4 §16,
//! "the semantics of the document language").
//!
//! As HTML forms it: the `<colgroup>` children before the first row group
//! or row make the columns (a `<col>` its `span` of them, a `<colgroup>`
//! with no `<col>` its own `span`; a `<col>` outside a `<colgroup>` is no
//! column, and a later `<colgroup>` neither); the rows are the `<tr>`
//! children of the table — consecutive ones a group — and of its `<thead>`
//! / `<tbody>` in tree order, then of its `<tfoot>`s; the cells are each
//! row's `<td>` / `<th>` children, placed by [`assign_slots`].

use std::collections::HashMap;

use super::{CellSpan, assign_slots, column_span};
use crate::dom::Dom;
use crate::node_id::NodeId;

/// A `<table>`'s columns and cells.
#[derive(Debug, Default)]
pub(crate) struct HtmlTableModel {
    /// The table's width in columns.
    pub(crate) width: usize,
    /// Each cell's first column and the columns it spans.
    cells: HashMap<NodeId, (usize, usize)>,
    /// Each `<col>` / `<colgroup>` with the columns it represents, in
    /// tree order.
    columns: Vec<(NodeId, usize, usize)>,
}

impl HtmlTableModel {
    /// The columns `cell` spans: its first and their count.
    pub(crate) fn cell(&self, cell: NodeId) -> Option<(usize, usize)> {
        self.cells.get(&cell).copied()
    }

    /// The `<col>` / `<colgroup>` elements representing a column that
    /// `cell` spans.
    pub(crate) fn column_elements(&self, cell: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let (first, span) = self.cell(cell).unwrap_or((0, 0));
        self.columns
            .iter()
            .filter(move |&&(_, start, len)| {
                span > 0 && start < first + span && first < start + len
            })
            .map(|&(id, _, _)| id)
    }
}

impl<Ext> Dom<Ext> {
    /// Whether `id` is an element named one of `tags` (HTML's table
    /// elements, by local name, ASCII case-insensitively).
    fn tag_in(&self, id: NodeId, tags: &[&str]) -> bool {
        self.node(id)
            .tag_name()
            .is_some_and(|t| tags.iter().any(|w| t.eq_ignore_ascii_case(w)))
    }

    fn element_child_ids(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut cur = self.first_element_child_id(id);
        while let Some(c) = cur {
            out.push(c);
            cur = self.next_element_sibling_id(c);
        }
        out
    }

    /// The `<table>` whose model holds the cell `id` (a `<td>` / `<th>`
    /// child of a `<tr>` that is a child of the table or of its
    /// `<thead>` / `<tbody>` / `<tfoot>`); `None` for any other element.
    pub(crate) fn html_table_of_cell(&self, id: NodeId) -> Option<NodeId> {
        if !self.tag_in(id, &["td", "th"]) {
            return None;
        }
        let tr = self.node(id).parent_node()?.id();
        if !self.tag_in(tr, &["tr"]) {
            return None;
        }
        let up = self.node(tr).parent_node()?.id();
        let table = if self.tag_in(up, &["thead", "tbody", "tfoot"]) {
            self.node(up).parent_node()?.id()
        } else {
            up
        };
        self.tag_in(table, &["table"]).then_some(table)
    }

    /// Form the table model of the `<table>` element `table`.
    pub(crate) fn html_table_model(&self, table: NodeId) -> HtmlTableModel {
        let children = self.element_child_ids(table);
        let mut model = HtmlTableModel::default();
        // Column groups: the `<colgroup>`s before the first row group or
        // row.
        let mut x = 0usize;
        for &c in &children {
            if self.tag_in(c, &["thead", "tbody", "tfoot", "tr"]) {
                break;
            }
            if !self.tag_in(c, &["colgroup"]) {
                continue;
            }
            let start = x;
            let cols: Vec<NodeId> = self
                .element_child_ids(c)
                .into_iter()
                .filter(|&k| self.tag_in(k, &["col"]))
                .collect();
            if cols.is_empty() {
                x += column_span(self.node(c).get_attribute("span"));
            }
            for col in cols {
                let span = column_span(self.node(col).get_attribute("span"));
                model.columns.push((col, x, span));
                x += span;
            }
            model.columns.push((c, start, x - start));
        }
        // Rows: loose `<tr>`s and `<thead>` / `<tbody>` in tree order, the
        // `<tfoot>`s last.
        let mut groups: Vec<Vec<NodeId>> = Vec::new();
        let mut loose: Vec<NodeId> = Vec::new();
        let mut feet: Vec<NodeId> = Vec::new();
        let rows_of = |g: NodeId| -> Vec<NodeId> {
            self.element_child_ids(g)
                .into_iter()
                .filter(|&r| self.tag_in(r, &["tr"]))
                .collect()
        };
        for &c in &children {
            if self.tag_in(c, &["tr"]) {
                loose.push(c);
                continue;
            }
            if self.tag_in(c, &["thead", "tbody", "tfoot"]) && !loose.is_empty() {
                groups.push(std::mem::take(&mut loose));
            }
            if self.tag_in(c, &["thead", "tbody"]) {
                groups.push(rows_of(c));
            } else if self.tag_in(c, &["tfoot"]) {
                feet.push(c);
            }
        }
        if !loose.is_empty() {
            groups.push(loose);
        }
        groups.extend(feet.into_iter().map(rows_of));
        let cells: Vec<Vec<Vec<NodeId>>> = groups
            .iter()
            .map(|g| {
                g.iter()
                    .map(|&r| {
                        self.element_child_ids(r)
                            .into_iter()
                            .filter(|&k| self.tag_in(k, &["td", "th"]))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let spans: Vec<Vec<Vec<CellSpan>>> = cells
            .iter()
            .map(|g| {
                g.iter()
                    .map(|r| {
                        r.iter()
                            .map(|&k| {
                                let n = self.node(k);
                                CellSpan::from_attributes(
                                    n.get_attribute("colspan"),
                                    n.get_attribute("rowspan"),
                                )
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let slots = assign_slots(&spans);
        for (row, placed) in cells.iter().flatten().zip(&slots.cells) {
            for (&cell, slot) in row.iter().zip(placed) {
                model.cells.insert(cell, (slot.column, slot.columns));
            }
        }
        model.width = slots.columns.max(x);
        model
    }
}
