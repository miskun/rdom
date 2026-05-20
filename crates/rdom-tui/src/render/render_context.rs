//! `RenderContext` — bounded paint access for custom-rendered elements.
//!
//! Some elements need to paint themselves in response to a `render`
//! event rather than declaratively via text + styles. Examples:
//!
//! - Markdown element — parses its text content and emits styled
//!   runs + code blocks + headings
//! - Time-series chart — draws a sparkline from a data array
//! - Virtual-table row — cheaply fills only the visible rows from
//!   a large backing store
//!
//! The contract: an element's `render` listener gets a `RenderContext`
//! that exposes **only** the cells within its own `content_layout`
//! rect. All coordinates are **relative to that rect** (not absolute
//! grid coords) so elements can compose without thinking about their
//! position. All writes are clamped to the rect's bounds.
//!
//! This is the safe, borrow-checked replacement for legacy rdom's
//! `unsafe` raw-pointer render pattern.

use super::{Buffer, Cell, Rect, Style};

/// Safe, bounded access to a `Buffer` for a single element's paint.
///
/// The backing buffer covers the full terminal grid; this context
/// restricts writes to `self.area`. Coordinates in all methods are
/// **relative to `area.{x, y}`** — `(0, 0)` is the top-left cell of
/// the element's content rect. `scroll` lets the element know how
/// much of its own virtual content sits above/left of `area` so it
/// can skip drawing off-screen parts.
pub struct RenderContext<'a> {
    /// The grid rect this element owns this frame. Relative
    /// coordinates map to this rect's origin.
    area: Rect,
    /// The shared back-buffer being painted.
    buf: &'a mut Buffer,
    /// How much of the element's virtual content is hidden above
    /// (`scroll.1`) or left of (`scroll.0`) `area` due to scrolling.
    /// Elements that skip painting off-screen content use this.
    scroll: (u16, u16),
}

impl<'a> RenderContext<'a> {
    /// Construct. Typically only the Terminal / paint pass creates
    /// these; application code receives them via render-event callbacks.
    pub fn new(area: Rect, buf: &'a mut Buffer, scroll: (u16, u16)) -> Self {
        Self { area, buf, scroll }
    }

    /// The element's allocated area in absolute grid coords.
    pub fn area(&self) -> Rect {
        self.area
    }

    /// `(scroll_x, scroll_y)` — how many cells of the element's own
    /// content are above/left of `area`.
    pub fn scroll(&self) -> (u16, u16) {
        self.scroll
    }

    /// Width of the visible area in cells.
    pub fn width(&self) -> u16 {
        self.area.width
    }

    /// Height of the visible area in cells.
    pub fn height(&self) -> u16 {
        self.area.height
    }

    /// Convert relative (rel_x, rel_y) to absolute grid coords.
    /// Returns `None` if out of `area` bounds.
    fn to_grid(&self, rel_x: u16, rel_y: u16) -> Option<(u16, u16)> {
        if rel_x >= self.area.width || rel_y >= self.area.height {
            return None;
        }
        Some((
            self.area.x.saturating_add(rel_x),
            self.area.y.saturating_add(rel_y),
        ))
    }

    // ─── Writes ──────────────────────────────────────────────────────

    /// Write `cell` at relative position. Out-of-bounds silently skipped.
    pub fn set_cell(&mut self, rel_x: u16, rel_y: u16, cell: Cell) {
        if let Some((x, y)) = self.to_grid(rel_x, rel_y)
            && let Some(slot) = self.buf.cell_mut(x, y)
        {
            *slot = cell;
        }
    }

    /// Write a single char at relative position with `style`.
    pub fn set_char(&mut self, rel_x: u16, rel_y: u16, ch: char, style: Style) {
        if let Some((x, y)) = self.to_grid(rel_x, rel_y) {
            self.buf.set_char(x, y, ch, style);
        }
    }

