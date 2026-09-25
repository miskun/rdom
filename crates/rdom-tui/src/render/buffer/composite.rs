//! Group compositing: folding a layer a subtree painted at full
//! opacity back onto its backdrop at CSS group `opacity`.

use super::{BorderDirState, Buffer};
use crate::render::Color;
use crate::render::compose::alpha_blend;

impl Buffer {
    /// Composite `layer` — a copy of this buffer that a subtree painted
    /// into at full opacity — back onto this buffer at `alpha` (CSS
    /// group opacity). Per cell, against the backdrop this buffer
    /// holds: a background the layer changed blends in; a visible glyph
    /// the layer painted replaces the cell's glyph with its foreground
    /// blended against the backdrop; a blank or space the layer wrote
    /// keeps the backdrop's glyph (a translucent box cannot erase what
    /// is beneath it). Border contributions and links the layer added
    /// merge in; the layer clearing them does not clear the backdrop's.
    pub(crate) fn composite_group(&mut self, layer: &Buffer, alpha: f32) {
        debug_assert_eq!(self.area, layer.area);
        let canvas = Color::Rgb(0, 0, 0);
        for i in 0..self.content.len().min(layer.content.len()) {
            let before = &self.content[i];
            let after = &layer.content[i];
            if before == after {
                // Side tables may still differ (a border added in the
                // layer with no cell change).
            } else {
                let backdrop = if before.bg == Color::Reset {
                    canvas
                } else {
                    before.bg
                };
                let mut out = before.clone();
                if after.bg != before.bg {
                    out.bg = alpha_blend(after.bg, alpha, backdrop);
                }
                let painted_glyph = after.raw_symbol() != before.raw_symbol()
                    && after
                        .raw_symbol()
                        .is_some_and(|s| !s.is_empty() && !s.chars().all(char::is_whitespace));
                if painted_glyph {
                    out.set_symbol(after.symbol());
                    out.modifier = after.modifier;
                    out.diff = after.diff;
                    out.fg = alpha_blend(after.fg, alpha, backdrop);
                } else if after.fg != before.fg && before.raw_symbol().is_some() {
                    // A style-only write over an existing glyph.
                    out.fg = alpha_blend(after.fg, alpha, backdrop);
                }
                if after.link != before.link && after.link.is_some() {
                    out.set_link(after.link.as_deref());
                }
                self.content[i] = out;
            }
            for dir in 0..4 {
                let added = layer.border_dirs[i][dir];
                if added != self.border_dirs[i][dir] && added != BorderDirState::default() {
                    self.border_dirs[i][dir] = added;
                }
            }
            self.half_block_quads[i] |= layer.half_block_quads[i];
        }
    }
}
