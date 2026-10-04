//! Translucent paints (CSS Color 4 §4.2, C3-ALPHA). A cell is opaque,
//! so a paint in a color with alpha below opaque is made at full
//! opacity into a layer — a copy of the cells it covers — and the
//! layer composites back at the color's alpha by the group-opacity
//! rules (`composite.rs`): a translucent background blends with the
//! one beneath and tints the glyphs it leaves; a translucent glyph
//! contests the backdrop's and blends with the background; a
//! translucent border blends like a glyph. `opacity` and color alpha
//! share one set of per-cell rules.

use super::Buffer;
use crate::render::{Rect, Style};
use crate::style::Color;

/// `c`'s alpha as a fraction.
fn fraction(c: Color) -> f32 {
    f32::from(c.alpha()) / 255.0
}

/// True when `c` is neither opaque nor fully transparent.
fn translucent(c: Color) -> bool {
    (1..u8::MAX).contains(&c.alpha())
}

impl Buffer {
    /// Paint `paint` into a copy of `area` at full opacity, then
    /// composite it back at `alpha`.
    pub(crate) fn paint_translucent(
        &mut self,
        area: Rect,
        alpha: f32,
        paint: impl FnOnce(&mut Buffer),
    ) {
        if alpha <= 0.0 {
            return;
        }
        let mut layer = self.copy_region(area);
        if layer.area.is_empty() {
            return;
        }
        paint(&mut layer);
        self.composite_group(&layer, alpha);
    }

    /// Make a write in `style` over `span`: `write` paints into the
    /// buffer it is given with the style it is given. A translucent
    /// background is painted first, keeping the glyphs (a hidden-glyph
    /// write of the opaque background), then the glyph with its
    /// foreground — each translucent part through a layer at its own
    /// alpha. Returns `write`'s result for the glyph pass, `None` when a
    /// translucent glyph pass had no cell to paint.
    pub(crate) fn write_styled<T>(
        &mut self,
        span: Rect,
        style: Style,
        mut write: impl FnMut(&mut Buffer, Style) -> T,
    ) -> Option<T> {
        let mut style = style;
        if let Some(bg) = style.bg.filter(|c| translucent(*c)) {
            let background = Style::new().fg(Color::TRANSPARENT).bg(bg.opaque());
            self.paint_translucent(span, fraction(bg), |layer| {
                write(layer, background);
            });
            style.bg = None;
        }
        match style.fg.filter(|c| translucent(*c)) {
            Some(fg) => {
                let mut out = None;
                let opaque = Style {
                    fg: Some(fg.opaque()),
                    ..style
                };
                self.paint_translucent(span, fraction(fg), |layer| {
                    out = Some(write(layer, opaque));
                });
                out
            }
            None => Some(write(self, style)),
        }
    }

    /// Set the background of every cell of `area`, keeping the glyphs
    /// (a tint: a modal `::backdrop`, a highlighted row). A translucent
    /// background is a box over them: made as a blank fill in a layer,
    /// so compositing blends the background and tints the glyphs and
    /// borders it leaves toward it.
    pub(crate) fn tint(&mut self, area: Rect, bg: Color) {
        if bg.alpha() == 0 || bg == Color::Reset {
            return;
        }
        if translucent(bg) {
            let opaque = bg.opaque();
            self.paint_translucent(area, fraction(bg), |layer| {
                let region = layer.area;
                for y in region.y..region.bottom() {
                    for x in region.x..region.right() {
                        if let Some(cell) = layer.cell_mut(x, y) {
                            cell.reset();
                            cell.set_bg(opaque);
                        }
                        layer.clear_border_at(x, y);
                    }
                }
            });
            return;
        }
        let region = self.area.intersection(area);
        for y in region.y..region.bottom() {
            for x in region.x..region.right() {
                if let Some(cell) = self.cell_mut(x, y) {
                    cell.set_bg(bg);
                }
            }
        }
    }

    /// Set the foreground of every glyph in `area` (a modal
    /// `::backdrop`'s `color`); a translucent one blends through a layer.
    pub(crate) fn tint_glyphs(&mut self, area: Rect, fg: Color) {
        if fg.alpha() == 0 || fg == Color::Reset {
            return;
        }
        let set = |buf: &mut Buffer, fg: Color| {
            let region = buf.area.intersection(area);
            for y in region.y..region.bottom() {
                for x in region.x..region.right() {
                    if let Some(cell) = buf.cell_mut(x, y) {
                        cell.set_fg(fg);
                    }
                }
            }
        };
        if translucent(fg) {
            self.paint_translucent(area, fraction(fg), |layer| set(layer, fg.opaque()));
        } else {
            set(self, fg);
        }
    }
}
