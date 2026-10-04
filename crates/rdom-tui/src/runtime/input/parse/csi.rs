//! Control sequences (`ESC [ …`): framing and dispatch.
//!
//! A sequence is parameter bytes (0x30–0x3F), intermediate bytes
//! (0x20–0x2F) and one final byte (0x40–0x7E) — ECMA-48 §5.4 — so it
//! ends at its final byte whatever it means; one rdom does not know is
//! consumed whole. A byte outside those ranges cannot be inside a
//! sequence: the sequence is dropped and the byte read again (a typed
//! key is not lost behind a garbled report). Bracketed paste
//! (`CSI 200 ~ text CSI 201 ~`) runs to its end marker. The dispatch on
//! the final byte is crossterm 0.28's.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rdom_style::color::ColorScheme;

use super::{Input, Step, keys, mouse};

const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

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
        // The kitty protocol's legacy F1, F2, F4 (no `1;` without
        // modifiers).
        b'P' => Step::key(KeyCode::F(1)),
        b'Q' => Step::key(KeyCode::F(2)),
        b'S' => Step::key(KeyCode::F(4)),
        b'0'..=b'9' | b';' | b'<' | b'?' => framed(buf),
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
fn dispatch(buf: &[u8]) -> Step {
    let last = buf[buf.len() - 1];
    match buf[2] {
        b'<' if matches!(last, b'M' | b'm') => mouse::sgr(buf),
        b'<' => Step::consumed(),
        b'?' => private(&buf[3..buf.len() - 1], last),
        _ => match last {
            b'M' => mouse::rxvt(buf),
            b'~' => keys::special(buf),
            b'u' => keys::csi_u(buf),
            // A cursor position report (`CSI row ; col R`), which
            // crossterm also keeps out of the event stream.
            b'R' => Step::consumed(),
            _ => keys::modified(buf),
        },
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

/// `CSI 200 ~ text CSI 201 ~`: the text, as one event.
fn paste(buf: &[u8]) -> Step {
    if buf.len() >= PASTE_START.len() + PASTE_END.len() && buf.ends_with(PASTE_END) {
        let text = &buf[PASTE_START.len()..buf.len() - PASTE_END.len()];
        Step::event(Event::Paste(String::from_utf8_lossy(text).into_owned()))
    } else {
        Step::Pending
    }
}
