//! `Buffer` — a 2D grid of `Cell`s, the paint target.
//!
//! Row-major `Vec<Cell>` with length `area.width * area.height`.
//! `index(x, y) = (y - area.y) * area.width + (x - area.x)`. Same
//! shape as ratatui's — predictable cache behavior, easy to vectorize,
//! `chunks(width)` walks rows for snapshot rendering.
//!
//! Every public write method routes through `unicode-width` for
//! positioning. Writing `"中"` advances the cursor by **2 cells**, not
//! 3 bytes and not 1 codepoint.
//!
//! ## Wide-glyph handling
//!
//! `set_string` splits on grapheme clusters (`unicode-segmentation`).
//! For each cluster:
//!
//! - Width 0 (combining marks, ZWJ, variation selectors): absorbed
//!   into the previous cell's symbol. Already handled by the grapheme
//!   iterator — width-0 clusters never appear on their own from
//!   `UnicodeSegmentation::graphemes`.
//! - Width 1: one cell, normal write.
//! - Width 2: **primary cell** carries the full symbol; **trailing
//!   spacer cell** at x+1 gets `set_spacer()` (empty symbol). The diff
//!   iterator skips the spacer.
//!
//! ## Clipping
//!
//! All write methods clamp to `area`. Out-of-bounds positions are
//! ignored, not panicked. If a wide glyph would straddle the right
//! edge, the primary cell is replaced with `…` (ellipsis) in the last
//! visible column — matches the "count cells, never bytes" invariant.
//!
//! ## Layout
//!
//! - this file — the [`Buffer`] grid: construction, indexing, and
//!   bulk operations (`clear` / `fill` / `resize` / `merge`).
//! - `border` — the border-direction + half-block side tables.
//! - `write` — single-cell and string writes (wide-glyph handling).
//! - `composite` — group-opacity compositing.
//! - `diff` — the frame diff iterator.

use super::{Cell, Rect};

mod border;
mod composite;
mod diff;
mod write;

#[cfg(test)]
mod tests;

pub use border::{
    BorderCell, BorderContribution, BorderDirState, BorderSide, DIR_E, DIR_N, DIR_S, DIR_W,
    QUAD_BL, QUAD_BR, QUAD_TL, QUAD_TR,
};

/// 2D grid of cells.
///
/// `PartialEq` compares only `area` + `content`: the border-direction
/// and half-block side tables are joiner bookkeeping derived from the
/// paints that produced the content, not content of their own.
#[derive(Debug, Clone)]
pub struct Buffer {
    /// The rectangle this buffer covers in terminal grid coordinates.
    pub area: Rect,
    /// Row-major `width * height` cells.
    pub content: Vec<Cell>,
    /// Per-cell × per-direction border state. `paint_border` writes
    /// one contribution per (cell, direction) pair. CSS Tables 3
    /// §11.5 conflict resolution applies per direction independently
    /// — hidden kill-switch, style rank, then structural priority.
    /// The joiner reads each direction's winner to derive the final
    /// junction glyph + color (BORDER-MODEL-1).
    pub border_dirs: Vec<BorderCell>,
    /// Per-cell inward-quadrant accumulator for half-block borders. Each
    /// `u8` is a 4-bit set — `QUAD_TL|QUAD_TR|QUAD_BL|QUAD_BR` — of the
    /// quadrants a half-block border fills at that cell, OR-ed across every
    /// contributing element. `paint_border` writes a box's inward quadrants
    /// (corner → one quadrant, edge → a half); the joiner unions them and
    /// emits the matching block glyph. This is what lets two half-block
    /// borders *weld* (a tab onto a panel → `▟ █ ▌`) instead of falling back
    /// to box-drawing T-junctions — see `border_join::HALF_BLOCK_QUAD_TABLE`.
    pub half_block_quads: Vec<u8>,
}

impl PartialEq for Buffer {
    /// Compare only `area` + `content`; the side tables are joiner
    /// bookkeeping, not logical content.
    fn eq(&self, other: &Self) -> bool {
        self.area == other.area && self.content == other.content
    }
}

