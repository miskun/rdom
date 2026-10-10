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
//! - command strings — OSC (`ESC ] digits … BEL | ST`), DCS and the
//!   kitty graphics APC (`ESC _ G …`) — are consumed; an OSC 11 reply is the background color; one
//!   past 4 KiB is discarded to its end, and a byte that cannot be in a
//!   string aborts it and is read again (ECMA-48 §5.6);
//! - a bracketed paste is delivered capped at 1 MiB, the rest discarded
//!   to its end marker, and a Ctrl+C ends it and is read again (an
//!   unterminated paste swallows neither memory nor the interrupt);
//! - DA1 replies and mode 2031 reports are [`Input`]s, not dropped or
//!   held;
//! - a lone `ESC`, `ESC [`, `ESC O` or string introducer (`ESC ]`,
//!   `ESC P`, `ESC _`) that no byte follows is a key
//!   by itself once [`Parser::flush_prefix`] says so (the reader
//!   calls it after `ESC_GRACE`); crossterm takes a lone `ESC` at the
//!   end of a read as Esc at once, holds `ESC [` and `ESC O`, and reads
//!   an introducer as Alt + its key at once.
//! - `ESC ESC` is two Escs (crossterm reads one), and `ESC` before a CSI
//!   or SS3 key is Alt + that key — the legacy Alt encoding rxvt and
//!   Terminal.app send (`ESC ESC [ A`, Alt+Up).
//! - `CSI R` / `CSI 1 ; m R` is F3 (with modifiers), which crossterm
//!   takes for a cursor position report.
//!
//! Like crossterm under raw mode (the only mode rdom reads in), `\n` is
//! Ctrl+J, not Enter.
//!
//! - `keys` — plain bytes, UTF-8, SS3, and the CSI key encodings.
//! - `mouse` — X10, rxvt and SGR mouse reports.
//! - `csi` — control-sequence framing and dispatch, paste, the private
//!   (`CSI ?`) replies.
//! - `osc` — OSC strings and the OSC 11 color.
//! - `string` — command strings (OSC, DCS, APC): where one
//!   starts, the byte range, the terminators, the discard past the cap.

mod csi;
mod keys;
mod mouse;
mod osc;
mod string;

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
    /// A command string past its length cap: the bytes are dropped, and
    /// the rest of the string is discarded as it arrives.
    Discard,
    /// A paste past its length cap: the capped paste is delivered, and the
    /// rest of it discarded as it arrives.
    PasteOverflow(Input, csi::PasteDiscard),
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
    /// The rest of an over-long command string is being discarded.
    discard: Option<string::Discard>,
    /// The rest of an over-long paste is being discarded.
    paste_discard: Option<csi::PasteDiscard>,
}

impl Parser {
    /// Parse `bytes`, which continue whatever came before.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push(b);
        }
    }

    fn push(&mut self, byte: u8) {
        if let Some(discard) = self.paste_discard.take() {
            match discard.step(byte) {
                csi::PasteStep::Continue(d) => self.paste_discard = Some(d),
                csi::PasteStep::End => {}
                csi::PasteStep::Reread(b) => self.push(b),
            }
            return;
        }
        if let Some(discard) = self.discard {
            self.discard = None;
            match discard.step(byte) {
                string::DiscardStep::Continue(d) => self.discard = Some(d),
                string::DiscardStep::End => {}
                string::DiscardStep::Reread { esc, byte } => {
                    if esc {
                        self.push(ESC);
                    }
                    self.push(byte);
                }
            }
            return;
        }
        self.buf.push(byte);
        match parse(&self.buf) {
            Step::Pending => {}
            Step::Invalid => self.buf.clear(),
            Step::Discard => {
                self.buf.clear();
                self.discard = Some(string::Discard::START);
            }
            Step::PasteOverflow(input, discard) => {
                self.buf.clear();
                self.ready.push_back(input);
                self.paste_discard = Some(discard);
            }
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
        self.discard.is_some() || self.paste_discard.is_some() || self.buf.first() == Some(&ESC)
    }

    /// True while the buffer is a prefix that is also a key by itself —
    /// `ESC`, or `ESC` and `[`, `O` or a string introducer (`]`, `P`,
    /// `_`, `^`, `X`) — which [`Self::flush_prefix`]
    /// resolves when no more bytes come.
    pub(crate) fn awaits_prefix(&self) -> bool {
        match self.buf[..] {
            [ESC] | [ESC, ESC] | [ESC, ESC, b'[' | b'O'] => true,
            [ESC, b] => matches!(b, b'[' | b'O') || string::is_introducer(b),
            _ => false,
        }
    }

    /// No more bytes came: a lone `ESC` is Esc, and `ESC` + `[` / `O` /
    /// a string introducer is Alt + that key (crossterm's reading of the same bytes when
    /// a sequence does not follow); an `ESC` before one of those is an
    /// Esc of its own.
    pub(crate) fn flush_prefix(&mut self) {
        if self.buf.len() > 1 && self.buf[1] == ESC && self.awaits_prefix() {
            self.buf.remove(0);
            self.ready
                .push_back(Input::Event(Event::Key(KeyCode::Esc.into())));
            return self.flush_prefix();
        }
        let key = match self.buf[..] {
            [ESC] => KeyEvent::from(KeyCode::Esc),
            [ESC, b] if matches!(b, b'[' | b'O') || string::is_introducer(b) => {
                keys::char_key(b as char).with_alt()
            }
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
        b'P' | b'_' => string::parse(buf, |_| Step::consumed()),
        ESC => escape_then(buf),
        // `ESC` + a key: Alt + the key.
        _ => match keys::plain(&buf[1..]) {
            Step::Done(Some(Input::Event(Event::Key(k))), unread) => {
                Step::Done(Some(Input::Event(Event::Key(k.with_alt()))), unread)
            }
            other => other,
        },
    }
}

/// `ESC ESC …`: the legacy Alt encoding of a CSI or SS3 key (rxvt,
/// Terminal.app's Option-as-Meta: `ESC ESC [ A` is Alt+Up) — Alt + the
/// key; else Esc, and the second `ESC` is read again with what follows.
/// (crossterm reads one Esc for both `ESC`s.)
fn escape_then(buf: &[u8]) -> Step {
    let esc = || {
        Step::Done(
            Some(Input::Event(Event::Key(KeyCode::Esc.into()))),
            buf.len() - 1,
        )
    };
    match buf.get(2) {
        None => Step::Pending,
        Some(b'[' | b'O') => match parse(&buf[1..]) {
            Step::Pending => Step::Pending,
            Step::Done(Some(Input::Event(Event::Key(k))), unread) => {
                Step::Done(Some(Input::Event(Event::Key(k.with_alt()))), unread)
            }
            _ => esc(),
        },
        Some(_) => esc(),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
