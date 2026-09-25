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
//!   - Color codes: `30-37`, `90-97` (ANSI-16); `38;5;N` (Indexed);
//!     `38;2;R;G;B` (Rgb); `39` (reset fg). Same shape with 40+ for bg.
//!   - Modifier codes: `1` bold, `2` dim, `3` italic, `4` underline,
//!     `5`/`6` blink, `7` reversed, `8` hidden, `9` crossed-out
//!   - Off codes: `0` full reset, `22` bold+dim off, `23`, `24`, `25`,
//!     `27`, `28`, `29`
//! - **Cursor hide/show** `\x1b[?25l` / `\x1b[?25h` — tracked as a bool
//! - **Synchronized output** `\x1b[?2026h/l` — silently ignored
//!   (no visible effect in the grid)
//! - **Plain chars** — written at cursor, advances by
//!   `UnicodeWidthStr::width` (wide glyphs → primary + spacer cell)
//!
//! Unsupported sequences are silently skipped — this is a test utility,
//! not a full VT100 emulator. The set above covers everything our
//! backend emits.
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
            self.cursor.1 = self
                .cursor
                .1
                .saturating_add(1)
                .min(self.height.saturating_sub(1));
            self.cursor.0 = 0;
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
        let (x, y) = self.cursor;
        if y >= self.height || x >= self.width {
            return;
        }

        // Write primary cell.
        let mut cell = Cell::new(g);
        cell.fg = self.sgr.fg;
        cell.bg = self.sgr.bg;
        cell.modifier = self.sgr.modifier;
        let i = y as usize * self.width as usize + x as usize;
        self.cells[i] = cell;

        if width == 2 && x + 1 < self.width {
            let spacer_i = y as usize * self.width as usize + (x + 1) as usize;
            let mut spacer = Cell::EMPTY;
            spacer.fg = self.sgr.fg;
            spacer.bg = self.sgr.bg;
            spacer.modifier = self.sgr.modifier;
            spacer.set_spacer();
            self.cells[spacer_i] = spacer;
        }

        // Advance cursor.
        self.cursor.0 = (x + width).min(self.width);
    }
}
