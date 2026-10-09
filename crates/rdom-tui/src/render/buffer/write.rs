//! Cell and string writes: single-cell symbol / style / link writes
//! and the Unicode-width-aware `set_string` family with wide-glyph
//! spacers and right-edge ellipsis clipping.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::Buffer;
use crate::render::{Rect, Style};

/// Horizontal ellipsis used when a wide glyph is clipped.
const WIDE_CLIP_PLACEHOLDER: &str = "…";

impl Buffer {
    // ── Single-cell writes ────────────────────────────────────────────

    /// Write `symbol` at `(x, y)` with `style`. Grapheme-agnostic —
    /// use `set_string` if `symbol` may be multi-byte and you want
    /// automatic wide-glyph spacer handling.
    pub fn set_symbol(&mut self, x: u16, y: u16, symbol: &str, style: Style) {
        self.write_styled(Rect::new(x, y, 1, 1), style, |buf, style| {
            if let Some(c) = buf.cell_mut(x, y) {
                put_glyph(c, symbol, style);
            }
            buf.mark_at(x, y, painted(style));
        });
    }

    /// Write a single `char` at `(x, y)`. For multi-codepoint graphemes
    /// or strings, use `set_string`.
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: Style) {
        let mut buf = [0u8; 4];
        let s: &str = ch.encode_utf8(&mut buf);
        self.set_symbol(x, y, s, style);
    }

    /// Write just the style at `(x, y)`, preserving whatever symbol
    /// is currently there.
    pub fn set_style(&mut self, x: u16, y: u16, style: Style) {
        self.write_styled(Rect::new(x, y, 1, 1), style, |buf, style| {
            if let Some(c) = buf.cell_mut(x, y) {
                c.apply_style(style);
            }
            buf.mark_at(x, y, painted(style));
        });
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
        self.write_text(x, y, s, max_width, style, false)
    }

    /// [`Self::set_stringn`] for the paint pass, which also clears the
    /// border state of every cell a glyph takes (`occlude_borders`):
    /// content paints over the borders painted before it. A translucent
    /// style composites ([`Self::write_styled`]).
    pub(crate) fn write_text(
        &mut self,
        x: u16,
        y: u16,
        s: &str,
        max_width: u16,
        style: Style,
        occlude_borders: bool,
    ) -> (u16, u16) {
        let span = Rect::new(x, y, max_width.min(self.area.right().saturating_sub(x)), 1);
        self.write_styled(span, style, |buf, style| {
            buf.write_text_opaque(x, y, s, max_width, style, occlude_borders)
        })
        .unwrap_or((x, y))
    }

    /// The write itself, in a style whose colors paint as they are.
    fn write_text_opaque(
        &mut self,
        x: u16,
        y: u16,
        s: &str,
        max_width: u16,
        style: Style,
        occlude_borders: bool,
    ) -> (u16, u16) {
        let occlude = occlude_borders && !style.hides_glyph();
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
                    if let Some(c) = self.cell_mut(cursor_x, y) {
                        put_glyph(c, WIDE_CLIP_PLACEHOLDER, style);
                    }
                    self.mark_at(cursor_x, y, painted(style));
                    if occlude {
                        self.clear_border_at(cursor_x, y);
                    }
                    cursor_x = cursor_x.saturating_add(1);
                }
                break;
            }

            // Write the primary cell.
            if let Some(c) = self.cell_mut(cursor_x, y) {
                put_glyph(c, grapheme, style);
            }
            self.mark_at(cursor_x, y, painted(style));

            // For width-2 glyphs, write the trailing spacer.
            if w == 2 {
                let spacer_x = cursor_x.saturating_add(1);
                if let Some(c) = self.cell_mut(spacer_x, y) {
                    put_glyph(c, "", style);
                }
                self.mark_at(spacer_x, y, painted(style));
            }
            if occlude {
                for dx in 0..w {
                    self.clear_border_at(cursor_x.saturating_add(dx), y);
                }
            }

            cursor_x = cursor_x.saturating_add(w);
            cells_written += w;
        }

        (cursor_x, y)
    }
}

/// Write `symbol` (`""` for a wide glyph's spacer) into `cell` with
/// `style`. A glyph in a fully transparent color is invisible (CSS
/// Color 4 §6.3): the cell keeps its glyph, and only the style's
/// background paints.
fn put_glyph(cell: &mut crate::render::Cell, symbol: &str, style: Style) {
    if style.hides_glyph() {
        cell.apply_style(style.background_only());
        return;
    }
    // The new glyph shows in its own style: what the replaced glyph had —
    // its colour, weight, decorations — goes with it (ACID-FIX-3); the
    // background stays the boxes'.
    cell.set_symbol(symbol);
    cell.clear_glyph_style();
    cell.apply_style(style);
}

/// What a write in `style` paints of a cell (`coverage`): its glyph, and
/// its background when the style sets one.
fn painted(style: Style) -> u8 {
    use super::coverage::{BG, GLYPH};
    if style.bg.is_some() {
        GLYPH | BG
    } else {
        GLYPH
    }
}
