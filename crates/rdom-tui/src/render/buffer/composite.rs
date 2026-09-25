//! Group compositing: folding a layer a subtree painted at full
//! opacity back onto its backdrop at CSS group `opacity`.
//!
//! A terminal cell holds one glyph, one foreground and one
//! background, so compositing cannot mix two glyphs the way a pixel
//! renderer mixes coverage. The rules, per cell (DESIGN, "`opacity`
//! is group opacity"):
//!
//! - **Background.** A background the layer changed blends over the
//!   backdrop's: `α·layer + (1-α)·backdrop`.
//! - **Glyph contest.** A glyph the layer painted (text, or a border
//!   it contributed) takes the cell when the backdrop cell shows no
//!   glyph, or when `α ≥ 0.5` — the layer covers at least half of
//!   what shows. Its colour blends against the backdrop background
//!   and it carries the layer's cell state (modifiers, link — or no
//!   link). Otherwise the backdrop glyph stays.
//! - **Tint.** A backdrop glyph that stays under a background the
//!   layer changed is tinted toward it: `α·layer_bg + (1-α)·fg`.
//! - **Borders.** Border glyphs are materialised later by the joiner
//!   from the per-direction contributions, so compositing blends the
//!   colour the contributions carry instead of a glyph colour: the
//!   contributions the layer added take the glyph-contest rule and
//!   blend like a glyph; backdrop contributions under a changed
//!   background tint like a backdrop glyph. The joiner then joins
//!   translucent and opaque borders together from one table.
//! - **Wide glyphs** composite as a unit (primary + spacer), and the
//!   result never holds a primary without its spacer or an orphan
//!   spacer.
//! - `Color::Reset` resolves through the canvas model
//!   ([`CANVAS_FG`](crate::render::compose::CANVAS_FG) /
//!   [`CANVAS_BG`](crate::render::compose::CANVAS_BG)) before it
//!   blends. `α = 0` composites nothing.

use super::{BorderCell, Buffer};
use crate::render::Cell;
use crate::render::compose::{alpha_blend, canvas_bg, canvas_fg};

/// What the layer did to a cell's glyph.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LayerGlyph {
    /// Nothing visible: a blank, a space, or no change.
    None,
    /// A visible glyph different from the backdrop's.
    New,
    /// The backdrop's glyph repainted with other state (colour,
    /// modifiers, link) — e.g. a selection overlay.
    Same,
    /// The spacer of a wide glyph the layer placed in the previous
    /// cell.
    Spacer,
}

/// True when the cell shows a glyph: a visible symbol or a wide
/// glyph's spacer.
fn shows_glyph(cell: &Cell) -> bool {
    match cell.raw_symbol() {
        None => false,
        Some("") => true,
        Some(s) => !s.chars().all(char::is_whitespace),
    }
}

/// True when the cell carries a visible symbol of its own (not a
/// spacer, blank or whitespace).
fn visible_symbol(cell: &Cell) -> bool {
    cell.raw_symbol()
        .is_some_and(|s| !s.is_empty() && !s.chars().all(char::is_whitespace))
}

/// True when `layer` holds a border contribution `backdrop` does not:
/// a new visible winner or kill in some direction, or new half-block
/// quadrants.
fn adds_border(layer: &BorderCell, backdrop: &BorderCell, layer_q: u8, backdrop_q: u8) -> bool {
    layer
        .iter()
        .zip(backdrop)
        .any(|(l, b)| (l.winner.is_some() && l.winner != b.winner) || (l.killed && !b.killed))
        || layer_q & !backdrop_q != 0
}

impl Buffer {
    /// Composite `layer` — a subtree painted at full opacity over a
    /// copy of this buffer's cells in `layer.area` — back onto this
    /// buffer at `alpha` (CSS group opacity). Only `layer.area ∩
    /// self.area` is touched. See the module doc for the per-cell
    /// rules.
    pub(crate) fn composite_group(&mut self, layer: &Buffer, alpha: f32) {
        if alpha <= 0.0 {
            return;
        }
        let region = self.area.intersection(layer.area);
        if region.is_empty() {
            return;
        }
        for y in region.y..region.bottom() {
            let mut placed_wide = false;
            for x in region.x..region.right() {
                let (Some(i), Some(j)) = (self.index_of(x, y), layer.index_of(x, y)) else {
                    continue;
                };
                let next_blank = self
                    .index_of(x.saturating_add(1), y)
                    .is_none_or(|k| self.backdrop_blank(k));
                placed_wide = self.composite_cell(layer, i, j, alpha, placed_wide, next_blank);
            }
            self.repair_wide_pairs(y, region.x.saturating_sub(1), region.right());
        }
    }

