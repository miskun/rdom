//! `CrosstermBackend` — real terminal I/O.
//!
//! Wraps any `Write` (typically `stdout()`) and uses crossterm for
//! platform-specific bits (raw mode enter/exit, alt screen, size
//! detection, Windows compatibility). SGR and cursor bytes are
//! emitted by us — crossterm is used as a thin OS-portability layer,
//! not as a styling system.
//!
//! We do NOT import crossterm's `Color` or `Attribute` types. Our
//! `Color` / `Modifier` and `sgr::emit_*` produce the same bytes
//! crossterm would emit via `SetForegroundColor(...)`.

use std::io::{self, Write};

use crossterm::event::{
    DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal;
use crossterm::{cursor, execute};

use super::backend::{Backend, BackendState, draw_iter};
use super::sgr::emit_cup;
use super::{Cell, Rect};

pub struct CrosstermBackend<W: Write> {
    writer: W,
    state: BackendState,
}

impl<W: Write> CrosstermBackend<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            state: BackendState::default(),
        }
    }

    /// Borrow the underlying writer.
    pub fn writer(&self) -> &W {
        &self.writer
    }

    /// Mutably borrow the underlying writer. Use sparingly — writes
    /// made through it bypass our state cache.
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<W: Write> Write for CrosstermBackend<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write> Backend for CrosstermBackend<W> {
    fn size(&self) -> io::Result<Rect> {
        let (w, h) = terminal::size()?;
        Ok(Rect::new(0, 0, w, h))
    }

    fn clear(&mut self) -> io::Result<()> {
        execute!(
            self.writer,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
        )?;
        // Follow with our own SGR reset so state matches terminal.
        self.writer.write_all(b"\x1b[0m")?;
        self.state = BackendState::default();
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        execute!(self.writer, cursor::Hide)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        execute!(self.writer, cursor::Show)
    }

    fn set_cursor_position(&mut self, x: u16, y: u16) -> io::Result<()> {
        emit_cup(&mut self.writer, x, y)?;
        self.state.cursor = Some((x, y));
        Ok(())
    }

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        draw_iter(&mut self.writer, &mut self.state, content)
    }

    fn reset_style_cache(&mut self) {
        self.state = BackendState::default();
    }
}

// ─── Mode management (raw mode, alt screen) ─────────────────────────

/// Enable the standard "TUI app" mode: raw input, alternate screen,
/// cursor hidden, **mouse capture enabled**. Restored by
/// `leave_tui_mode` or the `TerminalGuard` (see `terminal.rs`) on
/// drop.
///
/// Mouse capture is on by default for rdom-tui apps — the runtime
/// routes mouse events (click, hover, wheel) through the DOM just
/// like a browser. An app that wants the terminal emulator's own
/// drag-to-select behavior (e.g., read-only viewers) can call
/// [`leave_mouse_capture`] + [`enter_mouse_capture`] around a
/// specific region — or opt out at startup with the
/// `no-mouse-capture` cargo feature on `rdom-tui`.
pub fn enter_tui_mode<W: Write>(writer: &mut W) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    // Single batched execute! — the ratatui-canonical pattern
    // that's known to work reliably on iTerm2 and other
    // terminals. Earlier in this codebase we
    // tried splitting the writes per-sequence + various
    // pre-resets / focus-cycle kicks chasing a hover bug; none of
    // those helped, because the actual root cause was elsewhere
    // (Terminal::draw was emitting BSU/ESU `?2026` on every
    // frame, which interacted badly with iTerm2 motion tracking).
    // With BSU/ESU removed (see `terminal.rs::draw`), this clean
    // setup is sufficient.
    execute!(
        writer,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        EnableMouseCapture,
        EnableFocusChange,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
        ),
    )?;
    Ok(())
}

/// Restore the terminal to its pre-`enter_tui_mode` state: disable
/// mouse capture, show cursor, leave alt screen, reset SGR, disable
/// raw mode. Safe to call from a drop handler — all crossterm
/// operations map to idempotent-enough ANSI sequences.
pub fn leave_tui_mode<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(
        writer,
        PopKeyboardEnhancementFlags,
        DisableFocusChange,
        DisableMouseCapture,
        cursor::Show,
        terminal::LeaveAlternateScreen,
    )?;
    writer.write_all(b"\x1b[0m")?;
    writer.flush()?;
    terminal::disable_raw_mode()
}

/// Explicitly enable mouse capture after `enter_tui_mode` has already
/// run (e.g., re-enable after a temporary `leave_mouse_capture`).
/// Idempotent at the terminal level — sending the sequence twice is
/// harmless.
pub fn enter_mouse_capture<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(writer, EnableMouseCapture)
}

