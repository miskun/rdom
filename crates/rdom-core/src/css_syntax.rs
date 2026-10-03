//! CSS Syntax 3 code-point primitives shared by every CSS reader in the
//! workspace: the selector parser here and the value tokenizer in
//! `rdom-style` both decode identifiers and escapes through these
//! functions, so `\31 0`, `\:` and `\3a ` mean the same thing in a
//! selector, a property name and a keyword value.
//!
//! All functions work on a `&str` that starts at the code point being
//! examined and return how many **bytes** they consumed, so a byte-wise
//! parser (the selector parser) and a char cursor (the tokenizer) can
//! both drive them. Input is taken as already preprocessed in the
//! sense of §3.3 except that `\r\n`, `\r` and `\x0C` are treated as
//! newlines where the spec checks for one.

/// §4.2 "newline": U+000A, plus the code points §3.3 preprocessing
/// turns into one (U+000D, U+000C).
pub fn is_newline(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\x0C')
}

/// §4.2 "whitespace": a newline, U+0009 tab or U+0020 space.
pub fn is_whitespace(c: char) -> bool {
    is_newline(c) || c == '\t' || c == ' '
}

/// §4.2 "ident-start code point": a letter, `_`, or any code point at
/// or above U+0080.
pub fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || !c.is_ascii()
}

/// §4.2 "ident code point": an ident-start code point, a digit or `-`.
pub fn is_ident_char(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit() || c == '-'
}

/// §4.3.8 "check if two code points are a valid escape": `\` not
/// followed by a newline (or by the end of input, which this treats
/// as valid like the spec does — it decodes to U+FFFD).
pub fn is_valid_escape(s: &str) -> bool {
    let mut it = s.chars();
    it.next() == Some('\\') && !it.next().is_some_and(is_newline)
}

/// §4.3.9 "check if three code points would start an ident sequence".
pub fn would_start_ident(s: &str) -> bool {
    let mut it = s.chars();
    match it.next() {
        Some('-') => {
            let rest = &s[1..];
            match rest.chars().next() {
                Some(c) if is_ident_start(c) || c == '-' => true,
                _ => is_valid_escape(rest),
            }
        }
        Some(c) if is_ident_start(c) => true,
        Some('\\') => is_valid_escape(s),
        _ => false,
    }
}

/// §4.3.7 "consume an escaped code point". `s` starts just **after**
/// the `\` (the caller has checked [`is_valid_escape`]). Returns the
/// decoded code point and the bytes consumed:
///
/// - 1–6 hex digits are a code point, and one whitespace after them
///   (`\r\n` counts as one) is part of the escape; zero, a surrogate or
///   a value above U+10FFFF decodes to U+FFFD;
/// - end of input decodes to U+FFFD;
/// - any other code point is itself.
pub fn consume_escape(s: &str) -> (char, usize) {
    let hex_len = s.bytes().take(6).take_while(u8::is_ascii_hexdigit).count();
    if hex_len > 0 {
        let value = u32::from_str_radix(&s[..hex_len], 16).unwrap_or(0);
        let rest = &s[hex_len..];
        let ws = if rest.starts_with("\r\n") {
            2
        } else {
            rest.chars()
                .next()
                .filter(|&c| is_whitespace(c))
                .map_or(0, char::len_utf8)
        };
        let c = match value {
            0 | 0xD800..=0xDFFF => '\u{FFFD}',
            v => char::from_u32(v).unwrap_or('\u{FFFD}'),
        };
        return (c, hex_len + ws);
    }
    match s.chars().next() {
        Some(c) => (c, c.len_utf8()),
        None => ('\u{FFFD}', 0),
    }
}

/// §4.3.11 "consume an ident sequence": ident code points and valid
/// escapes, decoded, up to the first code point that is neither.
/// Returns the decoded name and the bytes consumed. It does not check
/// that the sequence *starts* an ident ([`would_start_ident`] does).
pub fn consume_ident(s: &str) -> (String, usize) {
    let mut out = String::new();
    let mut pos = 0;
    while let Some(c) = s[pos..].chars().next() {
        if is_ident_char(c) {
            out.push(c);
            pos += c.len_utf8();
        } else if is_valid_escape(&s[pos..]) {
            let (decoded, used) = consume_escape(&s[pos + 1..]);
            out.push(decoded);
            pos += 1 + used;
        } else {
            break;
        }
    }
    (out, pos)
}

/// §4.3.5 "consume a string token" body. `s` starts just after the
/// opening `quote`; returns the decoded value and the bytes consumed
/// **including** the closing quote, or `None` when the input ends (or
/// an unescaped newline appears) before the closing quote. Escapes
/// decode per [`consume_escape`]; an escaped newline is dropped.
pub fn consume_string(s: &str, quote: char) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut pos = 0;
    while let Some(c) = s[pos..].chars().next() {
        pos += c.len_utf8();
        if c == quote {
            return Some((out, pos));
        }
        if is_newline(c) {
            return None;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        let rest = &s[pos..];
        match rest.chars().next() {
            None => {}
            Some('\r') if rest.starts_with("\r\n") => pos += 2,
            Some(n) if is_newline(n) => pos += n.len_utf8(),
            Some(_) => {
                let (decoded, used) = consume_escape(rest);
                out.push(decoded);
                pos += used;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §4.3.7: hex escapes take up to six digits and one whitespace.
    #[test]
    fn hex_escapes_decode_and_eat_one_whitespace() {
        assert_eq!(consume_escape("31 0"), ('1', 3));
        assert_eq!(consume_escape("31\r\n0"), ('1', 4));
        assert_eq!(consume_escape("3a b"), (':', 3));
        assert_eq!(consume_escape("00003Ab"), (':', 6));
        assert_eq!(consume_escape("0"), ('\u{FFFD}', 1));
        assert_eq!(consume_escape("D800"), ('\u{FFFD}', 4));
        assert_eq!(consume_escape("110000"), ('\u{FFFD}', 6));
        assert_eq!(consume_escape(":b"), (':', 1));
        assert_eq!(consume_escape(""), ('\u{FFFD}', 0));
    }

    /// §4.3.11 / §4.3.8: escapes continue an ident, an escaped newline
    /// does not.
    #[test]
    fn ident_sequences_decode_escapes() {
        assert_eq!(consume_ident(r"\31 0 x"), ("10".to_string(), 5));
        assert_eq!(consume_ident(r"a\:b{"), ("a:b".to_string(), 4));
        assert_eq!(consume_ident("a\\\nb"), ("a".to_string(), 1));
        assert!(would_start_ident(r"\31 0"));
        assert!(would_start_ident(r"-\31"));
        assert!(!would_start_ident("\\\n"));
        assert!(!would_start_ident("-5"));
    }

    #[test]
    fn strings_decode_escapes_and_drop_escaped_newlines() {
        assert_eq!(
            consume_string(r#"a\"b" tail"#, '"'),
            Some(("a\"b".into(), 5))
        );
        assert_eq!(consume_string("a\\\nb'", '\''), Some(("ab".into(), 5)));
        assert_eq!(consume_string(r"\22 x'", '\''), Some(("\"x".into(), 6)));
        assert_eq!(consume_string("abc", '"'), None);
    }
}
