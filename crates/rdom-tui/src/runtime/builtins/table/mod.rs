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
//! This pass walks every `<table>` in the DOM and resolves each
//! column's *used* width — **the column's author width if one is
//! specified, else its content width** — recording it on each cell's
//! [`TuiExt::table_used_width`](crate::TuiExt::table_used_width), a
//! **layout output** field flex reads as the cell's main size. Every
//! cell in column N gets the same width, so columns line up.
//!
//! `TABLE-COLSYNC-1`: the used width is *never* written back to
//! `inline_style` — author width (input) and computed width (output)
//! stay separate. That's why an explicit width survives a re-size
//! (`Column.width` works), and why no `data-rdom-colsync` re-cascade
//! hack is needed: the value is read by full layout each frame, not by
//! the incremental cascade. (Full CSS table layout — `display:table`,
//! the auto min/max algorithm, spanning, CSS-rule widths — is the
//! `TABLE-TFC-1` roadmap item; this is the bounded, web-faithful fix.)
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
//! ## Deferred to the full table model (`TABLE-TFC-1`)
//!
//! - **Explicit width source.** Author widths are read from
//!   `inline_style.width` (set directly / via `set_width` / a
//!   `Column` width), NOT from a CSS rule (`td { width }`) — this pass
//!   runs before cascade. The TFC computes widths in-layout, post-cascade.
//! - `colspan` / `rowspan` spanning-cell width distribution.
//! - `<col>` / `<colgroup>` width hints.
//! - Percentage column widths + the auto min/max-content redistribution
//!   when an explicit table width conflicts with content.
//! - Content measurement still eyeballs text via `UnicodeWidthStr` on
//!   text descendants (nested-element / replaced-content widths ignored).

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

/// Resolve each column's *used* width for a single `<table>` and record it on
/// every cell's [`TuiExt::table_used_width`](crate::TuiExt::table_used_width)
/// (a **layout output** field). Flex reads it as the cell's main size, so all
/// cells in a column line up. No-op when the table has no rows.
///
/// Per column the used width is **the column's author width if one is
/// specified, else its content width** (`TABLE-COLSYNC-1`):
/// - **Author width (input):** a cell's `inline_style.width: Fixed(n)`
///   (set directly, or via `set_width` / a `Column` width). Respected and
///   never overwritten — the max specified across the column wins. *(Divergence:
///   a width from a CSS **rule** — `td { width }` — is not honored here; this
///   pass runs before cascade and reads only inline/author widths. Full CSS
///   table layout is `TABLE-TFC-1`.)*
/// - **Content (fallback):** the widest cell's text width + the UA cell
///   padding ([`CELL_H_PADDING`]).
///
/// Crucially this **does not touch `inline_style`** (so author intent and the
/// computed result never conflate, the dead-`Column.width` / `::after`-clip
/// bugs go away) and needs **no cascade dirty signal** — the value is read by
/// full layout each frame, not by the incremental cascade.
pub fn size_columns(dom: &mut TuiDom, table: NodeId) {
    let rows = collect_rows(dom, table);
    if rows.is_empty() {
        return;
    }

    // Pass 1: per column, the max author width (if any cell specifies one) and
    // the max content width. Author width is the INPUT, read from inline style.
    let mut explicit: Vec<Option<u16>> = Vec::new();
    let mut content: Vec<u16> = Vec::new();
    for &row_id in &rows {
        let cells = collect_cells(dom, row_id);
        for (i, &cell) in cells.iter().enumerate() {
            if i >= content.len() {
                explicit.push(None);
                content.push(0);
            }
            if let Some(w) = cell_author_width(dom, cell) {
                explicit[i] = Some(explicit[i].map_or(w, |e| e.max(w)));
            }
            let total = text_content_width(dom, cell).saturating_add(CELL_H_PADDING);
            content[i] = content[i].max(total);
        }
    }

    // Used width = author width if specified, else content width.
    let used: Vec<u16> = content
        .iter()
        .enumerate()
        .map(|(i, &c)| explicit[i].unwrap_or(c))
        .collect();

    // Pass 2: record the used width on every cell as a LAYOUT field (flex reads
    // it). Never `inline_style` — that's the conflation `TABLE-COLSYNC-1` fixes.
    for &row_id in &rows {
        let cells = collect_cells(dom, row_id);
        for (i, &cell) in cells.iter().enumerate() {
            let Some(&w) = used.get(i) else {
                continue;
            };
            if let Some(ext) = dom.node_mut(cell).ext_mut() {
                ext.table_used_width = Some(w);
            }
        }
    }
}

/// The cell's *author-specified* fixed width (`inline_style.width: Fixed(n)`),
/// or `None`. The column-sizing input — read, never written.
fn cell_author_width(dom: &TuiDom, cell: NodeId) -> Option<u16> {
    match dom.node(cell).ext()?.inline_style.width {
        Some(Value::Specified(Size::Fixed(w))) => Some(w),
        _ => None,
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
