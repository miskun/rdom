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
//! `<col span>` / `<colgroup span>`) by HTML's parsing rules;
//! [`cell_span_of`] and [`column_span_of`] read them off an element, HTML
//! element names compared ASCII case-insensitively.
//!
//! The renderer's table formatting context and the column combinator
//! (`||`, Selectors 4 §16.1) share this placement, so a cell is in the
//! same column for both.

use crate::dom::Dom;
use crate::node_id::NodeId;

/// HTML §4.9.11: the largest `colspan`, `<col span>` and `<colgroup span>`.
const MAX_COLSPAN: usize = 1000;
/// The widest table grid: 65 535 columns. HTML caps each span at 1000 but
/// not their sum; a terminal cell offset is a `u16`, so a column past this
/// could never show. The renderer's grid and the column model the column
/// combinator and `:nth-col()` match stop here alike — a cell or column
/// starting past it has none, one reaching past it is cut.
pub const MAX_COLUMNS: usize = u16::MAX as usize;
/// HTML §4.9.11: the largest `rowspan`.
const MAX_ROWSPAN: usize = 65534;

/// A cell's spans as authored: the columns it spans (at least one) and the
/// rows (`0` spans to the end of its row group, HTML §4.9.11). Always
/// within HTML's ranges: the only ways to make one clamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpan {
    columns: usize,
    rows: usize,
}

impl CellSpan {
    /// Columns spanned, `1..=1000`.
    pub fn columns(self) -> usize {
        self.columns
    }

    /// Rows spanned, `0..=65534`; `0` to the last row of the row group.
    pub fn rows(self) -> usize {
        self.rows
    }

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

/// Whether `id` is the HTML element `tag` — by local name, ASCII
/// case-insensitively (HTML §4.9: `TD` is a `td`). The one tag test every
/// reader of the table model shares.
pub(crate) fn is_html<Ext>(dom: &Dom<Ext>, id: NodeId, tags: &[&str]) -> bool {
    dom.node(id)
        .tag_name()
        .is_some_and(|t| tags.iter().any(|w| t.eq_ignore_ascii_case(w)))
}

/// The spans the cell `id` gives (HTML §4.9.11): a `<td>` / `<th>`'s
/// `colspan` and `rowspan` ([`CellSpan::from_attributes`]); one slot for
/// any other element — CSS has no span property. Shared by the column
/// model and every renderer's table layout, so a cell is placed alike.
pub fn cell_span_of<Ext>(dom: &Dom<Ext>, id: NodeId) -> CellSpan {
    if is_html(dom, id, &["td", "th"]) {
        let node = dom.node(id);
        CellSpan::from_attributes(node.get_attribute("colspan"), node.get_attribute("rowspan"))
    } else {
        CellSpan::new(1, 1)
    }
}

/// The columns the column or column-group element `id` gives (HTML
/// §4.9.3, §4.9.4): a `<col>` / `<colgroup>`'s `span` ([`column_span`]);
/// one for any other element.
pub fn column_span_of<Ext>(dom: &Dom<Ext>, id: NodeId) -> usize {
    if is_html(dom, id, &["col", "colgroup"]) {
        column_span(dom.node(id).get_attribute("span"))
    } else {
        1
    }
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
        // The slots covered by a cell above (rowspans never leave the group).
        let mut covered = Coverage::default();
        for (r, row) in group.iter().enumerate() {
            let y = first_row + r;
            let mut x = 0usize;
            let mut placed = Vec::with_capacity(row.len());
            for cell in row {
                x = covered.next_free(x, y);
                let rows = match cell.rows {
                    0 => end - y,
                    n => n.min(end - y),
                };
                covered.cover(x, cell.columns, y + rows);
                placed.push(Slot {
                    row: y,
                    column: x,
                    columns: cell.columns,
                    rows,
                });
                x += cell.columns;
            }
            out.columns = out.columns.max(x).max(covered.width);
            out.cells.push(placed);
        }
        out.rows = end;
    }
    out
}

/// The slots of a row group covered from above, as runs of columns that
/// stay covered until the same row: `start → (end, until)`, disjoint,
/// adjacent runs with the same `until` merged. A row's cells skip a run
/// in one step whatever its width, and a run expired at a row stays
/// expired for the rows below (they only come later), so it is dropped
/// when met — the skip costs O(runs met), not O(columns).
#[derive(Debug, Default)]
struct Coverage {
    runs: std::collections::BTreeMap<usize, (usize, usize)>,
    /// One past the last column ever covered.
    width: usize,
}

impl Coverage {
    /// The first column at or after `x` not covered at row `y`.
    fn next_free(&mut self, mut x: usize, y: usize) -> usize {
        while let Some((&start, &(end, until))) = self.runs.range(..=x).next_back() {
            #[cfg(test)]
            probe::step();
            if end <= x {
                return x;
            }
            if until <= y {
                // Expired: no later row is covered by it either.
                self.runs.remove(&start);
                return x;
            }
            x = end;
        }
        x
    }

    /// Cover `columns` columns from `x` until row `until` (each slot the
    /// later of its cover and this one).
    fn cover(&mut self, x: usize, columns: usize, until: usize) {
        let end = x + columns;
        self.width = self.width.max(end);
        // The runs meeting `x..end`, cut out; the parts outside kept.
        let mut pieces: Vec<(usize, usize, usize)> = Vec::new();
        let first = self.runs.range(..x).next_back().map(|(&s, _)| s);
        let starts: Vec<usize> = first
            .into_iter()
            .chain(self.runs.range(x..end).map(|(&s, _)| s))
            .collect();
        for start in starts {
            let (run_end, run_until) = self.runs[&start];
            if run_end <= x {
                continue;
            }
            self.runs.remove(&start);
            if start < x {
                self.runs.insert(start, (x, run_until));
            }
            if run_end > end {
                self.runs.insert(end, (run_end, run_until));
            }
            pieces.push((start.max(x), run_end.min(end), run_until));
        }
        // `x..end` at `until`, and later where an old run outlasts it.
        let mut at = x;
        for (s, e, u) in pieces {
            if s > at {
                self.runs.insert(at, (s, until));
            }
            self.runs.insert(s, (e, u.max(until)));
            at = e;
        }
        if at < end {
            self.runs.insert(at, (end, until));
        }
        self.merge_around(x, end);
    }

    /// Merge the runs from the one before `x` to the one at `end` with
    /// their neighbours of the same `until`.
    fn merge_around(&mut self, x: usize, end: usize) {
        let from = self.runs.range(..x).next_back().map_or(x, |(&s, _)| s);
        let mut cur = self.runs.range(from..).next().map(|(&s, _)| s);
        while let Some(start) = cur {
            let (run_end, until) = self.runs[&start];
            match self.runs.get(&run_end).copied() {
                Some((next_end, next_until)) if next_until == until => {
                    self.runs.remove(&run_end);
                    self.runs.insert(start, (next_end, until));
                }
                _ => {
                    if start > end {
                        break;
                    }
                    cur = self.runs.range(start + 1..).next().map(|(&s, _)| s);
                }
            }
        }
    }
}

pub(crate) mod html;
#[cfg(test)]
mod tests;

/// Test-only: the steps slot assignment took skipping covered slots.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        static STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    pub(crate) fn step() {
        STEPS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn take() -> u64 {
        STEPS.with(|c| c.replace(0))
    }
}
