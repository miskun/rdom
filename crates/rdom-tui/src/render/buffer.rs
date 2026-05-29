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

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{Cell, CellDiff, Color, Rect, Style};
use crate::render::compose::alpha_blend;
use rdom_style::layout::{BorderStyle, CornerStyle};

/// Horizontal ellipsis used when a wide glyph is clipped.
const WIDE_CLIP_PLACEHOLDER: &str = "…";

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
#[derive(Debug, Clone, Copy, Default)]
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

/// 2D grid of cells.
///
/// `Eq` is intentionally omitted because `compose_alpha: f32`
/// blocks it. `PartialEq` compares only `area` + `content` —
/// transient compose-context state is excluded so two buffers with
/// the same rendered content are equal regardless of whatever
/// paint context happened to be active.
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
    /// Compose context — alpha applied to every cell write while
    /// active. `1.0` (the default) is the no-compose fast path.
    /// Set by `paint_node` via `enter_compose_ctx` before painting
    /// a translucent element; restored by `exit_compose_ctx`
    /// afterwards.
    compose_alpha: f32,
    /// Fallback bg used when the cell being written has
    /// `cell.bg == Color::Reset` — resolved per element in
    /// `paint_node` via the existing DOM walk. Reset itself
    /// (no resolved parent bg) blends against `#000000` per the
    /// `alpha_blend` canvas-model fallback.
    compose_parent_bg: Color,
}

