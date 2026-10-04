//! The terminal's color scheme (CSS Color Adjust 1 §2.1: the
//! document's preferred scheme, which `light-dark()` and — with C14 —
//! `prefers-color-scheme` follow).
//!
//! At startup the `App` asks the terminal for its background color
//! with OSC 11 and takes the scheme that background calls for
//! ([`ColorScheme::for_background`]); a terminal that does not answer
//! leaves the default, dark. An app sets the scheme itself with
//! `App::with_color_scheme` (which skips the query) or changes it with
//! `App::set_color_scheme`.
//!
//! Theme-change notifications (DEC mode 2031) are not listened to:
//! the input parser (crossterm 0.28) does not pass their report
//! (`CSI ? 997 ; n n`) through, and would hold the keystrokes after it,
//! so rdom does not enable the mode (DIVERGENCES).
//!
//! - `reply` — the reply scanner (pure).

mod reply;

use std::time::Duration;

use crate::style::Color;
use rdom_style::color::ColorScheme;

/// How long the startup query waits for the terminal's replies.
pub(crate) const QUERY_TIMEOUT: Duration = Duration::from_millis(200);

/// Ask the terminal for its background (OSC 11), reading the replies
/// from stdin until DA1 answers or `timeout` passes, and return the
/// scheme it calls for. `None` when stdin or stdout is not a terminal,
/// the platform has no raw read (non-Unix), or no background came.
///
/// Must run with the terminal in raw mode and before the input reader
/// starts (bytes read here are not seen by it).
pub(crate) fn query_terminal_scheme(timeout: Duration) -> Option<ColorScheme> {
    query_background(timeout).map(ColorScheme::for_background)
}

#[cfg(unix)]
fn query_background(timeout: Duration) -> Option<Color> {
    use rustix::event::{PollFd, PollFlags, poll};
    use rustix::io::Errno;
    use std::io::{IsTerminal, Write};
    use std::time::Instant;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    if !stdin.is_terminal() || !stdout.is_terminal() {
        return None;
    }
    stdout.write_all(reply::QUERY).ok()?;
    stdout.flush().ok()?;
    let deadline = Instant::now() + timeout;
    let mut replies = reply::Replies::default();
    let mut buf = [0u8; 256];
    while !replies.complete() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let mut fds = [PollFd::new(&stdin, PollFlags::IN)];
        let ms = i32::try_from(left.as_millis()).unwrap_or(i32::MAX).max(1);
        match poll(&mut fds, ms) {
            Ok(0) => break,
            Ok(_) => {}
            Err(Errno::INTR) => continue,
            Err(_) => break,
        }
        match rustix::io::read(&stdin, &mut buf) {
            Ok(0) => break,
            Ok(n) => replies.feed(&buf[..n]),
            Err(Errno::INTR | Errno::AGAIN) => continue,
            Err(_) => break,
        }
    }
    replies.background()
}

#[cfg(not(unix))]
fn query_background(_timeout: Duration) -> Option<Color> {
    None
}
