//! `calc()` expression AST + resolver.
//!
//! CSS Values L3 §10.1: `calc(<sum>)` where `<sum>` is a chain of
//! `+`/`-` operators on terms, terms are chains of `*`/`/` on
//! factors, and factors are leaf values or parenthesised sub-sums.
//!
//! Leaf value kinds rdom supports inside `calc()`:
//!
//! - **Number** — bare numeric literal (used in multiplication
//!   factors and division divisors; CSS forbids using a bare
//!   number where a length is required).
//! - **Length** — integer cells. Negative permitted.
//! - **Percentage** — resolved against a containing-block axis at
//!   layout time. The axis depends on which property the calc
//!   appears in (`width` → parent content width, `top` → parent
//!   content height, etc.). See `ResolveCtx::percent_basis`.
//!
//! Resolution returns a signed integer-cell value
//! (`i32` — rdom layout uses `i32` for offsets and clamps to
//! `i16`/`u16` at the property boundary). Rounding: half-to-even
//! after summing.
//!
//! ## What's NOT supported in 0.2.0
//!
//! - `min(...)` / `max(...)` / `clamp(...)` — CSS Values L4, future
//!   milestone.
//! - Mixed-unit `<length>` arithmetic (px / em / rem) — terminals
//!   are cell-only, so length operands are always cells.
//! - `<angle>` / `<time>` / colors in calc() — out of M6 scope.

use std::fmt;

/// One operator in a calc() expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for CalcOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CalcOp::Add => "+",
            CalcOp::Sub => "-",
            CalcOp::Mul => "*",
            CalcOp::Div => "/",
        };
        f.write_str(s)
    }
}

/// One node of a calc() expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum CalcExpr {
    /// Bare number (no unit). Used as a multiplier / divisor.
    Number(f64),
    /// Integer cells.
    Length(i32),
    /// Percentage (0..100 for typical values; CSS allows > 100).
    /// Resolves against the containing-block axis at layout time.
    Percent(f64),
    /// Binary operator + two operands.
    Binary {
        op: CalcOp,
        lhs: Box<CalcExpr>,
        rhs: Box<CalcExpr>,
    },
}

/// Resolution context — the dimensions the percentage operands
/// resolve against. Caller picks `percent_basis` based on which
/// property the calc() appears in:
///
/// - `width` / `min-width` / `max-width` / `left` / `right` →
///   parent content **width**.
/// - `height` / `min-height` / `max-height` / `top` / `bottom` →
///   parent content **height**.
/// - `padding-*` / `margin-*` per CSS resolve against parent
///   **width** for ALL sides (CSS Box Model §8.4).
#[derive(Debug, Clone, Copy)]
pub struct ResolveCtx {
    /// The dimension percentage operands resolve against, in
    /// cells. Caller provides — see doc above for which dimension
    /// each property uses.
    pub percent_basis: i32,
}

impl ResolveCtx {
    pub fn new(percent_basis: i32) -> Self {
        Self { percent_basis }
    }
}

impl CalcExpr {
    /// Resolve to an integer-cell value given the containing-block
    /// dimensions. Float arithmetic during the walk; round half-
    /// to-even on the final result.
    pub fn resolve(&self, cx: &ResolveCtx) -> i32 {
        let v = self.resolve_f64(cx);
        round_half_to_even(v)
    }

    /// Float-domain resolution. Pub for tests + paint paths that
    /// need the unrounded value.
    pub fn resolve_f64(&self, cx: &ResolveCtx) -> f64 {
        match self {
            CalcExpr::Number(n) => *n,
            CalcExpr::Length(c) => *c as f64,
            CalcExpr::Percent(p) => (*p / 100.0) * cx.percent_basis as f64,
            CalcExpr::Binary { op, lhs, rhs } => {
                let l = lhs.resolve_f64(cx);
                let r = rhs.resolve_f64(cx);
                match op {
                    CalcOp::Add => l + r,
                    CalcOp::Sub => l - r,
                    CalcOp::Mul => l * r,
                    CalcOp::Div => {
                        if r == 0.0 {
                            // CSS Values L3 §10.9: division by zero
                            // makes the calc() invalid. We can't
                            // signal "invalid" from here — return 0
                            // and trust the parser to have warned
                            // on a literal `/ 0`. Runtime-computed
                            // zero divisors (e.g., a percent that
                            // resolves to 0 in the denominator) just
                            // saturate.
                            0.0
                        } else {
                            l / r
                        }
                    }
                }
            }
        }
    }