    /// Write a string at relative position. Grapheme-walk + unicode-
    /// width aware (inherits Buffer's behavior). Truncates at the
    /// right edge of `area` with `…` for wide-glyph clips.
    /// Returns the relative cursor position after the write.
    pub fn set_string(&mut self, rel_x: u16, rel_y: u16, s: &str, style: Style) -> (u16, u16) {
        if rel_y >= self.area.height || rel_x >= self.area.width {
            return (rel_x, rel_y);
        }
        let x = self.area.x.saturating_add(rel_x);
        let y = self.area.y.saturating_add(rel_y);
        // Cap the write's width to the remaining room in area.
        let max = self.area.width.saturating_sub(rel_x);
        let (end_x, _) = self.buf.set_stringn(x, y, s, max, style);
        (end_x.saturating_sub(self.area.x), rel_y)
    }

    /// Fill a relative rect with `cell`. Clipped to `area`.
    pub fn fill(&mut self, rel_area: Rect, cell: Cell) {
        // Translate to absolute, intersect with our area.
        let abs = Rect::new(
            self.area.x.saturating_add(rel_area.x),
            self.area.y.saturating_add(rel_area.y),
            rel_area.width,
            rel_area.height,
        );
        self.buf.fill(self.area.intersection(abs), cell);
    }

    /// Apply a style to a single cell (symbol unchanged).
    pub fn set_style(&mut self, rel_x: u16, rel_y: u16, style: Style) {
        if let Some((x, y)) = self.to_grid(rel_x, rel_y) {
            self.buf.set_style(x, y, style);
        }
    }

    /// Borrow the underlying buffer for read-only introspection.
    /// Writes still clamp to `area` through the methods above; this
    /// exists for elements that want to peek at cells they wrote.
    pub fn buffer(&self) -> &Buffer {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Color, Modifier};

    fn buf() -> Buffer {
        Buffer::empty(Rect::new(0, 0, 10, 5))
    }

    #[test]
    fn area_and_scroll_expose_input() {
        let mut b = buf();
        let ctx = RenderContext::new(Rect::new(2, 1, 5, 3), &mut b, (4, 7));
        assert_eq!(ctx.area(), Rect::new(2, 1, 5, 3));
        assert_eq!(ctx.scroll(), (4, 7));
        assert_eq!(ctx.width(), 5);
        assert_eq!(ctx.height(), 3);
    }

