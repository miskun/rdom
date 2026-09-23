//! Tokenizer used by the declaration-block parser.
//!
//! Top-level (selector text → `{` → body → `}`) still uses the
//! cursor-level byte walk in `top_level.rs`. This tokenizer
//! operates on the captured body string and produces value-level
//! tokens that the property parsers consume.

use crate::parse::cursor::Cursor;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `[-_a-zA-Z][-_a-zA-Z0-9]*`. Includes custom-property names
    /// like `--accent` (CSS treats them as idents).
    Ident(String),
    /// `<number-token>` with the *integer* type flag (CSS Syntax 3
    /// §4.3.12): digits only, in `i32` range. Negative numbers are
    /// tokenized as `Delim('-')` followed by `Number` — value parsers
    /// compose.
    Number(i32),
    /// `<number-token>` with the *number* type flag: the literal had a
    /// fraction (`0.05`, `.5`), an exponent (`1e3`), or did not fit in
    /// `i32`. Consumed whole by the tokenizer — a decimal is never
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
}

#[derive(Debug)]
pub struct TokenizerError {
    pub kind: TokenizerErrorKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, PartialEq)]
pub enum TokenizerErrorKind {
    UnterminatedString,
    UnterminatedComment,
}

/// Tokenize `source` into a `Vec<Token>`. Whitespace and comments
/// are skipped; unterminated comments / strings produce a
/// `TokenizerError` and abort.
pub fn tokenize(source: &str) -> Result<Vec<Token>, TokenizerError> {
    let mut cursor = Cursor::new(source);
    let mut tokens = Vec::new();
    loop {
        skip_ws_and_comments(&mut cursor)?;
        match cursor.peek() {
            None => return Ok(tokens),
            Some(c) => {
                let tok = read_one(&mut cursor, c)?;
                tokens.push(tok);
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
    // CSS syntax: a leading `-` starts an identifier only when
    // followed by another ident-start char (letter / underscore /
    // dash). `-foo`, `--name`, `-_a` are idents; `-5`, `-)`,
    // `-` (alone) are punctuation and start a `Delim('-')` /
    // signed number sequence.
    if c == '-' {
        let next = cursor.peek_two().1;
        match next {
            Some(c2) if c2.is_ascii_alphabetic() || c2 == '_' || c2 == '-' => {
                return Ok(read_ident_or_function(cursor));
            }
            _ => {
                cursor.bump();
                return Ok(Token::Delim('-'));
            }
        }
    }
    if is_ident_start(c) {
        return Ok(read_ident_or_function(cursor));
    }
    if c.is_ascii_digit() || (c == '.' && cursor.peek_two().1.is_some_and(|d| d.is_ascii_digit())) {
        return Ok(read_number(cursor));
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

fn is_ident_start(c: char) -> bool {
    // `-` is handled specially in `read_one` (it's only an ident
    // start when followed by another ident-start char per CSS).
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn read_ident_or_function(cursor: &mut Cursor) -> Token {
    let mut name = String::new();
    while let Some(c) = cursor.peek() {
        if is_ident_continue(c) {
            name.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    if cursor.peek() == Some('(') {
        cursor.bump();
        return Token::Function(name);
    }
    Token::Ident(name)
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
    let value: f64 = text.parse().unwrap_or(f64::NAN);
    if cursor.peek() == Some('%') {
        cursor.bump();
        return Token::Percentage(value);
    }
    if is_integer && let Ok(n) = text.parse::<i32>() {
        return Token::Number(n);
    }
    // Fraction, exponent, or an integer literal outside `i32`: still
    // a CSS number, just not an integer-typed one.
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
            Some('\\') => {
                if let Some(esc) = cursor.bump() {
                    out.push(esc);
                }
            }
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
        // `e` not followed by a digit is an ident, not an exponent.
        assert_eq!(
            toks("1em"),
            vec![Token::Number(1), Token::Ident("em".to_string())]
        );
    }

    #[test]
    fn percentages_carry_fractions() {
        assert_eq!(toks("50%"), vec![Token::Percentage(50.0)]);
        assert_eq!(toks("12.5%"), vec![Token::Percentage(12.5)]);
        // Whitespace breaks the promotion.
        assert_eq!(toks("50 %"), vec![Token::Number(50), Token::Delim('%')]);
    }

    /// A literal too large for `i32` is still a CSS number; it must not
    /// silently become 0.
    #[test]
    fn oversized_integer_is_a_float_not_zero() {
        assert_eq!(toks("99999999999"), vec![Token::Float(99_999_999_999.0)]);
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

    #[test]
    fn dimension_is_number_then_ident() {
        assert_eq!(
            toks("1.05s"),
            vec![Token::Float(1.05), Token::Ident("s".to_string())]
        );
        assert_eq!(
            toks("200ms"),
            vec![Token::Number(200), Token::Ident("ms".to_string())]
        );
    }
}
