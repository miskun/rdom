//! Border-direction and half-block side tables: the per-cell
//! [`BorderDirState`] (CSS Tables 3 §11.5 conflict resolution), the
//! `DIR_*` / `QUAD_*` indices, and the [`Buffer`] accessors that
//! read and write those tables for `paint_border` and the joiner.

use super::Buffer;
use crate::render::Color;
use rdom_style::layout::{BorderStyle, CornerStyle};

/// Which physical side of the source element a border
/// contribution sits on. Direction-symmetric styles (Solid, Double,
/// Dashed, etc.) ignore it — `─` looks the same on a top or bottom
/// edge. Direction-asymmetric styles (currently `HalfBlock`) use
/// it to pick the right glyph: a top edge gets `▄` (lower half
/// block) while a bottom edge gets `▀` (upper half block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderSide {
    Top,
    Right,
    Bottom,
    Left,
}

/// One element's contribution to one cell × one direction's border.
/// Captures the data needed by the CSS Tables 3 §11.5 conflict-
/// resolution algorithm: the style (for hidden kill-switch + style-
/// rank tiebreak), the color (winner contributes both glyph AND
/// color), and a structural priority for elements that tie on style.
///
/// Priority uses a packed `u64` so a single integer compare resolves
/// the order. Bits 32-63 = `depth` (distance from root; bigger =
/// more nested = wins per CSS rule 5). Bits 0-31 =
/// `u32::MAX - dom_index` (earlier in DOM = leftmost / topmost in
/// geometric order = wins per CSS rule 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderContribution {
    pub style: BorderStyle,
    pub fg: Color,
    pub priority: u64,
    /// Source element's `corner-style`. Only meaningful when this
    /// contribution lands on one of the element's corner cells AND
    /// the cell remains a "lone" contributor (no other element
    /// painted to it). Joiner uses this to preserve rounded
    /// (`╭╮╰╯`) corner glyphs for single-element border rings;
    /// any overlap from a second element promotes to a square
    /// junction (Unicode has no rounded T-junctions).
    pub corner_style: CornerStyle,
    /// Which physical side of the source element this contribution
    /// came from. The joiner only consults this for direction-
    /// asymmetric styles (`HalfBlock`). For `Solid` and friends
    /// the field is recorded but ignored — the same glyph wins
    /// either way.
    pub side: BorderSide,
}

impl BorderContribution {
    /// Pack `depth` + `dom_index` into a single `u64` that compares
    /// the right way (higher = wins).
    pub fn pack_priority(depth: u16, dom_index: u32) -> u64 {
        ((depth as u64) << 32) | ((u32::MAX - dom_index) as u64)
    }
}

/// Per-cell, per-direction border state. Tracks the winning visible
/// contribution AND a `killed` flag for the `BorderStyle::Hidden`
/// kill-switch (CSS Tables 3 §11.5 rule 1: hidden suppresses the
/// edge regardless of any other contributor).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BorderDirState {
    /// Highest-rank visible contribution so far. `None` when no
    /// non-None / non-Hidden style has been written for this
    /// direction.
    pub winner: Option<BorderContribution>,
    /// `true` if any contributor was `BorderStyle::Hidden`. When
    /// set, the direction is suppressed at paint time regardless
    /// of `winner`.
    pub killed: bool,
}

impl BorderDirState {
    /// CSS Tables 3 §11.5 conflict resolution: a new contribution
    /// either wins (kills the cell), wins on rank, ties (no
    /// change), or loses. Hidden always kills. None never writes.
    pub fn merge(&mut self, new: BorderContribution) {
        if new.style.is_none() {
            return;
        }
        if new.style.is_hidden() {
            // Rule 1: hidden kill-switch is absolute.
            self.killed = true;
            return;
        }
        // Visible contribution. Compare against the current winner
        // by (style rank, priority). Higher wins.
        let new_key = (new.style.rank(), new.priority);
        let win = match self.winner {
            None => true,
            Some(prev) => {
                let prev_key = (prev.style.rank(), prev.priority);
                new_key > prev_key
            }
        };
        if win {
            self.winner = Some(new);
        }
    }

    /// Does this direction paint a glyph? True iff there's a
    /// non-hidden visible winner.
    pub fn is_visible(&self) -> bool {
        !self.killed && self.winner.is_some()
    }
}

/// Per-cell array of direction states — N, E, S, W in that order.
pub type BorderCell = [BorderDirState; 4];

/// Direction index into `BorderCell`. Constants used by paint and
/// the joiner. Match the layout `border_mask` bit ordering used
/// previously (N=0, E=1, S=2, W=3 corresponded to bits 0, 1, 2, 3).
pub const DIR_N: usize = 0;
pub const DIR_E: usize = 1;
pub const DIR_S: usize = 2;
pub const DIR_W: usize = 3;

