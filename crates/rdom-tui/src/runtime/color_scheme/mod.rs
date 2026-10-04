//! The terminal's color scheme (CSS Color Adjust 1 §2.1: the
//! document's preferred scheme, which `light-dark()` and — with C14 —
//! `prefers-color-scheme` follow).
//!
//! At startup the `App` asks the terminal for its background color
//! with OSC 11 and takes the scheme that background calls for
//! (`ColorScheme::for_background`); a terminal that does not answer
//! leaves the default, dark. The wait is 200 ms for a reply to begin,
//! and up to 800 ms more once one has (`reply::wait_left`). The replies
//! are read through the App's input reader (`runtime::input`), so keys
//! typed meanwhile are kept for the event loop, and a reply later than
//! the wait is still consumed (the App takes its background). An app
//! sets the scheme itself with `App::with_color_scheme` (which skips
//! the query) or changes it with `App::set_color_scheme`.
//!
//! Theme changes: `App::run` enables DEC mode 2031 on Unix, and the
//! input reader parses its reports (`CSI ? 997 ; 1|2 n`) into the new
//! scheme.
//!
//! - `reply` — the query's exchange and its wait (pure).

mod reply;

use std::time::Duration;

use crate::runtime::input::InputReader;
use crate::style::Color;

/// How long the startup query waits for a reply to begin.
pub(crate) const QUERY_TIMEOUT: Duration = Duration::from_millis(200);

/// How much longer it waits once a reply has begun arriving, for the
/// rest of it (a slow link delivers it in pieces).
pub(crate) const REPLY_GRACE: Duration = Duration::from_millis(800);

/// Ask the terminal for its background (OSC 11), reading the replies
/// through `input` until DA1 answers or the wait ends
/// ([`reply::wait_left`]: `timeout`, plus [`REPLY_GRACE`] once a reply
/// has begun), and return the background it reported. Inputs that are
/// not replies (keys typed during the wait) are put back into `input`
/// for the event loop. `None` when stdout is not a terminal, the
/// platform does not read replies (non-Unix), or no background came.
///
/// Must run with the terminal in raw mode.
pub(crate) fn query_terminal_background(
    input: &mut InputReader,
    timeout: Duration,
) -> Option<Color> {
    query_background(input, timeout)
}

#[cfg(unix)]
fn query_background(input: &mut InputReader, timeout: Duration) -> Option<Color> {
    use std::io::{IsTerminal, Write};
    use std::time::Instant;

    let mut stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return None;
    }
    stdout.write_all(reply::QUERY).ok()?;
    stdout.flush().ok()?;
    let start = Instant::now();
    let mut replies = reply::Replies::default();
    let mut held = Vec::new();
    loop {
        let left = reply::wait_left(
            Instant::now(),
            start,
            &replies,
            input.in_sequence(),
            timeout,
            REPLY_GRACE,
        );
        if left.is_zero() {
            break;
        }
        if input.poll(left).is_err() {
            break;
        }
        while let Some(next) = input.next() {
            held.extend(replies.take(next));
        }
    }
    input.unread(held);
    replies.background()
}

#[cfg(not(unix))]
fn query_background(_input: &mut InputReader, _timeout: Duration) -> Option<Color> {
    None
}
