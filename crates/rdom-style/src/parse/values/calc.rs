//! The math-function entry point (CSS Values 4 §10): a recursive-
//! descent parser from tokens to a [`CalcExpr`] AST. The evaluator lives
//! in [`crate::calc`].

use crate::calc::{CalcExpr, CalcOp, CalcUnit, MathFunction, RoundingStrategy};
use crate::parse::token::Token;

// Recursive-descent over the token stream. Grammar:
//
//   math       = calc | min | max | clamp | round | mod | rem | abs | sign
//              | sin | cos | tan | asin | acos | atan | atan2
//              | pow | sqrt | hypot | log | exp
//   calc       = 'calc(' sum ')'
//   min / max  = 'min(' sum [',' sum]* ')'   ('max(' likewise)
//   clamp      = 'clamp(' (sum | 'none') ',' sum ',' (sum | 'none') ')'
//   round      = 'round(' [strategy ','] sum [',' sum] ')'
//   mod / rem  = 'mod(' sum ',' sum ')'     ('rem(' likewise)
//   abs / sign = 'abs(' sum ')'             ('sign(' likewise)
//   strategy   = 'nearest' | 'up' | 'down' | 'to-zero'
//   unary      = 'sin(' sum ')'  (cos, tan, asin, acos, atan, sqrt, exp)
//   binary     = 'atan2(' sum ',' sum ')'   (pow)
//   hypot      = 'hypot(' sum [',' sum]* ')'
//   log        = 'log(' sum [',' sum] ')'
//   constant   = 'e' | 'pi' | 'infinity' | '-infinity' | 'NaN'
//   sum        = product (('+' | '-') product)*
//   product    = factor (('*' | '/') factor)*
//   factor     = leaf | '(' sum ')' | math | constant
//   leaf       = Number | Percentage | Dimension (a `CalcUnit`)
//
// The parsed tree is then type-checked (`CalcExpr::kind`, Values 4
// §10.9); one that does not type-check is invalid.
//
// Whitespace is already eaten by the tokenizer. Operator
// precedence follows CSS Values 4 §10.8: * and / bind tighter
// than + and -. Per CSS, `+` and `-` MUST be surrounded by
// whitespace at the source level (`5+5` is invalid; `5 + 5` is
// valid). Our tokenizer doesn't preserve whitespace, so we
// accept both forms — a deliberate relaxation documented in
// DIVERGENCES.md.

/// The math function a function-token name opens, ASCII
/// case-insensitive; `Ok(None)` for `calc`.
pub(super) fn math_function(name: &str) -> Option<Option<MathFunction>> {
    const TABLE: &[(&str, Option<MathFunction>)] = &[
        ("calc", None),
        ("min", Some(MathFunction::Min)),
        ("max", Some(MathFunction::Max)),
        ("clamp", Some(MathFunction::Clamp)),
        (
            "round",
            Some(MathFunction::Round(RoundingStrategy::Nearest)),
        ),
        ("mod", Some(MathFunction::Mod)),
        ("rem", Some(MathFunction::Rem)),
        ("abs", Some(MathFunction::Abs)),
        ("sign", Some(MathFunction::Sign)),
        ("sin", Some(MathFunction::Sin)),
        ("cos", Some(MathFunction::Cos)),
        ("tan", Some(MathFunction::Tan)),
        ("asin", Some(MathFunction::Asin)),
        ("acos", Some(MathFunction::Acos)),
        ("atan", Some(MathFunction::Atan)),
        ("atan2", Some(MathFunction::Atan2)),
        ("pow", Some(MathFunction::Pow)),
        ("sqrt", Some(MathFunction::Sqrt)),
        ("hypot", Some(MathFunction::Hypot)),
        ("log", Some(MathFunction::Log)),
        ("exp", Some(MathFunction::Exp)),
    ];
    TABLE
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, f)| *f)
}

/// A `<calc-keyword>` constant (CSS Values 4 §10.7.1), ASCII
/// case-insensitive.
fn constant(name: &str) -> Option<f64> {
    const TABLE: &[(&str, f64)] = &[
        ("e", std::f64::consts::E),
        ("pi", std::f64::consts::PI),
        ("infinity", f64::INFINITY),
        ("-infinity", f64::NEG_INFINITY),
        ("nan", f64::NAN),
    ];
    TABLE
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| *v)
}