impl Buffer {
    /// Empty buffer filled with `Cell::EMPTY`.
    pub fn empty(area: Rect) -> Self {
        Self::filled(area, Cell::EMPTY)
    }

    /// Fresh buffer filled with a specific cell. Primarily useful for
    /// tests and for quick background fills at startup.
    pub fn filled(area: Rect, cell: Cell) -> Self {
        let len = area.area() as usize;
        Buffer {
            area,
            content: vec![cell; len],
            border_dirs: vec![BorderCell::default(); len],
            half_block_quads: vec![0u8; len],
        }
    }

    /// Build a buffer from pre-existing cells. `cells.len()` must
    /// equal `area.area()`. Panics otherwise — caller bug.
    pub fn with_cells(area: Rect, cells: Vec<Cell>) -> Self {
        assert_eq!(
            cells.len(),
            area.area() as usize,
            "Buffer::with_cells: cell count {} != area.area() {}",
            cells.len(),
            area.area()
        );
        let len = area.area() as usize;
        Buffer {
            area,
            content: cells,
            border_dirs: vec![BorderCell::default(); len],
            half_block_quads: vec![0u8; len],
        }
    }

    pub fn index_of(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.area.x || y < self.area.y || x >= self.area.right() || y >= self.area.bottom() {
            return None;
        }
        let dx = (x - self.area.x) as usize;
        let dy = (y - self.area.y) as usize;
        Some(dy * self.area.width as usize + dx)
    }

    /// Get a cell. `None` if out of bounds.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index_of(x, y).map(|i| &self.content[i])
    }

    /// Get a mutable cell. `None` if out of bounds.
    pub fn cell_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        let i = self.index_of(x, y)?;
        Some(&mut self.content[i])
    }

    // ── Bulk operations ───────────────────────────────────────────────

    /// Wipe every cell to `Cell::EMPTY`. Also clears the per-cell
    /// per-direction border state so a fresh frame doesn't see
    /// stale connectivity / winners.
    pub fn clear(&mut self) {
        self.content.fill(Cell::EMPTY);
        for dir in &mut self.border_dirs {
            *dir = BorderCell::default();
        }
        self.half_block_quads.fill(0);
    }

    /// Fill `area` (intersected with the buffer's own area) with `cell`.
    /// Clips silently when `area` extends beyond the buffer.
    pub fn fill(&mut self, area: Rect, cell: Cell) {
        let clip = self.area.intersection(area);
        if clip.is_empty() {
            return;
        }
        for y in clip.y..clip.bottom() {
            for x in clip.x..clip.right() {
                if let Some(i) = self.index_of(x, y) {
                    self.content[i] = cell.clone();
                }
            }
        }
    }

    /// Resize to `new_area`. Preserves cells in the intersection of
    /// old and new areas; new cells are `EMPTY`. Chose preserve-over-
    /// truncate because a corrupted post-resize buffer is hard to
    /// debug; the O(area) cost is paid at most once per terminal
    /// resize event. Return `true` if the area actually changed.
    pub fn resize(&mut self, new_area: Rect) -> bool {
        if new_area == self.area {
            return false;
        }
        let mut next = Self::empty(new_area);
        let overlap = self.area.intersection(new_area);
        if !overlap.is_empty() {
            for y in overlap.y..overlap.bottom() {
                for x in overlap.x..overlap.right() {
                    if let (Some(src_i), Some(dst_i)) = (self.index_of(x, y), next.index_of(x, y)) {
                        next.content[dst_i] = self.content[src_i].clone();
                    }
                }
            }
        }
        *self = next;
        true
    }

    /// Copy every cell from `other` into `self`, position-aligned.
    /// Cells outside `self.area` are skipped. Useful for compositing
    /// a child buffer into a parent.
    pub fn merge(&mut self, other: &Buffer) {
        let overlap = self.area.intersection(other.area);
        if overlap.is_empty() {
            return;
        }
        for y in overlap.y..overlap.bottom() {
            for x in overlap.x..overlap.right() {
                if let (Some(src_i), Some(dst_i)) = (other.index_of(x, y), self.index_of(x, y)) {
                    self.content[dst_i] = other.content[src_i].clone();
                }
            }
        }
    }
}
