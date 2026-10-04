//! Command strings (ECMA-48 §5.6, §8.3.89 ST): the framing OSC strings
//! share — what one byte of a string does, and the discard of a string
//! past the length cap.
//!
//! A string's body is bytes in 0x08–0x0D and 0x20–0x7E. It ends at ST
//! (`ESC \`) or BEL (xterm's OSC terminator); CAN or SUB cancels it. An
//! `ESC` that does not start ST ends it, and is read again with its
//! byte. Any other byte — a C0 control, DEL, a non-ASCII byte — cannot
//! be in a string: the string is dropped and the byte read again, so a
//! typed Backspace or Ctrl+C is never lost inside one.

use super::ESC;

/// The longest command string read; a longer one is discarded to its
/// end.
pub(super) const MAX_LEN: usize = 4096;

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
