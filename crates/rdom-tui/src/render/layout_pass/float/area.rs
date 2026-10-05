//! The exclusion area of one block formatting context (CSS 2.1 §9.5):
//! the margin boxes of the floats placed in it so far, the float
//! placement rules (§9.5.1), the space they leave a line or a block
//! (§9.5: line boxes beside a float are shortened, a block formatting
//! context root does not overlap one) and the clearance they ask for
//! (§9.5.2). Pure geometry, in layout coordinates (cells).

use crate::layout::FloatSide;

/// A placed float's margin box: `[left, right)` × `[top, bottom)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Exclusion {
    pub(crate) side: FloatSide,
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl Exclusion {
    /// Whether the margin box spans any row of `[y, y + rows)`.
    fn spans(&self, y: i32, rows: u16) -> bool {
        self.top < y + i32::from(rows.max(1)) && self.bottom > y
    }
}

/// The free inline space on some rows: `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Band {
    pub(crate) start: i32,
    pub(crate) end: i32,
}

impl Band {
    pub(crate) fn width(self) -> i32 {
        self.end - self.start
    }
}

/// The floats of one block formatting context, in placement order.
#[derive(Debug, Default, Clone)]
pub(crate) struct ExclusionArea {
    floats: Vec<Exclusion>,
}

impl ExclusionArea {
    /// No float placed yet.
    pub(crate) fn is_empty(&self) -> bool {
        self.floats.is_empty()
    }

    /// How many floats are placed.
    pub(crate) fn len(&self) -> usize {
        self.floats.len()
    }

    /// Forget the floats placed after the first `len`.
    pub(crate) fn truncate(&mut self, len: usize) {
        self.floats.truncate(len);
    }

    /// The part of `[x0, x1)` no float covers on any of the rows `[y, y +
    /// rows)` (one row when `rows` is 0): right of the left floats there,
    /// left of the right ones (§9.5).
    pub(crate) fn band(&self, x0: i32, x1: i32, y: i32, rows: u16) -> Band {
        let mut band = Band { start: x0, end: x1 };
        for f in self.floats.iter().filter(|f| f.spans(y, rows)) {
            match f.side {
                FloatSide::Left => band.start = band.start.max(f.right),
                FloatSide::Right => band.end = band.end.min(f.left),
            }
        }
        band.end = band.end.max(band.start);
        band
    }

    /// The first row below `y` where a float spanning `[y, y + rows)`
    /// ends — where the band over those rows next widens.
    pub(crate) fn next_bottom(&self, y: i32, rows: u16) -> Option<i32> {
        self.floats
            .iter()
            .filter(|f| f.spans(y, rows))
            .map(|f| f.bottom)
            .filter(|&b| b > y)
            .min()
    }

    /// The bottom outer edge of the lowest float on the sides asked for
    /// (§9.5.2: what an element with `clear` is placed below).
    pub(crate) fn clearance(&self, left: bool, right: bool) -> Option<i32> {
        self.floats
            .iter()
            .filter(|f| match f.side {
                FloatSide::Left => left,
                FloatSide::Right => right,
            })
            .map(|f| f.bottom)
            .max()
    }

    /// The bottom of the lowest float (CSS 2.1 §10.6.7: a block
    /// formatting context root's automatic height reaches it).
    pub(crate) fn lowest(&self) -> Option<i32> {
        self.floats.iter().map(|f| f.bottom).max()
    }

    /// Where a block-level box `width` × `rows` cells, whose top may not
    /// be above `y`, goes in `[x0, x1)` without overlapping a float
    /// (§9.5: "the border box of ... an element in the normal flow that
    /// establishes a new block formatting context ... must not overlap the
    /// margin box of any floats"): the first top at or below `y` whose
    /// band over its rows is `fits` — moving down past the floats in the
    /// way — and that band.
    pub(crate) fn opening(
        &self,
        x0: i32,
        x1: i32,
        y: i32,
        rows: u16,
        fits: impl Fn(Band) -> bool,
    ) -> (i32, Band) {
        let mut y = y;
        loop {
            let band = self.band(x0, x1, y, rows);
            let narrowed = band.start > x0 || band.end < x1;
            if !narrowed || fits(band) {
                return (y, band);
            }
            match self.next_bottom(y, rows) {
                Some(next) => y = next,
                None => return (y, band),
            }
        }
    }

