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
    /// §4.3.12): digits only, the value as written (saturating past
    /// `i64`). The token keeps it whole — a custom property passes it on
    /// unchanged — and a property that consumes an `<integer>` clamps it
    /// to its own range (CSS Values 4 §5.1). Negative numbers are
    /// tokenized as `Delim('-')` followed by `Number` — value parsers
    /// compose.
    Number(i64),
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
    /// consumed. `url(` followed by a quote is a function too (CSS
    /// Syntax 3 §4.3.4); an unquoted one is a [`Token::Url`].
    Function(String),
    /// `<url-token>` (CSS Syntax 3 §4.3.6): `url(` + unquoted text + `)`,
    /// the text raw (escapes decoded, surrounding whitespace dropped) —
    /// `url(0001.png)` keeps `0001.png`, where reading it as tokens would
    /// make it the number 1.
    Url(String),
    /// `<bad-url-token>`: an unquoted `url(` holding whitespace inside,
    /// a quote, a `(` or a non-printable code point. No grammar accepts
    /// it, so the declaration holding it is invalid (§4.3.14).
    BadUrl,
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
    tokenize_spans(source, line, col).map(|s| (s.tokens, s.positions))
}

/// A token's byte range in its source.
pub type TokenSpan = std::ops::Range<usize>;

/// [`tokenize_spans`]' result: the tokens, and parallel to them their
/// positions and byte ranges — one entry per token in each list, so
/// property parsers keep taking a plain `&[Token]` slice and a caller
/// indexes the others alongside.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpannedTokens {
    /// The tokens, whitespace and comments dropped.
    pub tokens: Vec<Token>,
    /// Each token's `(line, column)` in the enclosing document.
    pub positions: Vec<TokenPos>,
    /// Each token's byte range in the source.
    pub spans: Vec<TokenSpan>,
}

/// [`tokenize_at`] with each token's byte range in `source` too, so a
/// caller can cut a value's text out as written — the whitespace and
/// comments between its tokens included (CSS Variables 1 §2: a custom
/// property's value is the token sequence as written).
pub fn tokenize_spans(source: &str, line: u32, col: u32) -> Result<SpannedTokens, TokenizerError> {
    let mut cursor = Cursor::at(source, line, col);
    let mut out = SpannedTokens::default();
    loop {
        skip_ws_and_comments(&mut cursor)?;
        match cursor.peek() {
            None => return Ok(out),
            Some(c) => {
                let pos = (cursor.line(), cursor.col());
                let start = cursor.offset();
                let tok = read_one(&mut cursor, c)?;
                out.tokens.push(tok);
                out.positions.push(pos);
                out.spans.push(start..cursor.offset());
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
        if name.eq_ignore_ascii_case("url") {
            // §4.3.4: whitespace, then a quote → a function whose
            // argument is a string; anything else → a url token.
            let ws = cursor
                .rest()
                .chars()
                .take_while(|&c| css_syntax::is_whitespace(c))
                .map(char::len_utf8)
                .sum();
            if !cursor.rest()[ws..].starts_with(['"', '\'']) {
                cursor.advance(ws);
                return read_url(cursor);
            }
        }
        return Token::Function(name);
    }
    Token::Ident(name)
}

/// §4.3.6 "consume a url token", `url(` and its leading whitespace
/// already consumed.
fn read_url(cursor: &mut Cursor) -> Token {
    let mut url = String::new();
    loop {
        match cursor.peek() {
            None => return Token::Url(url),
            Some(')') => {
                cursor.bump();
                return Token::Url(url);
            }
            Some(c) if css_syntax::is_whitespace(c) => {
                while cursor.peek().is_some_and(css_syntax::is_whitespace) {
                    cursor.bump();
                }
                return match cursor.peek() {
                    None => Token::Url(url),
                    Some(')') => {
                        cursor.bump();
                        Token::Url(url)
                    }
                    Some(_) => bad_url_remnants(cursor),
                };
            }
            Some('"' | '\'' | '(') => return bad_url_remnants(cursor),
            Some(c) if is_non_printable(c) => return bad_url_remnants(cursor),
            Some('\\') if css_syntax::is_valid_escape(cursor.rest()) => {
                cursor.bump();
                let (c, used) = css_syntax::consume_escape(cursor.rest());
                cursor.advance(used);
                url.push(c);
            }
            Some('\\') => return bad_url_remnants(cursor),
            Some(c) => {
                cursor.bump();
                url.push(c);
            }
        }
    }
}

/// §4.3.14 "consume the remnants of a bad url": up to and including the
/// next `)` not inside an escape.
fn bad_url_remnants(cursor: &mut Cursor) -> Token {
    loop {
        match cursor.peek() {
            None => return Token::BadUrl,
            Some(')') => {
                cursor.bump();
                return Token::BadUrl;
            }
            Some('\\') if css_syntax::is_valid_escape(cursor.rest()) => {
                cursor.bump();
                let (_, used) = css_syntax::consume_escape(cursor.rest());
                cursor.advance(used);
            }
            Some(_) => {
                cursor.bump();
            }
        }
    }
}

/// §4.2 "non-printable code point".
fn is_non_printable(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{E}'..='\u{1F}' | '\u{7F}')
}

/// CSS Syntax 3 §4.3.3 "consume a numeric token": a number, then a
/// `Dimension` when an identifier starts right after it.
fn read_numeric(cursor: &mut Cursor) -> Token {
    let number = read_number(cursor);
    let (value, integer) = match number {
        Token::Number(n) => (n as f64, true),
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
        // their size, and keep their value — the clamp to a property's
        // range (CSS Values 4 §5.1) is the consumer's. Past `i64` the
        // literal saturates.
        return Token::Number(text.parse::<i64>().unwrap_or(i64::MAX));
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
#[path = "token_tests.rs"]
mod tests;
