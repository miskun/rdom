//! Char-level cursor with line / column tracking.
//!
//! The parser walks the source via [`SourceCursor::peek`] / [`SourceCursor::bump`]
//! plus a couple of compound helpers. Line and column track the
//! *next* character so warnings/errors point at the offending token,
//! not the one that came before.

#[derive(Debug)]
pub struct SourceCursor<'a> {
    source: &'a str,
    pos: usize,
    line: u32,
    col: u32,
    /// The blocks entered through [`SourceCursor::in_block`] and not left.
    depth: usize,
}

impl<'a> SourceCursor<'a> {
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
            depth: 0,
        }
    }

    /// How many blocks deep the parser reading through this cursor is:
    /// the [`in_block`](Self::in_block) calls it is inside. A parser that
    /// recurses per block caps this against hostile nesting (rdom-css's
    /// `MAX_BLOCK_DEPTH`).
    pub fn block_depth(&self) -> usize {
        self.depth
    }

    /// Run `f` one block deeper ([`block_depth`](Self::block_depth)).
    pub fn in_block<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.depth += 1;
        let out = f(self);
        self.depth -= 1;
        out
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

    /// The unconsumed remainder of the source.
    /// The byte offset into the source the cursor has reached.
    pub fn offset(&self) -> usize {
        self.pos
    }

    pub fn rest(&self) -> &'a str {
        &self.source[self.pos..]
    }

    /// Consume `bytes` bytes (a count a `rdom_core::css_syntax`
    /// function returned for [`SourceCursor::rest`], so it ends on a `char`
    /// boundary), keeping line and column current.
    pub fn advance(&mut self, bytes: usize) {
        let end = self.pos + bytes;
        while self.pos < end && self.bump().is_some() {}
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
