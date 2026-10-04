//! The terminal's color scheme (CSS Color Adjust 1 §2.1: the
//! document's preferred scheme, which `light-dark()` and — with C14 —
//! `prefers-color-scheme` follow).
//!
//! At startup the `App` asks the terminal for its background color
//! with OSC 11 and takes the scheme that background calls for
//! (`ColorScheme::for_background`); a terminal that does not answer
//! leaves the default, dark. The wait is 200 ms for a reply to begin,
//! and up to 800 ms more once one has (`reply::wait_left`); keys typed
//! meanwhile are read here, not by the input reader, and dropped. An app sets the scheme itself with
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

/// How long the startup query waits for a reply to begin.
pub(crate) const QUERY_TIMEOUT: Duration = Duration::from_millis(200);

/// How much longer it waits once a reply has begun arriving, for the
/// rest of it (a slow link delivers it in pieces).
pub(crate) const REPLY_GRACE: Duration = Duration::from_millis(800);

/// Ask the terminal for its background (OSC 11), reading the replies
/// until DA1 answers or the wait ends ([`reply::wait_left`]: `timeout`,
/// plus [`REPLY_GRACE`] once a reply has begun), and return the
/// background it reported. `None` when stdout is not a terminal, there
/// is no terminal to read (stdin, or `/dev/tty` when stdin is
/// redirected, as crossterm reads input), the platform has no raw read
/// (non-Unix), or no background came.
///
/// Must run with the terminal in raw mode and before the input reader
/// starts: bytes read here are not seen by it, so a key typed during
/// the wait is dropped.
pub(crate) fn query_terminal_background(timeout: Duration) -> Option<Color> {
    query_background(timeout)
}

#[cfg(unix)]
fn query_background(timeout: Duration) -> Option<Color> {
    use std::io::{IsTerminal, Write};

    let mut stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return None;
    }
    let stdin = std::io::stdin();
    // crossterm reads input from `/dev/tty` when stdin is not a terminal
    // (`some-command | app`); so do the replies come.
    let tty;
    let input: std::os::fd::BorrowedFd<'_> = if stdin.is_terminal() {
        std::os::fd::AsFd::as_fd(&stdin)
    } else {
        tty = std::fs::OpenOptions::new()
            .read(true)
            .open("/dev/tty")
            .ok()?;
        std::os::fd::AsFd::as_fd(&tty)
    };
    stdout.write_all(reply::QUERY).ok()?;
    stdout.flush().ok()?;
    read_replies(input, timeout).background()
}

/// Read the terminal's replies from `input` until they are complete or
/// the wait ends.
#[cfg(unix)]
fn read_replies(input: std::os::fd::BorrowedFd<'_>, timeout: Duration) -> reply::Replies {
    use rustix::event::{PollFd, PollFlags, poll};
    use rustix::io::Errno;
    use std::time::Instant;

    let start = Instant::now();
    let mut replies = reply::Replies::default();
    let mut buf = [0u8; 256];
    loop {
        let left = reply::wait_left(Instant::now(), start, &replies, timeout, REPLY_GRACE);
        if left.is_zero() {
            break;
        }
        let mut fds = [PollFd::new(&input, PollFlags::IN)];
        let ms = i32::try_from(left.as_millis()).unwrap_or(i32::MAX).max(1);
        match poll(&mut fds, ms) {
            // Timed out: the next `wait_left` decides — a reply that
            // started in the meantime extends the wait.
            Ok(_) | Err(Errno::INTR) => {}
            Err(_) => break,
        }
        if fds[0].revents().is_empty() {
            continue;
        }
        match rustix::io::read(input, &mut buf) {
            Ok(0) => break,
            Ok(n) => replies.feed(&buf[..n]),
            Err(Errno::INTR | Errno::AGAIN) => continue,
            Err(_) => break,
        }
    }
    replies
}

#[cfg(not(unix))]
fn query_background(_timeout: Duration) -> Option<Color> {
    None
}
