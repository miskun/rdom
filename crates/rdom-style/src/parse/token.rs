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
    /// Decimal integer. Negative numbers are tokenized as
    /// `Delim('-')` followed by `Number` — value parsers compose.
    Number(i32),
    /// `<n>%`. Emitted when a number is immediately followed by
    /// `%` (no whitespace). Matches the CSS Syntax Module
    /// `<percentage-token>`. Sign is independent — negative
    /// percentages tokenize as `Delim('-')` + `Percentage(n)`
    /// just like negative numbers.
    Percentage(i32),
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
    if c.is_ascii_digit() {
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

fn read_number(cursor: &mut Cursor) -> Token {
    let mut digits = String::new();
    while let Some(c) = cursor.peek() {
        if c.is_ascii_digit() {
            digits.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    let value: i32 = digits.parse().unwrap_or(0);
    // A `%` immediately after the digits promotes the token to a
    // `Percentage` per CSS Syntax Module §4.3. Whitespace breaks
    // the promotion (`50 %` tokenizes as `Number(50)` + `Delim('%')`).
    if cursor.peek() == Some('%') {
        cursor.bump();
        return Token::Percentage(value);
    }
    Token::Number(value)
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
