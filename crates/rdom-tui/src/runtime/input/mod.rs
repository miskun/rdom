//! Terminal input: rdom reads the terminal itself (`C3G-INPUT-READER`).
//!
//! On Unix, [`InputReader`] reads the terminal's bytes (stdin, or
//! `/dev/tty` when stdin is redirected — crossterm's rule) and the
//! window-size signal, and [`Parser`] turns the bytes into [`Input`]s:
//! crossterm's own `Event` / `KeyEvent` / `MouseEvent` values for keys,
//! mouse, paste, focus and resize — so the rest of the runtime is
//! unchanged — plus the terminal's replies and reports, which are not
//! keystrokes:
//!
//! - an OSC 11 reply (`ESC ] 11 ; rgb:… ST`), the background color —
//!   the startup query's answer, or one that came after its wait (other
//!   command strings — OSC, DCS, the kitty graphics APC — are consumed);
//! - a DA1 reply (`CSI ? … c`), which ends the startup query;
//! - a DEC mode 2031 theme report (`CSI ? 997 ; 1|2 n`), the terminal's
//!   new color scheme.
//!
//! crossterm (0.28) stays for output, terminal modes and the Windows
//! path, where the reader wraps `crossterm::event` (no replies or
//! reports are read there). Its Unix parser holds every byte after a
//! `CSI ?` sequence that does not end in `u` or `c`, which is why
//! rdom reads input itself.
//!
//! - `parse` — the escape-sequence parser (pure, modeled on crossterm's
//!   `event/sys/unix/parse.rs`).
//! - `reader` — the reader over the terminal (poll + read, the
//!   escape grace, the window-size signal).

mod parse;
mod reader;

use std::time::Duration;

use crossterm::event::Event;
use rdom_style::color::ColorScheme;

use crate::style::Color;

pub(crate) use parse::Parser;
pub(crate) use reader::InputReader;

/// One thing the terminal sent.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Input {
    /// A key, mouse, paste, focus or resize event.
    Event(Event),
    /// The background color, in reply to OSC 11.
    Background(Color),
    /// The reply to the primary device attributes query (DA1).
    DeviceAttributes,
    /// The terminal's color scheme: a DEC mode 2031 report
    /// (`CSI ? 997 ; 1 n` dark, `; 2 n` light).
    ColorScheme(ColorScheme),
}

/// How long a lone `ESC` (or `ESC [`, `ESC O`, a string introducer
/// `ESC ]` / `P` / `_` / `^` / `X`) waits for the
/// rest of a sequence before it is taken as a key by itself (Esc, or
/// Alt + the second byte). crossterm decides at the end of each read;
/// the grace also joins a sequence a slow link splits right after its
/// `ESC`.
pub(crate) const ESC_GRACE: Duration = Duration::from_millis(25);
