//! `VirtualScreen` — pure-Rust terminal emulator for testing.
//!
//! Test-only: compiled for this crate's own unit tests and, for
//! downstream test suites, behind the `test-util` cargo feature.
//!
//! Consumes ANSI bytes (as emitted by our `CrosstermBackend` or any
//! other backend) and maintains a grid of `Cell`s representing what
//! the terminal actually shows. Tests can assert on the grid state
//! after any sequence of paint / resize / clear operations, catching
//! the class of "stale cell" / "terminal out of sync" bugs that only
//! show up when you watch a real terminal.
//!
//! ## Supported SGR / control sequences
//!
//! - **CUP** `\x1b[y;xH` — move cursor (1-indexed in, 0-indexed in
//!   our `(x, y)` model)
//! - **Clear screen** `\x1b[2J` — every cell → `Cell::EMPTY`
//! - **SGR** `\x1b[Nm`, `\x1b[N;M;...m` — fg/bg/modifier state
//!   - Color codes: `30-37`, `90-97` (ANSI-16, as the palette indices
//!     `Indexed(0..=15)` they select — what rdom emits at 16 colors,
//!     C16G-COLOR-DEPTH); `38;5;N` (Indexed);
//!     `38;2;R;G;B` (Rgb); `39` (reset fg). Same shape with 40+ for bg.
//!   - Modifier codes: `1` bold, `3` italic, `4` underline, `5`/`6`
//!     blink, `8` hidden, `9` crossed-out; `2` (dim) and `7` (reverse
//!     video) are accepted and change nothing — rdom has neither
//!     modifier, and emits neither
//!   - Off codes: `0` full reset, `22` bold off, `23`, `24`, `25`, `27`,
//!     `28`, `29`
//!   - Decorations: `4:0`–`4:5` (underline off / styles), `53` / `55`
//!     overline, `58:2::R:G:B` / `58:5:N` (and their `;` forms) / `59`
//!     underline color
//! - **Cursor hide/show** `\x1b[?25l` / `\x1b[?25h` — tracked as a bool
//! - **OSC** `\x1b]…` to ST (`\x1b\\`) or BEL — consumed, never
//!   printed; OSC 8 (`\x1b]8;;URI`) sets the link (`Cell::link`) of the
//!   cells written until an empty URI closes it
//! - **Synchronized output** `\x1b[?2026h/l` — silently ignored
//!   (no visible effect in the grid)
//! - **Plain chars** — written at cursor, advances by
//!   `UnicodeWidthStr::width` (wide glyphs → primary + spacer cell)
//!
//! - **Autowrap** (xterm's default `DECAWM`), deferred: a glyph written
//!   in the last column leaves the cursor past it, and the next glyph
//!   wraps to the next row — scrolling the screen up from the last row,
//!   as a line feed there does; a two-cell glyph that does not fit before
//!   the edge wraps whole. A CUP cancels the pending wrap.
//! - **A wide glyph partly overwritten** is erased whole: its other half
//!   becomes a blank keeping its own colours.
//!
//! Every other sequence — an unknown CSI (erase in line, cursor forward,
//! a scroll region, a partial erase in display, a mode that is not
//! private), an unknown SGR code or escape — changes nothing and is
//! recorded in [`VirtualScreen::unsupported`], so a test can assert the
//! stream held nothing the screen could not show (C16G-ACID-RIGOUR). This
//! is a test utility, not a full VT100 emulator; the set above covers
//! everything rdom's backends emit.
//!
//! ## Layout
//!
//! - this file — the screen model: grid, cursor, SGR state, resize,
//!   and grapheme writes at the cursor.
//! - `parse` — the ANSI byte-stream parser (CSI / SGR decoding) that
//!   drives the model.

use unicode_width::UnicodeWidthStr;

use super::Cell;
use super::sgr::SgrState;

mod parse;

#[cfg(test)]
mod tests;

/// Headless model of a terminal. Apply ANSI bytes; inspect the grid.
pub struct VirtualScreen {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
    cursor: (u16, u16),
    sgr: SgrState,
    cursor_visible: bool,
    /// The OSC 8 hyperlink open at the cursor, which the cells written
    /// next carry.
    link: Option<Box<str>>,
    /// The sequences applied that the screen does not model, in order.
    unsupported: Vec<String>,
}

impl VirtualScreen {
    /// Construct a blank screen of the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![Cell::EMPTY; len],
            cursor: (0, 0),
            sgr: SgrState::default(),
            cursor_visible: true,
            link: None,
            unsupported: Vec::new(),
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Current cursor position (0-indexed).
    pub fn cursor(&self) -> (u16, u16) {
        self.cursor
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Current SGR state.
    pub fn sgr(&self) -> SgrState {
        self.sgr
    }

    /// The sequences applied so far that the screen does not model — each
    /// as `CSI <params><final>`, `SGR <code>` or `ESC <byte>` — so a test
    /// can assert the stream held nothing it could not show.
    pub fn unsupported(&self) -> &[String] {
        &self.unsupported
    }

    /// Look up a cell. `None` if out of bounds.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = y as usize * self.width as usize + x as usize;
        self.cells.get(i)
    }

