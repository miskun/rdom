//! Grid item placement (CSS Grid 2 §8.5): which grid area each item
//! occupies, and how many tracks the implicit grid has on each axis.
//!
//! With no placement properties yet (C7-GRID-PLACE), every item is
//! auto-placed with a span of one in `grid-auto-flow: row` order (§8.5
//! step 4, sparse): each item takes the next cell of the current row,
//! the next row starting when the row is full; the rows past the
//! explicit grid are implicit (§7.5), sized by `grid-auto-rows` (§7.6).

use super::template::MAX_TRACKS;
use super::track::Span;
use crate::layout::Sides;
use crate::render::layout_pass::items::Item;

/// An item and its grid area.
#[derive(Debug, Clone)]
pub(super) struct Placed {
    pub(super) item: Item,
    pub(super) columns: Span,
    pub(super) rows: Span,
    /// Its physical margins `margin-trim` drops (`grid::trim`), set once
    /// the grid's size is known.
    pub(super) trim: Sides<bool>,
}

/// The items placed, and the implicit grid's size: how many tracks it
/// has on each axis, and how many of them come before the explicit grid
/// (§7.5 — the explicit grid's first line is track `before`'s start).
#[derive(Debug, Clone)]
pub(super) struct Placement {
    pub(super) items: Vec<Placed>,
    pub(super) columns: usize,
    pub(super) rows: usize,
    pub(super) columns_before: usize,
    pub(super) rows_before: usize,
}

/// Place `items` (in order-modified document order) in a grid of
/// `explicit_columns` × `explicit_rows` explicit tracks. With no explicit
/// column, the implicit grid has one (§8.5 step 3: as many as the widest
/// item spans).
pub(super) fn place(items: Vec<Item>, explicit_columns: usize, explicit_rows: usize) -> Placement {
    let columns = explicit_columns.clamp(1, MAX_TRACKS);
    let mut placed = Vec::with_capacity(items.len());
    for (k, item) in items.into_iter().enumerate() {
        // Past the clamped grid the last row takes every item.
        let row = (k / columns).min(MAX_TRACKS - 1);
        let column = k % columns;
        placed.push(Placed {
            item,
            columns: Span::new(column, column + 1),
            rows: Span::new(row, row + 1),
            trim: Sides::default(),
        });
    }
    let rows = placed
        .last()
        .map_or(0, |p| p.rows.end)
        .max(explicit_rows)
        .min(MAX_TRACKS);
    Placement {
        items: placed,
        columns,
        rows,
        columns_before: 0,
        rows_before: 0,
    }
}
