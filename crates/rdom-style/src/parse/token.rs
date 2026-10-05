//! Tokenizer used by the declaration-block parser.
//!
//! Top-level (selector text → `{` → body → `}`) still uses the
//! cursor-level byte walk in `top_level.rs`. This tokenizer
//! operates on the captured body string and produces value-level
//! tokens that the property parsers consume.

use rdom_core::css_syntax;

use crate::parse::cursor::Cursor;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Token {
    /// An ident sequence (CSS Syntax 3 §4.3.11), escapes decoded
    /// (`col\6f r` is `color`). Includes custom-property names like
    /// `--accent` (CSS treats them as idents).
    Ident(String),
    /// `<number-token>` with the *integer* type flag (CSS Syntax 3
    /// §4.3.12): digits only; a literal past `i32` is clamped to
    /// `i32::MAX` (CSS Values 4 §5.1). Negative numbers are tokenized as
    /// `Delim('-')` followed by `Number` — value parsers compose.
    Number(i32),
    /// `<number-token>` with the *number* type flag: the literal had a
    /// fraction (`0.05`, `.5`) or an exponent (`1e3`). Consumed whole by the tokenizer — a decimal is never
    /// `Number Delim('.') Number`, which loses the fraction's leading
    /// zeros.
    Float(f64),
    /// `<percentage-token>`: a numeric literal immediately followed by
    /// `%` (no whitespace). Carries the fraction (`12.5%`). Sign is
    /// independent — negative percentages tokenize as `Delim('-')` +
    /// `Percentage(n)` just like negative numbers.
    Percentage(f64),
    /// `"…"` or `'…'` with backslash escapes resolved.
    String(String),
    /// `#…` followed by 3..=8 hex digits.
    HexColor(String),
    /// `<ident>(` — emitted as a single token, the trailing `(` is
    /// consumed.
    Function(String),
    Colon,
    Semicolon,
    Comma,
    Bang,
    LParen,
    RParen,
    /// Any other single character: `/`, `*`, `+`, `>`, `~`, `=`, etc.
    Delim(char),
    /// `<dimension-token>` (CSS Syntax 3 §4.3.3): a numeric literal
    /// immediately followed by an identifier — `1fr`, `300ms`, `1.5s`
    /// (escapes in the unit decoded). With whitespace between, the two
    /// are a `Number` / `Float` and an `Ident`, which no unit grammar
    /// accepts, and so are a number token from a `var()` and an ident
    /// after it (`var(--n)fr`, CSS Variables 1 §3). The sign is not
    /// part of it, as for numbers.
    Dimension {
        value: f64,
        /// The number part alone would be a `Number` (integer-typed; its
        /// value then clamped to `i32`) rather than a `Float`.
        integer: bool,
        unit: String,
    },
}

