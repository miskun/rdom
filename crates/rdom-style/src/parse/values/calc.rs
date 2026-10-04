//! The math-function entry point (CSS Values 4 §10): a recursive-
//! descent parser from tokens to a [`CalcExpr`] AST. The evaluator lives
//! in [`crate::calc`].

use crate::calc::{CalcExpr, CalcOp, MathFunction, RoundingStrategy};
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
//   leaf       = Number | Length | Percentage
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
fn math_function(name: &str) -> Option<Option<MathFunction>> {
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

/// Parser cursor over a `&[Token]`. Tracks position only.
struct CalcParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> CalcParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
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

    fn parse_sum(&mut self) -> Option<CalcExpr> {
        let mut lhs = self.parse_product()?;
        loop {
            let op = match self.peek() {
                Some(Token::Delim('+')) => CalcOp::Add,
                Some(Token::Delim('-')) => CalcOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_product()?;
            lhs = CalcExpr::binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_product(&mut self) -> Option<CalcExpr> {
        let mut lhs = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Delim('*')) => CalcOp::Mul,
                Some(Token::Delim('/')) => CalcOp::Div,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_factor()?;
            // CSS Values 4 §10.9: dividing by a literal zero makes the
            // whole `calc()` invalid at parse time — never a silent 0
            // at layout time.
            if op == CalcOp::Div && matches!(rhs, CalcExpr::Number(z) if z == 0.0) {
                return None;
            }
            lhs = CalcExpr::binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_factor(&mut self) -> Option<CalcExpr> {
        match self.peek()? {
            // A bare number is a `<number>` leaf; in a length property
            // it reads as cells (rdom's unitless length).
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Some(CalcExpr::Number(f64::from(n)))
            }
            Token::Float(f) => {
                let f = *f;
                self.advance();
                Some(CalcExpr::Number(f))
            }
            Token::Percentage(n) => {
                let n = *n;
                self.advance();
                Some(CalcExpr::Percent(n))
            }
            Token::Delim('-') => {
                // Unary minus — accept `-5` as a literal.
                self.advance();
                match self.advance()? {
                    Token::Number(n) => Some(CalcExpr::Number(-f64::from(*n))),
                    Token::Float(f) => Some(CalcExpr::Number(-*f)),
                    Token::Percentage(n) => Some(CalcExpr::Percent(-*n)),
                    _ => None,
                }
            }
            Token::Delim('+') => {
                // Unary plus — accept and ignore.
                self.advance();
                self.parse_factor()
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_sum()?;
                self.expect(&Token::RParen)?;
                Some(inner)
            }
            Token::Function(name) => {
                let func = math_function(name)?;
                self.advance();
                self.parse_function_body(func)
            }
            Token::Ident(name) => {
                let value = constant(name)?;
                self.advance();
                Some(CalcExpr::Number(value))
            }
            _ => None,
        }
    }

    /// The arguments of a math function and its closing `)`, the
    /// function token already consumed.
    fn parse_function_body(&mut self, func: Option<MathFunction>) -> Option<CalcExpr> {
        let expr = match func {
            None => self.parse_sum()?,
            Some(f @ (MathFunction::Min | MathFunction::Max | MathFunction::Hypot)) => {
                let mut args = vec![self.parse_sum()?];
                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_sum()?);
                }
                CalcExpr::function(f, args)
            }
            Some(MathFunction::Clamp) => {
                let lo = self.parse_bound()?;
                self.expect(&Token::Comma)?;
                let val = self.parse_sum()?;
                self.expect(&Token::Comma)?;
                let hi = self.parse_bound()?;
                CalcExpr::function(MathFunction::Clamp, vec![lo, val, hi])
            }
            Some(MathFunction::Round(_)) => {
                let strategy = self.parse_rounding_strategy();
                let mut args = vec![self.parse_sum()?];
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_sum()?);
                }
                CalcExpr::function(MathFunction::Round(strategy), args)
            }
            Some(
                f @ (MathFunction::Mod
                | MathFunction::Rem
                | MathFunction::Atan2
                | MathFunction::Pow),
            ) => {
                let a = self.parse_sum()?;
                self.expect(&Token::Comma)?;
                CalcExpr::function(f, vec![a, self.parse_sum()?])
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
            ) => CalcExpr::function(f, vec![self.parse_sum()?]),
            Some(MathFunction::Log) => {
                let mut args = vec![self.parse_sum()?];
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_sum()?);
                }
                CalcExpr::function(MathFunction::Log, args)
            }
        };
        self.expect(&Token::RParen)?;
        Some(expr)
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
    fn parse_bound(&mut self) -> Option<CalcExpr> {
        match self.peek()? {
            Token::Ident(s) if s.eq_ignore_ascii_case("none") => {
                self.advance();
                Some(CalcExpr::None)
            }
            _ => self.parse_sum(),
        }
    }
}

/// Parse a math function (`calc()`, `min()`, `round()`, `sin()`, …)
/// that is the whole of `tokens`. `None` on a parse failure, an
/// unbalanced parenthesis, trailing tokens or an expression that does
/// not type-check (CSS Values 4 §10.9) — whether its type suits the
/// property is the caller's check ([`CalcExpr::kind`]).
pub fn parse_calc(tokens: &[Token]) -> Option<CalcExpr> {
    if !looks_like_calc(tokens) {
        return None;
    }
    let mut parser = CalcParser::new(tokens);
    let expr = parser.parse_factor()?;
    // A math function must be the entire value.
    if parser.peek().is_some() {
        return None;
    }
    expr.kind()?;
    Some(expr)
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
