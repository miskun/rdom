//! `Backend` — abstraction over "write cells to a TTY."
//!
//! The contract: a Backend knows its own size, tracks its SGR state
//! across `draw()` calls (for the cross-frame style cache), and emits
//! ANSI bytes via an internal writer. `Terminal<B>` drives the
//! Backend — the caller never talks to the Backend directly.
//!
//! Two implementations ship:
//!
//! - [`TestBackend`] — captures bytes into a `Vec<u8>` for tests.
//!   Also exposes its internal state model (cursor position, cursor
//!   visibility) for assertion.
//! - [`CrosstermBackend`] (see `backend_crossterm.rs`) — real
//!   terminal I/O via crossterm.
//!
//! ## Diff-driven draw
//!
//! `draw(iter)` accepts an iterator of `(x, y, &Cell)` updates — the
//! output of `Buffer::diff_iter`. The Backend emits:
//!
//! 1. Cursor-position (CUP) when the next cell isn't adjacent to the
//!    previously-emitted one.
//! 2. Any SGR transitions needed vs the previous cell's style.
//! 3. The cell's symbol bytes.
//!
//! Style state persists across `draw()` calls (decision #5).

use std::io;

use super::sgr::{SgrState, emit_cup, emit_sgr_transition};
use super::{Cell, Rect};

/// The minimum a Backend must do.
///
/// All backends also implement `io::Write` — the terminal RAII guard
/// and `Terminal::draw`'s BSU/ESU wrappers write raw bytes through this.
pub trait Backend: io::Write {
    /// Current size of the terminal (or fake size, for tests).
    fn size(&self) -> io::Result<Rect>;

    /// Clear the whole screen and reset SGR state. Emits `\x1b[2J` +
    /// `\x1b[0m` + cursor-to-home. Resets internal state cache too.
    fn clear(&mut self) -> io::Result<()>;

    /// Hide the cursor (`\x1b[?25l`). Should be called once at
    /// Terminal setup; Terminal's panic guard pairs with `show_cursor`.
    fn hide_cursor(&mut self) -> io::Result<()>;

    /// Show the cursor (`\x1b[?25h`).
    fn show_cursor(&mut self) -> io::Result<()>;

    /// Move cursor to an explicit position. Emits CUP.
    fn set_cursor_position(&mut self, x: u16, y: u16) -> io::Result<()>;

    /// Write a batch of cell updates — typically the output of
    /// `Buffer::diff_iter`. The iterator yields primary cells only;
    /// Backend handles CUP / SGR / symbol emission per cell.
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>;

    /// Forget the last emitted SGR state and cursor position. Next
    /// draw() call will emit full state. Use after something else
    /// may have written to stdout (e.g., an uncaught `println!`).
    fn reset_style_cache(&mut self);
}

/// Internal tracking shared by both backend implementations. Both
/// `TestBackend` and `CrosstermBackend` use this to produce correct
/// diff-driven output.
#[derive(Debug, Clone, Default)]
pub(crate) struct BackendState {
    /// Last SGR state emitted. Compared against each cell's style to
    /// produce the minimal transition.
    pub sgr: SgrState,
    /// Where we last told the terminal to position its cursor. `None`
    /// after `clear` or a backend reset — forces CUP before the next
    /// symbol.
    pub cursor: Option<(u16, u16)>,
    /// Polish #9 — currently-open OSC 8 hyperlink URL. `None` when
    /// no link is active. The draw loop emits an open sequence when
    /// transitioning from `None` → `Some(url)`, a close sequence on
    /// `Some(_)` → `None`, and close+open when the URL changes.
    pub link: Option<String>,
}

