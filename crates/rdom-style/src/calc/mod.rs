//! Math-function expression AST + resolver (CSS Values 4 §10).
//!
//! `calc(<sum>)` where `<sum>` is a chain of `+`/`-` operators on terms,
//! terms are chains of `*`/`/` on factors, and factors are leaf values,
//! parenthesised sub-sums or nested math functions ([`MathFunction`]:
//! comparison, stepped-value, trigonometric, exponential and
//! sign-related functions) and constants (`e`, `pi`, `infinity`, `NaN`).
//! Every expression has a type ([`CalcKind`], §10.9): a property takes
//! the ones its grammar allows. Angles are radians inside the
//! evaluator.
//!
//! Leaf value kinds rdom supports:
//!
//! - **Number** — bare numeric literal. A bare number doubles as rdom's
//!   cell (DIVERGENCES §1), so `calc(100% - 4)` subtracts four cells.
//! - **Length** — integer cells. Negative permitted.
//! - **Percentage** — resolved against a containing-block axis at
//!   layout time. The axis depends on which property the expression
//!   appears in (`width` → parent content width, `top` → parent
//!   content height, etc.). See `ResolveCtx::percent_basis`.
//!
//! Resolution evaluates in `f64` and returns a signed integer-cell value
//! (`i32` — rdom layout uses `i32` for offsets and clamps to
//! `i16`/`u16` at the property boundary), rounded half-to-even once,
//! after the whole expression: the value becomes a length there. A NaN
//! result is 0 and an infinite one clamps to the range (Values 4
//! §10.9).

use std::fmt;

mod functions;
mod types;

use functions::eval_function;
pub use functions::{MathFunction, RoundingStrategy};
pub use types::CalcKind;

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

/// One node of a math-function expression tree.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
    /// A math function over its arguments.
    Function {
        func: MathFunction,
        args: Vec<CalcExpr>,
    },
    /// The keyword `none` in a `clamp()` bound: no bound.
    None,
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
#[non_exhaustive]
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
    /// to-even on the final result. NaN resolves to 0 and ±∞ clamps
    /// to the `i32` range (CSS Values 4 §10.9).
    pub fn resolve(&self, cx: &ResolveCtx) -> i32 {
        let v = self.resolve_f64(cx);
        if v.is_nan() {
            0
        } else {
            round_half_to_even(v.clamp(f64::from(i32::MIN), f64::from(i32::MAX)))
        }
    }

    /// Float-domain resolution. Pub for tests + paint paths that
    /// need the unrounded value. NaN propagates (Values 4 §10.9).
    pub fn resolve_f64(&self, cx: &ResolveCtx) -> f64 {
        match self {
            CalcExpr::Number(n) => *n,
            CalcExpr::Length(c) => f64::from(*c),
            CalcExpr::Percent(p) => (*p / 100.0) * f64::from(cx.percent_basis),
            CalcExpr::None => f64::NAN,
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
            CalcExpr::Function { func, args } => eval_function(*func, args, cx),
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
            CalcExpr::Number(_) | CalcExpr::Length(_) | CalcExpr::None => false,
            CalcExpr::Binary { lhs, rhs, .. } => lhs.contains_percent() || rhs.contains_percent(),
            CalcExpr::Function { args, .. } => args.iter().any(CalcExpr::contains_percent),
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

    /// Convenience for math-function node construction.
    pub fn function(func: MathFunction, args: Vec<CalcExpr>) -> CalcExpr {
        CalcExpr::Function { func, args }
    }
}

/// Round half-to-even (banker's rounding) for the final calc()
/// result. Matches CSS rounding when integer-quantised.
pub fn round_half_to_even(v: f64) -> i32 {
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
mod tests;
