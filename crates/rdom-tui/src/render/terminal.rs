//! `Terminal<B>` — front+back buffer management + diff-driven draw loop.
//!
//! Owns a `Backend`, a front buffer (last-drawn state, matches what
//! the TTY actually shows), and a back buffer (the frame being
//! prepared). `draw(|buf| …)` hands the back buffer to the caller,
//! diffs vs front, emits only what changed, then swaps.
//!
//! ## Synchronized output (BSU/ESU)
//!
//! Each `draw()` call is wrapped in DEC private mode 2026:
//!
//! - `\x1b[?2026h` — Begin Synchronized Update
//! - `\x1b[?2026l` — End Synchronized Update
//!
//! Modern terminals buffer everything between these markers and flush
//! atomically, preventing mid-frame tearing. Non-supporting terminals
//! ignore the sequences. Disable with the `no-synchronized-output`
//! Cargo feature if needed.
//!
//! ## Autoresize
//!
//! Before each draw, `Terminal` polls `backend.size()`. If it changed,
//! both buffers are resized (content preserved on the intersection)
//! and a force-full-redraw is scheduled — we can't trust the front
//! buffer reflects reality post-resize.
//!
//! ## Panic safety
//!
//! `TerminalGuard` is an RAII guard that calls `leave_tui_mode` on
//! drop. Pair it with `enter_tui_mode` at program start — even if the
//! user's code panics inside `draw(|...|)`, the guard restores the
//! terminal to a usable state.

use std::io;

use super::backend::Backend;
use super::{Buffer, Rect};

/// Front+back buffer terminal with diff-driven updates.
pub struct Terminal<B: Backend> {
    backend: B,
    /// The buffer that matches what's currently on the terminal.
    front: Buffer,
    /// The buffer being prepared this frame.
    back: Buffer,
    /// Set when something happened (resize, explicit clear) that
    /// invalidates the front buffer. Next draw emits every cell.
    force_full_redraw: bool,
}

/// Returned by `draw` so callers can inspect what happened this frame.
#[derive(Debug, Clone, Copy)]
pub struct CompletedFrame {
    pub area: Rect,
    pub cells_emitted: usize,
    pub was_full_redraw: bool,
}

impl<B: Backend> Terminal<B> {
    /// Construct a Terminal from a backend. Both buffers start at the
    /// backend's current size.
    pub fn new(backend: B) -> io::Result<Self> {
        let size = backend.size()?;
        Ok(Self {
            backend,
            front: Buffer::empty(size),
            back: Buffer::empty(size),
            force_full_redraw: true,
        })
    }

    /// Borrow the backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Mutably borrow the backend. Bypasses our state invariants —
    /// use sparingly.
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Current viewport.
    pub fn size(&self) -> Rect {
        self.back.area
    }

    /// Queue a full repaint for the next `draw()`.
    pub fn queue_full_redraw(&mut self) {
        self.force_full_redraw = true;
    }

    /// Forget the front buffer and reset the backend's style cache.
    /// Use after out-of-band writes to stdout might have corrupted
    /// our idea of the terminal state.
    pub fn clear(&mut self) -> io::Result<()> {
        self.backend.clear()?;
        self.front.clear();
        self.back.clear();
        self.force_full_redraw = true;
        Ok(())
    }

    /// Check if the backend's reported size matches ours; resize if
    /// not. Called automatically by `draw`.
    ///
    /// When the size changes we also emit `\x1b[2J` via
    /// `backend.clear()` so the terminal is blanked before the next
    /// frame paints. Without this, stale cells from the old frame
    /// persist at positions that are either no longer painted (resize
    /// smaller → content is now out of buffer bounds but still on
    /// screen until overwritten) or now empty (resize larger → new
    /// rows show whatever was in the terminal before).
    pub fn autoresize(&mut self) -> io::Result<()> {
        let actual = self.backend.size()?;
        if actual != self.back.area {
            self.back.resize(actual);
            self.front.resize(actual);
            // Front no longer reflects the terminal — force a full
            // repaint next frame AND wipe the terminal first so stale
            // cells outside the repaint set can't leak through.
            self.force_full_redraw = true;
            self.backend.clear()?;
        }
        Ok(())
    }