#[derive(Debug)]
#[non_exhaustive]
pub struct TokenizerError {
    pub kind: TokenizerErrorKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum TokenizerErrorKind {
    UnterminatedString,
    UnterminatedComment,
}

/// Tokenize `source` into a `Vec<Token>`. Whitespace and comments
/// are skipped; unterminated comments / strings produce a
/// `TokenizerError` and abort.
pub fn tokenize(source: &str) -> Result<Vec<Token>, TokenizerError> {
    tokenize_at(source, 1, 1).map(|(tokens, _)| tokens)
}

/// `(line, column)` of a token's first character in the source.
pub type TokenPos = (u32, u32);

/// [`tokenize`] with positions: `source` is taken to start at
/// `line:col` of the enclosing document (a declaration block cut out
/// of a stylesheet), every token's start position is returned in a
/// parallel `Vec`, and a tokenizer error carries the absolute
/// position too. Parallel rather than zipped so property parsers keep
/// taking plain `&[Token]` slices.
pub fn tokenize_at(
    source: &str,
    line: u32,
    col: u32,
) -> Result<(Vec<Token>, Vec<TokenPos>), TokenizerError> {
    let mut cursor = Cursor::at(source, line, col);
    let mut tokens = Vec::new();
    let mut positions = Vec::new();
    loop {
        skip_ws_and_comments(&mut cursor)?;
        match cursor.peek() {
            None => return Ok((tokens, positions)),
            Some(c) => {
                let pos = (cursor.line(), cursor.col());
                let tok = read_one(&mut cursor, c)?;
                tokens.push(tok);
                positions.push(pos);
            }
        }
    }
}

fn skip_ws_and_comments(cursor: &mut Cursor) -> Result<(), TokenizerError> {
    loop {
        match cursor.peek() {
            Some(c) if c.is_whitespace() => {
                cursor.bump();
            }
            Some('/') => match cursor.peek_two() {
                (_, Some('*')) => {
                    let line = cursor.line();
                    let col = cursor.col();
                    cursor.bump();
                    cursor.bump();
                    if !skip_comment_body(cursor) {
                        return Err(TokenizerError {
                            kind: TokenizerErrorKind::UnterminatedComment,
                            line,
                            column: col,
                        });
                    }
                }
                _ => return Ok(()),
            },
            _ => return Ok(()),
        }
    }
}

fn skip_comment_body(cursor: &mut Cursor) -> bool {
    loop {
        match cursor.bump() {
            None => return false,
            Some('*') => {
                if let Some('/') = cursor.peek() {
                    cursor.bump();
                    return true;
                }
            }
            Some(_) => {}
        }
    }
}

fn read_one(cursor: &mut Cursor, c: char) -> Result<Token, TokenizerError> {
    // §4.3.1: an ident-start code point, a valid escape (`\31`), or a
    // `-` followed by either of those or another `-` begins an
    // identifier. `-5`, `-)` and a lone `-` are punctuation and start a
    // `Delim('-')` / signed number sequence; a `\` before a newline is
    // a `Delim('\\')` (parse error).
    if css_syntax::would_start_ident(cursor.rest()) {
        return Ok(read_ident_or_function(cursor));
    }
    if c == '-' {
        cursor.bump();
        return Ok(Token::Delim('-'));
    }
    if c.is_ascii_digit() || (c == '.' && cursor.peek_two().1.is_some_and(|d| d.is_ascii_digit())) {
        return Ok(read_numeric(cursor));
    }
    if c == '#' {
        return Ok(read_hash(cursor));
    }
    if c == '"' || c == '\'' {
        return read_string(cursor);
    }
    cursor.bump();
    let tok = match c {
        ':' => Token::Colon,
        ';' => Token::Semicolon,
        ',' => Token::Comma,
        '!' => Token::Bang,
        '(' => Token::LParen,
        ')' => Token::RParen,
        other => Token::Delim(other),
    };
    Ok(tok)
}

/// §4.3.4 "consume an ident-like token" (minus `url(`): an ident
/// sequence with its escapes decoded (§4.3.11), promoted to a
/// `Function` when `(` follows directly.
fn read_ident_or_function(cursor: &mut Cursor) -> Token {
    let (name, used) = css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    if cursor.peek() == Some('(') {
        cursor.bump();
        return Token::Function(name);
    }
    Token::Ident(name)
}

/// CSS Syntax 3 §4.3.3 "consume a numeric token": a number, then a
/// `Dimension` when an identifier starts right after it.
fn read_numeric(cursor: &mut Cursor) -> Token {
    let number = read_number(cursor);
    let (value, integer) = match number {
        Token::Number(n) => (f64::from(n), true),
        Token::Float(f) => (f, false),
        other => return other,
    };
    if !css_syntax::would_start_ident(cursor.rest()) {
        return number;
    }
    let (unit, used) = css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    Token::Dimension {
        value,
        integer,
        unit,
    }
}

/// CSS Syntax 3 §4.3.12 "consume a number": digits, an optional
/// `.digits` fraction, an optional `e[+-]digits` exponent (only when a
/// digit follows, so `1em` stays `1` + `em`), then the `%` promotion.
/// The sign is not part of the literal here — `read_one` emits
/// `Delim('-')` first — so this only ever sees an unsigned literal.
fn read_number(cursor: &mut Cursor) -> Token {
    let mut text = String::new();
    let mut is_integer = true;
    while let Some(c) = cursor.peek() {
        if c.is_ascii_digit() {
            text.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    if let (Some('.'), Some(d)) = cursor.peek_two()
        && d.is_ascii_digit()
    {
        is_integer = false;
        text.push('.');
        cursor.bump();
        while let Some(c) = cursor.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                cursor.bump();
            } else {
                break;
            }
        }
    }
    if let (Some('e' | 'E'), Some(next)) = cursor.peek_two() {
        // `e` followed by a digit, or by a sign and then a digit.
        let exponent_follows = next.is_ascii_digit()
            || ((next == '+' || next == '-')
                && cursor.peek_third().is_some_and(|d| d.is_ascii_digit()));
        if exponent_follows {
            is_integer = false;
            text.push('e');
            cursor.bump();
            if next == '+' || next == '-' {
                text.push(next);
                cursor.bump();
            }
            while let Some(c) = cursor.peek() {
                if c.is_ascii_digit() {
                    text.push(c);
                    cursor.bump();
                } else {
                    break;
                }
            }
        }
    }
    // A `%` immediately after the literal promotes it to a
    // `Percentage`. Whitespace breaks the promotion (`50 %`
    // tokenizes as `Number(50)` + `Delim('%')`).
    // `text` always holds at least one digit, so this parses; a literal
    // beyond f64's range comes back infinite and CSS Values 4 §7 says to
    // clamp to the largest representable finite value.
    let value: f64 = text
        .parse::<f64>()
        .map(|v| if v.is_finite() { v } else { f64::MAX })
        .unwrap_or(f64::MAX);
    if cursor.peek() == Some('%') {
        cursor.bump();
        return Token::Percentage(value);
    }
    if is_integer {
        // CSS Syntax 3 §4.3.12: digits alone are integer-typed whatever
        // their size; a value past the implementation's range (`i32`) is
        // clamped (CSS Values 4 §5.1), not made a non-integer — an
        // integer-only grammar must still see an integer.
        return Token::Number(text.parse::<i32>().unwrap_or(i32::MAX));
    }
    // A fraction or an exponent: a CSS number, not an integer-typed one.
    Token::Float(value)
}

fn read_hash(cursor: &mut Cursor) -> Token {
    cursor.bump(); // consume '#'
    let mut hex = String::new();
    while let Some(c) = cursor.peek() {
        if c.is_ascii_hexdigit() {
            hex.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    Token::HexColor(hex)
}

fn read_string(cursor: &mut Cursor) -> Result<Token, TokenizerError> {
    let line = cursor.line();
    let col = cursor.col();
    let quote = cursor
        .bump()
        .expect("read_string called with non-quote peek");
    let mut out = String::new();
    loop {
        match cursor.bump() {
            None => {
                return Err(TokenizerError {
                    kind: TokenizerErrorKind::UnterminatedString,
                    line,
                    column: col,
                });
            }
            Some(c) if c == quote => return Ok(Token::String(out)),
            Some('\\') => match cursor.peek() {
                // §4.3.5: an escaped newline inside a string is dropped.
                Some('\n') => {
                    cursor.bump();
                }
                Some(_) => {
                    let (decoded, used) = css_syntax::consume_escape(cursor.rest());
                    out.push(decoded);
                    cursor.advance(used);
                }
                None => {} // `\` at EOF: the unterminated-string path reports it
            },
            Some(c) => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        tokenize(src).unwrap()
    }

    /// CSS Syntax 3 §4.3.12: a numeric literal is consumed whole. An
    /// integer stays `Number`; anything with a fraction or exponent is
    /// a `Float` — never `Number Delim('.') Number`, which loses the
    /// leading zeros of the fraction (`0.05` came out as 0.5).
    #[test]
    fn decimals_are_single_float_tokens() {
        assert_eq!(toks("0.05"), vec![Token::Float(0.05)]);
        assert_eq!(toks("1.5"), vec![Token::Float(1.5)]);
        assert_eq!(toks(".5"), vec![Token::Float(0.5)]);
        assert_eq!(toks("12"), vec![Token::Number(12)]);
    }

    #[test]
    fn exponents_are_part_of_the_number() {
        assert_eq!(toks("1e3"), vec![Token::Float(1000.0)]);
        assert_eq!(toks("2.5E-1"), vec![Token::Float(0.25)]);
        // `e` not followed by a digit starts a unit, not an exponent.
        assert_eq!(toks("1em"), vec![dim(1.0, true, "em")]);
    }

    #[test]
    fn percentages_carry_fractions() {
        assert_eq!(toks("50%"), vec![Token::Percentage(50.0)]);
        assert_eq!(toks("12.5%"), vec![Token::Percentage(12.5)]);
        // Whitespace breaks the promotion.
        assert_eq!(toks("50 %"), vec![Token::Number(50), Token::Delim('%')]);
    }

    /// C4G-NUMBER-RANGE — CSS Syntax 3 §4.3.12 / §4.3.13: a literal of
    /// digits alone has the *integer* type flag whatever its size, and
    /// CSS Values 4 §5.1 clamps a value outside the implementation's
    /// range — so a literal past `i32` is the integer `i32::MAX`, not a
    /// `Float` (nor 0). A dimension's number part likewise; a percentage
    /// has no type flag and keeps its value.
    #[test]
    fn oversized_integer_clamps_and_stays_integer() {
        assert_eq!(toks("99999999999"), vec![Token::Number(i32::MAX)]);
        assert_eq!(
            toks("99999999999px"),
            vec![Token::Dimension {
                value: f64::from(i32::MAX),
                integer: true,
                unit: "px".to_string(),
            }]
        );
        assert_eq!(
            toks("99999999999%"),
            vec![Token::Percentage(99_999_999_999.0)]
        );
        assert_eq!(toks("99999999999.5"), vec![Token::Float(99_999_999_999.5)]);
    }

    /// CSS Values 4 §7: numbers that overflow the representable range
    /// clamp to the largest finite value — no `inf` or `NaN` token ever
    /// reaches a value parser.
    #[test]
    fn overflowing_literals_clamp_to_finite() {
        for src in ["1e400", "1e400%", ".1e400"] {
            for tok in toks(src) {
                match tok {
                    Token::Float(f) | Token::Percentage(f) => {
                        assert!(f.is_finite(), "{src} → {f}");
                        assert_eq!(f, f64::MAX, "{src}");
                    }
                    other => panic!("{src} → {other:?}"),
                }
            }
        }
    }

    #[test]
    fn more_number_edge_cases() {
        assert_eq!(toks("1e"), vec![dim(1.0, true, "e")]);
        assert_eq!(toks("1e+"), vec![dim(1.0, true, "e"), Token::Delim('+')]);
        assert_eq!(toks("-.5"), vec![Token::Delim('-'), Token::Float(0.5)]);
        assert_eq!(toks("1.5.5"), vec![Token::Float(1.5), Token::Float(0.5)]);
        assert_eq!(
            toks("50%%"),
            vec![Token::Percentage(50.0), Token::Delim('%')]
        );
        assert_eq!(toks("1E3"), vec![Token::Float(1000.0)]);
    }

    /// A trailing `.` with no digit after it is not part of the number.
    #[test]
    fn dot_without_following_digit_is_a_delim() {
        assert_eq!(toks("1."), vec![Token::Number(1), Token::Delim('.')]);
        assert_eq!(
            toks("a.b"),
            vec![
                Token::Ident("a".to_string()),
                Token::Delim('.'),
                Token::Ident("b".to_string())
            ]
        );
    }

    /// CSS Syntax 3 §4.3.7 "consume an escaped code point": `\` + 1..6
    /// hex digits is a code point (one following whitespace is eaten);
    /// `\` + any other char is that char; an escaped newline inside a
    /// string is dropped (§4.3.5).
    #[test]
    fn string_escapes_decode_hex_and_literal_forms() {
        assert_eq!(
            toks(r#""\201C""#),
            vec![Token::String("\u{201C}".to_string())]
        );
        assert_eq!(
            toks(r#""\201c quote""#),
            vec![Token::String("\u{201C}quote".to_string())]
        );
        assert_eq!(toks(r#""a\"b""#), vec![Token::String("a\"b".to_string())]);
        assert_eq!(toks(r#""a\\b""#), vec![Token::String("a\\b".to_string())]);
        assert_eq!(toks("\"a\\\nb\""), vec![Token::String("ab".to_string())]);
        // Out-of-range / surrogate code points become U+FFFD.
        assert_eq!(
            toks(r#""\110000""#),
            vec![Token::String("\u{FFFD}".to_string())]
        );
    }

    /// Identifiers may contain non-ASCII code points (§4.3.9): `größe`
    /// and `--größe` are single idents, not ident + garbage.
    #[test]
    fn non_ascii_identifiers_are_single_tokens() {
        assert_eq!(toks("größe"), vec![Token::Ident("größe".to_string())]);
        assert_eq!(toks("--größe"), vec![Token::Ident("--größe".to_string())]);
        assert_eq!(toks("日本語"), vec![Token::Ident("日本語".to_string())]);
    }

    /// CSS Syntax 3 §4.3.7 / §4.3.11: an identifier may contain (and
    /// start with) escapes, decoded in the token; `\` + newline is not
    /// a valid escape (§4.3.8), so it ends the identifier and is a
    /// `Delim('\\')`.
    #[test]
    fn identifier_escapes_decode() {
        let ident = |s: &str| Token::Ident(s.to_string());
        assert_eq!(toks(r"col\6f r"), vec![ident("color")]);
        assert_eq!(toks(r"\31 0"), vec![ident("10")]);
        assert_eq!(toks(r"-\31 x"), vec![ident("-1x")]);
        assert_eq!(toks(r"a\:b"), vec![ident("a:b")]);
        assert_eq!(toks(r"r\67 b("), vec![Token::Function("rgb".to_string())]);
        assert_eq!(
            toks("a\\\nb"),
            vec![ident("a"), Token::Delim('\\'), ident("b")]
        );
    }

    #[test]
    fn times_are_dimensions() {
        assert_eq!(toks("1.05s"), vec![dim(1.05, false, "s")]);
        assert_eq!(toks("200ms"), vec![dim(200.0, true, "ms")]);
    }

    /// CSS Syntax 3 §4.3.3: a number immediately followed by an ident
    /// is one `<dimension-token>`; with whitespace between, a number and
    /// an ident (`C1G-VAR-TOKENS`).
    #[test]
    fn a_number_and_a_following_ident_form_a_dimension_only_when_adjacent() {
        assert_eq!(toks("1fr"), vec![dim(1.0, true, "fr")]);
        assert_eq!(toks("1.5s"), vec![dim(1.5, false, "s")]);
        assert_eq!(
            toks("1 fr"),
            vec![Token::Number(1), Token::Ident("fr".to_string())]
        );
        assert_eq!(toks(r"2\66 r"), vec![dim(2.0, true, "fr")]);
    }

    fn dim(value: f64, integer: bool, unit: &str) -> Token {
        Token::Dimension {
            value,
            integer,
            unit: unit.to_string(),
        }
    }
}
