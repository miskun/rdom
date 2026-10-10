//! Control sequences (`ESC [ …`): framing and dispatch.
//!
//! A sequence is parameter bytes (0x30–0x3F), intermediate bytes
//! (0x20–0x2F) and one final byte (0x40–0x7E) — ECMA-48 §5.4 — so it
//! ends at its final byte whatever it means; one rdom does not know is
//! consumed whole. A byte outside those ranges cannot be inside a
//! sequence: the sequence is dropped and the byte read again (a typed
//! key is not lost behind a garbled report). A sequence with
//! intermediate bytes or a private marker other than `<` / `?` is
//! consumed (a DA2 reply, `CSI > … c`). Bracketed paste
//! (`CSI 200 ~ text CSI 201 ~`) runs to its end marker. The dispatch on
//! the final byte is crossterm 0.28's, except that `CSI … R` is F3, not
//! a cursor position report (rdom never sends DSR 6).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rdom_style::color::ColorScheme;

use super::{Input, Step, keys, mouse};

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

/// The longest paste delivered, in bytes (C16G-HARDENING).
pub(super) const MAX_PASTE_LEN: usize = 1 << 20;

/// Parse a buffer that starts `ESC [`.
pub(super) fn parse(buf: &[u8]) -> Step {
    let Some(&first) = buf.get(2) else {
        return Step::Pending;
    };
    match first {
        // The Linux console's F1–F5: `ESC [ [ A` … `E`.
        b'[' => match buf.get(3) {
            None => Step::Pending,
            Some(&v @ b'A'..=b'E') => Step::key(KeyCode::F(1 + v - b'A')),
            Some(_) => Step::Invalid,
        },
        b'D' => Step::key(KeyCode::Left),
        b'C' => Step::key(KeyCode::Right),
        b'A' => Step::key(KeyCode::Up),
        b'B' => Step::key(KeyCode::Down),
        b'H' => Step::key(KeyCode::Home),
        b'F' => Step::key(KeyCode::End),
        b'Z' => Step::key(KeyEvent::new_with_kind(
            KeyCode::BackTab,
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        )),
        b'M' => mouse::x10(buf),
        b'I' => Step::event(Event::FocusGained),
        b'O' => Step::event(Event::FocusLost),
        // The legacy F1–F4 (no `1;` without modifiers).
        b'P' => Step::key(KeyCode::F(1)),
        b'Q' => Step::key(KeyCode::F(2)),
        // F3: rdom never asks for a cursor position report (DSR 6),
        // whose reply also ends in `R`.
        b'R' => Step::key(KeyCode::F(3)),
        b'S' => Step::key(KeyCode::F(4)),
        // Parameter (0x30–0x3F) and intermediate (0x20–0x2F) bytes.
        0x20..=0x3f => framed(buf),
        _ => Step::Invalid,
    }
}

/// A sequence with parameters: wait for the final byte.
fn framed(buf: &[u8]) -> Step {
    if buf.starts_with(PASTE_START) {
        return paste(buf);
    }
    match buf[buf.len() - 1] {
        0x40..=0x7e => dispatch(buf),
        0x20..=0x3f => Step::Pending,
        // Not part of any sequence: drop the sequence, read the byte
        // again.
        _ => Step::Done(None, 1),
    }
}

/// A complete sequence, by its first parameter byte and final byte.
/// One with intermediate bytes, or a private marker rdom does not read
/// (`>`, `=`, `:` — a DA2 reply is `CSI > … c`), is consumed: no key
/// or report rdom reads has them.
fn dispatch(buf: &[u8]) -> Step {
    let last = buf[buf.len() - 1];
    let params = &buf[2..buf.len() - 1];
    if params.iter().any(|b| matches!(b, 0x20..=0x2f)) {
        return Step::consumed();
    }
    match buf[2] {
        b'<' if matches!(last, b'M' | b'm') => mouse::sgr(buf),
        b'?' => private(&buf[3..buf.len() - 1], last),
        b'0'..=b'9' | b';' => match last {
            b'M' => mouse::rxvt(buf),
            b'~' => keys::special(buf),
            b'u' => keys::csi_u(buf),
            // `CSI 1 ; m R` is F3 with modifiers, not a cursor
            // position report: rdom never sends DSR 6.
            _ => keys::modified(buf),
        },
        _ => Step::consumed(),
    }
}