/// A dimension leaf, `None` for a unit rdom does not take — in a pixel
/// expression (`pixels`), any unit but the pixel-family ones, which
/// normalize to `px` (CSS Values 4 §6.2).
fn dimension(value: f64, unit: &str, pixels: bool) -> Option<CalcExpr> {
    if pixels {
        return super::border::px_per(unit).map(|px| CalcExpr::Dimension {
            value: value * px,
            unit: CalcUnit::Px,
        });
    }
    Some(CalcExpr::Dimension {
        value,
        unit: CalcUnit::parse(unit)?,
    })
}

/// How many math functions and parentheses may nest in one value.
///
/// CSS Values 4 §10 sets no limit, but math functions take attribute
/// data (`attr()` with `type(<length>)`, Values 5 §8.7), and the parser
/// recurses once per level: a hostile attribute could exhaust the stack
/// and abort the process. 32 levels is far past any hand-written value;
/// a deeper one is invalid, like any other parse failure.
pub const MAX_CALC_NESTING: usize = 32;

/// How deep a parsed expression tree may be (a leaf is depth 1).
///
/// Every walker of a [`CalcExpr`] — the type check, evaluation, viewport
/// folding, serialization, drop — recurses down the tree, and a flat
/// chain (`1 + 1 + 1 …`) builds a left-deep tree as deep as it is long.
/// Capping the depth bounds them all: a chain of up to 256 operands
/// parses, a longer one is invalid. (A tree built in Rust is the
/// builder's to bound.)
pub const MAX_CALC_DEPTH: usize = 256;

/// A parsed sub-expression and the depth of its tree.
struct Node {
    expr: CalcExpr,
    depth: usize,
}

impl Node {
    fn leaf(expr: CalcExpr) -> Node {
        Node { expr, depth: 1 }
    }

    /// `None` when the result would be deeper than [`MAX_CALC_DEPTH`].
    fn binary(op: CalcOp, lhs: Node, rhs: Node) -> Option<Node> {
        let depth = 1 + lhs.depth.max(rhs.depth);
        (depth <= MAX_CALC_DEPTH).then(|| Node {
            expr: CalcExpr::binary(op, lhs.expr, rhs.expr),
            depth,
        })
    }

    /// `None` when the result would be deeper than [`MAX_CALC_DEPTH`].
    fn function(func: MathFunction, args: Vec<Node>) -> Option<Node> {
        let depth = 1 + args.iter().map(|a| a.depth).max().unwrap_or(0);
        (depth <= MAX_CALC_DEPTH).then(|| Node {
            expr: CalcExpr::function(func, args.into_iter().map(|a| a.expr).collect()),
            depth,
        })
    }
}

/// Parser cursor over a `&[Token]`: the position and the current
/// nesting of math functions and parentheses.
struct CalcParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    nesting: usize,
    /// A pixel expression ([`parse_pixel_calc`]): pixel-family lengths
    /// only, no percentage.
    pixels: bool,
    /// The anchor functions the property takes ([`ANCHOR_EDGE`] |
    /// [`ANCHOR_SIZE`]; CSS Anchor Positioning 1 §5).
    anchors: u8,
}

/// `anchor()` may appear (the inset properties).
pub(crate) const ANCHOR_EDGE: u8 = 1;
/// `anchor-size()` may appear (the inset, sizing and margin properties).
pub(crate) const ANCHOR_SIZE: u8 = 2;

