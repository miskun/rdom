//! The table grid: which slots each cell of a table covers (HTML
//! §4.9.12.1 "forming a table"; CSS Tables 3 §3.3, which takes the same
//! algorithm for tables built from `display` values).
//!
//! Renderer-free: a cell is its two spans, a row a list of cells, a row
//! group a list of rows. [`assign_slots`] places every cell in the first
//! slot of its row that no cell covers — the slots under an earlier row's
//! `rowspan` are covered — and grows the grid to the widest row. A
//! `rowspan` never reaches past its row group's last row (CSS Tables 3
//! §3.3; HTML adds empty rows instead, which no browser renders), and
//! `rowspan="0"` spans to that row. [`CellSpan::from_attributes`] and
//! [`column_span`] read the HTML attributes (`colspan` / `rowspan`,
//! `<col span>` / `<colgroup span>`) by HTML's parsing rules.
//!
//! The renderer's table formatting context and the column combinator
//! (`||`, Selectors 4 §16.1) share this placement, so a cell is in the
//! same column for both.

/// HTML §4.9.11: the largest `colspan`, `<col span>` and `<colgroup span>`.
const MAX_COLSPAN: usize = 1000;
/// HTML §4.9.11: the largest `rowspan`.
const MAX_ROWSPAN: usize = 65534;

/// A cell's spans as authored: the columns it spans (at least one) and the
/// rows (`0` spans to the end of its row group, HTML §4.9.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpan {
    /// Columns spanned, `1..=1000`.
    pub columns: usize,
    /// Rows spanned, `0..=65534`; `0` to the last row of the row group.
    pub rows: usize,
}

impl CellSpan {
    /// Spans of `columns` × `rows`, clamped to HTML's ranges (`columns` at
    /// least one).
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns: columns.clamp(1, MAX_COLSPAN),
            rows: rows.min(MAX_ROWSPAN),
        }
    }

    /// The spans a `<td>` / `<th>`'s `colspan` and `rowspan` attributes
    /// give (HTML §4.9.11): each parsed by the rules for parsing
    /// non-negative integers (§2.3.4.2); a missing, invalid or zero
    /// `colspan` is 1, a missing or invalid `rowspan` 1.
    pub fn from_attributes(colspan: Option<&str>, rowspan: Option<&str>) -> Self {
        let columns = colspan
            .and_then(parse_non_negative)
            .filter(|&n| n > 0)
            .unwrap_or(1);
        let rows = rowspan.and_then(parse_non_negative).unwrap_or(1);
        Self::new(columns, rows)
    }
}

/// The columns a `<col>` or `<colgroup>`'s `span` attribute gives (HTML
/// §4.9.3, §4.9.4): a valid non-negative integer greater than zero,
/// clamped to 1000; missing, invalid or zero is 1.
pub fn column_span(span: Option<&str>) -> usize {
    span.and_then(parse_non_negative)
        .filter(|&n| n > 0)
        .unwrap_or(1)
        .min(MAX_COLSPAN)
}

/// HTML §2.3.4.2, the rules for parsing non-negative integers: leading
/// ASCII white space and a `+` skipped, then the digits up to the first
/// non-digit (`"4px"` is 4); `None` with no digit or a `-`. Saturates at
/// `usize::MAX` (every caller clamps far below it).
fn parse_non_negative(s: &str) -> Option<usize> {
    let s = s.trim_start_matches(['\t', '\n', '\u{c}', '\r', ' ']);
    let s = s.strip_prefix('+').unwrap_or(s);
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    Some(s[..digits].bytes().fold(0usize, |n, d| {
        n.saturating_mul(10).saturating_add(usize::from(d - b'0'))
    }))
}

/// One cell placed in the grid: its top-left slot and the slots it covers,
/// its row span clamped to its row group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The row of its top-left slot, counted over every row group.
    pub row: usize,
    /// The column of its top-left slot.
    pub column: usize,
    /// Columns covered (its `colspan`).
    pub columns: usize,
    /// Rows covered: at least one, never past its row group's last row.
    pub rows: usize,
}

/// A table's grid: its size and each cell's [`Slot`], row by row in the
/// order given (`cells[r][i]` is row `r`'s `i`th cell).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableSlots {
    /// The grid's width: the widest row, spans included.
    pub columns: usize,
    /// The rows, every row group's.
    pub rows: usize,
    /// Each row's cells placed, in order.
    pub cells: Vec<Vec<Slot>>,
}

/// Place the cells of `groups` — row groups in order, each a list of rows,
/// each a list of cell spans — in the table grid (HTML §4.9.12.1, the
/// algorithm for processing rows; CSS Tables 3 §3.3).
pub fn assign_slots(groups: &[Vec<Vec<CellSpan>>]) -> TableSlots {
    let mut out = TableSlots::default();
    for group in groups {
        let first_row = out.rows;
        let end = first_row + group.len();
        // `covered_until[x]`: the first row at which column `x` is no
        // longer covered by a cell above (rowspans never leave the group).
        let mut covered_until: Vec<usize> = Vec::new();
        for (r, row) in group.iter().enumerate() {
            let y = first_row + r;
            let mut x = 0usize;
            let mut placed = Vec::with_capacity(row.len());
            for cell in row {
                while covered_until.get(x).is_some_and(|&until| until > y) {
                    x += 1;
                }
                let rows = match cell.rows {
                    0 => end - y,
                    n => n.min(end - y),
                };
                if covered_until.len() < x + cell.columns {
                    covered_until.resize(x + cell.columns, 0);
                }
                for until in &mut covered_until[x..x + cell.columns] {
                    *until = (*until).max(y + rows);
                }
                placed.push(Slot {
                    row: y,
                    column: x,
                    columns: cell.columns,
                    rows,
                });
                x += cell.columns;
            }
            out.columns = out.columns.max(x).max(covered_until.len());
            out.cells.push(placed);
        }
        out.rows = end;
    }
    out
}

pub(crate) mod html;
#[cfg(test)]
mod tests;