/// `CSI ? params final` — replies and reports.
fn private(params: &[u8], last: u8) -> Step {
    match (last, params) {
        // DA1: the startup query's end marker.
        (b'c', _) => Step::input(Input::DeviceAttributes),
        // DEC mode 2031 (contour's "dark and light mode detection"):
        // 1 dark, 2 light.
        (b'n', b"997;1") => Step::input(Input::ColorScheme(ColorScheme::Dark)),
        (b'n', b"997;2") => Step::input(Input::ColorScheme(ColorScheme::Light)),
        // The kitty keyboard flags reply (`CSI ? flags u`), mode
        // reports, anything else: consumed.
        _ => Step::consumed(),
    }
}

/// `CSI 200 ~ text CSI 201 ~`: the text, as one event — capped at
/// [`MAX_PASTE_LEN`] bytes, the rest discarded to the end marker
/// ([`PasteDiscard`]); and ended by Ctrl+C (ETX), which terminals take
/// out of what they paste, so an unterminated paste cannot swallow it
/// (C16G-HARDENING).
fn paste(buf: &[u8]) -> Step {
    let body = &buf[PASTE_START.len().min(buf.len())..];
    if buf.len() >= PASTE_START.len() + PASTE_END.len() && buf.ends_with(PASTE_END) {
        return Step::event(paste_event(&body[..body.len() - PASTE_END.len()]));
    }
    match body.last() {
        Some(&ETX) => Step::Done(Some(Input::Event(paste_event(&body[..body.len() - 1]))), 1),
        _ => {
            // A trailing start of the end marker is not text yet.
            let marker = end_marker_prefix(body);
            if body.len() - marker > MAX_PASTE_LEN {
                let cut = char_boundary(body, MAX_PASTE_LEN);
                Step::PasteOverflow(
                    Input::Event(paste_event(&body[..cut])),
                    PasteDiscard { matched: marker },
                )
            } else {
                Step::Pending
            }
        }
    }
}

/// Ctrl+C.
const ETX: u8 = 0x03;

fn paste_event(text: &[u8]) -> Event {
    Event::Paste(String::from_utf8_lossy(text).into_owned())
}

/// How many bytes at the end of `body` start the end marker.
fn end_marker_prefix(body: &[u8]) -> usize {
    (1..PASTE_END.len())
        .rev()
        .find(|&n| body.ends_with(&PASTE_END[..n]))
        .unwrap_or(0)
}

/// The largest UTF-8 character boundary of `text` at or below `at`.
fn char_boundary(text: &[u8], at: usize) -> usize {
    (0..=at.min(text.len()))
        .rev()
        .find(|&i| i == text.len() || text[i] & 0xc0 != 0x80)
        .unwrap_or(0)
}

/// The discard of a paste past [`MAX_PASTE_LEN`]: its bytes are dropped,
/// without buffering, until its end marker — or a Ctrl+C, which is read
/// again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::runtime::input) struct PasteDiscard {
    /// How many bytes of the end marker the last bytes matched.
    matched: usize,
}

/// What a byte does to a discarded paste.
#[derive(Debug, PartialEq)]
pub(super) enum PasteStep {
    /// Still discarding.
    Continue(PasteDiscard),
    /// The paste ended.
    End,
    /// The paste ended at a Ctrl+C, read again.
    Reread(u8),
}

impl PasteDiscard {
    /// Feed one byte.
    pub(super) fn step(self, b: u8) -> PasteStep {
        if b == ETX {
            return PasteStep::Reread(b);
        }
        if b == PASTE_END[self.matched] {
            return match self.matched + 1 {
                n if n == PASTE_END.len() => PasteStep::End,
                n => PasteStep::Continue(PasteDiscard { matched: n }),
            };
        }
        let matched = usize::from(b == PASTE_END[0]);
        PasteStep::Continue(PasteDiscard { matched })
    }
}
