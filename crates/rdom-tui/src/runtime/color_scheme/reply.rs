//! The startup color query's exchange: what it sends, the replies it
//! has read, and how long it still waits.
//!
//! The query asks for the background color with OSC 11 (xterm's
//! "dynamic colors": `ESC ] 11 ; ? ST`), then for the primary device
//! attributes (DA1, `ESC [ c`). Every VT-compatible terminal answers
//! DA1, and answers queries in order, so the DA1 reply marks the end of
//! the exchange whether or not the terminal knew OSC 11. The input
//! parser (`runtime::input`) reads the replies.

use std::time::{Duration, Instant};

use crate::runtime::input::Input;
use crate::style::Color;

/// The query: OSC 11 for the background, then DA1 as the sentinel.
pub(crate) const QUERY: &[u8] = b"\x1b]11;?\x1b\\\x1b[c";

/// The replies read so far.
#[derive(Debug, Default)]
pub(crate) struct Replies {
    background: Option<Color>,
    complete: bool,
}

impl Replies {
    /// Take `input` if it is a reply to the query; anything else (a key
    /// typed during the wait) is handed back.
    pub(crate) fn take(&mut self, input: Input) -> Option<Input> {
        match input {
            Input::Background(color) => self.background = Some(color),
            Input::DeviceAttributes => self.complete = true,
            other => return Some(other),
        }
        None
    }

    /// True once a reply has begun arriving: one has been read, or the
    /// reader is partway through an escape sequence (`mid_sequence`).
    /// Keys typed meanwhile start nothing.
    pub(crate) fn started(&self, mid_sequence: bool) -> bool {
        self.background.is_some() || self.complete || mid_sequence
    }

    /// True once the DA1 reply has arrived.
    pub(crate) fn complete(&self) -> bool {
        self.complete
    }

    /// The background of the OSC 11 reply, if it came and parses.
    pub(crate) fn background(&self) -> Option<Color> {
        self.background
    }
}

/// How much longer the query started at `start` waits at `now`:
/// nothing once the replies are complete; until `start + timeout` while
/// no reply has started; and, once one has (`mid_sequence`: the reader
/// holds part of one), `grace` longer — a reply arriving in pieces over
/// a slow link is read whole. A reply that has not started by
/// `start + timeout` is not waited for; if it comes later, the input
/// reader still consumes it (and the App takes its background).
pub(crate) fn wait_left(
    now: Instant,
    start: Instant,
    replies: &Replies,
    mid_sequence: bool,
    timeout: Duration,
    grace: Duration,
) -> Duration {
    if replies.complete() {
        return Duration::ZERO;
    }
    let deadline = if replies.started(mid_sequence) {
        start + timeout + grace
    } else {
        start + timeout
    };
    deadline.saturating_duration_since(now)
}

#[cfg(test)]
#[path = "reply_tests.rs"]
mod tests;