/// Temporarily disable mouse capture without leaving TUI mode — the
/// terminal emulator gets mouse events back, so the user can use its
/// native drag-to-select / right-click-menu. Pair with
/// [`enter_mouse_capture`] to re-enable.
pub fn leave_mouse_capture<W: Write>(writer: &mut W) -> io::Result<()> {
    execute!(writer, DisableMouseCapture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Color, Style};

    // We can test CrosstermBackend's byte output by pointing it at a
    // Vec<u8> instead of stdout. Mode / size operations require a real
    // terminal and are NOT tested here — covered by manual smoke tests.

    struct Sink(Vec<u8>);
    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn backend() -> CrosstermBackend<Sink> {
        CrosstermBackend::new(Sink(Vec::new()))
    }

    fn bytes(b: &CrosstermBackend<Sink>) -> &[u8] {
        &b.writer.0
    }

    #[test]
    fn set_cursor_position_emits_cup() {
        let mut b = backend();
        b.set_cursor_position(5, 2).unwrap();
        assert_eq!(bytes(&b), b"\x1b[3;6H");
    }

    #[test]
    fn draw_basic_cell() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 2));
        let mut next = prev.clone();
        next.set_string(2, 0, "hi", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut b = backend();
        b.draw(next.diff_iter(&prev)).unwrap();
        assert_eq!(bytes(&b), b"\x1b[1;3H\x1b[38;2;255;0;0mhi");
    }

    #[test]
    fn draw_persists_state_across_calls() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 2));
        let mut n1 = prev.clone();
        n1.set_string(0, 0, "A", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut b = backend();
        b.draw(n1.diff_iter(&prev)).unwrap();
        let snapshot_len = b.writer.0.len();

        // Next frame: same style, adjacent position → no CUP, no SGR.
        let mut n2 = prev.clone();
        n2.set_string(1, 0, "B", Style::new().fg(Color::Rgb(255, 0, 0)));
        b.draw(n2.diff_iter(&prev)).unwrap();
        let delta = &b.writer.0[snapshot_len..];
        assert_eq!(delta, b"B");
    }

    #[test]
    fn reset_style_cache_drops_history() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 2));
        let mut n1 = prev.clone();
        n1.set_string(0, 0, "A", Style::new().fg(Color::Rgb(255, 0, 0)));

        let mut b = backend();
        b.draw(n1.diff_iter(&prev)).unwrap();
        b.writer.0.clear();
        b.reset_style_cache();

        // Second frame re-emits CUP + SGR (we lost cache).
        let mut n2 = prev.clone();
        n2.set_string(0, 0, "B", Style::new().fg(Color::Rgb(255, 0, 0)));
        b.draw(n2.diff_iter(&prev)).unwrap();
        assert_eq!(bytes(&b), b"\x1b[1;1H\x1b[38;2;255;0;0mB");
    }

    // ── Polish #9: OSC 8 hyperlink emission ──────────────────────

    #[test]
    fn link_cells_emit_osc8_wrappers() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 1));
        let mut next = prev.clone();
        next.set_string(0, 0, "hi", Style::new());
        next.set_link_range(0, 0, 2, Some("https://example.com"));

        let mut b = backend();
        b.draw(next.diff_iter(&prev)).unwrap();
        let out = bytes(&b);
        // Open sequence precedes the content; close sequence
        // follows the end of the draw.
        let expected = b"\x1b[1;1H\x1b]8;;https://example.com\x1b\\hi\x1b]8;;\x1b\\";
        assert_eq!(out, expected, "got {:?}", std::str::from_utf8(out));
    }

    #[test]
    fn adjacent_same_link_run_emits_one_open_and_one_close() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 1));
        let mut next = prev.clone();
        next.set_string(0, 0, "abc", Style::new());
        next.set_link_range(0, 0, 3, Some("x"));

        let mut b = backend();
        b.draw(next.diff_iter(&prev)).unwrap();
        let out = String::from_utf8_lossy(bytes(&b)).to_string();
        // Exactly one open + one close despite three cells sharing
        // the link.
        assert_eq!(out.matches("\x1b]8;;x\x1b\\").count(), 1);
        assert_eq!(out.matches("\x1b]8;;\x1b\\").count(), 1);
    }

    #[test]
    fn link_transition_closes_old_opens_new() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 1));
        let mut next = prev.clone();
        next.set_string(0, 0, "ab", Style::new());
        next.set_link_range(0, 0, 1, Some("first"));
        next.set_link_range(1, 0, 1, Some("second"));

        let mut b = backend();
        b.draw(next.diff_iter(&prev)).unwrap();
        let out = String::from_utf8_lossy(bytes(&b)).to_string();
        // Open first → 'a' → close+open second → 'b' → trailing close.
        assert!(out.contains("\x1b]8;;first\x1b\\a\x1b]8;;\x1b\\\x1b]8;;second\x1b\\b"));
        // Exactly two opens, two closes.
        assert_eq!(out.matches("\x1b]8;;first\x1b\\").count(), 1);
        assert_eq!(out.matches("\x1b]8;;second\x1b\\").count(), 1);
        assert_eq!(out.matches("\x1b]8;;\x1b\\").count(), 2); // close-first + final-close
    }

    #[test]
    fn draw_without_any_links_emits_no_osc8() {
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 1));
        let mut next = prev.clone();
        next.set_string(0, 0, "plain", Style::new());

        let mut b = backend();
        b.draw(next.diff_iter(&prev)).unwrap();
        let out = String::from_utf8_lossy(bytes(&b)).to_string();
        assert!(!out.contains("\x1b]8;;"));
    }

    #[test]
    fn draw_closes_open_link_at_end_of_frame() {
        // A frame that ends mid-link should close before the draw
        // returns, so subsequent output by other code isn't
        // mistakenly wrapped.
        use crate::render::Buffer;
        let prev = Buffer::empty(Rect::new(0, 0, 10, 1));
        let mut next = prev.clone();
        next.set_string(0, 0, "x", Style::new());
        next.set_link_range(0, 0, 1, Some("u"));

        let mut b = backend();
        b.draw(next.diff_iter(&prev)).unwrap();
        let out = bytes(&b);
        assert!(out.ends_with(b"\x1b]8;;\x1b\\"));
    }
}
