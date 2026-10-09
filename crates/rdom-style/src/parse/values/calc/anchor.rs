//! The anchor functions in a math expression (CSS Anchor Positioning 1
//! §5.1, §5.2): `anchor()` and `anchor-size()` — which properties take
//! them, their arguments, and a value that is one.

use super::{CalcParser, Node, math_function};
use crate::calc::CalcExpr;
use crate::parse::token::Token;

/// `anchor()` may appear (the inset properties).
pub(crate) const ANCHOR_EDGE: u8 = 1;
/// `anchor-size()` may appear (the inset, sizing and margin properties).
pub(crate) const ANCHOR_SIZE: u8 = 2;

impl CalcParser<'_> {
    /// An anchor function's arguments and its closing `)`, the function
    /// token consumed (CSS Anchor Positioning 1 §5.1, §5.2): `anchor(
    /// <anchor-name>? && <anchor-side>, <length-percentage>? )` or —
    /// `size` — `anchor-size( [ <anchor-name> || <anchor-size> ]?,
    /// <length-percentage>? )`.
    pub(super) fn parse_anchor_body(&mut self, size: bool) -> Option<Node> {
        use crate::calc::{AnchorFunction, AnchorSide, AnchorSize};
        let mut name: Option<std::sync::Arc<str>> = None;
        let mut side: Option<AnchorSide> = None;
        let mut extent: Option<AnchorSize> = None;
        loop {
            match self.peek()? {
                Token::Ident(s) if s.starts_with("--") && s.len() > 2 && name.is_none() => {
                    name = Some(s.as_str().into());
                }
                Token::Ident(s) if !size && side.is_none() => {
                    side = Some(
                        AnchorSide::KEYWORDS
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(s))
                            .map(|(_, v)| *v)?,
                    );
                }
                Token::Percentage(p) if !size && side.is_none() => {
                    side = Some(AnchorSide::Percent(*p));
                }
                Token::Ident(s) if size && extent.is_none() => {
                    extent = Some(
                        AnchorSize::KEYWORDS
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(s))
                            .map(|(_, v)| *v)?,
                    );
                }
                _ => break,
            }
            self.advance();
        }
        let named = name.is_some() || side.is_some() || extent.is_some();
        let fallback = if self.peek() == Some(&Token::Comma) && (named || !size) {
            self.advance();
            Some(self.parse_sum()?.expr)
        } else {
            None
        };
        self.expect(&Token::RParen)?;
        let f = if size {
            AnchorFunction::Size {
                name,
                size: extent,
                fallback,
            }
        } else {
            AnchorFunction::Edge {
                name,
                side: side?,
                fallback,
            }
        };
        Some(Node::leaf(CalcExpr::Anchor(Box::new(f))))
    }
}

/// Whether a function-token name is an anchor function: `Some(false)`
/// for `anchor`, `Some(true)` for `anchor-size` (ASCII case-insensitive).
pub(super) fn anchor_function(name: &str) -> Option<bool> {
    if name.eq_ignore_ascii_case("anchor") {
        Some(false)
    } else if name.eq_ignore_ascii_case("anchor-size") {
        Some(true)
    } else {
        None
    }
}

/// A value that is an anchor function, or a math function holding one,
/// of a property that takes `anchors` ([`ANCHOR_EDGE`] | [`ANCHOR_SIZE`];
/// CSS Anchor Positioning 1 §5): its expression, a `<length>` (a
/// percentage fallback typed with it). `None` for a value without an
/// anchor function — the property's own parser reads it — or an invalid
/// one.
pub(crate) fn parse_anchored(tokens: &[Token], anchors: u8) -> Option<CalcExpr> {
    let opens = match tokens.first() {
        Some(Token::Function(n)) => anchor_function(n).is_some() || math_function(n).is_some(),
        _ => false,
    };
    if !opens {
        return None;
    }
    let mut parser = CalcParser::new(tokens);
    parser.anchors = anchors;
    let expr = parser.parse_factor()?.expr;
    if parser.peek().is_some() || !expr.contains_anchor() {
        return None;
    }
    expr.kind()
        .is_some_and(|k| k.is_length() || k == crate::calc::CalcKind::Percent)
        .then_some(expr)
}
