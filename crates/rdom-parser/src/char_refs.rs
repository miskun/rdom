//! Character references (HTML §13.2.5.72–80): `&name;`, `&#nn;` and
//! `&#xhh;` decoded against the WHATWG named-reference table — the
//! scanner text, attribute values and RCDATA bodies share. Split out of
//! `parser.rs` (C7G-SIZES).

use crate::entities::{LONGEST_LEGACY_NAME, LONGEST_NAME, NAMED_REFERENCES};

/// Where a character reference sits. HTML §13.2.5.73 leaves a legacy
/// no-semicolon reference literal inside an attribute value when `=` or
/// an alphanumeric follows it (`?a=1&copy=2`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefContext {
    Text,
    Attribute,
}

/// HTML §13.2.5.80: the C1 control range 0x80–0x9F is remapped to the
/// Windows-1252 code points (`&#146;` is `’`, `&#150;` is `–`); the five
/// unmapped positions (0x81, 0x8D, 0x8F, 0x90, 0x9D) pass through.
const C1_REMAP: [(u32, char); 27] = [
    (0x80, '\u{20AC}'),
    (0x82, '\u{201A}'),
    (0x83, '\u{0192}'),
    (0x84, '\u{201E}'),
    (0x85, '\u{2026}'),
    (0x86, '\u{2020}'),
    (0x87, '\u{2021}'),
    (0x88, '\u{02C6}'),
    (0x89, '\u{2030}'),
    (0x8A, '\u{0160}'),
    (0x8B, '\u{2039}'),
    (0x8C, '\u{0152}'),
    (0x8E, '\u{017D}'),
    (0x91, '\u{2018}'),
    (0x92, '\u{2019}'),
    (0x93, '\u{201C}'),
    (0x94, '\u{201D}'),
    (0x95, '\u{2022}'),
    (0x96, '\u{2013}'),
    (0x97, '\u{2014}'),
    (0x98, '\u{02DC}'),
    (0x99, '\u{2122}'),
    (0x9A, '\u{0161}'),
    (0x9B, '\u{203A}'),
    (0x9C, '\u{0153}'),
    (0x9E, '\u{017E}'),
    (0x9F, '\u{0178}'),
];

/// Decode the digits of a numeric reference per HTML §13.2.5.80: U+0000,
/// surrogates and values above U+10FFFF (including anything that
/// overflows `u32`) yield U+FFFD; the C1 range is remapped.
fn decode_numeric(digits: &str, radix: u32) -> char {
    let n = u32::from_str_radix(digits, radix).unwrap_or(u32::MAX);
    match n {
        0 | 0xD800..=0xDFFF => '\u{FFFD}',
        0x80..=0x9F => C1_REMAP
            .iter()
            .find(|(from, _)| *from == n)
            .map_or_else(|| char::from_u32(n).unwrap_or('\u{FFFD}'), |(_, to)| *to),
        _ => char::from_u32(n).unwrap_or('\u{FFFD}'),
    }
}

/// Look `name` (without `&`, with or without `;`) up in the WHATWG table.
fn named_reference(name: &str) -> Option<&'static str> {
    NAMED_REFERENCES
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| NAMED_REFERENCES[i].1)
}

/// Scan a character reference whose `&` has just been consumed
/// (`after_amp` starts right after it). Returns the decoded text and the
/// number of bytes to consume, or `None` when the caller should keep the
/// `&` literal. HTML §13.2.5.72–80:
///
/// - numeric: `#` + digits (or `#x` + hex digits), an optional `;`;
/// - named: the *longest* table prefix wins, so `&notit;` is `¬it;`
///   (the legacy `not`) and `&notin;` is `∉`;
/// - a legacy (no-`;`) match inside an attribute value is left literal
///   when the next character is `=` or alphanumeric, so query strings
///   like `?a=1&copy=2` survive.
pub(crate) fn scan_reference(after_amp: &str, context: RefContext) -> Option<(String, usize)> {
    let bytes = after_amp.as_bytes();
    if bytes.first() == Some(&b'#') {
        // Decimal / hexadecimal character reference states consume digits
        // of their radix only; whatever follows (`;` or not) is left for
        // the caller (`&#65abc;` → `A` + `abc;`).
        let (start, radix): (usize, u32) = match bytes.get(1) {
            Some(b'x' | b'X') => (2, 16),
            _ => (1, 10),
        };
        let mut n = start;
        while n < bytes.len() && (bytes[n] as char).is_digit(radix) {
            n += 1;
        }
        if n == start {
            return None;
        }
        let c = decode_numeric(&after_amp[start..n], radix);
        let consumed = if bytes.get(n) == Some(&b';') {
            n + 1
        } else {
            n
        };
        return Some((c.to_string(), consumed));
    }
    let mut n = 0;
    while n < bytes.len() && n < LONGEST_NAME && bytes[n].is_ascii_alphanumeric() {
        n += 1;
    }
    if n == 0 {
        return None;
    }
    if bytes.get(n) == Some(&b';')
        && let Some(v) = named_reference(&after_amp[..=n])
    {
        return Some((v.to_string(), n + 1));
    }
    // Only legacy names can match without `;`, and they are short.
    for len in (1..=n.min(LONGEST_LEGACY_NAME)).rev() {
        let Some(v) = named_reference(&after_amp[..len]) else {
            continue;
        };
        if context == RefContext::Attribute
            && let Some(&next) = bytes.get(len)
            && (next == b'=' || next.is_ascii_alphanumeric())
        {
            return None;
        }
        return Some((v.to_string(), len));
    }
    None
}

/// Decode every character reference in `text` (RCDATA bodies) with the
/// same scanner the text path uses.
pub(crate) fn decode_character_references(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        match scan_reference(after, RefContext::Text) {
            Some((decoded, consumed)) => {
                out.push_str(&decoded);
                rest = &after[consumed..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}