    /// Render a frame. `f` receives a mutable reference to the back
    /// buffer; paint into it. On return, we diff back vs front, emit
    /// only the changed cells (wrapped in BSU/ESU when enabled), then
    /// swap so the back becomes the new front for next frame.
    pub fn draw<F>(&mut self, f: F) -> io::Result<CompletedFrame>
    where
        F: FnOnce(&mut Buffer) -> io::Result<()>,
    {
        self.autoresize()?;

        // Clear the back buffer so the caller starts from a clean slate
        // each frame. (Paint pass composes destructively — no need for
        // incremental composition.)
        self.back.clear();

        // Caller paints.
        f(&mut self.back)?;

        // Begin Synchronized Update (ignored by non-supporting
        // terminals).
        #[cfg(not(feature = "no-synchronized-output"))]
        self.backend.write_all(b"\x1b[?2026h")?;

        let mut cells_emitted = 0usize;
        let was_full_redraw = self.force_full_redraw;

        if self.force_full_redraw {
            // Emit every non-blank cell + every cell whose style
            // differs from default. Cheap substitute: diff vs an
            // empty buffer of the same size.
            let blank = Buffer::empty(self.back.area);
            for (x, y, cell) in self.back.diff_iter(&blank) {
                cells_emitted += 1;
                // Emit one-at-a-time via the backend's draw. We
                // construct a trivial iter for each cell.
                self.backend.draw(std::iter::once((x, y, cell)))?;
            }
            self.force_full_redraw = false;
        } else {
            // Normal incremental diff.
            let count = self.back.diff_iter(&self.front).count();
            cells_emitted = count;
            self.backend.draw(self.back.diff_iter(&self.front))?;
        }

        // End Synchronized Update.
        #[cfg(not(feature = "no-synchronized-output"))]
        self.backend.write_all(b"\x1b[?2026l")?;

        self.backend.flush()?;

        // Swap: back becomes the new front.
        std::mem::swap(&mut self.front, &mut self.back);

        Ok(CompletedFrame {
            area: self.front.area,
            cells_emitted,
            was_full_redraw,
        })
    }

    /// Hide the cursor (passthrough to backend).
    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.backend.hide_cursor()
    }

    /// Show the cursor.
    pub fn show_cursor(&mut self) -> io::Result<()> {
        self.backend.show_cursor()
    }

    /// Position the cursor. Use after `draw()` if you want the cursor
    /// at a specific location (e.g., for text input).
    pub fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.backend.set_cursor_position(x, y)
    }

    /// Dissolve into the backend. Useful when you need to hand the
    /// writer back to something else after tearing down the Terminal.
    pub fn into_backend(self) -> B {
        self.backend
    }
}

// ─── RAII mode guard ────────────────────────────────────────────────