/// Quadrant bits for the half-block inward-quadrant accumulator
/// (`Buffer::half_block_quads`). A cell's filled region is the union of
/// these across every contributing half-block border.
pub const QUAD_TL: u8 = 0b0001;
pub const QUAD_TR: u8 = 0b0010;
pub const QUAD_BL: u8 = 0b0100;
pub const QUAD_BR: u8 = 0b1000;

impl Buffer {
    /// OR `quads` into the half-block inward-quadrant accumulator at
    /// `(x, y)`. Out-of-bounds writes silently no-op. Welding is just
    /// the union of every contributing element's inward quadrants.
    pub fn add_half_block_quads(&mut self, x: u16, y: u16, quads: u8) {
        if let Some(i) = self.index_of(x, y) {
            self.half_block_quads[i] |= quads;
        }
    }

    /// Read the accumulated half-block quadrants at `(x, y)`. `0` for
    /// out-of-bounds or cells with no half-block border.
    pub fn half_block_quads_at(&self, x: u16, y: u16) -> u8 {
        self.index_of(x, y)
            .map(|i| self.half_block_quads[i])
            .unwrap_or(0)
    }

    /// Clear the half-block quadrants at `(x, y)`. Used by opaque
    /// content paint (bg/glyph) to occlude an underlying half-block
    /// border, matching `set_border_dir`'s direction-state clear.
    pub fn clear_half_block_quads(&mut self, x: u16, y: u16) {
        if let Some(i) = self.index_of(x, y) {
            self.half_block_quads[i] = 0;
        }
    }

    /// Clear ALL border state at `(x, y)` — every direction's contribution
    /// plus the half-block quadrants. Used by content paint to occlude a
    /// lower element's border beneath it (z-aware borders): the joiner runs
    /// last, so a cleared cell won't have a border re-derived over the
    /// content.
    pub fn clear_border_at(&mut self, x: u16, y: u16) {
        if let Some(i) = self.index_of(x, y) {
            self.border_dirs[i] = BorderCell::default();
            self.half_block_quads[i] = 0;
        }
    }

    /// Add a border contribution for `(x, y)` in direction `dir`
    /// (use `DIR_N` / `DIR_E` / `DIR_S` / `DIR_W`). Applies CSS
    /// Tables 3 §11.5 conflict resolution: hidden kills, otherwise
    /// the higher-rank (then higher-priority) contribution wins.
    /// Out-of-bounds writes silently no-op.
    pub fn add_border_dir(&mut self, x: u16, y: u16, dir: usize, contribution: BorderContribution) {
        debug_assert!(dir < 4, "BorderCell index out of range");
        if let Some(i) = self.index_of(x, y) {
            self.border_dirs[i][dir].merge(contribution);
        }
    }

    /// Read the per-direction state at `(x, y)`. Default
    /// (all-empty) for out-of-bounds.
    pub fn border_dir_at(&self, x: u16, y: u16, dir: usize) -> BorderDirState {
        debug_assert!(dir < 4, "BorderCell index out of range");
        self.index_of(x, y)
            .map(|i| self.border_dirs[i][dir])
            .unwrap_or_default()
    }

    /// Overwrite the per-direction state at `(x, y)`. Used by
    /// opaque `fill_bg` to clear earlier border contributions
    /// before the joiner sees the cell. Out-of-bounds writes
    /// silently no-op.
    pub fn set_border_dir(&mut self, x: u16, y: u16, dir: usize, state: BorderDirState) {
        debug_assert!(dir < 4, "BorderCell index out of range");
        if let Some(i) = self.index_of(x, y) {
            self.border_dirs[i][dir] = state;
        }
    }

    /// 4-bit visible-direction mask at `(x, y)` (compatibility
    /// helper). Bit 0 = N, bit 1 = E, bit 2 = S, bit 3 = W. A
    /// direction is "visible" iff its state has a winner and is
    /// NOT killed by a Hidden contributor. Used by the joiner to
    /// pick the right junction-table entry.
    pub fn border_mask_at(&self, x: u16, y: u16) -> u8 {
        let Some(i) = self.index_of(x, y) else {
            return 0;
        };
        let cell = &self.border_dirs[i];
        let mut mask = 0u8;
        if cell[DIR_N].is_visible() {
            mask |= 0b0001;
        }
        if cell[DIR_E].is_visible() {
            mask |= 0b0010;
        }
        if cell[DIR_S].is_visible() {
            mask |= 0b0100;
        }
        if cell[DIR_W].is_visible() {
            mask |= 0b1000;
        }
        mask
    }
}
