//! `ParseError` — human-friendly error reporting for malformed templates.

use std::fmt;

/// Parse error with position + optional hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable error message.
    pub msg: String,
    /// 1-indexed line number where the error was detected.
    pub line: u32,
    /// 1-indexed column number (in bytes, not graphemes — sufficient
    /// for source-error purposes; human debugging uses the source text).
    pub col: u32,
    /// Byte offset into the input for tooling.
    pub pos: usize,
    /// Optional suggestion for how to fix.
    pub hint: Option<String>,
}

impl ParseError {
    pub fn new(msg: impl Into<String>, line: u32, col: u32, pos: usize) -> Self {
        Self {
            msg: msg.into(),
            line,
            col,
            pos,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at line {}, col {}: {}",
            self.line, self.col, self.msg
        )?;
        if let Some(hint) = &self.hint {
            write!(f, " (hint: {})", hint)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// `Result` alias for parser operations.
pub type Result<T> = std::result::Result<T, ParseError>;
