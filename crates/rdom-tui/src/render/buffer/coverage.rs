//! Coverage: which cells, and which of their parts, the paints into a
//! layer wrote (C15-FILTER). A filter maps the colors its element paints —
//! not the backdrop's beside or beneath them — and a color the element
//! paints may equal the backdrop's (a black box on a black page), so what
//! was painted cannot be read off the cells: every writer marks it.
//!
//! Only a layer made for a graphical effect tracks coverage (`Buffer::
//! track_coverage`), and the layers copied from it — a nested group's, a
//! translucent paint's scratch — whose coverage compositing ORs back
//! (C15G-FILTER-COVERAGE); every other buffer — the frame, a layer copied
//! from it — holds none, and a mark is one branch.

use super::Buffer;
use crate::render::{Cell, Rect};
use crate::style::Color;

/// The cell's background was painted.
pub(crate) const BG: u8 = 0b0001;
/// Its glyph (symbol, foreground, modifiers) was painted.
pub(crate) const GLYPH: u8 = 0b0010;
/// A border contribution was added or cleared there.
pub(crate) const BORDER: u8 = 0b0100;
/// A drop shadow shaded it (it is not the element's own paint).
pub(crate) const SHADOW: u8 = 0b1000;
/// Everything a fill paints over.
pub(crate) const ALL: u8 = BG | GLYPH | BORDER;

impl Buffer {
    /// Start tracking coverage: nothing painted yet.
    pub(crate) fn track_coverage(&mut self) {
        let len = self.content.len();
        let mut bits = self.coverage.take().unwrap_or_default();
        bits.clear();
        bits.resize(len, 0);
        self.coverage = Some(bits);
    }

    /// Whether the buffer tracks coverage.
    pub(crate) fn tracks_coverage(&self) -> bool {
        self.coverage.is_some()
    }

    /// The coverage bits of cell `i` (0 when untracked).
    pub(crate) fn coverage_of(&self, i: usize) -> u8 {
        self.coverage.as_ref().map_or(0, |c| c[i])
    }

    /// Mark cell `i` as painted in `bits`.
    #[inline]
    pub(crate) fn mark(&mut self, i: usize, bits: u8) {
        if let Some(c) = &mut self.coverage {
            c[i] |= bits;
        }
    }

    /// Mark the cell at `(x, y)` as painted in `bits`.
    #[inline]
    pub(crate) fn mark_at(&mut self, x: u16, y: u16, bits: u8) {
        if self.coverage.is_some()
            && let Some(i) = self.index_of(x, y)
        {
            self.mark(i, bits);
        }
    }

    /// Map the colors of the cells of `area` that `select` (given the
    /// cell's position and coverage) accepts, part by part: the background through `bg`, the glyph's foreground and
    /// underline color and the border contributions' colors through `fg`
    /// — each given the color and returning the new one.
    pub(crate) fn map_colors(
        &mut self,
        area: Rect,
        select: impl Fn(u16, u16, u8) -> (bool, bool, bool),
        bg: impl Fn(usize, Color) -> Color,
        fg: impl Fn(usize, Color) -> Color,
    ) {
        let region = self.area.intersection(area);
        for y in region.y..region.bottom() {
            for x in region.x..region.right() {
                let Some(i) = self.index_of(x, y) else {
                    continue;
                };
                let (b, g, border) = select(x, y, self.coverage_of(i));
                let cell: &mut Cell = &mut self.content[i];
                if b {
                    cell.bg = bg(i, cell.bg);
                }
                if g {
                    cell.fg = fg(i, cell.fg);
                    if cell.underline_color != Color::Reset {
                        cell.underline_color = fg(i, cell.underline_color);
                    }
                }
                if border {
                    for d in &mut self.border_dirs[i] {
                        if let Some(w) = &mut d.winner {
                            w.fg = fg(i, w.fg);
                        }
                    }
                }
            }
        }
    }
}
