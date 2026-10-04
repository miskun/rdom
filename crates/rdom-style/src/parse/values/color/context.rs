//! The state of one color parse, and the text a kept function is
//! serialized as.
//!
//! At parse time there is no element: a `currentcolor` inside a
//! function (`color-mix(in srgb, currentcolor, blue)`), a
//! `light-dark()`, or a system color that is the terminal's default
//! becomes a node of the function's parsed form (`expr::ColorExpr`),
//! which the cascade computes per element (`TuiColor::Function`).

use std::cell::Cell;

use crate::parse::token::Token;

/// The state of one color parse: how deep in nested color functions
/// it is.
pub(super) struct ColorCx {
    /// How many color functions the parse is inside
    /// ([`MAX_COLOR_NESTING`](super::MAX_COLOR_NESTING) at most).
    nesting: Cell<usize>,
}

impl ColorCx {
    /// A parse at the top level.
    pub fn new() -> Self {
        ColorCx {
            nesting: Cell::new(0),
        }
    }

    /// Parse one nested color function with `inner`; `None` past
    /// [`MAX_COLOR_NESTING`](super::MAX_COLOR_NESTING) — checked before
    /// `inner` scans anything, so a hostile value costs at most that many
    /// passes over its tokens, not one per level.
    pub fn nested<T>(&self, inner: impl FnOnce() -> Option<T>) -> Option<T> {
        let depth = self.nesting.get();
        if depth >= super::MAX_COLOR_NESTING {
            return None;
        }
        self.nesting.set(depth + 1);
        let out = inner();
        self.nesting.set(depth);
        out
    }
}

/// `tokens` (one color function) as CSS text: tokens separated by one
/// space, except after `(` and before `)` and `,`.
pub(super) fn render(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut prev: Option<&Token> = None;
    for t in tokens {
        let tight = matches!(prev, None | Some(Token::Function(_) | Token::LParen))
            || matches!(t, Token::RParen | Token::Comma);
        if !tight {
            out.push(' ');
        }
        out.push_str(&crate::parse::values::render_value(std::slice::from_ref(t)));
        prev = Some(t);
    }
    out
}
