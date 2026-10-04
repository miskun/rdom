//! The math functions besides `calc()` (CSS Values 4 §10.2 – §10.7) and
//! their evaluation over `f64`: comparison, stepped-value, trigonometric,
//! exponential and sign-related functions. NaN propagates (§10.9).

use super::{CalcExpr, ResolveCtx};

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
    /// [`CalcExpr::NoBound`] in place of an absent bound.
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
    /// `sin(A)` — A a `<number>` (radians) or `<angle>` (§10.4).
    Sin,
    /// `cos(A)` (§10.4).
    Cos,
    /// `tan(A)` (§10.4).
    Tan,
    /// `asin(A)` — an `<angle>` (§10.4).
    Asin,
    /// `acos(A)` — an `<angle>` (§10.4).
    Acos,
    /// `atan(A)` — an `<angle>` (§10.4).
    Atan,
    /// `atan2(A, B)` — the `<angle>` of the point (B, A) (§10.4).
    Atan2,
    /// `pow(A, B)` (§10.5).
    Pow,
    /// `sqrt(A)` (§10.5).
    Sqrt,
    /// `hypot(A, …)` — the length of the vector of its arguments (§10.5).
    Hypot,
    /// `log(A, B?)` — base B, `e` when omitted (§10.5).
    Log,
    /// `exp(A)` (§10.5).
    Exp,
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
            MathFunction::Sin => "sin",
            MathFunction::Cos => "cos",
            MathFunction::Tan => "tan",
            MathFunction::Asin => "asin",
            MathFunction::Acos => "acos",
            MathFunction::Atan => "atan",
            MathFunction::Atan2 => "atan2",
            MathFunction::Pow => "pow",
            MathFunction::Sqrt => "sqrt",
            MathFunction::Hypot => "hypot",
            MathFunction::Log => "log",
            MathFunction::Exp => "exp",
        }
    }
}

/// Evaluate a math function over its arguments (CSS Values 4 §10).
/// Any NaN argument makes the result NaN (§10.9).
pub(super) fn eval_function(func: MathFunction, args: &[CalcExpr], cx: &ResolveCtx) -> f64 {
    let values = args.iter().map(|a| a.resolve_f64(cx));
    match func {
        MathFunction::Min => values.fold(f64::INFINITY, nan_min),
        MathFunction::Max => values.fold(f64::NEG_INFINITY, nan_max),
        MathFunction::Clamp => {
            let [lo, val, hi] = args else {
                return f64::NAN;
            };
            let bound = |e: &CalcExpr, absent: f64| match e {
                CalcExpr::NoBound => absent,
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
        MathFunction::Abs => unary(values, f64::abs),
        MathFunction::Sign => unary(values, |a| {
            if a == 0.0 || a.is_nan() {
                a
            } else {
                a.signum()
            }
        }),
        // Angles are radians inside the evaluator, so a `<number>`
        // argument and an `<angle>` one need no conversion here.
        MathFunction::Sin => unary(values, f64::sin),
        MathFunction::Cos => unary(values, f64::cos),
        MathFunction::Tan => unary(values, f64::tan),
        MathFunction::Asin => unary(values, f64::asin),
        MathFunction::Acos => unary(values, f64::acos),
        MathFunction::Atan => unary(values, f64::atan),
        MathFunction::Atan2 => binary(values, f64::atan2),
        MathFunction::Pow => binary(values, f64::powf),
        MathFunction::Sqrt => unary(values, f64::sqrt),
        MathFunction::Hypot => values.fold(0.0, f64::hypot),
        MathFunction::Log => {
            let mut v = values;
            let a = v.next().unwrap_or(f64::NAN);
            match v.next() {
                Some(base) => a.ln() / base.ln(),
                None => a.ln(),
            }
        }
        MathFunction::Exp => unary(values, f64::exp),
    }
}

/// `f` of the single argument.
fn unary(mut values: impl Iterator<Item = f64>, f: fn(f64) -> f64) -> f64 {
    values.next().map_or(f64::NAN, f)
}

/// `f` of the two arguments.
fn binary(mut values: impl Iterator<Item = f64>, f: fn(f64, f64) -> f64) -> f64 {
    match (values.next(), values.next()) {
        (Some(a), Some(b)) => f(a, b),
        _ => f64::NAN,
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