impl<'a> CalcParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            nesting: 0,
            pixels: false,
            anchors: 0,
        }
    }

    /// Parse one nested level (a math function's body, a parenthesized
    /// sum) with `inner`; `None` past [`MAX_CALC_NESTING`].
    fn nested(&mut self, inner: impl FnOnce(&mut Self) -> Option<Node>) -> Option<Node> {
        if self.nesting >= MAX_CALC_NESTING {
            return None;
        }
        self.nesting += 1;
        let out = inner(self);
        self.nesting -= 1;
        out
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Consume `expected` or fail.
    fn expect(&mut self, expected: &Token) -> Option<()> {
        (self.advance()? == expected).then_some(())
    }

    fn parse_sum(&mut self) -> Option<Node> {
        let mut lhs = self.parse_product()?;
        loop {
            let op = match self.peek() {
                Some(Token::Delim('+')) => CalcOp::Add,
                Some(Token::Delim('-')) => CalcOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_product()?;
            lhs = Node::binary(op, lhs, rhs)?;
        }
        Some(lhs)
    }

    fn parse_product(&mut self) -> Option<Node> {
        let mut lhs = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Delim('*')) => CalcOp::Mul,
                Some(Token::Delim('/')) => CalcOp::Div,
                _ => break,
            };
            self.advance();
            // Dividing by zero is not a parse error: it is IEEE-754 at
            // evaluation (CSS Values 4 §10.9).
            let rhs = self.parse_factor()?;
            lhs = Node::binary(op, lhs, rhs)?;
        }
        Some(lhs)
    }

    fn parse_factor(&mut self) -> Option<Node> {
        // Unary plus — accept and ignore (a loop: a run of them must not
        // recurse).
        while self.peek() == Some(&Token::Delim('+')) {
            self.advance();
        }
        let leaf = match self.peek()? {
            // A bare number is a `<number>` leaf; in a length property
            // it reads as cells (rdom's unitless length).
            Token::Number(n) => CalcExpr::Number(*n as f64),
            Token::Float(f) => CalcExpr::Number(*f),
            Token::Percentage(_) if self.pixels => return None,
            Token::Percentage(n) => CalcExpr::Percent(*n),
            Token::Dimension { value, unit, .. } => dimension(*value, unit, self.pixels)?,
            Token::Delim('-') => {
                // Unary minus — accept `-5` as a literal.
                self.advance();
                return match self.advance()? {
                    Token::Number(n) => Some(CalcExpr::Number(-(*n as f64))),
                    Token::Float(f) => Some(CalcExpr::Number(-*f)),
                    Token::Percentage(n) => Some(CalcExpr::Percent(-*n)),
                    Token::Dimension { value, unit, .. } => dimension(-*value, unit, self.pixels),
                    _ => None,
                }
                .map(Node::leaf);
            }
            Token::LParen => {
                self.advance();
                return self.nested(|p| {
                    let inner = p.parse_sum()?;
                    p.expect(&Token::RParen)?;
                    Some(inner)
                });
            }
            Token::Function(name) if self.anchors != 0 && anchor_function(name).is_some() => {
                let size = anchor_function(name)?;
                if self.anchors & if size { ANCHOR_SIZE } else { ANCHOR_EDGE } == 0 {
                    return None;
                }
                self.advance();
                return self.nested(|p| p.parse_anchor_body(size));
            }
            Token::Function(name) => {
                let func = math_function(name)?;
                self.advance();
                return self.nested(|p| p.parse_function_body(func));
            }
            Token::Ident(name) => CalcExpr::Number(constant(name)?),
            _ => return None,
        };
        self.advance();
        Some(Node::leaf(leaf))
    }

    /// The arguments of a math function and its closing `)`, the
    /// function token already consumed.
    fn parse_function_body(&mut self, func: Option<MathFunction>) -> Option<Node> {
        let expr = match func {
            None => self.parse_sum()?,
            Some(f @ (MathFunction::Min | MathFunction::Max | MathFunction::Hypot)) => {
                let mut args = vec![self.parse_sum()?];
                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_sum()?);
                }
                Node::function(f, args)?
            }
            Some(MathFunction::Clamp) => {
                let lo = self.parse_bound()?;
                self.expect(&Token::Comma)?;
                let val = self.parse_sum()?;
                self.expect(&Token::Comma)?;
                let hi = self.parse_bound()?;
                Node::function(MathFunction::Clamp, vec![lo, val, hi])?
            }
            Some(MathFunction::Round(_)) => {
                let strategy = self.parse_rounding_strategy();
                let mut args = vec![self.parse_sum()?];
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_sum()?);
                }
                Node::function(MathFunction::Round(strategy), args)?
            }
            Some(
                f @ (MathFunction::Mod
                | MathFunction::Rem
                | MathFunction::Atan2
                | MathFunction::Pow),
            ) => {
                let a = self.parse_sum()?;
                self.expect(&Token::Comma)?;
                Node::function(f, vec![a, self.parse_sum()?])?
            }
            Some(
                f @ (MathFunction::Abs
                | MathFunction::Sign
                | MathFunction::Sin
                | MathFunction::Cos
                | MathFunction::Tan
                | MathFunction::Asin
                | MathFunction::Acos
                | MathFunction::Atan
                | MathFunction::Sqrt
                | MathFunction::Exp),
            ) => Node::function(f, vec![self.parse_sum()?])?,
            Some(MathFunction::Log) => {
                let mut args = vec![self.parse_sum()?];
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_sum()?);
                }
                Node::function(MathFunction::Log, args)?
            }
        };
        self.expect(&Token::RParen)?;
        Some(expr)
    }

    /// An anchor function's arguments and its closing `)`, the function
    /// token consumed (CSS Anchor Positioning 1 §5.1, §5.2): `anchor(
    /// <anchor-name>? && <anchor-side>, <length-percentage>? )` or —
    /// `size` — `anchor-size( [ <anchor-name> || <anchor-size> ]?,
    /// <length-percentage>? )`.
    fn parse_anchor_body(&mut self, size: bool) -> Option<Node> {
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

    /// `round()`'s optional leading `<rounding-strategy> ,`; `nearest`
    /// when absent.
    fn parse_rounding_strategy(&mut self) -> RoundingStrategy {
        const STRATEGIES: &[(&str, RoundingStrategy)] = &[
            ("nearest", RoundingStrategy::Nearest),
            ("up", RoundingStrategy::Up),
            ("down", RoundingStrategy::Down),
            ("to-zero", RoundingStrategy::ToZero),
        ];
        let found = match (self.peek(), self.tokens.get(self.pos + 1)) {
            (Some(Token::Ident(s)), Some(Token::Comma)) => STRATEGIES
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(s))
                .map(|(_, v)| *v),
            _ => None,
        };
        if let Some(strategy) = found {
            self.pos += 2;
            strategy
        } else {
            RoundingStrategy::Nearest
        }
    }

    /// A `clamp()` bound: a sum, or `none` for no bound.
    fn parse_bound(&mut self) -> Option<Node> {
        match self.peek()? {
            Token::Ident(s) if s.eq_ignore_ascii_case("none") => {
                self.advance();
                Some(Node::leaf(CalcExpr::NoBound))
            }
            _ => self.parse_sum(),
        }
    }
}