/// Guards a terminal session's mode. Constructed after `enter_tui_mode`;
/// on drop, runs `leave_tui_mode`. Works even on panic.
///
/// ```ignore
/// use rdom_tui::render::{Terminal, CrosstermBackend, TerminalGuard};
/// use rdom_tui::render::backend_crossterm::{enter_tui_mode, leave_tui_mode};
///
/// let mut stdout = std::io::stdout();
/// enter_tui_mode(&mut stdout)?;
/// let _guard = TerminalGuard::new();
/// let backend = CrosstermBackend::new(stdout);
/// let mut term = Terminal::new(backend)?;
/// // Even if this panics, TerminalGuard::drop restores the terminal.
/// term.draw(|buf| { … })?;
/// ```
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    /// Construct a guard. Does NOT enter TUI mode — caller is expected
    /// to have done that already. Drop will attempt to leave.
    pub fn new() -> Self {
        Self { active: true }
    }

    /// Deactivate without restoring — use when you've manually
    /// restored the terminal (e.g., clean shutdown) and want to skip
    /// the drop-time restore.
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Default for TerminalGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = super::backend_crossterm::leave_tui_mode(&mut io::stdout());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::backend::TestBackend;
    use crate::render::{Color, Style};

    // ── Basic draw cycle ─────────────────────────────────────────────

    #[test]
    fn construct_initial_full_redraw() {
        let tb = TestBackend::new(10, 3);
        let term = Terminal::new(tb).unwrap();
        assert_eq!(term.size(), Rect::new(0, 0, 10, 3));
    }

    #[test]
    fn first_draw_is_full_redraw() {
        let tb = TestBackend::new(10, 3);
        let mut term = Terminal::new(tb).unwrap();
        let frame = term
            .draw(|buf| {
                buf.set_string(0, 0, "hi", Style::new().fg(Color::Rgb(255, 0, 0)));
                Ok(())
            })
            .unwrap();
        assert!(frame.was_full_redraw);
        assert_eq!(frame.cells_emitted, 2); // 'h' and 'i'
    }

    #[test]
    fn second_draw_is_incremental() {
        let tb = TestBackend::new(10, 3);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf| {
            buf.set_string(0, 0, "hi", Style::new());
            Ok(())
        })
        .unwrap();
        let f2 = term
            .draw(|buf| {
                buf.set_string(0, 0, "hi", Style::new());
                Ok(())
            })
            .unwrap();
        assert!(!f2.was_full_redraw);
        assert_eq!(f2.cells_emitted, 0); // unchanged
    }

    #[test]
    fn changed_cells_are_emitted() {
        let tb = TestBackend::new(10, 3);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf| {
            buf.set_string(0, 0, "hello", Style::new());
            Ok(())
        })
        .unwrap();

        let frame = term
            .draw(|buf| {
                buf.set_string(0, 0, "hELLo", Style::new());
                Ok(())
            })
            .unwrap();
        assert_eq!(frame.cells_emitted, 3); // E, L, L
    }

    // ── BSU/ESU ──────────────────────────────────────────────────────

    #[cfg(not(feature = "no-synchronized-output"))]
    #[test]
    fn sync_output_wraps_frame() {
        let tb = TestBackend::new(5, 1);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf| {
            buf.set_string(0, 0, "X", Style::new());
            Ok(())
        })
        .unwrap();
        let bytes = term.backend().bytes();
        assert!(bytes.starts_with(b"\x1b[?2026h"), "BSU missing");
        assert!(bytes.ends_with(b"\x1b[?2026l"), "ESU missing");
    }

    // ── Resize ───────────────────────────────────────────────────────

    #[test]
    fn autoresize_resizes_both_buffers() {
        let tb = TestBackend::new(5, 3);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf| {
            buf.set_string(0, 0, "ab", Style::new());
            Ok(())
        })
        .unwrap();

        term.backend_mut().resize(10, 3);
        assert_eq!(term.backend().size().unwrap(), Rect::new(0, 0, 10, 3));

        let frame = term
            .draw(|buf| {
                buf.set_string(0, 0, "ab", Style::new());
                Ok(())
            })
            .unwrap();
        // After resize, next frame is a full redraw.
        assert!(frame.was_full_redraw);
        assert_eq!(term.size(), Rect::new(0, 0, 10, 3));
    }

    // ── Clear ────────────────────────────────────────────────────────

    #[test]
    fn clear_forces_full_redraw_next_frame() {
        let tb = TestBackend::new(5, 2);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf| {
            buf.set_string(0, 0, "X", Style::new());
            Ok(())
        })
        .unwrap();
        term.clear().unwrap();

        let frame = term
            .draw(|buf| {
                buf.set_string(0, 0, "X", Style::new());
                Ok(())
            })
            .unwrap();
        assert!(frame.was_full_redraw);
    }

    // ── Cursor control passthrough ───────────────────────────────────

    #[test]
    fn hide_show_cursor_passes_through() {
        let tb = TestBackend::new(5, 2);
        let mut term = Terminal::new(tb).unwrap();
        term.hide_cursor().unwrap();
        assert!(term.backend().bytes().contains(&b'l'));
        term.show_cursor().unwrap();
        assert!(term.backend().bytes().contains(&b'h'));
    }

    // ── Full render pipeline integration ─────────────────────────────

    #[test]
    fn end_to_end_dom_to_ansi() {
        use crate::prelude::*;

        let mut dom = TuiDom::new();
        let root = dom.root();
        let span = dom.create_element("span");
        let t = dom.create_text_node("hi");
        dom.append_child(span, t).unwrap();
        dom.append_child(root, span).unwrap();

        let sheet =
            Stylesheet::bare().rule_unchecked("span", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
        dom.cascade(&sheet);

        let tb = TestBackend::new(10, 1);
        let mut term = Terminal::new(tb).unwrap();
        let viewport = term.size();

        term.draw(|buf| {
            dom.layout_dom(viewport);
            dom.paint_dom(buf, viewport);
            Ok(())
        })
        .unwrap();

        let bytes = term.backend().bytes();
        let s = std::str::from_utf8(bytes).unwrap();
        assert!(s.contains("hi"), "got: {:?}", s);
        assert!(
            s.contains("\x1b[38;2;255;0;0m"),
            "expected Red fg truecolor SGR in: {:?}",
            s,
        );
    }

    // ── Multi-frame persistence ──────────────────────────────────────

    #[test]
    fn unchanged_frames_emit_minimal_bytes() {
        let tb = TestBackend::new(10, 1);
        let mut term = Terminal::new(tb).unwrap();
        term.draw(|buf| {
            buf.set_string(0, 0, "stable", Style::new().fg(Color::Rgb(255, 0, 0)));
            Ok(())
        })
        .unwrap();
        term.backend_mut().take_bytes(); // clear counter

        for _ in 0..5 {
            term.draw(|buf| {
                buf.set_string(0, 0, "stable", Style::new().fg(Color::Rgb(255, 0, 0)));
                Ok(())
            })
            .unwrap();
        }
        let bytes = term.backend().bytes();
        // Steady state: only BSU/ESU wrappers per frame. No cells emitted.
        // 5 frames × 2 escape sequences = at most 80-ish bytes.
        #[cfg(not(feature = "no-synchronized-output"))]
        assert!(
            bytes.len() <= 100,
            "expected tiny steady-state emit, got {} bytes",
            bytes.len()
        );
    }
}
