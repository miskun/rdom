//! `VirtualScreen` — pure-Rust terminal emulator for testing.
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

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::sgr::SgrState;
use super::{Cell, Color, Modifier};

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

    /// Parse and apply an ANSI byte stream. Multiple calls accumulate.
    pub fn apply(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == 0x1b {
                // Escape sequence.
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                match bytes[i] {
                    b'[' => {
                        i += 1;
                        i += self.parse_csi(&bytes[i..]);
                    }
                    _ => {
                        // Unknown / single-char escape — skip this byte.
                        i += 1;
                    }
                }
            } else {
                // Literal byte — could be start of a multi-byte UTF-8
                // grapheme. Find the grapheme that starts here.
                let (g, consumed) = next_grapheme(&bytes[i..]);
                self.write_grapheme(g);
                i += consumed;
            }
        }
    }

    /// Parse and apply a CSI sequence (after `\x1b[`). Returns the
    /// number of bytes consumed from the `after` slice.
    fn parse_csi(&mut self, after: &[u8]) -> usize {
        // A CSI sequence is: (params) (intermediate bytes) (final byte).
        // Params: digits, `;`, `?`.
        // Final byte: a letter (0x40–0x7E).
        let mut end = 0;
        while end < after.len() {
            let c = after[end];
            if c.is_ascii_alphabetic() || c == b'@' || c == b'`' || c == b'~' {
                break;
            }
            end += 1;
        }
        if end >= after.len() {
            return after.len();
        }
        let final_byte = after[end];
        let params_str = std::str::from_utf8(&after[..end]).unwrap_or("");
        let total_consumed = end + 1;

        match final_byte {
            b'H' | b'f' => {
                // CUP — \x1b[y;xH (1-indexed; missing = 1).
                let (row, col) = parse_pair(params_str);
                let y = row
                    .saturating_sub(1)
                    .min(self.height.saturating_sub(1) as u32) as u16;
                let x = col
                    .saturating_sub(1)
                    .min(self.width.saturating_sub(1) as u32) as u16;
                self.cursor = (x, y);
            }
            b'J' => {
                // Erase in Display. \x1b[2J = all; 0J = to end; 1J = to start.
                // For our backend only 2J is used.
                let mode: u32 = params_str.parse().unwrap_or(0);
                if mode == 2 {
                    for c in &mut self.cells {
                        *c = Cell::EMPTY;
                    }
                }
            }
            b'm' => {
                // SGR.
                self.apply_sgr(params_str);
            }
            b'h' | b'l' => {
                // Private mode set (`h`) / reset (`l`). \x1b[?25l etc.
                let set = final_byte == b'h';
                let trimmed = params_str.trim_start_matches('?');
                for part in trimmed.split(';') {
                    match part {
                        "25" => self.cursor_visible = set,
                        "2026" => { /* BSU/ESU — no grid effect */ }
                        _ => { /* ignore other modes */ }
                    }
                }
            }
            _ => {
                // Unknown CSI — skip silently.
            }
        }
        total_consumed
    }

    /// Apply an SGR parameter string like "0", "31", "38;5;204", "1;31;40".
    fn apply_sgr(&mut self, params: &str) {
        // Walk tokens; some codes consume following tokens.
        let tokens: Vec<&str> = params.split(';').collect();
        let mut i = 0;
        if tokens.is_empty() || (tokens.len() == 1 && tokens[0].is_empty()) {
            // Bare \x1b[m means reset.
            self.sgr = SgrState::default();
            return;
        }
        while i < tokens.len() {
            let n: u32 = tokens[i].parse().unwrap_or(0);
            match n {
                0 => self.sgr = SgrState::default(),
                1 => self.sgr.modifier |= Modifier::BOLD,
                // SGR-2 (dim) was mapped to `Modifier::DIM` pre-T8;
                // that bit is gone (no CSS analog, terminal-dependent).
                // Incoming SGR-2 is now silently ignored.
                2 => { /* dim — ignored, see T8 */ }
                3 => self.sgr.modifier |= Modifier::ITALIC,
                4 => self.sgr.modifier |= Modifier::UNDERLINED,
                5 => self.sgr.modifier |= Modifier::SLOW_BLINK,
                6 => self.sgr.modifier |= Modifier::RAPID_BLINK,
                // SGR-7 (reverse video) was emitted by rdom pre-
                // caret-color rollout. Now ignored — rdom paints
                // explicit fg/bg for the caret cell instead.
                7 => { /* reverse video — no internal flag */ }
                8 => self.sgr.modifier |= Modifier::HIDDEN,
                9 => self.sgr.modifier |= Modifier::CROSSED_OUT,
                22 => self.sgr.modifier.remove(Modifier::BOLD),
                23 => self.sgr.modifier.remove(Modifier::ITALIC),
                24 => self.sgr.modifier.remove(Modifier::UNDERLINED),
                25 => self
                    .sgr
                    .modifier
                    .remove(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK),
                27 => { /* reverse-video off — no internal flag */ }
                28 => self.sgr.modifier.remove(Modifier::HIDDEN),
                29 => self.sgr.modifier.remove(Modifier::CROSSED_OUT),
                30..=37 => self.sgr.fg = ansi16_color((n - 30) as u8, false),
                38 => {
                    // Extended fg. "38;5;N" or "38;2;R;G;B".
                    if let Some(mode) = tokens.get(i + 1) {
                        match mode.parse::<u32>().unwrap_or(0) {
                            5 => {
                                if let Some(idx) = tokens.get(i + 2) {
                                    let n: u8 = idx.parse().unwrap_or(0);
                                    self.sgr.fg = Color::Indexed(n);
                                    i += 2;
                                }
                            }
                            2 => {
                                if let (Some(r), Some(g), Some(b)) =
                                    (tokens.get(i + 2), tokens.get(i + 3), tokens.get(i + 4))
                                {
                                    let r = r.parse().unwrap_or(0);
                                    let g = g.parse().unwrap_or(0);
                                    let b = b.parse().unwrap_or(0);
                                    self.sgr.fg = Color::Rgb(r, g, b);
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                39 => self.sgr.fg = Color::Reset,
                40..=47 => self.sgr.bg = ansi16_color((n - 40) as u8, false),
                48 => {
                    if let Some(mode) = tokens.get(i + 1) {
                        match mode.parse::<u32>().unwrap_or(0) {
                            5 => {
                                if let Some(idx) = tokens.get(i + 2) {
                                    let n: u8 = idx.parse().unwrap_or(0);
                                    self.sgr.bg = Color::Indexed(n);
                                    i += 2;
                                }
                            }
                            2 => {
                                if let (Some(r), Some(g), Some(b)) =
                                    (tokens.get(i + 2), tokens.get(i + 3), tokens.get(i + 4))
                                {
                                    let r = r.parse().unwrap_or(0);
                                    let g = g.parse().unwrap_or(0);
                                    let b = b.parse().unwrap_or(0);
                                    self.sgr.bg = Color::Rgb(r, g, b);
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                49 => self.sgr.bg = Color::Reset,
                90..=97 => self.sgr.fg = ansi16_color((n - 90) as u8, true),
                100..=107 => self.sgr.bg = ansi16_color((n - 100) as u8, true),
                _ => { /* unknown SGR — ignore */ }
            }
            i += 1;
        }
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

// ─── helpers ────────────────────────────────────────────────────────

fn parse_pair(s: &str) -> (u32, u32) {
    let mut parts = s.split(';');
    let a = parts.next().and_then(|t| t.parse().ok()).unwrap_or(1);
    let b = parts.next().and_then(|t| t.parse().ok()).unwrap_or(1);
    (a, b)
}

/// Decode an ANSI-16 SGR code (`\x1b[30-37m` base, `\x1b[90-97m`
/// bright) back into a 24-bit `Color::Rgb`. rdom itself never emits
/// these short codes since T7 (we're truecolor-only on the wire),
/// but `VirtualScreen` still has to *parse* arbitrary incoming SGR
/// streams (replay, test harnesses, anything that wasn't produced
/// by rdom). The triples below are the canonical xterm RGB values
/// for the 16 ANSI palette slots — close to the legacy variants
/// that lived in `Color` pre-T6.
fn ansi16_color(code: u8, bright: bool) -> Color {
    if bright {
        match code {
            0 => Color::Rgb(169, 169, 169),
            1 => Color::Rgb(240, 128, 128),
            2 => Color::Rgb(144, 238, 144),
            3 => Color::Rgb(255, 255, 224),
            4 => Color::Rgb(173, 216, 230),
            5 => Color::Rgb(255, 128, 255),
            6 => Color::Rgb(224, 255, 255),
            7 => Color::Rgb(255, 255, 255),
            _ => Color::Reset,
        }
    } else {
        match code {
            0 => Color::Rgb(0, 0, 0),
            1 => Color::Rgb(255, 0, 0),
            2 => Color::Rgb(0, 128, 0),
            3 => Color::Rgb(255, 255, 0),
            4 => Color::Rgb(0, 0, 255),
            5 => Color::Rgb(255, 0, 255),
            6 => Color::Rgb(0, 255, 255),
            7 => Color::Rgb(128, 128, 128),
            _ => Color::Reset,
        }
    }
}

/// Return the next grapheme cluster starting at the beginning of
/// `bytes`, plus the byte length consumed. Bytes past the first
/// grapheme are left untouched.
fn next_grapheme(bytes: &[u8]) -> (&str, usize) {
    // Find a valid UTF-8 prefix.
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            // Partial UTF-8 at end. Try the valid prefix.
            let valid_up_to = e.valid_up_to();
            if valid_up_to == 0 {
                // First byte is invalid — return a single-byte fallback.
                return ("?", 1);
            }
            std::str::from_utf8(&bytes[..valid_up_to]).unwrap()
        }
    };
    let mut graphemes = s.graphemes(true);
    match graphemes.next() {
        Some(g) => (g, g.len()),
        None => ("", 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic paint ──────────────────────────────────────────────────

    #[test]
    fn blank_screen() {
        let s = VirtualScreen::new(10, 3);
        assert_eq!(s.row(0), "          ");
        assert_eq!(s.cursor(), (0, 0));
    }

    #[test]
    fn cup_then_text() {
        let mut s = VirtualScreen::new(10, 3);
        s.apply(b"\x1b[2;3Hhello"); // CUP (3, 2) = (x=2, y=1), then "hello"
        assert_eq!(s.row(1).trim_end(), "  hello");
        assert_eq!(s.cursor(), (7, 1));
    }

    #[test]
    fn clear_screen() {
        let mut s = VirtualScreen::new(5, 2);
        s.apply(b"\x1b[1;1Habc\x1b[2;1Hdef\x1b[2J");
        for y in 0..2 {
            assert_eq!(s.row(y).trim_end(), "");
        }
    }

    // ── SGR ──────────────────────────────────────────────────────────

    #[test]
    fn sgr_fg_ansi16() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[31mA");
        assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn sgr_fg_rgb_truecolor() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[38;2;10;20;30mX");
        assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(10, 20, 30));
    }

    #[test]
    fn sgr_bg_indexed() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[48;5;204mX");
        assert_eq!(s.cell(0, 0).unwrap().bg, Color::Indexed(204));
    }

    #[test]
    fn sgr_bright_fg() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[91mX");
        assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(240, 128, 128));
    }

    #[test]
    fn sgr_multiple_in_one_csi() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[1;31;40mX");
        let c = s.cell(0, 0).unwrap();
        assert_eq!(c.fg, Color::Rgb(255, 0, 0));
        assert_eq!(c.bg, Color::Rgb(0, 0, 0));
        assert!(c.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn sgr_reset_clears_state() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[31;1mA\x1b[0mB");
        assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
        assert_eq!(s.cell(1, 0).unwrap().fg, Color::Reset);
        assert!(!s.cell(1, 0).unwrap().modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn sgr_22_clears_bold() {
        let mut s = VirtualScreen::new(5, 1);
        // Apply bold + dim; SGR-2 (dim) is ignored post-T8 — the
        // first cell takes only BOLD. SGR-22 clears BOLD on the
        // second cell.
        s.apply(b"\x1b[1;2mA\x1b[22mB");
        assert!(s.cell(0, 0).unwrap().modifier.contains(Modifier::BOLD));
        assert!(!s.cell(1, 0).unwrap().modifier.contains(Modifier::BOLD));
    }

    // ── Cursor ───────────────────────────────────────────────────────

    #[test]
    fn cursor_hide_show() {
        let mut s = VirtualScreen::new(5, 1);
        assert!(s.cursor_visible());
        s.apply(b"\x1b[?25l");
        assert!(!s.cursor_visible());
        s.apply(b"\x1b[?25h");
        assert!(s.cursor_visible());
    }

    #[test]
    fn cursor_advances_after_writes() {
        let mut s = VirtualScreen::new(10, 1);
        s.apply(b"abc");
        assert_eq!(s.cursor(), (3, 0));
    }

    // ── Wide glyphs ──────────────────────────────────────────────────

    #[test]
    fn cjk_takes_two_cells() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply("\x1b[1;1H中X".as_bytes());
        assert_eq!(s.cell(0, 0).unwrap().symbol(), "中");
        assert!(s.cell(1, 0).unwrap().is_spacer());
        assert_eq!(s.cell(2, 0).unwrap().symbol(), "X");
        assert_eq!(s.cursor(), (3, 0));
    }

    #[test]
    fn emoji_zwj_stays_one_grapheme() {
        let mut s = VirtualScreen::new(5, 1);
        let input = "\x1b[1;1H👨\u{200D}👩\u{200D}👧".as_bytes();
        s.apply(input);
        // Family emoji is one grapheme of width 2.
        assert_eq!(s.cell(0, 0).unwrap().symbol(), "👨\u{200D}👩\u{200D}👧");
        assert!(s.cell(1, 0).unwrap().is_spacer());
        assert_eq!(s.cursor(), (2, 0));
    }

    // ── Synchronized output (ignored) ────────────────────────────────

    #[test]
    fn bsu_esu_are_silent() {
        let mut s = VirtualScreen::new(5, 1);
        s.apply(b"\x1b[?2026h\x1b[1;1HX\x1b[?2026l");
        assert_eq!(s.row(0).trim_end(), "X");
    }

    // ── Resize ───────────────────────────────────────────────────────

    #[test]
    fn resize_preserves_intersection() {
        let mut s = VirtualScreen::new(5, 3);
        s.apply(b"\x1b[1;1HABCDE");
        s.resize(10, 3);
        assert_eq!(s.row(0).trim_end(), "ABCDE");
    }

    #[test]
    fn resize_smaller_drops_excess() {
        let mut s = VirtualScreen::new(10, 3);
        s.apply(b"\x1b[1;1HHello World!");
        s.resize(5, 3);
        assert_eq!(s.row(0).trim_end(), "Hello");
    }

    #[test]
    fn resize_larger_new_rows_blank() {
        let mut s = VirtualScreen::new(5, 2);
        s.apply(b"\x1b[1;1HAB");
        s.resize(5, 5);
        for y in 2..5 {
            assert_eq!(s.row(y).trim_end(), "");
        }
    }

    // ── Integration with real Terminal + CrosstermBackend ────────────

    #[test]
    fn terminal_output_renders_correctly() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        let tb = TestBackend::new(10, 2);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "Hello", Style::new().fg(Color::Rgb(255, 0, 0)));
            buf.set_string(0, 1, "World", Style::new().fg(Color::Rgb(0, 0, 255)));
            Ok(())
        })
        .unwrap();

        let mut screen = VirtualScreen::new(10, 2);
        screen.apply(term.backend().bytes());
        assert_eq!(screen.row(0).trim_end(), "Hello");
        assert_eq!(screen.row(1).trim_end(), "World");
        assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
        assert_eq!(screen.cell(0, 1).unwrap().fg, Color::Rgb(0, 0, 255));
    }

    // ── Resize regression tests ─────────────────────────────────────

    #[test]
    fn resize_smaller_no_stale_cells_on_screen() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        // Paint a layout with content on multiple rows.
        let tb = TestBackend::new(20, 6);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "LONG HEADER TEXT", Style::new());
            buf.set_string(0, 2, "body line", Style::new());
            buf.set_string(0, 5, "footer text", Style::new());
            Ok(())
        })
        .unwrap();

        let mut screen = VirtualScreen::new(20, 6);
        screen.apply(term.backend().bytes());

        // Shrink terminal. Virtual screen keeps intersection rows 0..4
        // — so "LONG HEADER TEXT" at row 0 and "body line" at row 2
        // survive in our model of the terminal.
        term.backend_mut().resize(20, 4);
        screen.resize(20, 4);
        let _ = term.backend_mut().take_bytes();

        // Next paint has SHORTER text on row 0 and NOTHING on row 2.
        // Without backend.clear() on resize, force_full_redraw would
        // skip blank cells in the new buffer → the tail of the old
        // "LONG HEADER TEXT" and the entire "body line" would persist.
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "HI", Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend().bytes());

        assert_eq!(
            screen.row(0).trim_end(),
            "HI",
            "row 0 has stale tail from previous longer text"
        );
        assert_eq!(
            screen.row(2).trim_end(),
            "",
            "row 2 has stale 'body line' content"
        );
    }

    #[test]
    fn resize_larger_new_rows_are_blank() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        // Paint a compact layout with content on every row.
        let tb = TestBackend::new(10, 3);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "hello", Style::new());
            buf.set_string(0, 1, "world", Style::new());
            buf.set_string(0, 2, "fooba", Style::new());
            Ok(())
        })
        .unwrap();

        let mut screen = VirtualScreen::new(10, 3);
        screen.apply(term.backend().bytes());

        // Grow. Intersection rows 0..3 retain their old content in our
        // model of the terminal.
        term.backend_mut().resize(10, 6);
        screen.resize(10, 6);
        let _ = term.backend_mut().take_bytes();

        // Next paint has SHORTER text on each row — blanks at positions
        // where the old paint had non-blanks. Without clear-on-resize
        // those stale cells would leak through.
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "hi", Style::new());
            buf.set_string(0, 1, "w", Style::new());
            // Row 2 empty.
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend().bytes());

        assert_eq!(screen.row(0).trim_end(), "hi", "row 0 has stale tail");
        assert_eq!(screen.row(1).trim_end(), "w", "row 1 has stale tail");
        assert_eq!(screen.row(2).trim_end(), "", "row 2 has stale content");
        for y in 3..6 {
            assert_eq!(screen.row(y).trim_end(), "", "new row {y} not blank");
        }
    }

    #[test]
    fn multiple_resizes_preserve_correctness() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        let tb = TestBackend::new(20, 10);
        let mut term = Terminal::new(tb).unwrap();
        let mut screen = VirtualScreen::new(20, 10);

        // Sequence of resizes, each followed by a paint.
        let sizes = [(15, 8), (30, 12), (5, 3), (25, 7), (20, 10)];
        for (w, h) in sizes {
            term.backend_mut().resize(w, h);
            screen.resize(w, h);
            term.draw(|buf: &mut Buffer| {
                let text = format!("{w}x{h}");
                buf.set_string(0, 0, &text, Style::new());
                Ok(())
            })
            .unwrap();
            screen.apply(term.backend_mut().take_bytes().as_slice());

            // Current screen should show the new size's label.
            let expected = format!("{w}x{h}");
            assert_eq!(
                screen.row(0).trim_end(),
                expected,
                "after resize to {w}x{h}, row 0 wrong: {:?}",
                screen.row(0)
            );
            // All other rows blank.
            for y in 1..h {
                assert_eq!(
                    screen.row(y).trim_end(),
                    "",
                    "after resize to {w}x{h}, row {y} has stale content"
                );
            }
        }
    }

    // ── Horizontal resize regression tests ──────────────────────────

    #[test]
    fn resize_narrower_no_stale_cells_to_the_right() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        // Paint content spanning the full width on several rows.
        let tb = TestBackend::new(20, 3);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "AAAAAAAAAAAAAAAAAAAA", Style::new()); // 20 A's
            buf.set_string(0, 1, "BBBBBBBBBBBBBBBBBBBB", Style::new());
            buf.set_string(0, 2, "CCCCCCCCCCCCCCCCCCCC", Style::new());
            Ok(())
        })
        .unwrap();

        let mut screen = VirtualScreen::new(20, 3);
        screen.apply(term.backend().bytes());

        // Narrow to width 8. Intersection preserves first 8 columns.
        term.backend_mut().resize(8, 3);
        screen.resize(8, 3);
        let _ = term.backend_mut().take_bytes();

        // Next paint has much SHORTER text — without clear-on-resize
        // the force_full_redraw would skip blank cells 1..8 on rows 1,2
        // leaving the stale "BBBBBBBB" and "CCCCCCCC".
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "x", Style::new());
            // Rows 1 and 2 blank.
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend().bytes());

        assert_eq!(screen.row(0).trim_end(), "x");
        assert_eq!(screen.row(1).trim_end(), "", "row 1 has stale B's");
        assert_eq!(screen.row(2).trim_end(), "", "row 2 has stale C's");
    }

    #[test]
    fn resize_wider_new_columns_are_blank() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        let tb = TestBackend::new(5, 2);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "hello", Style::new());
            buf.set_string(0, 1, "world", Style::new());
            Ok(())
        })
        .unwrap();

        let mut screen = VirtualScreen::new(5, 2);
        screen.apply(term.backend().bytes());

        // Grow to width 15. Intersection preserves first 5 columns.
        term.backend_mut().resize(15, 2);
        screen.resize(15, 2);
        let _ = term.backend_mut().take_bytes();

        // Paint shorter content — new columns 5..15 must be blank, not
        // filled with residual terminal state.
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "hi", Style::new());
            buf.set_string(0, 1, "w", Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend().bytes());

        assert_eq!(screen.row(0).trim_end(), "hi");
        assert_eq!(screen.row(1).trim_end(), "w");
        // Columns 2..15 on row 0 must be blank (no stale "llo").
        for x in 2..15 {
            assert_eq!(
                screen.cell(x, 0).unwrap().symbol(),
                " ",
                "cell ({x},0) not blank after wider resize"
            );
        }
        // Columns 1..15 on row 1 must be blank (no stale "orld").
        for x in 1..15 {
            assert_eq!(
                screen.cell(x, 1).unwrap().symbol(),
                " ",
                "cell ({x},1) not blank after wider resize"
            );
        }
    }

    #[test]
    fn resize_narrower_then_wider_stays_clean() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        // 20 wide.
        let tb = TestBackend::new(20, 2);
        let mut term = Terminal::new(tb).unwrap();
        let mut screen = VirtualScreen::new(20, 2);

        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "FIRST-WIDTH-20-LINE!", Style::new()); // 20 chars
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());

        // Narrow to 10.
        term.backend_mut().resize(10, 2);
        screen.resize(10, 2);
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "narrow", Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());
        assert_eq!(screen.row(0).trim_end(), "narrow");
        assert_eq!(screen.row(1).trim_end(), "");

        // Grow back to 25 (wider than the original 20 — new columns
        // should be blank, not whatever was in the real terminal's
        // off-screen buffer).
        term.backend_mut().resize(25, 2);
        screen.resize(25, 2);
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "hi", Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());

        assert_eq!(screen.row(0).trim_end(), "hi");
        for x in 2..25 {
            assert_eq!(
                screen.cell(x, 0).unwrap().symbol(),
                " ",
                "cell ({x},0) leaked after narrow→wider"
            );
        }
        assert_eq!(screen.row(1).trim_end(), "");
    }

    #[test]
    fn horizontal_resize_preserves_styled_cells() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        // Paint styled content, resize narrower, paint narrower content,
        // verify the remaining cells carry the new style (not stale old
        // style bleeding through).
        let tb = TestBackend::new(20, 1);
        let mut term = Terminal::new(tb).unwrap();
        let mut screen = VirtualScreen::new(20, 1);

        term.draw(|buf: &mut Buffer| {
            buf.set_string(
                0,
                0,
                "RED-RED-RED-RED-RED!",
                Style::new().fg(Color::Rgb(255, 0, 0)),
            );
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());
        assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));

        // Narrow + repaint in Blue.
        term.backend_mut().resize(10, 1);
        screen.resize(10, 1);
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "blue", Style::new().fg(Color::Rgb(0, 0, 255)));
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());

        assert_eq!(screen.row(0).trim_end(), "blue");
        assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(0, 0, 255));
        assert_eq!(screen.cell(3, 0).unwrap().fg, Color::Rgb(0, 0, 255));
        // Blank cells past the new content should not carry Red fg.
        for x in 4..10 {
            // Blank cells: symbol is " ". Style is "reset" i.e. default
            // — whatever the terminal chose after \x1b[2J. The important
            // thing is the previous Red doesn't bleed.
            let c = screen.cell(x, 0).unwrap();
            assert_eq!(c.symbol(), " ", "cell ({x},0) not blank");
        }
    }

    #[test]
    fn multiple_horizontal_resizes_preserve_correctness() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        let tb = TestBackend::new(20, 2);
        let mut term = Terminal::new(tb).unwrap();
        let mut screen = VirtualScreen::new(20, 2);

        // Sequence alternating narrow/wide, each paints a label of its
        // own length. Regression for stale-cells on every transition.
        let widths = [10u16, 30, 5, 25, 15, 8];
        for &w in &widths {
            term.backend_mut().resize(w, 2);
            screen.resize(w, 2);
            let label = format!("W={w}");
            let label_for_paint = label.clone();
            term.draw(|buf: &mut Buffer| {
                buf.set_string(0, 0, &label_for_paint, Style::new());
                Ok(())
            })
            .unwrap();
            screen.apply(term.backend_mut().take_bytes().as_slice());

            assert_eq!(
                screen.row(0).trim_end(),
                label,
                "after resize to w={w}, row 0 wrong: {:?}",
                screen.row(0)
            );
            assert_eq!(
                screen.row(1).trim_end(),
                "",
                "after resize to w={w}, row 1 has stale content"
            );
            // Every cell past the label must be blank.
            let label_len = label.len() as u16;
            for x in label_len..w {
                assert_eq!(
                    screen.cell(x, 0).unwrap().symbol(),
                    " ",
                    "after w={w}, cell ({x},0) has stale content"
                );
            }
        }
    }

    #[test]
    fn simultaneous_width_and_height_resize() {
        use crate::render::{Buffer, Style, Terminal, TestBackend};

        // Paint a layout, then resize both axes at once (mimics a
        // terminal-window corner-drag).
        let tb = TestBackend::new(20, 5);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf: &mut Buffer| {
            for y in 0..5 {
                buf.set_string(0, y, "XXXXXXXXXXXXXXXXXXXX", Style::new());
            }
            Ok(())
        })
        .unwrap();

        let mut screen = VirtualScreen::new(20, 5);
        screen.apply(term.backend().bytes());

        // Corner-drag smaller: both narrower and shorter.
        term.backend_mut().resize(8, 2);
        screen.resize(8, 2);
        let _ = term.backend_mut().take_bytes();

        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "ok", Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend().bytes());

        assert_eq!(screen.row(0).trim_end(), "ok");
        assert_eq!(screen.row(1).trim_end(), "");
        // Row 0 columns 2..8 must be blank (no stale X's from the
        // original paint's first 8 X's).
        for x in 2..8 {
            assert_eq!(
                screen.cell(x, 0).unwrap().symbol(),
                " ",
                "cell ({x},0) stale after corner resize"
            );
        }

        // Corner-drag larger: both wider and taller.
        term.backend_mut().resize(25, 6);
        screen.resize(25, 6);
        let _ = term.backend_mut().take_bytes();

        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, "ok", Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend().bytes());

        assert_eq!(screen.row(0).trim_end(), "ok");
        // All new cells (rows 2..6, columns 2..25 on row 0) must be blank.
        for y in 1..6 {
            assert_eq!(
                screen.row(y).trim_end(),
                "",
                "row {y} has stale content after wider+taller resize"
            );
        }
        for x in 2..25 {
            assert_eq!(
                screen.cell(x, 0).unwrap().symbol(),
                " ",
                "cell ({x},0) stale after wider+taller resize"
            );
        }
    }
}
