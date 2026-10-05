//! Command strings (ECMA-48 §5.6, §8.3.89 ST): OSC (`ESC ]`), DCS
//! (`ESC P`) and APC (`ESC _`) — their framing, what one byte of a
//! string does, and the discard of a string past the length cap. OSC 11
//! is read (`osc`); every other string is consumed.
//!
//! crossterm reads each introducer as Alt + its key and the string as
//! typed text. rdom takes the introducer as a string's start only when
//! the next byte starts a string a terminal sends — OSC: a digit (every
//! OSC reply begins with its number); DCS: a parameter or intermediate
//! byte (0x20–0x3F, as every DCS reply — XTVERSION's `>|`, DECRQSS's
//! `1$r`); APC: `G` (the kitty graphics reply, the one APC terminals
//! send) — and as Alt + the key otherwise, or when nothing follows
//! within the escape grace. PM (`ESC ^`) and SOS (`ESC X`) are not
//! framed at all: no terminal replies with one, and taking any byte as
//! their start swallowed what was typed after Alt+^ or Alt+Shift+X
//! (`C5G-STRING-INTRO`).
//!
//! A string's body is bytes in 0x08–0x0D and 0x20–0x7E. It ends at ST
//! (`ESC \`) or BEL (xterm's OSC terminator); CAN or SUB cancels it. An
//! `ESC` that does not start ST ends it, and is read again with its
//! byte. Any other byte — a C0 control, DEL, a non-ASCII byte — cannot
//! be in a string: the string is dropped and the byte read again, so a
//! typed Backspace or Ctrl+C is never lost inside one.

use crossterm::event::Event;

use super::{ESC, Input, Step, WithAlt, keys};

/// The longest command string read; a longer one is discarded to its
/// end.
pub(super) const MAX_LEN: usize = 4096;

/// True when `intro` after `ESC` introduces a command string.
pub(super) fn is_introducer(intro: u8) -> bool {
    matches!(intro, b']' | b'P' | b'_')
}

/// Parse a buffer that starts `ESC` + a string introducer: Alt + the
/// introducer's key when the next byte cannot start the string (that
/// byte is read again); else the string, its body handed to `finish`
/// once it ends.
pub(super) fn parse(buf: &[u8], finish: fn(&[u8]) -> Step) -> Step {
    let Some(&first) = buf.get(2) else {
        return Step::Pending;
    };
    if !starts(buf[1], first) {
        let alt = keys::char_key(char::from(buf[1])).with_alt();
        return Step::Done(Some(Input::Event(Event::Key(alt))), 1);
    }
    let n = buf.len();
    let last = buf[n - 1];
    if n > 3 && buf[n - 2] == ESC {
        return match last {
            b'\\' => finish(&buf[2..n - 2]),
            // The `ESC` ended the string: read it again with its byte.
            _ => Step::Done(None, 2),
        };
    }
    match byte(last) {
        StringByte::Bell => finish(&buf[2..n - 1]),
        StringByte::Cancel => Step::consumed(),
        StringByte::Esc => Step::Pending,
        StringByte::Body if n > MAX_LEN => Step::Discard,
        StringByte::Body => Step::Pending,
        StringByte::Other => Step::Done(None, 1),
    }
}

/// True when `first` can start the string `intro` introduces.
fn starts(intro: u8, first: u8) -> bool {
    match intro {
        b']' => first.is_ascii_digit(),
        b'P' => matches!(first, 0x20..=0x3f),
        b'_' => first == b'G',
        _ => false,
    }
}

/// What one byte does to a command string.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum StringByte {
    /// Part of the body.
    Body,
    /// `ESC`: the start of ST, or the end of the string.
    Esc,
    /// BEL: the end of the string.
    Bell,
    /// CAN / SUB: the string is cancelled.
    Cancel,
    /// Not part of any string: the string ends, the byte is read again.
    Other,
}

/// Classify `byte` inside a command string.
pub(super) fn byte(byte: u8) -> StringByte {
    match byte {
        ESC => StringByte::Esc,
        0x07 => StringByte::Bell,
        0x18 | 0x1a => StringByte::Cancel,
        0x08..=0x0d | 0x20..=0x7e => StringByte::Body,
        _ => StringByte::Other,
    }
}

/// The discard of a string past [`MAX_LEN`]: its bytes are dropped,
/// without buffering, until it ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Discard {
    /// The last byte was `ESC` (ST may follow).
    after_esc: bool,
}

/// What a byte does to a discarded string.
#[derive(Debug, PartialEq)]
pub(super) enum DiscardStep {
    /// Still discarding.
    Continue(Discard),
    /// The string ended; nothing to read again.
    End,
    /// The string ended; `byte` is read again — after an `ESC` when
    /// `esc` (an `ESC` that did not start ST ends the string).
    Reread { esc: bool, byte: u8 },
}

impl Discard {
    pub(super) const START: Discard = Discard { after_esc: false };

    /// Feed one byte.
    pub(super) fn step(self, b: u8) -> DiscardStep {
        if self.after_esc {
            return match b {
                b'\\' => DiscardStep::End,
                _ => DiscardStep::Reread { esc: true, byte: b },
            };
        }
        match byte(b) {
            StringByte::Body => DiscardStep::Continue(self),
            StringByte::Esc => DiscardStep::Continue(Discard { after_esc: true }),
            StringByte::Bell | StringByte::Cancel => DiscardStep::End,
            StringByte::Other => DiscardStep::Reread {
                esc: false,
                byte: b,
            },
        }
    }
}