/// Parse a math function (`calc()`, `min()`, `round()`, `sin()`, …)
/// that is the whole of `tokens`. `None` on a parse failure, an
/// unbalanced parenthesis, trailing tokens, nesting past
/// [`MAX_CALC_NESTING`], a tree deeper than [`MAX_CALC_DEPTH`] or an
/// expression that does not type-check (CSS Values 4 §10.9) — whether its type suits the
/// property is the caller's check ([`CalcExpr::kind`]).
pub fn parse_calc(tokens: &[Token]) -> Option<CalcExpr> {
    let expr = parse_math(tokens)?;
    expr.kind()?;
    Some(expr)
}

/// A math function over pixel-family lengths (C4G-PX-CALC): every
/// length leaf is a pixel-family unit, normalized to
/// [`CalcUnit::Px`] (CSS Values 4 §6.2), numbers are factors, and any
/// other unit or a percentage makes it `None` — cells beside pixels are
/// a geometry question with no pixel answer. Not type-checked: the
/// caller asks for a `<length>` ([`CalcExpr::kind_strict`]).
pub(crate) fn parse_pixel_calc(tokens: &[Token]) -> Option<CalcExpr> {
    parse_math_as(tokens, true)
}

/// [`parse_calc`] without the type check, for a caller that types the
/// expression itself ([`CalcExpr::kind_as_number`]).
pub(crate) fn parse_math(tokens: &[Token]) -> Option<CalcExpr> {
    parse_math_as(tokens, false)
}

fn parse_math_as(tokens: &[Token], pixels: bool) -> Option<CalcExpr> {
    if !looks_like_calc(tokens) {
        return None;
    }
    let mut parser = CalcParser::new(tokens);
    parser.pixels = pixels;
    let expr = parser.parse_factor()?.expr;
    // A math function must be the entire value.
    if parser.peek().is_some() {
        return None;
    }
    Some(expr)
}

/// Whether a function-token name is an anchor function: `Some(false)`
/// for `anchor`, `Some(true)` for `anchor-size` (ASCII case-insensitive).
fn anchor_function(name: &str) -> Option<bool> {
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

/// `true` iff `tokens` starts with a math-function token (`calc(`,
/// `min(`, `sin(`, …). Used by per-property parsers to detect
/// the math path before trying the bare-value patterns.
pub fn looks_like_calc(tokens: &[Token]) -> bool {
    matches!(tokens.first(), Some(Token::Function(n)) if math_function(n).is_some())
}

#[cfg(test)]
#[path = "calc_tests.rs"]
mod calc_parser_tests;
