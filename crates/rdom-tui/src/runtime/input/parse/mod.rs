//! The escape-sequence parser: terminal bytes → [`Input`]s.
//!
//! Modeled on crossterm 0.28's `event/sys/unix/parse.rs`, whose results
//! it reproduces for every sequence crossterm parses (keys with
//! modifiers, the kitty keyboard protocol's `CSI … u`, SS3 keys, X10 /
//! rxvt / SGR mouse, bracketed paste, focus). Where it differs, it is
//! because crossterm would stall or turn a reply into keystrokes:
//!
//! - every control sequence is framed by its final byte (ECMA-48 §5.4),
//!   so an unknown one — `CSI ? 997 ; 1 n`, which crossterm holds
//!   forever along with every byte after it — is consumed whole;
//! - a C0 control byte inside a control sequence ends it, and is read
//!   again as a key;
//! - OSC strings (`ESC ] digits … BEL | ST`) are consumed; an OSC 11
//!   reply is the background color;
//! - DA1 replies and mode 2031 reports are [`Input`]s, not dropped or
//!   held;
//! - a lone `ESC`, `ESC [`, `ESC O` or `ESC ]` that no byte follows is
//!   a key by itself once [`Parser::flush_prefix`] says so (the reader
//!   calls it after `ESC_GRACE`); crossterm takes a lone `ESC` at the
//!   end of a read as Esc at once, and holds the other three.
//!
//! Like crossterm under raw mode (the only mode rdom reads in), `\n` is
//! Ctrl+J, not Enter.
//!
//! - `keys` — plain bytes, UTF-8, SS3, and the CSI key encodings.
//! - `mouse` — X10, rxvt and SGR mouse reports.
//! - `csi` — control-sequence framing and dispatch, paste, the private
//!   (`CSI ?`) replies.
//! - `osc` — OSC strings and the OSC 11 color.

mod csi;
mod keys;
mod mouse;
mod osc;

use std::collections::VecDeque;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::Input;

const ESC: u8 = 0x1b;

/// What the bytes buffered so far amount to.
#[derive(Debug, PartialEq)]
pub(super) enum Step {
    /// The start of a sequence: more bytes are needed.
    Pending,
    /// Not something rdom reads: the bytes are dropped.
    Invalid,
    /// A complete sequence — what it means (`None`: consumed, nothing to
    /// deliver) — and how many trailing bytes were not part of it and
    /// are read again.
    Done(Option<Input>, usize),
}

impl Step {
    fn input(input: Input) -> Step {
        Step::Done(Some(input), 0)
    }

    fn event(event: Event) -> Step {
        Step::input(Input::Event(event))
    }

    fn key(key: impl Into<KeyEvent>) -> Step {
        Step::event(Event::Key(key.into()))
    }

    /// Consumed, nothing to deliver (a reply rdom does not use).
    fn consumed() -> Step {
        Step::Done(None, 0)
    }
}

/// The parser: bytes in ([`Parser::feed`]), [`Input`]s out
/// ([`Parser::next`]).
#[derive(Debug, Default)]
pub(crate) struct Parser {
    /// The bytes of the sequence being read.
    buf: Vec<u8>,
    /// Parsed inputs not yet taken.
    ready: VecDeque<Input>,
}

impl Parser {
    /// Parse `bytes`, which continue whatever came before.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push(b);
        }
    }

    fn push(&mut self, byte: u8) {
        self.buf.push(byte);
        match parse(&self.buf) {
            Step::Pending => {}
            Step::Invalid => self.buf.clear(),
            Step::Done(input, unread) => {
                let tail = self.buf.split_off(self.buf.len() - unread);
                self.buf.clear();
                self.ready.extend(input);
                for b in tail {
                    self.push(b);
                }
            }
        }
    }

    /// The next parsed input.
    pub(crate) fn next(&mut self) -> Option<Input> {
        self.ready.pop_front()
    }

    /// True when an input is ready.
    pub(crate) fn has_ready(&self) -> bool {
        !self.ready.is_empty()
    }

    /// Queue an input that did not come from bytes (a resize).
    pub(crate) fn deliver(&mut self, input: Input) {
        self.ready.push_back(input);
    }

    /// Put inputs back in front of the queue, in order (the startup
    /// query reads past keys typed during its wait).
    pub(crate) fn unread(&mut self, inputs: Vec<Input>) {
        for input in inputs.into_iter().rev() {
            self.ready.push_front(input);
        }
    }

    /// True while the buffer holds the start of an escape sequence (a
    /// reply that has begun arriving, say).
    pub(crate) fn in_sequence(&self) -> bool {
        self.buf.first() == Some(&ESC)
    }

    /// True while the buffer is a prefix that is also a key by itself —
    /// `ESC`, or `ESC` and `[`, `O` or `]` — which [`Self::flush_prefix`]
    /// resolves when no more bytes come.
    pub(crate) fn awaits_prefix(&self) -> bool {
        matches!(self.buf[..], [ESC] | [ESC, b'[' | b'O' | b']'])
    }

    /// No more bytes came: a lone `ESC` is Esc, and `ESC` + `[` / `O` /
    /// `]` is Alt + that key (crossterm's reading of the same bytes when
    /// a sequence does not follow).
    pub(crate) fn flush_prefix(&mut self) {
        let key = match self.buf[..] {
            [ESC] => KeyEvent::from(KeyCode::Esc),
            [ESC, b] if matches!(b, b'[' | b'O' | b']') => keys::char_key(b as char).with_alt(),
            _ => return,
        };
        self.buf.clear();
        self.ready.push_back(Input::Event(Event::Key(key)));
    }
}

/// Add Alt to a key event (`ESC` + a key, crossterm's Alt encoding).
trait WithAlt {
    fn with_alt(self) -> Self;
}

impl WithAlt for KeyEvent {
    fn with_alt(mut self) -> Self {
        self.modifiers |= KeyModifiers::ALT;
        self
    }
}

/// Parse the buffered bytes of one sequence.
fn parse(buf: &[u8]) -> Step {
    if buf[0] != ESC {
        return keys::plain(buf);
    }
    let Some(&second) = buf.get(1) else {
        return Step::Pending;
    };
    match second {
        b'O' => keys::ss3(buf),
        b'[' => csi::parse(buf),
        b']' => osc::parse(buf),
        ESC => Step::key(KeyCode::Esc),
        // `ESC` + a key: Alt + the key.
        _ => match keys::plain(&buf[1..]) {
            Step::Done(Some(Input::Event(Event::Key(k))), unread) => {
                Step::Done(Some(Input::Event(Event::Key(k.with_alt()))), unread)
            }
            other => other,
        },
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
