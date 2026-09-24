//! Char-level cursor with line / column tracking.
//!
//! The parser walks the source via [`Cursor::peek`] / [`Cursor::bump`]
//! plus a couple of compound helpers. Line and column track the
//! *next* character so warnings/errors point at the offending token,
//! not the one that came before.

#[derive(Debug)]
pub struct Cursor<'a> {
    source: &'a str,
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self::at(source, 1, 1)
    }

    /// A cursor over `source` that reports positions as if `source`
    /// started at `line:col` of a larger document — for tokenizing a
    /// declaration block that was cut out of a stylesheet.
    pub fn at(source: &'a str, line: u32, col: u32) -> Self {
        Self {
            source,
            pos: 0,
            line,
            col,
        }
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn col(&self) -> u32 {
        self.col
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    pub fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    pub fn peek_two(&self) -> (Option<char>, Option<char>) {
        let mut it = self.source[self.pos..].chars();
        (it.next(), it.next())
    }

    /// Third character from the cursor, without consuming.
    pub fn peek_third(&self) -> Option<char> {
        self.source[self.pos..].chars().nth(2)
    }

    pub fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }
}
