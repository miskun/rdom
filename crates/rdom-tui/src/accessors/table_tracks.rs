//! [`TableTracks`]: a laid-out table's used columns and rows, read
//! through [`TuiAccessors::table_tracks`](super::TuiAccessors::table_tracks).

use std::ops::Range;

/// A table's used columns and rows after layout (CSS 2.1 §17.5), in
/// cells — the table's analogue of [`GridTracks`](super::GridTracks), for
/// a consumer that draws a header, a rule or a resize handle aligned to
/// the columns, or sizes a virtualized window by them.
///
/// Each track is the cells between its two lines, a half-open range
/// measured from the table box's content edge — its left edge for the
/// columns, its top edge for the rows — before the table's own scroll
/// offset. The table box is the one its border is on: captions are
/// outside it (§17.4), so the first row starts below a top caption. The
/// lines are outside the ranges: in the separated model (§17.6.1)
/// `border-spacing` is the gap between two ranges and before the first,
/// and the table's border and padding are outside the content edge — a
/// column is exactly the border box of a cell that spans only it; in the
/// collapsing model (§17.6.2) the table has no padding and its border is
/// the grid's outer line, so the content edge is its border edge, and
/// each one-cell border line is the gap (a bordered cell's border box
/// covers the lines on its sides). Tracks are in column and row order:
/// an `rtl` table's first column is its rightmost range, as an `rtl`
/// grid's. A `visibility: collapse` column or row is an empty range
/// where it would be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableTracks {
    columns: Vec<Range<i32>>,
    rows: Vec<Range<i32>>,
}

impl TableTracks {
    /// The tracks `columns` and `rows`, each in column / row order.
    pub fn new(columns: Vec<Range<i32>>, rows: Vec<Range<i32>>) -> Self {
        Self { columns, rows }
    }

    /// The columns, in column order (the first column first).
    pub fn columns(&self) -> &[Range<i32>] {
        &self.columns
    }

    /// The rows, in row order (a `<tfoot>`'s last, as laid out).
    pub fn rows(&self) -> &[Range<i32>] {
        &self.rows
    }
}
