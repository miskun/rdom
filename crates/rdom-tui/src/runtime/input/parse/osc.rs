//! Operating System Commands (`ESC ] Ps ; Pt BEL|ST`): the terminal's
//! replies to OSC queries. OSC 11 is the background color (xterm's
//! "dynamic colors"); any other OSC is consumed.
//!
//! crossterm reads `ESC ]` as Alt+`]` and the reply's text as keys.
//! rdom takes `ESC ]` followed by a digit as the start of a string —
//! every OSC reply begins with its number — and `ESC ]` followed by
//! anything else (or nothing, after the escape grace) as Alt+`]`. The
//! string's framing is `string`'s: it ends at BEL or ST (`ESC \`); CAN
//! or SUB cancels it; an `ESC` that does not start ST ends it, and is
//! read again with its byte; a byte outside the string range aborts it
//! and is read again; past 4 KiB the rest is discarded to its end.

use crate::style::Color;

use super::string::{self, StringByte};
use super::{ESC, Input, Step, WithAlt, keys};

/// Parse a buffer that starts `ESC ]`.
pub(super) fn parse(buf: &[u8]) -> Step {
    let Some(&first) = buf.get(2) else {
        return Step::Pending;
    };
    if !first.is_ascii_digit() {
        let alt = keys::char_key(']').with_alt();
        return Step::Done(Some(Input::Event(crossterm::event::Event::Key(alt))), 1);
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
    match string::byte(last) {
        StringByte::Bell => finish(&buf[2..n - 1]),
        StringByte::Cancel => Step::consumed(),
        StringByte::Esc => Step::Pending,
        StringByte::Body if n > string::MAX_LEN => Step::Discard,
        StringByte::Body => Step::Pending,
        StringByte::Other => Step::Done(None, 1),
    }
}

/// A complete string's body (`Ps ; Pt`).
fn finish(body: &[u8]) -> Step {
    match body
        .strip_prefix(b"11;")
        .and_then(|spec| std::str::from_utf8(spec).ok())
    {
        Some(spec) => match color_spec(spec) {
            Some(color) => Step::input(Input::Background(color)),
            None => Step::consumed(),
        },
        None => Step::consumed(),
    }
}

/// A color in the X11 forms terminals reply with: `rgb:R/G/B` or
/// `rgba:R/G/B/A` (1–4 hex digits per component, each scaled from its
/// own width), or `#RRGGBB`.
fn color_spec(spec: &str) -> Option<Color> {
    if let Some(hex) = spec.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        return Some(Color::Rgb(byte(0)?, byte(2)?, byte(4)?));
    }
    let (components, count) = if let Some(c) = spec.strip_prefix("rgb:") {
        (c, 3)
    } else if let Some(c) = spec.strip_prefix("rgba:") {
        (c, 4)
    } else {
        return None;
    };
    let parts: Vec<&str> = components.split('/').collect();
    if parts.len() != count {
        return None;
    }
    let scale = |part: &str| -> Option<u8> {
        if part.is_empty() || part.len() > 4 {
            return None;
        }
        let value = u32::from_str_radix(part, 16).ok()?;
        let max = (1u32 << (4 * part.len())) - 1;
        Some(((value * 255 + max / 2) / max) as u8)
    };
    Some(Color::Rgb(
        scale(parts[0])?,
        scale(parts[1])?,
        scale(parts[2])?,
    ))
}
