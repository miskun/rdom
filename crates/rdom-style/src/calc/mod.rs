//! Math-function expression AST + resolver (CSS Values 4 §10).
//!
//! `calc(<sum>)` where `<sum>` is a chain of `+`/`-` operators on terms,
//! terms are chains of `*`/`/` on factors, and factors are leaf values,
//! parenthesised sub-sums or nested math functions — `min()`, `max()`,
//! `clamp()` ([`MathFunction`]).
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

/// A math function other than `calc()` (CSS Values 4 §10), which is a
/// plain parenthesised sum in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MathFunction {
    /// `min(A, B, …)` — the smallest argument (§10.2).
    Min,
    /// `max(A, B, …)` — the largest argument (§10.2).
    Max,
    /// `clamp(MIN, VAL, MAX)` — `max(MIN, min(VAL, MAX))`, so `MIN`
    /// wins a conflict (§10.2). A `none` bound is absent: the
    /// arguments are `[MIN?, VAL, MAX?]` as written, with
    /// [`CalcExpr::None`] in place of an absent bound.
    Clamp,
    /// `round(<strategy>?, A, B?)` — A rounded to a multiple of B (1
    /// when omitted) (§10.3.1).
    Round(RoundingStrategy),
    /// `mod(A, B)` — the remainder with B's sign (§10.3.2).
    Mod,
    /// `rem(A, B)` — the remainder with A's sign (§10.3.2).
    Rem,
    /// `abs(A)` (§10.7.1).
    Abs,
    /// `sign(A)` — -1, 0 or 1 (zero keeps its sign) (§10.7.2).
    Sign,
}

/// `round()`'s `<rounding-strategy>` (CSS Values 4 §10.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoundingStrategy {
    /// The nearer multiple; a tie goes toward +∞. The default.
    #[default]
    Nearest,
    /// The multiple toward +∞.
    Up,
    /// The multiple toward −∞.
    Down,
    /// The multiple toward zero.
    ToZero,
}

impl RoundingStrategy {
    /// The strategy's CSS keyword.
    pub fn keyword(self) -> &'static str {
        match self {
            RoundingStrategy::Nearest => "nearest",
            RoundingStrategy::Up => "up",
            RoundingStrategy::Down => "down",
            RoundingStrategy::ToZero => "to-zero",
        }
    }
}

impl MathFunction {
    /// The function's CSS name.
    pub fn name(self) -> &'static str {
        match self {
            MathFunction::Min => "min",
            MathFunction::Max => "max",
            MathFunction::Clamp => "clamp",
            MathFunction::Round(_) => "round",
            MathFunction::Mod => "mod",
            MathFunction::Rem => "rem",
            MathFunction::Abs => "abs",
            MathFunction::Sign => "sign",
        }
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

/// Evaluate a math function over its arguments (CSS Values 4 §10).
/// Any NaN argument makes the result NaN (§10.9).
fn eval_function(func: MathFunction, args: &[CalcExpr], cx: &ResolveCtx) -> f64 {
    let values = args.iter().map(|a| a.resolve_f64(cx));
    match func {
        MathFunction::Min => values.fold(f64::INFINITY, nan_min),
        MathFunction::Max => values.fold(f64::NEG_INFINITY, nan_max),
        MathFunction::Clamp => {
            let [lo, val, hi] = args else {
                return f64::NAN;
            };
            let bound = |e: &CalcExpr, absent: f64| match e {
                CalcExpr::None => absent,
                e => e.resolve_f64(cx),
            };
            let (lo, val, hi) = (
                bound(lo, f64::NEG_INFINITY),
                val.resolve_f64(cx),
                bound(hi, f64::INFINITY),
            );
            nan_max(lo, nan_min(val, hi))
        }
        MathFunction::Round(strategy) => {
            let mut v = values;
            let a = v.next().unwrap_or(f64::NAN);
            round(strategy, a, v.next().unwrap_or(1.0))
        }
        MathFunction::Mod | MathFunction::Rem => {
            let mut v = values;
            let (a, b) = (v.next().unwrap_or(f64::NAN), v.next().unwrap_or(f64::NAN));
            remainder(func == MathFunction::Mod, a, b)
        }
        MathFunction::Abs => values.map(f64::abs).next().unwrap_or(f64::NAN),
        MathFunction::Sign => values
            .map(|a| {
                if a == 0.0 || a.is_nan() {
                    a
                } else {
                    a.signum()
                }
            })
            .next()
            .unwrap_or(f64::NAN),
    }
}

/// `round(strategy, a, b)` (CSS Values 4 §10.3.1). B's sign is
/// irrelevant (its multiples are the same); a zero B is NaN.
fn round(strategy: RoundingStrategy, a: f64, b: f64) -> f64 {
    let b = b.abs();
    if a.is_nan() || b.is_nan() || b == 0.0 || (a.is_infinite() && b.is_infinite()) {
        return f64::NAN;
    }
    if a.is_infinite() {
        return a;
    }
    if b.is_infinite() {
        // A finite A rounds to zero (keeping its sign) or, toward an
        // infinity in the strategy's direction, to that infinity.
        return match strategy {
            RoundingStrategy::Up if a > 0.0 => f64::INFINITY,
            RoundingStrategy::Down if a < 0.0 => f64::NEG_INFINITY,
            _ => 0.0f64.copysign(a),
        };
    }
    let lower = (a / b).floor() * b;
    if lower == a {
        return a;
    }
    let upper = lower + b;
    match strategy {
        RoundingStrategy::Nearest => {
            if a - lower < upper - a {
                lower
            } else {
                upper
            }
        }
        RoundingStrategy::Up => upper,
        RoundingStrategy::Down => lower,
        RoundingStrategy::ToZero => {
            if a > 0.0 {
                lower
            } else {
                upper
            }
        }
    }
}

/// `mod(a, b)` (`modulo`: the result takes B's sign) or `rem(a, b)`
/// (A's sign) (CSS Values 4 §10.3.2). A zero B or an infinite A is NaN;
/// an infinite B leaves A, except that `mod()` of oppositely signed
/// values is NaN.
fn remainder(modulo: bool, a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() || b == 0.0 || a.is_infinite() {
        return f64::NAN;
    }
    if b.is_infinite() {
        return if modulo && a != 0.0 && a.is_sign_negative() != b.is_sign_negative() {
            f64::NAN
        } else {
            a
        };
    }
    let r = a % b; // `rem()`: truncated division, A's sign
    if modulo && r != 0.0 && (r < 0.0) != (b < 0.0) {
        r + b
    } else {
        r
    }
}

/// `f64::min` that propagates NaN instead of ignoring it.
fn nan_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// `f64::max` that propagates NaN instead of ignoring it.
fn nan_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
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