    /// Row `y` as a plain `String`. Useful for snapshot assertions:
    /// `assert_eq!(screen.row(0).trim_end(), "Hello")`.
    pub fn row(&self, y: u16) -> String {
        if y >= self.height {
            return String::new();
        }
        let mut out = String::new();
        for x in 0..self.width {
            let c = self.cell(x, y).expect("in-bounds");
            if c.is_spacer() {
                continue; // absorbed into the previous wide glyph
            }
            out.push_str(c.symbol());
        }
        out
    }

    /// Every row as a `Vec<String>`. Handy for formatted error messages
    /// in failing tests.
    pub fn rows(&self) -> Vec<String> {
        (0..self.height).map(|y| self.row(y)).collect()
    }

    /// Resize the virtual screen. Cells in the intersection preserved;
    /// new cells are `Cell::EMPTY`. Cursor clamped into new bounds.
    pub fn resize(&mut self, width: u16, height: u16) {
        let mut new = vec![Cell::EMPTY; width as usize * height as usize];
        let common_w = self.width.min(width);
        let common_h = self.height.min(height);
        for y in 0..common_h {
            for x in 0..common_w {
                let old_i = y as usize * self.width as usize + x as usize;
                let new_i = y as usize * width as usize + x as usize;
                new[new_i] = self.cells[old_i].clone();
            }
        }
        self.width = width;
        self.height = height;
        self.cells = new;
        self.cursor.0 = self.cursor.0.min(width.saturating_sub(1));
        self.cursor.1 = self.cursor.1.min(height.saturating_sub(1));
    }

    fn write_grapheme(&mut self, g: &str) {
        if g.is_empty() {
            return;
        }
        // Control chars — advance or just drop.
        if g == "\n" {
            self.next_row();
            return;
        }
        if g == "\r" {
            self.cursor.0 = 0;
            return;
        }
        if g.as_bytes().iter().all(|b| b.is_ascii_control()) {
            return;
        }

        let width = UnicodeWidthStr::width(g).max(1) as u16;
        // Autowrap (DECAWM, xterm's default), deferred: a cursor past the
        // last column wraps before the next glyph, and a two-cell glyph
        // that does not fit before the edge wraps whole.
        if self.cursor.0 >= self.width || (width == 2 && self.cursor.0 + 1 >= self.width) {
            self.next_row();
        }
        let (x, y) = self.cursor;
        if y >= self.height || x >= self.width {
            return;
        }
        // A wide glyph partly overwritten is erased whole.
        self.break_wide_at(x, y);
        if width == 2 {
            self.break_wide_at(x + 1, y);
        }

        // Write primary cell.
        let mut cell = Cell::new(g);
        cell.fg = self.sgr.fg;
        cell.bg = self.sgr.bg;
        cell.modifier = self.sgr.modifier;
        cell.underline_color = self.sgr.underline_color;
        cell.link = self.link.clone();
        let i = self.index(x, y);
        self.cells[i] = cell;

        if width == 2 {
            let spacer_i = self.index(x + 1, y);
            let mut spacer = Cell::EMPTY;
            spacer.fg = self.sgr.fg;
            spacer.bg = self.sgr.bg;
            spacer.modifier = self.sgr.modifier;
            spacer.underline_color = self.sgr.underline_color;
            spacer.link = self.link.clone();
            spacer.set_spacer();
            self.cells[spacer_i] = spacer;
        }

        // Advance the cursor; past the last column it waits to wrap.
        self.cursor.0 = x + width;
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// The cursor to the start of the next row, scrolling the screen up a
    /// row from the last.
    fn next_row(&mut self) {
        self.cursor.0 = 0;
        if self.cursor.1 + 1 < self.height {
            self.cursor.1 += 1;
            return;
        }
        let w = self.width as usize;
        self.cells.drain(..w.min(self.cells.len()));
        self.cells.extend(std::iter::repeat_n(Cell::EMPTY, w));
    }

    /// Before a glyph is written at `(x, y)`: when the cell there is half of
    /// a two-cell glyph, the other half becomes a blank keeping its own
    /// colours — a terminal erases a wide glyph that is partly overwritten.
    fn break_wide_at(&mut self, x: u16, y: u16) {
        if x >= self.width {
            return;
        }
        let here = self.index(x, y);
        let other = if self.cells[here].is_spacer() {
            x.checked_sub(1)
        } else if self.cells[here].cell_width() == 2 {
            (x + 1 < self.width).then_some(x + 1)
        } else {
            None
        };
        if let Some(ox) = other {
            let oi = self.index(ox, y);
            let old = &self.cells[oi];
            let mut blank = Cell::EMPTY;
            blank.fg = old.fg;
            blank.bg = old.bg;
            blank.modifier = old.modifier;
            blank.underline_color = old.underline_color;
            blank.link = old.link.clone();
            self.cells[oi] = blank;
        }
    }

    /// Record a sequence the screen does not model.
    fn unsupported_sequence(&mut self, what: String) {
        self.unsupported.push(what);
    }
}