    /// Move the last float's bottom edge down by `rows` (up when
    /// negative, never above its top): its box turned out taller or
    /// shorter than measured when it was placed.
    pub(crate) fn grow_last(&mut self, rows: i32) {
        if let Some(f) = self.floats.last_mut() {
            f.bottom = (f.bottom + rows).max(f.top);
        }
    }

    /// Place a float whose margin box is `width` × `rows` on `side` of
    /// the containing block `[x0, x1)`, its top not above `y` (CSS 2.1
    /// §9.5.1): not above an earlier float's top (rule 5), as high as it
    /// fits beside the earlier floats (rules 2, 3, 7, 8 — below them when
    /// it does not), and as far left (right) as it can (rules 1, 9).
    /// Returns its margin box.
    pub(crate) fn place(
        &mut self,
        side: FloatSide,
        width: u16,
        rows: u16,
        y: i32,
        x0: i32,
        x1: i32,
    ) -> Exclusion {
        let w = i32::from(width);
        let earliest = self.floats.last().map_or(y, |f| y.max(f.top));
        let (top, band) = self.opening(x0, x1, earliest, rows, |b| b.width() >= w);
        let left = match side {
            FloatSide::Left => band.start,
            FloatSide::Right => band.end - w,
        };
        let placed = Exclusion {
            side,
            left,
            top,
            right: left + w,
            bottom: top + i32::from(rows),
        };
        self.floats.push(placed);
        placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use FloatSide::{Left, Right};

    /// §9.5.1 rules 1, 2, 8 and 9: left floats line up from the left
    /// edge, right floats from the right, each as high as it fits.
    #[test]
    fn floats_line_up_from_their_edges() {
        let mut area = ExclusionArea::default();
        let a = area.place(Left, 3, 1, 0, 0, 10);
        let b = area.place(Left, 3, 1, 0, 0, 10);
        let c = area.place(Right, 2, 1, 0, 0, 10);
        assert_eq!((a.left, b.left, c.left), (0, 3, 8));
        assert_eq!(area.band(0, 10, 0, 1), Band { start: 6, end: 8 });
    }

    /// Rules 2 / 3 / 7: a float that does not fit beside the earlier ones
    /// goes below the first of them that ends; rule 5: never above an
    /// earlier float's top.
    #[test]
    fn a_float_that_does_not_fit_goes_below() {
        let mut area = ExclusionArea::default();
        area.place(Right, 6, 2, 0, 0, 10);
        area.place(Left, 3, 1, 0, 0, 10);
        let wide = area.place(Left, 5, 1, 0, 0, 10);
        assert_eq!((wide.left, wide.top), (0, 2));
        let next = area.place(Left, 1, 1, 0, 0, 10);
        assert_eq!(next.top, 2, "not above the earlier float's top");
    }

    /// §9.5.2: clearance is the bottom of the lowest float on the sides
    /// cleared; §10.6.7 the lowest of all.
    #[test]
    fn clearance_reads_the_sides_cleared() {
        let mut area = ExclusionArea::default();
        area.place(Left, 2, 3, 0, 0, 10);
        area.place(Right, 2, 5, 0, 0, 10);
        assert_eq!(area.clearance(true, false), Some(3));
        assert_eq!(area.clearance(false, true), Some(5));
        assert_eq!(area.clearance(false, false), None);
        assert_eq!(area.lowest(), Some(5));
    }

    /// §9.5: a block formatting context root moves down to where its
    /// rows are clear enough; with no float in the way, its own top.
    #[test]
    fn an_opening_moves_down_past_floats() {
        let mut area = ExclusionArea::default();
        area.place(Left, 6, 2, 0, 0, 10);
        let (y, band) = area.opening(0, 10, 0, 1, |b| b.width() >= 5);
        assert_eq!((y, band), (2, Band { start: 0, end: 10 }));
        let (y, band) = area.opening(0, 10, 0, 3, |b| b.width() >= 4);
        assert_eq!((y, band), (0, Band { start: 6, end: 10 }));
    }

    /// A zero-height float takes no row (it is placed, but excludes
    /// nothing).
    #[test]
    fn a_zero_height_float_excludes_nothing() {
        let mut area = ExclusionArea::default();
        area.place(Left, 4, 0, 0, 0, 10);
        assert_eq!(area.band(0, 10, 0, 1), Band { start: 0, end: 10 });
    }
}