    /// `true` iff this expression contains any percentage operand
    /// (directly or in a sub-expression). Used by the parser to
    /// decide whether a calc() result can be evaluated at parse
    /// time (constant) or must be deferred to layout (context-
    /// dependent).
    pub fn contains_percent(&self) -> bool {
        match self {
            CalcExpr::Percent(_) => true,
            CalcExpr::Number(_) | CalcExpr::Length(_) => false,
            CalcExpr::Binary { lhs, rhs, .. } => lhs.contains_percent() || rhs.contains_percent(),
        }
    }

    /// Convenience for binary node construction.
    pub fn binary(op: CalcOp, lhs: CalcExpr, rhs: CalcExpr) -> CalcExpr {
        CalcExpr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

/// Round half-to-even (banker's rounding) for the final calc()
/// result. Matches CSS rounding when integer-quantised.
fn round_half_to_even(v: f64) -> i32 {
    let f = v.round();
    if (v - v.floor() - 0.5).abs() < f64::EPSILON {
        // Exactly halfway — pick the even neighbor.
        let floor = v.floor() as i32;
        if floor % 2 == 0 { floor } else { floor + 1 }
    } else {
        f as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx(basis: i32) -> ResolveCtx {
        ResolveCtx::new(basis)
    }

    #[test]
    fn number_resolves_to_self() {
        assert_eq!(CalcExpr::Number(5.0).resolve(&cx(100)), 5);
    }

    #[test]
    fn length_resolves_to_cell_count() {
        assert_eq!(CalcExpr::Length(7).resolve(&cx(100)), 7);
    }

    #[test]
    fn percent_resolves_against_basis() {
        assert_eq!(CalcExpr::Percent(50.0).resolve(&cx(100)), 50);
        assert_eq!(CalcExpr::Percent(50.0).resolve(&cx(40)), 20);
        assert_eq!(CalcExpr::Percent(25.0).resolve(&cx(80)), 20);
    }

    #[test]
    fn add_percent_and_length_resolves_against_basis() {
        // calc(50% + 2) where basis = 40 → 22.
        let e = CalcExpr::binary(CalcOp::Add, CalcExpr::Percent(50.0), CalcExpr::Length(2));
        assert_eq!(e.resolve(&cx(40)), 22);
    }

    #[test]
    fn sub_basis_minus_length() {
        // calc(100% - 4) where basis = 40 → 36.
        let e = CalcExpr::binary(CalcOp::Sub, CalcExpr::Percent(100.0), CalcExpr::Length(4));
        assert_eq!(e.resolve(&cx(40)), 36);
    }

    #[test]
    fn mul_basis_by_number() {
        // calc(50% * 2) where basis = 40 → 40.
        let e = CalcExpr::binary(CalcOp::Mul, CalcExpr::Percent(50.0), CalcExpr::Number(2.0));
        assert_eq!(e.resolve(&cx(40)), 40);
    }

    #[test]
    fn div_basis_by_number() {
        // calc(100% / 2) where basis = 40 → 20.
        let e = CalcExpr::binary(CalcOp::Div, CalcExpr::Percent(100.0), CalcExpr::Number(2.0));
        assert_eq!(e.resolve(&cx(40)), 20);
    }

    #[test]
    fn div_by_zero_saturates_to_zero() {
        let e = CalcExpr::binary(CalcOp::Div, CalcExpr::Length(10), CalcExpr::Number(0.0));
        assert_eq!(e.resolve(&cx(100)), 0);
    }

    #[test]
    fn contains_percent_walks_subtree() {
        let constant = CalcExpr::binary(CalcOp::Add, CalcExpr::Length(3), CalcExpr::Length(4));
        assert!(!constant.contains_percent());

        let withp = CalcExpr::binary(
            CalcOp::Add,
            CalcExpr::Length(3),
            CalcExpr::binary(CalcOp::Mul, CalcExpr::Percent(50.0), CalcExpr::Number(1.0)),
        );
        assert!(withp.contains_percent());
    }

    #[test]
    fn half_to_even_rounding() {
        // 0.5 → 0, 1.5 → 2, 2.5 → 2, 3.5 → 4 (banker's rounding)
        assert_eq!(round_half_to_even(0.5), 0);
        assert_eq!(round_half_to_even(1.5), 2);
        assert_eq!(round_half_to_even(2.5), 2);
        assert_eq!(round_half_to_even(3.5), 4);
    }
}