/// Core draw helper shared by concrete backends. Writes diff output
/// to `writer`, updating `state`. Extracted so `TestBackend` and
/// `CrosstermBackend` can share the same emission logic.
pub(crate) fn draw_iter<'a, W, I>(
    writer: &mut W,
    state: &mut BackendState,
    iter: I,
) -> io::Result<()>
where
    W: io::Write,
    I: Iterator<Item = (u16, u16, &'a Cell)>,
{
    for (x, y, cell) in iter {
        // CUP if not adjacent to last position.
        let need_cup = match state.cursor {
            Some((lx, ly)) => !(ly == y && lx == x),
            None => true,
        };
        if need_cup {
            emit_cup(writer, x, y)?;
        }

        // SGR transition.
        let new_sgr = SgrState {
            fg: cell.fg,
            bg: cell.bg,
            modifier: cell.modifier,
        };
        state.sgr = emit_sgr_transition(writer, state.sgr, new_sgr)?;

        // OSC 8 hyperlink transition (Polish #9). Emit close when
        // leaving a link, open when entering one, close+open when
        // switching URLs.
        let new_link = cell.link();
        let matches_current = match (&state.link, new_link) {
            (Some(cur), Some(next)) => cur == next,
            (None, None) => true,
            _ => false,
        };
        if !matches_current {
            if state.link.is_some() {
                // Close the previous link.
                writer.write_all(b"\x1b]8;;\x1b\\")?;
            }
            if let Some(url) = new_link {
                // Open the new link. Empty params field between
                // the two semicolons — we don't use the `id=`
                // parameter, which only matters for splitting a
                // single link across multiple display runs.
                writer.write_all(b"\x1b]8;;")?;
                writer.write_all(url.as_bytes())?;
                writer.write_all(b"\x1b\\")?;
                state.link = Some(url.to_string());
            } else {
                state.link = None;
            }
        }

        // Symbol bytes.
        let symbol = cell.symbol();
        writer.write_all(symbol.as_bytes())?;

        // Cursor advances by the cell's visible width.
        let advance = cell.cell_width().max(1);
        let new_x = x.saturating_add(advance);
        state.cursor = Some((new_x, y));
    }
    // Close any still-open link at the end of the draw. Keeps the
    // terminal's link state clean so subsequent writes (by other
    // apps, by a later frame) don't accidentally inherit it.
    if state.link.is_some() {
        writer.write_all(b"\x1b]8;;\x1b\\")?;
        state.link = None;
    }
    Ok(())
}

// ────────────────────────── TestBackend ──────────────────────────────

/// In-memory backend for tests. Implements `Backend` by appending
/// ANSI bytes to a `Vec<u8>`; size is fixed at construction.
pub struct TestBackend {
    size: Rect,
    buffer: Vec<u8>,
    state: BackendState,
    cursor_visible: bool,
}

impl TestBackend {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            size: Rect::new(0, 0, width, height),
            buffer: Vec::new(),
            state: BackendState::default(),
            cursor_visible: true,
        }
    }

    /// Borrow the accumulated ANSI bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Take the accumulated bytes and reset. Useful between test
    /// phases.
    pub fn take_bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }

    /// Reset just the byte buffer (leaves state cache alone).
    pub fn clear_bytes(&mut self) {
        self.buffer.clear();
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Resize the fake terminal.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.size = Rect::new(0, 0, width, height);
    }

    /// Borrow the internal backend state — tests only.
    #[allow(dead_code)]
    pub(crate) fn state(&self) -> &BackendState {
        &self.state
    }
}

impl io::Write for TestBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Backend for TestBackend {
    fn size(&self) -> io::Result<Rect> {
        Ok(self.size)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.buffer.extend_from_slice(b"\x1b[2J\x1b[0m\x1b[1;1H");
        self.state = BackendState::default();
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.buffer.extend_from_slice(b"\x1b[?25l");
        self.cursor_visible = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.buffer.extend_from_slice(b"\x1b[?25h");
        self.cursor_visible = true;
        Ok(())
    }

