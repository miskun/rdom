//! Frame diffing: the row-major iterator over cells that changed
//! since the previous frame, honoring spacers and `CellDiff` flags.

use super::Buffer;
use crate::render::{Cell, CellDiff, Rect};

impl Buffer {
    // ── Diff ──────────────────────────────────────────────────────────

    /// Iterate cells that differ from `previous`, in row-major order.
    /// Skips trailing spacer cells of wide glyphs — only primary cells
    /// are yielded. Cells with `diff == Skip` are never yielded; cells
    /// with `diff == AlwaysUpdate` are yielded even when equal.
    ///
    /// The areas of `self` and `previous` **must match**. Panics
    /// otherwise — typically caller would `resize` one to match the
    /// other first.
    pub fn diff_iter<'a>(
        &'a self,
        previous: &'a Buffer,
    ) -> impl Iterator<Item = (u16, u16, &'a Cell)> + 'a {
        assert_eq!(
            self.area, previous.area,
            "Buffer::diff_iter: area mismatch (self {:?} vs previous {:?})",
            self.area, previous.area
        );

        let area = self.area;
        self.content
            .iter()
            .zip(previous.content.iter())
            .enumerate()
            .filter_map(move |(i, (new, old))| {
                if new.is_spacer() {
                    return None;
                }
                match new.diff {
                    CellDiff::Skip => None,
                    CellDiff::AlwaysUpdate => {
                        let (x, y) = Self::xy_at(area, i);
                        Some((x, y, new))
                    }
                    CellDiff::Normal => {
                        if new == old {
                            None
                        } else {
                            let (x, y) = Self::xy_at(area, i);
                            Some((x, y, new))
                        }
                    }
                }
            })
    }

    /// Inverse of `index_of`: flat index → (x, y).
    fn xy_at(area: Rect, i: usize) -> (u16, u16) {
        let w = area.width as usize;
        let dy = (i / w) as u16;
        let dx = (i % w) as u16;
        (area.x + dx, area.y + dy)
    }
}