    #[test]
    fn set_char_relative_to_area_origin() {
        let mut b = buf();
        {
            let mut ctx = RenderContext::new(Rect::new(2, 1, 5, 3), &mut b, (0, 0));
            ctx.set_char(0, 0, 'A', Style::new().fg(Color::Rgb(255, 0, 0)));
        }
        assert_eq!(b.cell(2, 1).unwrap().symbol(), "A");
        assert_eq!(b.cell(2, 1).unwrap().fg, Color::Rgb(255, 0, 0));
        // Outside the area not touched.
        assert_eq!(b.cell(0, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(1, 1).unwrap().symbol(), " ");
    }

    #[test]
    fn set_char_out_of_area_is_noop() {
        let mut b = buf();
        {
            let mut ctx = RenderContext::new(Rect::new(2, 1, 3, 2), &mut b, (0, 0));
            // rel_x=3 == area.width → out of bounds
            ctx.set_char(3, 0, 'X', Style::new());
            // rel_y=2 == area.height → out of bounds
            ctx.set_char(0, 2, 'Y', Style::new());
        }
        // Corresponding absolute cells (5, 1) and (2, 3) must NOT be
        // touched.
        assert_eq!(b.cell(5, 1).unwrap().symbol(), " ");
        assert_eq!(b.cell(2, 3).unwrap().symbol(), " ");
    }

    #[test]
    fn set_string_clips_at_area_right_edge() {
        let mut b = buf();
        {
            let mut ctx = RenderContext::new(Rect::new(0, 0, 5, 1), &mut b, (0, 0));
            // "helloworld" is 10 chars, area width is 5 → only "hello" fits.
            let end = ctx.set_string(0, 0, "helloworld", Style::new());
            assert_eq!(end, (5, 0));
        }
        for x in 0..5 {
            assert_eq!(
                b.cell(x, 0).unwrap().symbol(),
                &("hello"[x as usize..x as usize + 1])
            );
        }
        // Cell 5 (outside area) untouched.
        assert_eq!(b.cell(5, 0).unwrap().symbol(), " ");
    }

    #[test]
    fn set_string_cjk_width_respected() {
        // "中国" = 4 cells; area width 3 → only "中" fits (2 cells),
        // then the next wide glyph gets replaced with `…` in cell 2.
        let mut b = buf();
        {
            let mut ctx = RenderContext::new(Rect::new(0, 0, 3, 1), &mut b, (0, 0));
            let end = ctx.set_string(0, 0, "中国", Style::new());
            assert_eq!(end, (3, 0));
        }
        assert_eq!(b.cell(0, 0).unwrap().symbol(), "中");
        assert!(b.cell(1, 0).unwrap().is_spacer());
        assert_eq!(b.cell(2, 0).unwrap().symbol(), "…");
    }

    #[test]
    fn fill_clamps_to_area() {
        let mut b = buf();
        {
            let mut ctx = RenderContext::new(Rect::new(2, 1, 3, 2), &mut b, (0, 0));
            // Relative fill over 100×100 — should only affect area's 3×2.
            ctx.fill(Rect::new(0, 0, 100, 100), Cell::new("#"));
        }
        // Inside area: filled.
        for y in 1..3 {
            for x in 2..5 {
                assert_eq!(b.cell(x, y).unwrap().symbol(), "#");
            }
        }
        // Outside area: untouched.
        assert_eq!(b.cell(0, 0).unwrap().symbol(), " ");
        assert_eq!(b.cell(5, 1).unwrap().symbol(), " ");
        assert_eq!(b.cell(2, 3).unwrap().symbol(), " ");
    }

    #[test]
    fn set_cell_composes_everything() {
        let mut b = buf();
        let mut my_cell = Cell::new("Z");
        my_cell.fg = Color::Rgb(0, 0, 255);
        my_cell.bg = Color::Rgb(255, 0, 0);
        my_cell.modifier = Modifier::BOLD;
        {
            let mut ctx = RenderContext::new(Rect::new(1, 1, 3, 3), &mut b, (0, 0));
            ctx.set_cell(1, 1, my_cell.clone());
        }
        let placed = b.cell(2, 2).unwrap();
        assert_eq!(placed.symbol(), "Z");
        assert_eq!(placed.fg, Color::Rgb(0, 0, 255));
        assert_eq!(placed.bg, Color::Rgb(255, 0, 0));
        assert_eq!(placed.modifier, Modifier::BOLD);
    }

    #[test]
    fn set_style_preserves_symbol() {
        let mut b = buf();
        b.set_char(3, 2, 'X', Style::new().fg(Color::Rgb(255, 255, 255)));
        {
            let mut ctx = RenderContext::new(Rect::new(3, 2, 1, 1), &mut b, (0, 0));
            ctx.set_style(0, 0, Style::new().fg(Color::Rgb(255, 0, 0)));
        }
        let c = b.cell(3, 2).unwrap();
        assert_eq!(c.symbol(), "X"); // symbol unchanged
        assert_eq!(c.fg, Color::Rgb(255, 0, 0)); // style applied
    }

    #[test]
    fn elements_cannot_paint_outside_their_rect() {
        // Even if an element tries to write at huge offsets, nothing
        // outside `area` gets touched. Sandbox invariant.
        let mut b = buf();
        {
            let mut ctx = RenderContext::new(Rect::new(2, 1, 3, 2), &mut b, (0, 0));
            // Large relative offsets are clamped out.
            ctx.set_char(100, 100, '!', Style::new());
            ctx.set_string(50, 50, "escape", Style::new());
        }
        // Every cell outside area is still blank.
        let blank = buf();
        for y in 0..5 {
            for x in 0..10 {
                // Only the cells we didn't write at all should match blank.
                if !((2..5).contains(&x) && (1..3).contains(&y)) {
                    assert_eq!(b.cell(x, y).unwrap(), blank.cell(x, y).unwrap());
                }
            }
        }
    }

    #[test]
    fn buffer_accessor_is_readable() {
        let mut b = buf();
        b.set_char(0, 0, 'Q', Style::new());
        let ctx = RenderContext::new(Rect::new(2, 2, 3, 3), &mut b, (0, 0));
        // Outside our area but inside the shared buffer — we can READ it.
        let buffer_snapshot = ctx.buffer();
        assert_eq!(buffer_snapshot.cell(0, 0).unwrap().symbol(), "Q");
    }
}