    /// True when backdrop cell `i` shows nothing a layer glyph could
    /// hide: no glyph and no border.
    fn backdrop_blank(&self, i: usize) -> bool {
        !shows_glyph(&self.content[i])
            && !self.border_dirs[i].iter().any(|d| d.is_visible())
            && self.half_block_quads[i] == 0
    }

    /// Composite one cell. `after_wide` says the previous cell took a
    /// wide layer glyph (so a layer spacer here belongs to it);
    /// `next_blank` whether the backdrop's next cell is blank (a wide
    /// layer glyph covers it too). Returns whether this cell took a
    /// wide layer glyph.
    fn composite_cell(
        &mut self,
        layer: &Buffer,
        i: usize,
        j: usize,
        alpha: f32,
        after_wide: bool,
        next_blank: bool,
    ) -> bool {
        let before = &self.content[i];
        let after = &layer.content[j];
        let glyph = if after.is_spacer() && after_wide {
            LayerGlyph::Spacer
        } else if !visible_symbol(after) || after == before {
            LayerGlyph::None
        } else if after.raw_symbol() == before.raw_symbol() {
            LayerGlyph::Same
        } else {
            LayerGlyph::New
        };
        let blank = self.backdrop_blank(i);
        let wide = glyph == LayerGlyph::New && after.cell_width() == 2;
        let takes = alpha >= 0.5 || (blank && (!wide || next_blank));
        let text_wins = match glyph {
            LayerGlyph::None => false,
            LayerGlyph::New => takes,
            LayerGlyph::Same | LayerGlyph::Spacer => true,
        };
        let border_wins = adds_border(
            &layer.border_dirs[j],
            &self.border_dirs[i],
            layer.half_block_quads[j],
            self.half_block_quads[i],
        ) && (alpha >= 0.5 || blank);

        let backdrop_bg = canvas_bg(before.bg);
        let bg_changed = after.bg != before.bg;
        let layer_bg = canvas_bg(after.bg);
        let mut out = before.clone();
        if bg_changed {
            out.bg = alpha_blend(layer_bg, alpha, backdrop_bg);
        }
        match glyph {
            LayerGlyph::New | LayerGlyph::Spacer if text_wins => {
                match after.raw_symbol() {
                    Some(s) => out.set_symbol(s),
                    None => out.set_blank(),
                };
                out.modifier = after.modifier;
                out.diff = after.diff;
                out.link = after.link.clone();
                out.fg = alpha_blend(canvas_fg(after.fg), alpha, backdrop_bg);
            }
            LayerGlyph::Same => {
                out.modifier = after.modifier;
                out.diff = after.diff;
                out.link = after.link.clone();
                if after.fg != before.fg {
                    // Same glyph shape: its pixels mix the two colours.
                    out.fg = alpha_blend(canvas_fg(after.fg), alpha, canvas_fg(before.fg));
                }
            }
            _ => {
                if bg_changed && shows_glyph(before) {
                    out.fg = alpha_blend(layer_bg, alpha, canvas_fg(before.fg));
                }
            }
        }
        self.content[i] = out;

        if text_wins || border_wins {
            // The layer's glyph takes the cell: its border state (the
            // contributions it added blended; a text glyph's cleared
            // state occluding the backdrop's border) replaces ours.
            let backdrop = self.border_dirs[i];
            let mut state = layer.border_dirs[j];
            for (l, b) in state.iter_mut().zip(backdrop) {
                if l.winner != b.winner
                    && let Some(w) = &mut l.winner
                {
                    w.fg = alpha_blend(canvas_fg(w.fg), alpha, backdrop_bg);
                }
            }
            self.border_dirs[i] = state;
            self.half_block_quads[i] = layer.half_block_quads[j];
        } else if bg_changed {
            // The backdrop's border stays, tinted by the layer's bg.
            for d in &mut self.border_dirs[i] {
                if let Some(w) = &mut d.winner {
                    w.fg = alpha_blend(layer_bg, alpha, canvas_fg(w.fg));
                }
            }
        }
        wide && text_wins
    }

    /// Restore the wide-glyph invariant on row `y` over `[from, to]`:
    /// a wide primary whose spacer was replaced, or a spacer whose
    /// primary was replaced, becomes a blank space.
    fn repair_wide_pairs(&mut self, y: u16, from: u16, to: u16) {
        let right = self.area.right();
        for x in from.max(self.area.x)..=to.min(right.saturating_sub(1)) {
            let Some(i) = self.index_of(x, y) else {
                continue;
            };
            let orphan = if self.content[i].is_spacer() {
                x == self.area.x || self.content[i - 1].cell_width() != 2
            } else if self.content[i].cell_width() == 2 {
                x + 1 >= right || !self.content[i + 1].is_spacer()
            } else {
                false
            };
            if orphan {
                self.content[i].set_symbol(" ");
            }
        }
    }
}