impl PartialEq for Buffer {
    /// Compare only `area` + `content`. The compose context is
    /// transient paint state and not part of the buffer's logical
    /// content.
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
            compose_alpha: 1.0,
            compose_parent_bg: Color::Reset,
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
            compose_alpha: 1.0,
            compose_parent_bg: Color::Reset,
        }
    }

    // ── Compose context ───────────────────────────────────────────────
    //
    // The compose context is set by `paint_node` before painting an
    // element and restored on exit. While active, every cell write
    // through `set_symbol` / `set_stringn` / `set_string` / `set_style`
    // applies the element's `opacity` to the painter's style: the
    // painter's `fg` and `bg` are alpha-blended against the cell's
    // *existing* bg (falling back to `compose_parent_bg`, then to
    // `#000000`, when the existing bg is `Color::Reset`). The atomic-
    // glyph rule still applies — the painter's symbol (if any)
    // overwrites the existing symbol; the painter's modifiers ride
    // along with the painter's glyph. Modifiers don't blend (terminal
    // can't render half-bold).
    //
    // `compose_alpha = 1.0` is the opaque fast path: no blend, painter
    // writes its colors straight to the cell.

    /// Enter a new compose context, returning the previous one so the
    /// caller can restore it after the element's paint completes.
    /// Use as `let saved = buf.enter_compose_ctx(alpha, parent_bg);
    /// ...paint...; buf.exit_compose_ctx(saved);`.
    pub(crate) fn enter_compose_ctx(&mut self, alpha: f32, parent_bg: Color) -> (f32, Color) {
        let saved = (self.compose_alpha, self.compose_parent_bg);
        self.compose_alpha = alpha;
        self.compose_parent_bg = parent_bg;
        saved
    }

    /// Restore a previously-saved compose context.
    pub(crate) fn exit_compose_ctx(&mut self, saved: (f32, Color)) {
        self.compose_alpha = saved.0;
        self.compose_parent_bg = saved.1;
    }

    /// Resolve the effective destination bg for a per-cell compose.
    /// Picks the cell's existing bg if it's a real color, else the
    /// compose context's `parent_bg`, else `#000000` (canvas-model
    /// fallback — terminals don't expose their actual default bg).
    fn compose_dst_bg(&self, cell_bg: Color) -> Color {
        if cell_bg != Color::Reset {
            cell_bg
        } else if self.compose_parent_bg != Color::Reset {
            self.compose_parent_bg
        } else {
            Color::Rgb(0, 0, 0)
        }
    }

    /// Compose `style` against the cell at `(x, y)` for the current
    /// alpha. Returns the style to actually apply. Pass-through when
    /// `compose_alpha >= 1.0` (the opaque fast path).
    fn compose_style_for_cell(&self, x: u16, y: u16, style: Style) -> Style {
        if self.compose_alpha >= 1.0 {
            return style;
        }
        let cell_bg = self.cell(x, y).map(|c| c.bg).unwrap_or(Color::Reset);
        let dst = self.compose_dst_bg(cell_bg);
        Style {
            fg: style.fg.map(|fg| alpha_blend(fg, self.compose_alpha, dst)),
            bg: style.bg.map(|bg| alpha_blend(bg, self.compose_alpha, dst)),
            add_modifier: style.add_modifier,
            sub_modifier: style.sub_modifier,
        }
    }

    /// Compose a raw bg color against the cell at `(x, y)` for the
    /// current alpha. Used by `fill_bg`'s translucent path (it
    /// writes `cell.bg` directly rather than through `set_symbol`).
    /// Returns the bg value to write.
    pub(crate) fn compose_bg_for_cell(&self, x: u16, y: u16, bg: Color) -> Color {
        if self.compose_alpha >= 1.0 {
            return bg;
        }
        let cell_bg = self.cell(x, y).map(|c| c.bg).unwrap_or(Color::Reset);
        let dst = self.compose_dst_bg(cell_bg);
        alpha_blend(bg, self.compose_alpha, dst)
    }

    /// Lookup: cell index from (x, y) terminal coords. Returns `None`
    /// if outside `area`.
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

    // ── Single-cell writes ────────────────────────────────────────────

    /// Write `symbol` at `(x, y)` with `style`. Grapheme-agnostic —
    /// use `set_string` if `symbol` may be multi-byte and you want
    /// automatic wide-glyph spacer handling.
    ///
    /// If the current compose context is translucent
    /// (`compose_alpha < 1.0`), the `style`'s `fg` and `bg` are
    /// alpha-blended against the cell's existing bg before being
    /// applied. See `compose_style_for_cell` for the rule.
    pub fn set_symbol(&mut self, x: u16, y: u16, symbol: &str, style: Style) {
        let composed = self.compose_style_for_cell(x, y, style);
        if let Some(c) = self.cell_mut(x, y) {
            c.set_symbol(symbol);
            c.apply_style(composed);
        }
    }

    /// Write a single `char` at `(x, y)`. For multi-codepoint graphemes
    /// or strings, use `set_string`.
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: Style) {
        let mut buf = [0u8; 4];
        let s: &str = ch.encode_utf8(&mut buf);
        self.set_symbol(x, y, s, style);
    }

    /// Write just the style at `(x, y)`, preserving whatever symbol
    /// is currently there. Honors the compose context the same way
    /// `set_symbol` does.
    pub fn set_style(&mut self, x: u16, y: u16, style: Style) {
        let composed = self.compose_style_for_cell(x, y, style);
        if let Some(c) = self.cell_mut(x, y) {
            c.apply_style(composed);
        }
    }

    /// OSC 8 hyperlink range (Polish #9). Sets the `link` field on
    /// each cell in the `width`-wide row starting at `(x, y)`.
    /// Cells outside the buffer are silently skipped. `link: None`
    /// clears the run — useful when a previous frame left stale
    /// links that shouldn't apply now.
    ///
    /// Called by the paint pass after writing an anchor's text so
    /// the backend can emit matching `ESC ] 8 ;; <URL> ESC \\ …
    /// ESC ] 8 ;; ESC \\` wrapping sequences.
    pub fn set_link_range(&mut self, x: u16, y: u16, width: u16, link: Option<&str>) {
        for dx in 0..width {
            let cx = x.saturating_add(dx);
            if let Some(cell) = self.cell_mut(cx, y) {
                cell.set_link(link);
            }
        }
    }

    // ── String writes ─────────────────────────────────────────────────

    /// Unicode-width-aware string write.
    ///
    /// Walks graphemes; each cluster occupies its `unicode-width` in
    /// terminal cells. Writes stop at the buffer's right edge. Returns
    /// the `(x, y)` cursor position **after** the write — useful for
    /// chaining calls on the same row.
    ///
    /// Positioning is always in **visible cells**, never bytes or
    /// codepoints. `set_string(5, 0, "中")` advances the cursor to
    /// `(7, 0)`, not `(8, 0)` and not `(6, 0)`.
    pub fn set_string(&mut self, x: u16, y: u16, s: &str, style: Style) -> (u16, u16) {
        self.set_stringn(x, y, s, self.area.width, style)
    }

    /// Like `set_string`, but writes at most `max_width` visible cells.
    /// Truncates on grapheme boundaries (never mid-codepoint). If a
    /// double-width glyph would be partially clipped (one cell visible,
    /// the next clipped), the visible cell is replaced with `…`.
    pub fn set_stringn(
        &mut self,
        x: u16,
        y: u16,
        s: &str,
        max_width: u16,
        style: Style,
    ) -> (u16, u16) {
        // Row out of buffer → no-op, return input pos.
        if y < self.area.y || y >= self.area.bottom() {
            return (x, y);
        }
        // x fully past the right edge → no-op.
        if x >= self.area.right() {
            return (x, y);
        }

        // Effective per-write budget = min(requested max, buffer room).
        let buffer_room = self.area.right().saturating_sub(x);
        let budget = max_width.min(buffer_room);
        if budget == 0 {
            return (x, y);
        }

        let mut cursor_x = x;
        let mut cells_written: u16 = 0;

        for grapheme in s.graphemes(true) {
            // Skip control characters: the unicode-width crate reports
            // width 1 for LF/TAB/CR, but emitting them into the ANSI
            // stream moves the terminal cursor and corrupts the frame.
            // Standalone combining marks and ZWJ tokens are also width
            // 0 — safe to skip either way.
            let first = grapheme.chars().next().unwrap_or(' ');
            if first.is_control() {
                continue;
            }
            let w = UnicodeWidthStr::width(grapheme) as u16;
            if w == 0 {
                continue;
            }

            // Would this grapheme exceed the budget?
            if cells_written + w > budget {
                // If it's a wide glyph and we have exactly 1 cell left,
                // write the ellipsis placeholder so the boundary is
                // visually clean instead of leaving a naked half-glyph.
                if w == 2 && budget - cells_written == 1 {
                    let composed = self.compose_style_for_cell(cursor_x, y, style);
                    if let Some(c) = self.cell_mut(cursor_x, y) {
                        c.set_symbol(WIDE_CLIP_PLACEHOLDER);
                        c.apply_style(composed);
                    }
                    cursor_x = cursor_x.saturating_add(1);
                }
                break;
            }

            // Write the primary cell.
            let composed = self.compose_style_for_cell(cursor_x, y, style);
            if let Some(c) = self.cell_mut(cursor_x, y) {
                c.set_symbol(grapheme);
                c.apply_style(composed);
            }

            // For width-2 glyphs, write the trailing spacer.
            if w == 2 {
                let spacer_x = cursor_x.saturating_add(1);
                let spacer_composed = self.compose_style_for_cell(spacer_x, y, style);
                if let Some(c) = self.cell_mut(spacer_x, y) {
                    c.set_spacer();
                    c.apply_style(spacer_composed);
                }
            }

            cursor_x = cursor_x.saturating_add(w);
            cells_written += w;
        }

        (cursor_x, y)
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Color, Modifier};
    use proptest::prelude::*;

    fn ascii_style() -> Style {
        Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .add_modifier(Modifier::BOLD)
    }

    // ── Construction + indexing ──────────────────────────────────────

    #[test]
    fn empty_has_correct_cell_count() {
        let b = Buffer::empty(Rect::new(0, 0, 10, 5));
        assert_eq!(b.content.len(), 50);
        assert!(b.content.iter().all(|c| *c == Cell::EMPTY));
    }

    #[test]
    fn filled_has_expected_cell() {
        let mut c = Cell::new("X");
        c.fg = Color::Rgb(0, 0, 255);
        let b = Buffer::filled(Rect::new(0, 0, 3, 2), c.clone());
        for cell in &b.content {
            assert_eq!(*cell, c);
        }
    }

    #[test]
    fn index_of_inside_and_outside() {
        let b = Buffer::empty(Rect::new(10, 20, 5, 4));
        assert_eq!(b.index_of(10, 20), Some(0));
        assert_eq!(b.index_of(14, 23), Some(19)); // last cell
        assert_eq!(b.index_of(9, 20), None); // left of
        assert_eq!(b.index_of(15, 20), None); // right edge (exclusive)
        assert_eq!(b.index_of(10, 24), None); // below
    }

    #[test]
    fn cell_returns_copy_of_initial_empty() {
        let b = Buffer::empty(Rect::new(0, 0, 3, 3));
        assert_eq!(b.cell(0, 0), Some(&Cell::EMPTY));
        assert_eq!(b.cell(99, 99), None);
    }

    // ── set_string (Unicode-width-aware) ─────────────────────────────

    #[test]
    fn ascii_set_string() {
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let end = b.set_string(0, 0, "hello", ascii_style());
        assert_eq!(end, (5, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "h");
        assert_eq!(b.cell(4, 0).unwrap().symbol(), "o");
        assert_eq!(b.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn control_chars_are_skipped_not_written_as_cells() {
        // The `unicode-width` crate reports width 1 for LF/TAB/CR, but
        // writing a control char into a cell would emit raw \n / \t into
        // the ANSI stream and move the terminal cursor. set_stringn must
        // treat control chars as width-0 and skip them entirely.
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let end = b.set_string(0, 0, "\n  \n\t\r  AB", ascii_style());
        // Cursor advanced by the 4 space/ASCII graphemes that survived
        // plus AB = 2 + 2 + 2 = 6. Control chars did not advance it.
        assert_eq!(end, (6, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(1, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(2, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(3, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(4, 0).unwrap().symbol(), "A");
        assert_eq!(b.cell(5, 0).unwrap().symbol(), "B");
    }

    #[test]
    fn cjk_advances_by_two_cells_not_three_bytes() {
        // THE Unicode-width invariant. "中" is 3 bytes UTF-8, 1 codepoint,
        // 1 grapheme, 2 cells wide. Cursor must end at x=2, not x=3
        // (bytes) and not x=1 (codepoints).
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let end = b.set_string(0, 0, "中", ascii_style());
        assert_eq!(end, (2, 0), "CJK must advance by visible cell width");
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "中");
        assert!(b.cell(1, 0).unwrap().is_spacer());
    }

    #[test]
    fn emoji_cursor_advance() {
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let end = b.set_string(0, 0, "🦀A", ascii_style());
        // 🦀 (width 2) + "A" (width 1) = 3 cells.
        assert_eq!(end, (3, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "🦀");
        assert!(b.cell(1, 0).unwrap().is_spacer());
        assert_eq!(b.cell(2, 0).unwrap().symbol(), "A");
    }

    #[test]
    fn zwj_family_is_one_grapheme() {
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let fam = "👨\u{200D}👩\u{200D}👧"; // family emoji
        let end = b.set_string(0, 0, fam, ascii_style());
        assert_eq!(end, (2, 0), "ZWJ family must be one wide glyph");
        // The primary cell carries the full ZWJ sequence.
        assert_eq!(b.cell(0, 0).unwrap().symbol(), fam);
        assert!(b.cell(1, 0).unwrap().is_spacer());
    }

    #[test]
    fn combining_mark_folds_into_grapheme() {
        // "é" as "e\u{0301}" is 2 codepoints but 1 grapheme, width 1.
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let end = b.set_string(0, 0, "e\u{0301}f", ascii_style());
        assert_eq!(end, (2, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "e\u{0301}");
        assert_eq!(b.cell(1, 0).unwrap().symbol(), "f");
    }

    #[test]
    fn regional_indicator_flag_is_one_grapheme_width_2() {
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
        let end = b.set_string(0, 0, "🇺🇸", ascii_style());
        assert_eq!(end, (2, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "🇺🇸");
        assert!(b.cell(1, 0).unwrap().is_spacer());
    }

    #[test]
    fn mixed_ascii_cjk_emoji() {
        let mut b = Buffer::empty(Rect::new(0, 0, 20, 1));
        // "A" (1) + "中" (2) + "🦀" (2) + "B" (1) = 6 cells
        let end = b.set_string(0, 0, "A中🦀B", ascii_style());
        assert_eq!(end, (6, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "A");
        assert_eq!(b.cell(1, 0).unwrap().symbol(), "中");
        assert!(b.cell(2, 0).unwrap().is_spacer());
        assert_eq!(b.cell(3, 0).unwrap().symbol(), "🦀");
        assert!(b.cell(4, 0).unwrap().is_spacer());
        assert_eq!(b.cell(5, 0).unwrap().symbol(), "B");
    }

    // ── Clipping / truncation ────────────────────────────────────────

    #[test]
    fn clip_past_right_edge_stops() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
        let end = b.set_string(0, 0, "0123456789", ascii_style());
        assert_eq!(end, (5, 0));
        assert_eq!(b.cell(4, 0).unwrap().symbol(), "4");
        // No cell past the edge.
        assert_eq!(b.cell(5, 0), None);
    }

    #[test]
    fn wide_glyph_clipped_at_right_edge_emits_ellipsis() {
        // Buffer 5 wide; write "中中中" (6 cells requested). Only 2 full
        // wide glyphs fit (4 cells) + 1 cell left → the last wide glyph
        // gets replaced with `…`.
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
        let end = b.set_string(0, 0, "中中中", ascii_style());
        assert_eq!(end, (5, 0));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "中");
        assert_eq!(b.cell(2, 0).unwrap().symbol(), "中");
        assert_eq!(b.cell(4, 0).unwrap().symbol(), "…");
    }

    #[test]
    fn set_stringn_enforces_max_width() {
        let mut b = Buffer::empty(Rect::new(0, 0, 20, 1));
        let end = b.set_stringn(0, 0, "hello world", 5, ascii_style());
        assert_eq!(end, (5, 0));
        assert_eq!(b.cell(4, 0).unwrap().symbol(), "o");
        assert_eq!(b.cell(5, 0).unwrap().symbol(), " "); // untouched blank
    }

    #[test]
    fn row_out_of_range_is_noop() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
        let end = b.set_string(0, 5, "hello", ascii_style());
        assert_eq!(end, (0, 5));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), " "); // untouched
    }

    #[test]
    fn x_past_right_edge_is_noop() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
        let end = b.set_string(99, 0, "hello", ascii_style());
        assert_eq!(end, (99, 0));
    }

    // ── Style writes ─────────────────────────────────────────────────

    #[test]
    fn set_style_preserves_symbol() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
        b.set_string(0, 0, "hi", Style::new());
        b.set_style(0, 0, Style::new().fg(Color::Rgb(255, 0, 0)));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "h");
        assert_eq!(b.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn set_char_writes_single_codepoint() {
        let mut b = Buffer::empty(Rect::new(0, 0, 3, 1));
        b.set_char(1, 0, 'X', Style::new().fg(Color::Rgb(0, 128, 0)));
        assert_eq!(b.cell(1, 0).unwrap().symbol(), "X");
        assert_eq!(b.cell(1, 0).unwrap().fg, Color::Rgb(0, 128, 0));
    }

    // ── fill ─────────────────────────────────────────────────────────

    #[test]
    fn fill_overwrites_region() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 3));
        b.fill(Rect::new(1, 1, 3, 1), Cell::new("#"));
        assert_eq!(b.cell(0, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(1, 1).unwrap().symbol(), "#");
        assert_eq!(b.cell(2, 1).unwrap().symbol(), "#");
        assert_eq!(b.cell(3, 1).unwrap().symbol(), "#");
        assert_eq!(b.cell(4, 1).unwrap().symbol(), " ");
        assert_eq!(b.cell(1, 0).unwrap().symbol(), " ");
    }

    #[test]
    fn fill_clips_to_buffer() {
        let mut b = Buffer::empty(Rect::new(0, 0, 3, 3));
        // Overflow past the right — should not panic, only cells 0..2 in x get filled.
        b.fill(Rect::new(0, 0, 100, 100), Cell::new("#"));
        for x in 0..3 {
            for y in 0..3 {
                assert_eq!(b.cell(x, y).unwrap().symbol(), "#");
            }
        }
    }

    // ── clear ────────────────────────────────────────────────────────

    #[test]
    fn clear_restores_all_empty() {
        let mut b = Buffer::empty(Rect::new(0, 0, 3, 3));
        b.set_string(0, 0, "hello", Style::new().fg(Color::Rgb(255, 0, 0)));
        b.clear();
        for c in &b.content {
            assert_eq!(*c, Cell::EMPTY);
        }
    }

    // ── merge ────────────────────────────────────────────────────────

    #[test]
    fn merge_copies_overlapping_cells() {
        let mut dst = Buffer::empty(Rect::new(0, 0, 5, 3));
        let mut src = Buffer::empty(Rect::new(1, 1, 3, 1));
        src.set_string(1, 1, "foo", Style::new());
        dst.merge(&src);
        assert_eq!(dst.cell(1, 1).unwrap().symbol(), "f");
        assert_eq!(dst.cell(2, 1).unwrap().symbol(), "o");
        assert_eq!(dst.cell(3, 1).unwrap().symbol(), "o");
        assert_eq!(dst.cell(0, 0).unwrap().symbol(), " "); // untouched
    }

    #[test]
    fn merge_non_overlapping_is_noop() {
        let before = Buffer::empty(Rect::new(0, 0, 3, 3));
        let mut dst = before.clone();
        let src = Buffer::filled(Rect::new(100, 100, 3, 3), Cell::new("X"));
        dst.merge(&src);
        assert_eq!(dst, before);
    }

    // ── resize ───────────────────────────────────────────────────────

    #[test]
    fn resize_preserves_intersection() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 3));
        b.set_string(0, 0, "hello", Style::new().fg(Color::Rgb(255, 0, 0)));
        let changed = b.resize(Rect::new(0, 0, 3, 5));
        assert!(changed);
        // "hel" preserved; "lo" dropped.
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "h");
        assert_eq!(b.cell(2, 0).unwrap().symbol(), "l");
        assert_eq!(b.cell(2, 4).unwrap().symbol(), " "); // new cell, empty
    }

    #[test]
    fn resize_identity_returns_false() {
        let mut b = Buffer::empty(Rect::new(0, 0, 5, 3));
        assert!(!b.resize(Rect::new(0, 0, 5, 3)));
    }

    // ── diff ─────────────────────────────────────────────────────────

    #[test]
    fn diff_empty_on_identical() {
        let a = Buffer::empty(Rect::new(0, 0, 10, 5));
        let b = Buffer::empty(Rect::new(0, 0, 10, 5));
        assert_eq!(a.diff_iter(&b).count(), 0);
    }

    #[test]
    fn diff_yields_changed_cells() {
        let a = Buffer::empty(Rect::new(0, 0, 5, 1));
        let mut b = a.clone();
        b.set_string(2, 0, "ab", Style::new());
        let diffs: Vec<_> = b
            .diff_iter(&a)
            .map(|(x, y, c)| (x, y, c.symbol().to_string()))
            .collect();
        assert_eq!(diffs, vec![(2, 0, "a".into()), (3, 0, "b".into())]);
    }

    #[test]
    fn diff_skips_spacer_cells() {
        let a = Buffer::empty(Rect::new(0, 0, 5, 1));
        let mut b = a.clone();
        // "中" writes primary at 0 + spacer at 1.
        b.set_string(0, 0, "中", Style::new());
        let diffs: Vec<_> = b.diff_iter(&a).collect();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].0, 0);
        assert_eq!(diffs[0].2.symbol(), "中");
    }

    #[test]
    fn diff_honors_skip_flag() {
        let a = Buffer::empty(Rect::new(0, 0, 3, 1));
        let mut b = a.clone();
        b.set_string(0, 0, "abc", Style::new());
        b.cell_mut(1, 0).unwrap().diff = CellDiff::Skip;
        let positions: Vec<_> = b.diff_iter(&a).map(|(x, _, _)| x).collect();
        assert_eq!(positions, vec![0, 2]);
    }

    #[test]
    fn diff_honors_always_update() {
        let a = Buffer::empty(Rect::new(0, 0, 3, 1));
        let mut b = a.clone();
        // Cells are identical to `a` but we force `AlwaysUpdate` on one.
        b.cell_mut(1, 0).unwrap().diff = CellDiff::AlwaysUpdate;
        let positions: Vec<_> = b.diff_iter(&a).map(|(x, _, _)| x).collect();
        assert_eq!(positions, vec![1]);
    }

    // ── Property tests ───────────────────────────────────────────────

    proptest! {
        #[test]
        fn set_string_cursor_matches_unicode_width(s in r"[A-Za-z0-9\u{4e00}-\u{9fff}\u{1f300}-\u{1f5ff} ]{0,20}") {
            let mut b = Buffer::empty(Rect::new(0, 0, 100, 1));
            let expected = unicode_width::UnicodeWidthStr::width(s.as_str()) as u16;
            let (end_x, _) = b.set_string(0, 0, &s, Style::new());
            prop_assert_eq!(
                end_x,
                expected.min(100),
                "cursor advance must equal UnicodeWidthStr::width, not s.len() or s.chars().count()"
            );
        }

        #[test]
        fn spacer_after_every_wide_glyph(s in r"[\u{4e00}-\u{9fff}A-Za-z]{0,15}") {
            // After any write, every cell with cell_width == 2 is followed
            // by a spacer (or the buffer right edge).
            let mut b = Buffer::empty(Rect::new(0, 0, 50, 1));
            b.set_string(0, 0, &s, Style::new());
            for x in 0..49 {
                let c = b.cell(x, 0).unwrap();
                if c.cell_width() == 2 {
                    let next = b.cell(x + 1, 0).unwrap();
                    prop_assert!(next.is_spacer(), "cell at x={} is wide but x+1 is not a spacer", x);
                }
            }
        }

        #[test]
        fn resize_preserves_overlapping_symbols(
            w1 in 1u16..30, h1 in 1u16..10,
            w2 in 1u16..30, h2 in 1u16..10
        ) {
            let mut b = Buffer::empty(Rect::new(0, 0, w1, h1));
            // Paint each cell with its (x, y) as a char (mod 94 for printable ASCII).
            for y in 0..h1 {
                for x in 0..w1 {
                    let ch = char::from_u32(33 + ((x as u32 + y as u32 * 31) % 94)).unwrap();
                    b.set_char(x, y, ch, Style::new());
                }
            }
            let snapshot = b.clone();
            b.resize(Rect::new(0, 0, w2, h2));
            // Intersection cells match.
            for y in 0..h1.min(h2) {
                for x in 0..w1.min(w2) {
                    prop_assert_eq!(
                        b.cell(x, y).unwrap().symbol(),
                        snapshot.cell(x, y).unwrap().symbol()
                    );
                }
            }
        }

        #[test]
        fn diff_self_vs_self_is_empty(w in 1u16..20, h in 1u16..5) {
            let a = Buffer::empty(Rect::new(0, 0, w, h));
            prop_assert_eq!(a.diff_iter(&a).count(), 0);
        }
    }
}
