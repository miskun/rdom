//! [`GridTracks`]: a laid-out grid's used tracks, read through
//! [`TuiAccessors::grid_tracks`](super::TuiAccessors::grid_tracks).

use std::ops::Range;

/// A grid container's used tracks after layout — rdom's analogue of the
/// resolved value of `grid-template-columns` / `grid-template-rows`
/// (CSS Grid 2 §7.2.6: the used track sizes, implicit tracks included),
/// in cells, for a consumer that draws headers or rules aligned to the
/// grid.
///
/// Each track is the cells it covers, a half-open range measured from
/// the container's content box — its left edge for the columns, its top
/// edge for the rows — before the container's own scroll offset (as
/// [`TuiNodeExt::content_layout_rect`](crate::TuiNodeExt::content_layout_rect)
/// is). Tracks are in grid order, the first line's first: an `rtl`
/// grid's first column is its rightmost range. A gutter is the gap
/// between two ranges; a collapsed `auto-fit` track is an empty one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridTracks {
    columns: Vec<Range<i32>>,
    rows: Vec<Range<i32>>,
}

impl GridTracks {
    /// The tracks `columns` and `rows`, each in grid order.
    pub fn new(columns: Vec<Range<i32>>, rows: Vec<Range<i32>>) -> Self {
        Self { columns, rows }
    }

    /// The columns, in grid order.
    pub fn columns(&self) -> &[Range<i32>] {
        &self.columns
    }

    /// The rows, in grid order.
    pub fn rows(&self) -> &[Range<i32>] {
        &self.rows
    }
}
