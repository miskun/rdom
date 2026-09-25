//! The `calc()` entry point: a recursive-descent parser from tokens
//! to a [`CalcExpr`] AST. The evaluator lives in [`crate::calc`].

use crate::calc::{CalcExpr, CalcOp};
use crate::parse::token::Token;

// Recursive-descent over the token stream. Grammar:
//
//   calc       = 'calc' '(' sum ')'
//   sum        = product (('+' | '-') product)*
//   product    = factor (('*' | '/') factor)*
//   factor     = leaf | '(' sum ')' | calc
//   leaf       = Number | Length | Percentage
//
// Whitespace is already eaten by the tokenizer. Operator
// precedence follows CSS Values L3 §10.2: * and / bind tighter
// than + and -. Per CSS, `+` and `-` MUST be surrounded by
// whitespace at the source level (`5+5` is invalid; `5 + 5` is
// valid). Our tokenizer doesn't preserve whitespace, so we
// accept both forms — a deliberate relaxation documented in
// DIVERGENCES.md.

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

    /// Top-level entry: parse a `calc(<sum>)` form. The leading
    /// `Function("calc")` token must already be matched by the
    /// caller (this fn starts after the opening paren).
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
            Token::Number(n) => {
                let n = *n;
                self.advance();
                // A number followed by `fr` is a flex unit and
                // doesn't make sense inside calc(); other unit
                // idents (px / em / rem / ch) would be terminal-
                // incompatible. We accept bare numbers as
                // unitless "Number" leaves; the cell-vs-number
                // distinction is by syntax (bare `5` = number,
                // `5` with explicit cell typing in the property
                // wrapper). For value-position calc operands
                // (e.g., `calc(100% - 4)`) the `4` is a number
                // that resolves as a length because the
                // containing property is a length.
                Some(CalcExpr::Number(n as f64))
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
                match self.advance()? {
                    Token::RParen => Some(inner),
                    _ => None,
                }
            }
            Token::Function(name) if name.eq_ignore_ascii_case("calc") => {
                self.advance();
                let inner = self.parse_sum()?;
                match self.advance()? {
                    Token::RParen => Some(inner),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Parse a `calc(<sum>)` expression starting from the
/// `Function("calc")` token. Returns the AST + the position
/// AFTER the closing `)`. None on parse failure or unbalanced
/// parens.
pub fn parse_calc(tokens: &[Token]) -> Option<CalcExpr> {
    if tokens.is_empty() {
        return None;
    }
    let mut parser = CalcParser::new(tokens);
    // First token must be `calc(`.
    match parser.advance()? {
        Token::Function(name) if name.eq_ignore_ascii_case("calc") => {}
        _ => return None,
    }
    let expr = parser.parse_sum()?;
    match parser.advance()? {
        Token::RParen => {}
        _ => return None,
    }
    // Reject trailing tokens — a calc() must be the entire value.
    if parser.peek().is_some() {
        return None;
    }
    Some(expr)
}

/// `true` iff `tokens` is exactly a single `calc(...)` form.
/// Used by per-property parsers to detect the calc path before
/// trying the bare-value patterns.
pub fn looks_like_calc(tokens: &[Token]) -> bool {
    matches!(tokens.first(), Some(Token::Function(n)) if n.eq_ignore_ascii_case("calc"))
}

#[cfg(test)]
mod calc_parser_tests {
    use super::*;
    use crate::layout::{Length, Size};
    use crate::parse::token::Token;
    use crate::parse::values::{parse_length, parse_size};

    fn calc_tokens(inner: Vec<Token>) -> Vec<Token> {
        let mut v = vec![Token::Function("calc".to_string())];
        v.extend(inner);
        v.push(Token::RParen);
        v
    }

    #[test]
    fn bare_number() {
        let tokens = calc_tokens(vec![Token::Number(5)]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(e, CalcExpr::Number(5.0));
    }

    #[test]
    fn bare_percent() {
        let tokens = calc_tokens(vec![Token::Percentage(50.0)]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(e, CalcExpr::Percent(50.0));
    }

    #[test]
    fn add_percent_and_number() {
        let tokens = calc_tokens(vec![
            Token::Percentage(50.0),
            Token::Delim('+'),
            Token::Number(2),
        ]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(
            e,
            CalcExpr::binary(CalcOp::Add, CalcExpr::Percent(50.0), CalcExpr::Number(2.0))
        );
    }

    #[test]
    fn sub_full_minus_constant() {
        let tokens = calc_tokens(vec![
            Token::Percentage(100.0),
            Token::Delim('-'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(
            e,
            CalcExpr::binary(CalcOp::Sub, CalcExpr::Percent(100.0), CalcExpr::Number(4.0))
        );
    }

    #[test]
    fn mul_binds_tighter_than_add() {
        // calc(2 + 3 * 4) → Add(2, Mul(3, 4))
        let tokens = calc_tokens(vec![
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::Delim('*'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        let expected = CalcExpr::binary(
            CalcOp::Add,
            CalcExpr::Number(2.0),
            CalcExpr::binary(CalcOp::Mul, CalcExpr::Number(3.0), CalcExpr::Number(4.0)),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn parens_override_precedence() {
        // calc((2 + 3) * 4) → Mul(Add(2,3), 4)
        let tokens = calc_tokens(vec![
            Token::LParen,
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::RParen,
            Token::Delim('*'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        let expected = CalcExpr::binary(
            CalcOp::Mul,
            CalcExpr::binary(CalcOp::Add, CalcExpr::Number(2.0), CalcExpr::Number(3.0)),
            CalcExpr::Number(4.0),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn nested_calc() {
        // calc(calc(2 + 3) * 4) — semantically same as the parens form.
        let tokens = calc_tokens(vec![
            Token::Function("calc".to_string()),
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::RParen,
            Token::Delim('*'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        let expected = CalcExpr::binary(
            CalcOp::Mul,
            CalcExpr::binary(CalcOp::Add, CalcExpr::Number(2.0), CalcExpr::Number(3.0)),
            CalcExpr::Number(4.0),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn unary_minus() {
        let tokens = calc_tokens(vec![
            Token::Number(5),
            Token::Delim('-'),
            Token::Delim('-'),
            Token::Number(3),
        ]);
        // calc(5 - -3) = Sub(5, -3) — and -3 is a Number(-3.0).
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(
            e,
            CalcExpr::binary(CalcOp::Sub, CalcExpr::Number(5.0), CalcExpr::Number(-3.0))
        );
    }

    #[test]
    fn invalid_form_returns_none() {
        // Missing closing paren.
        let tokens = vec![Token::Function("calc".to_string()), Token::Number(5)];
        assert!(parse_calc(&tokens).is_none());

        // Trailing tokens after the calc.
        let tokens = calc_tokens(vec![Token::Number(5)]);
        let mut with_trail = tokens.clone();
        with_trail.push(Token::Number(99));
        assert!(parse_calc(&with_trail).is_none());

        // Not a calc() at all.
        let tokens = vec![Token::Number(5)];
        assert!(parse_calc(&tokens).is_none());
    }

    #[test]
    fn looks_like_calc_detects_function_token() {
        let yes = vec![Token::Function("calc".to_string())];
        let no = vec![Token::Number(5)];
        assert!(looks_like_calc(&yes));
        assert!(!looks_like_calc(&no));
    }

    // ─── Parse-time constant-eval integration ───────────────────────

    #[test]
    fn parse_size_accepts_constant_calc() {
        // `width: calc(2 + 3)` → `Size::Fixed(5)`.
        let tokens = calc_tokens(vec![Token::Number(2), Token::Delim('+'), Token::Number(3)]);
        assert_eq!(parse_size(&tokens), Some(Size::Fixed(5)));
    }

    #[test]
    fn parse_size_accepts_constant_calc_with_precedence() {
        // `width: calc(2 + 3 * 4)` → `Size::Fixed(14)`.
        let tokens = calc_tokens(vec![
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::Delim('*'),
            Token::Number(4),
        ]);
        assert_eq!(parse_size(&tokens), Some(Size::Fixed(14)));
    }

    #[test]
    fn parse_size_carries_percent_bearing_calc_as_calc_variant() {
        // M6 full: percent-bearing calc parses into Size::Calc and
        // resolves at layout time.
        let tokens = calc_tokens(vec![
            Token::Percentage(100.0),
            Token::Delim('-'),
            Token::Number(4),
        ]);
        match parse_size(&tokens) {
            Some(Size::Calc(expr)) => {
                assert!(expr.contains_percent());
            }
            other => panic!("expected Size::Calc, got {other:?}"),
        }
    }

    #[test]
    fn parse_size_clamps_negative_constant_calc_to_zero() {
        // `width: calc(2 - 10)` → -8 cells → clamped to 0.
        let tokens = calc_tokens(vec![Token::Number(2), Token::Delim('-'), Token::Number(10)]);
        assert_eq!(parse_size(&tokens), Some(Size::Fixed(0)));
    }

    #[test]
    fn parse_length_accepts_constant_calc_negative_result() {
        // `top: calc(-3 * 2)` → -6 → Length::Cells(-6).
        let tokens = calc_tokens(vec![
            Token::Delim('-'),
            Token::Number(3),
            Token::Delim('*'),
            Token::Number(2),
        ]);
        assert_eq!(parse_length(&tokens), Some(Length::Cells(-6)));
    }

    #[test]
    fn parse_length_carries_percent_bearing_calc_as_calc_variant() {
        let tokens = calc_tokens(vec![
            Token::Percentage(50.0),
            Token::Delim('+'),
            Token::Number(2),
        ]);
        match parse_length(&tokens) {
            Some(Length::Calc(expr)) => {
                assert!(expr.contains_percent());
            }
            other => panic!("expected Length::Calc, got {other:?}"),
        }
    }
}
