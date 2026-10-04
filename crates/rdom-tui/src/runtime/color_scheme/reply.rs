//! Scanning the terminal's replies to the startup color query.
//!
//! The query asks for the background color with OSC 11 (xterm's
//! "dynamic colors": `ESC ] 11 ; ? ST`), then for the primary device
//! attributes (DA1, `ESC [ c`). Every VT-compatible terminal answers
//! DA1, and answers queries in order, so the DA1 reply marks the end of
//! the exchange whether or not the terminal knew OSC 11.

use crate::style::Color;

/// The query: OSC 11 for the background, then DA1 as the sentinel.
pub(crate) const QUERY: &[u8] = b"\x1b]11;?\x1b\\\x1b[c";

/// The bytes read so far.
#[derive(Debug, Default)]
pub(crate) struct Replies {
    buf: Vec<u8>,
}

impl Replies {
    /// Add bytes read from the terminal.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// True once the DA1 reply (`ESC [ ? … c`) has arrived.
    pub(crate) fn complete(&self) -> bool {
        let mut rest = &self.buf[..];
        while let Some(i) = find(rest, b"\x1b[?") {
            let tail = &rest[i + 3..];
            let params = tail
                .iter()
                .take_while(|b| b.is_ascii_digit() || **b == b';')
                .count();
            if tail.get(params) == Some(&b'c') {
                return true;
            }
            rest = tail;
        }
        false
    }

    /// The background color of the OSC 11 reply, if it came and parses.
    pub(crate) fn background(&self) -> Option<Color> {
        let start = find(&self.buf, b"\x1b]11;")? + 5;
        let body = &self.buf[start..];
        let end = body
            .iter()
            .position(|b| *b == 0x07)
            .into_iter()
            .chain(find(body, b"\x1b\\"))
            .min()?;
        parse_color_spec(std::str::from_utf8(&body[..end]).ok()?)
    }
}

/// A color in the X11 forms terminals reply with: `rgb:R/G/B` or
/// `rgba:R/G/B/A` (1–4 hex digits per component, each scaled from its
/// own width), or `#RRGGBB`.
fn parse_color_spec(spec: &str) -> Option<Color> {
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

/// The index of `needle` in `hay`.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
#[path = "reply_tests.rs"]
mod tests;
