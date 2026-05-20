//! `<table>` column-width synchronization pass.
//!
//! ## Why this exists
//!
//! The C.8a UA rules give `<td>` / `<th>` a default `width: Auto`,
//! which makes each cell size to its own content. Inside a single
//! `<tr>` (our flex-direction:Row container) cells sit next to
//! each other at content-driven widths, but DIFFERENT rows would
//! land on different widths — so columns don't line up.
//!
//! This pre-pass walks every `<table>` in the DOM, computes the
//! max content width per column index across all rows, and writes
//! those widths as inline style on every cell. After cascade +
//! layout, every cell in column N has the same width, so columns
//! align the way HTML table readers expect.
//!
//! ## Content measurement
//!
//! v1 measures display width via `UnicodeWidthStr` on the
//! concatenated text descendants. Nested element widths are NOT
//! accounted for — a cell containing `<b>bold</b>` reports the
//! bold text's width; a cell with a `<progress>` bar reports 0
//! (no text descendants). Authors who need richer measurement
//! override with explicit `Fixed` widths via author CSS.
//!
//! Padding budget: the UA `<td>` / `<th>` rule uses `padding:
//! 0 1 0 1` (2 horizontal cells). The pre-pass adds that 2 to the
//! content width. Authors who override padding have to override
//! the width too.
//!
//! ## When it runs
//!
//! [`size_all_tables`] is called once from `App::build` after the
//! other builtin installs. Apps that mutate table content at
//! runtime can call it themselves to re-sync.
//!
//! ## Not done in v1
//!
//! - `colspan` / `rowspan` — a spanning cell contributes its
//!   total width / height across N columns / rows, which requires
//!   a real table layout algorithm. Polish item.
//! - `<col>` / `<colgroup>` width hints via attributes — would
//!   need to read `width` / `span` attrs off `<col>` elements
//!   and apply as initial column-width constraints. Polish.
//! - Post-cascade measurement — we eyeball text widths with
//!   `UnicodeWidthStr` rather than running the full intrinsic-
//!   size machinery. Good enough for most text tables.

use rdom_core::{NodeId, NodeType};
use unicode_width::UnicodeWidthStr;

use crate::TuiDom;
use crate::layout::Size;
use crate::style::Value;

/// Horizontal padding implied by the UA `<td>` / `<th>` rule
/// (`padding: 0 1 0 1`). The pre-pass adds this to measured
/// content widths.
const CELL_H_PADDING: u16 = 2;

/// Walk the whole DOM; size columns on every `<table>` found.
pub fn size_all_tables(dom: &mut TuiDom) {
    let tables = collect_tables(dom, dom.root());
    for table in tables {
        size_columns(dom, table);
    }
}

/// Compute per-column max content widths for a single `<table>`
/// and write them as inline-style `width: Fixed(…)` on every
/// cell. No-op when the table has no rows.
pub fn size_columns(dom: &mut TuiDom, table: NodeId) {
    let rows = collect_rows(dom, table);
    if rows.is_empty() {
        return;
    }

    // First pass: max content width per column across all rows.
    let mut col_widths: Vec<u16> = Vec::new();
    for &row_id in &rows {
        let cells = collect_cells(dom, row_id);
        for (i, &cell) in cells.iter().enumerate() {
            let content = text_content_width(dom, cell);
            let total = content.saturating_add(CELL_H_PADDING);
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(total);
            } else {
                col_widths.push(total);
            }
        }
    }

    // Second pass: write the computed widths onto every cell.
    for &row_id in &rows {
        let cells = collect_cells(dom, row_id);
        for (i, &cell) in cells.iter().enumerate() {
            let Some(&w) = col_widths.get(i) else {
                continue;
            };
            if let Some(ext) = dom.node_mut(cell).ext_mut() {
                ext.inline_style.width = Some(Value::Specified(Size::Fixed(w)));
            }
        }
    }
}

// ── Tree traversal helpers ─────────────────────────────────────────

fn collect_tables(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_tables(dom, root, &mut out);
    out
}

fn walk_tables(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some("table") {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_tables(dom, child.id(), out);
    }
}

/// Collect every `<tr>` under a `<table>`, descending through the
/// optional `<thead>` / `<tbody>` / `<tfoot>` row groups.
fn collect_rows(dom: &TuiDom, table: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    for child in dom.node(table).child_nodes() {
        match child.tag_name() {
            Some("tr") => out.push(child.id()),
            Some("thead") | Some("tbody") | Some("tfoot") => {
                for grand in child.child_nodes() {
                    if grand.tag_name() == Some("tr") {
                        out.push(grand.id());
                    }
                }
            }
            // `<caption>`, `<colgroup>` etc. — skip.
            _ => {}
        }
    }
    out
}

fn collect_cells(dom: &TuiDom, row: NodeId) -> Vec<NodeId> {
    dom.node(row)
        .child_nodes()
        .filter(|c| matches!(c.tag_name(), Some("td") | Some("th")))
        .map(|c| c.id())
        .collect()
}

/// Display width of the cell's concatenated text descendants —
/// walks text nodes recursively. Wide CJK glyphs contribute 2
/// cells each via `UnicodeWidthStr`. Empty cells measure 0.
fn text_content_width(dom: &TuiDom, cell: NodeId) -> u16 {
    let mut text = String::new();
    collect_text(dom, cell, &mut text);
    UnicodeWidthStr::width(text.as_str()) as u16
}

fn collect_text(dom: &TuiDom, id: NodeId, out: &mut String) {
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text {
            if let Some(s) = child.node_value() {
                out.push_str(s);
            }
        } else {
            collect_text(dom, child.id(), out);
        }
    }
}

#[cfg(test)]
mod tests;