    fn set_cursor_position(&mut self, x: u16, y: u16) -> io::Result<()> {
        emit_cup(&mut self.buffer, x, y)?;
        self.state.cursor = Some((x, y));
        Ok(())
    }

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        // Split the borrow: draw_iter needs &mut state and the writer.
        // We wrap `self.buffer` as a short-lived writer.
        let Self { buffer, state, .. } = self;
        draw_iter(buffer, state, content)
    }

    fn reset_style_cache(&mut self) {
        self.state = BackendState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Buffer, Color, Modifier, Style};

    fn paint<F: FnOnce(&mut Buffer)>(f: F) -> Buffer {
        let mut b = Buffer::empty(Rect::new(0, 0, 10, 3));
        f(&mut b);
        b
    }

    // ── TestBackend basics ───────────────────────────────────────────

    #[test]
    fn new_test_backend() {
        let tb = TestBackend::new(80, 24);
        assert_eq!(tb.size().unwrap(), Rect::new(0, 0, 80, 24));
        assert!(tb.cursor_visible());
        assert!(tb.bytes().is_empty());
    }

    #[test]
    fn clear_emits_expected_bytes() {
        let mut tb = TestBackend::new(10, 3);
        tb.clear().unwrap();
        assert_eq!(tb.bytes(), b"\x1b[2J\x1b[0m\x1b[1;1H");
    }

    #[test]
    fn hide_show_cursor() {
        let mut tb = TestBackend::new(10, 3);
        tb.hide_cursor().unwrap();
        assert_eq!(tb.bytes(), b"\x1b[?25l");
        assert!(!tb.cursor_visible());
        tb.clear_bytes();
        tb.show_cursor().unwrap();
        assert_eq!(tb.bytes(), b"\x1b[?25h");
        assert!(tb.cursor_visible());
    }

    #[test]
    fn set_cursor_position_emits_cup() {
        let mut tb = TestBackend::new(10, 3);
        tb.set_cursor_position(3, 1).unwrap();
        assert_eq!(tb.bytes(), b"\x1b[2;4H"); // 1-indexed
    }

    // ── draw diff emission ───────────────────────────────────────────

    #[test]
    fn draw_empty_iter_emits_nothing() {
        let mut tb = TestBackend::new(10, 3);
        tb.draw(std::iter::empty()).unwrap();
        assert!(tb.bytes().is_empty());
    }

    #[test]
    fn draw_single_cell() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(2, 1, "A", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();

        // Expect CUP to (2,1) → SGR fg Red (truecolor) → "A"
        assert_eq!(tb.bytes(), b"\x1b[2;3H\x1b[38;2;255;0;0mA");
    }

    #[test]
    fn draw_adjacent_cells_skip_cup() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "abc", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();

        // CUP once, SGR once (truecolor), then 3 chars with no more CUP.
        assert_eq!(tb.bytes(), b"\x1b[1;1H\x1b[38;2;255;0;0mabc");
    }

    #[test]
    fn draw_non_adjacent_cells_emit_cup_each() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "A", Style::new());
        next.set_string(5, 1, "B", Style::new());

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();

        // First cell gets CUP + 'A', second cell gets another CUP + 'B'.
        assert_eq!(tb.bytes(), b"\x1b[1;1HA\x1b[2;6HB");
    }

    #[test]
    fn draw_preserves_sgr_state_across_adjacent_cells() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        // All same style → only one SGR transition at start.
        next.set_string(0, 0, "abc", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        assert_eq!(tb.bytes(), b"\x1b[1;1H\x1b[38;2;255;0;0mabc");
    }

    #[test]
    fn draw_emits_sgr_on_change() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "A", Style::new().fg(Color::Rgb(255, 0, 0)));
        next.set_string(1, 0, "B", Style::new().fg(Color::Rgb(0, 0, 255)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        // CUP (1,1), fg Red, 'A', (adjacent, no CUP), fg Blue, 'B' —
        // both fg's in truecolor form.
        assert_eq!(
            tb.bytes(),
            b"\x1b[1;1H\x1b[38;2;255;0;0mA\x1b[38;2;0;0;255mB",
        );
    }

    #[test]
    fn draw_style_cache_persists_across_draws() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "A", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        tb.clear_bytes();

        // Second frame: same fg → no SGR emitted. New cell at (2,0)
        // needs CUP (cursor is at (1,0), not (2,0)) but no new SGR.
        let mut next2 = prev.clone();
        next2.set_string(2, 0, "B", Style::new().fg(Color::Rgb(255, 0, 0)));
        tb.draw(next2.diff_iter(&prev)).unwrap();
        assert_eq!(tb.bytes(), b"\x1b[1;3HB");
    }

    #[test]
    fn reset_style_cache_forces_next_frame_to_reemit() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "A", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        tb.clear_bytes();
        tb.reset_style_cache();

        // Second frame — cache was reset, so CUP + SGR re-emitted.
        let mut next2 = prev.clone();
        next2.set_string(0, 0, "B", Style::new().fg(Color::Rgb(255, 0, 0)));
        tb.draw(next2.diff_iter(&prev)).unwrap();
        assert_eq!(tb.bytes(), b"\x1b[1;1H\x1b[38;2;255;0;0mB");
    }

    #[test]
    fn draw_wide_glyph_advances_cursor_by_two() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "中X", Style::new());

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        // CUP (1,1), "中", then "X" (adjacent at x=2, y=0). Cursor
        // advances by 2 after "中" — "X" should NOT emit CUP.
        let s = std::str::from_utf8(tb.bytes()).unwrap();
        assert!(s.starts_with("\x1b[1;1H中X"), "got: {:?}", s);
    }

    #[test]
    fn clear_resets_style_and_cursor_cache() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(0, 0, "A", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        tb.clear().unwrap();
        tb.clear_bytes();

        // After clear, next draw must re-emit CUP + SGR.
        let mut next2 = prev.clone();
        next2.set_string(0, 0, "B", Style::new().fg(Color::Rgb(255, 0, 0)));
        tb.draw(next2.diff_iter(&prev)).unwrap();
        assert_eq!(tb.bytes(), b"\x1b[1;1H\x1b[38;2;255;0;0mB");
    }

    #[test]
    fn modifier_composition() {
        let prev = paint(|_| {});
        let mut next = prev.clone();
        next.set_string(
            0,
            0,
            "X",
            Style::new()
                .fg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::BOLD),
        );

        let mut tb = TestBackend::new(10, 3);
        tb.draw(next.diff_iter(&prev)).unwrap();
        assert_eq!(tb.bytes(), b"\x1b[1;1H\x1b[1m\x1b[38;2;255;255;255mX");
    }
}
