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
//! - **Dimension** — a number with a unit ([`CalcUnit`]): `ch` is a
//!   column.
//! - **Percentage** — resolved against a containing-block axis at
//!   layout time. The axis depends on which property the expression
//!   appears in (`width` → parent content width, `top` → parent
//!   content height, etc.). See `ResolveCtx::percent_basis`.
//!
//! Resolution evaluates in `f64` and returns a signed integer-cell value
//! (`i32` — rdom layout uses `i32` for offsets and clamps to
//! `i16`/`u16` at the property boundary), rounded half-to-even once,
//! after the whole expression: the value becomes a length there.
//! Division is IEEE-754 (`1/0` is +∞, `0/0` NaN); a NaN result is 0
//! and an infinite one clamps to the range (Values 4 §10.9).

use std::fmt;

mod anchor;
mod context;
mod functions;
mod types;
mod units;

pub use anchor::{AnchorFunction, AnchorSide, AnchorSize};
pub use context::{UnitContext, UnitReads, Viewport};
use functions::eval_function;
pub use functions::{MathFunction, RoundingStrategy};
pub use types::CalcKind;
pub use units::{CalcUnit, ViewportAxis, ViewportSize, ViewportUnit};

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

/// One node of a math-function expression tree. Closed data (DESIGN
/// "Which public types are `#[non_exhaustive]`"): a walker must handle
/// every node.
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
    /// A number with a unit (`2ch`) — see [`CalcUnit`].
    Dimension { value: f64, unit: CalcUnit },
    /// A math function over its arguments.
    Function {
        func: MathFunction,
        args: Vec<CalcExpr>,
    },
    /// The keyword `none` in a `clamp()` bound: no bound. (Not `None`,
    /// which would shadow `Option::None` under a glob import.)
    NoBound,
    /// An anchor function (CSS Anchor Positioning 1 §5): `anchor()` or
    /// `anchor-size()`, a length once layout resolves it against the
    /// anchor ([`CalcExpr::substitute_anchors`]); unresolved, its
    /// fallback (or NaN without one).
    Anchor(Box<AnchorFunction>),
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
    /// The viewport the viewport-percentage units resolve against;
    /// `None` in layout. The cascade makes those units absolute
    /// ([`ComputedStyle::resolve_viewport_units`](crate::ComputedStyle::resolve_viewport_units)),
    /// so layout never meets one: one evaluated without a viewport is a
    /// computed-style field the cascade missed — a debug assertion, 0
    /// cells in release.
    pub viewport: Option<Viewport>,
}

impl ResolveCtx {
    /// A layout context: percentages resolve against `percent_basis`,
    /// and there is no viewport (see [`ResolveCtx::viewport`]).
    pub fn new(percent_basis: i32) -> Self {
        Self {
            percent_basis,
            viewport: None,
        }
    }

    /// This context with `viewport` for the viewport-percentage units.
    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self
    }
}

impl CalcExpr {
    /// Resolve to an integer-cell value given the containing-block
    /// dimensions. Float arithmetic during the walk; round half-
    /// to-even on the final result. NaN resolves to 0 and ±∞ clamps
    /// to `±i32::MAX` (CSS Values 4 §10.9; symmetric, so a consumer
    /// can negate the result).
    pub fn resolve(&self, cx: &ResolveCtx) -> i32 {
        to_cells(self.resolve_f64(cx))
    }

    /// Float-domain resolution. Pub for tests + paint paths that
    /// need the unrounded value. NaN propagates (Values 4 §10.9).
    pub fn resolve_f64(&self, cx: &ResolveCtx) -> f64 {
        match self {
            CalcExpr::Number(n) => *n,
            CalcExpr::Length(c) => f64::from(*c),
            CalcExpr::Percent(p) => (*p / 100.0) * f64::from(cx.percent_basis),
            CalcExpr::NoBound => f64::NAN,
            CalcExpr::Dimension { value, unit } => unit.canonical(*value, cx),
            CalcExpr::Binary { op, lhs, rhs } => {
                let l = lhs.resolve_f64(cx);
                let r = rhs.resolve_f64(cx);
                match op {
                    CalcOp::Add => l + r,
                    CalcOp::Sub => l - r,
                    CalcOp::Mul => l * r,
                    // IEEE-754, as CSS Values 4 §10.9 requires: `1/0`
                    // is +∞, `0/0` NaN — at any depth; the top level
                    // clamps (`resolve`).
                    CalcOp::Div => l / r,
                }
            }
            CalcExpr::Function { func, args } => eval_function(*func, args, cx),
            CalcExpr::Anchor(f) => f.fallback().map_or(f64::NAN, |e| e.resolve_f64(cx)),
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
            CalcExpr::Number(_)
            | CalcExpr::Length(_)
            | CalcExpr::NoBound
            | CalcExpr::Dimension { .. } => false,
            CalcExpr::Binary { lhs, rhs, .. } => lhs.contains_percent() || rhs.contains_percent(),
            CalcExpr::Function { args, .. } => args.iter().any(CalcExpr::contains_percent),
            // Layout resolves it: never folded at parse time.
            CalcExpr::Anchor(_) => true,
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

/// A top-level result in whole cells (CSS Values 4 §10.9): NaN is 0,
/// anything else clamps to `±i32::MAX` — a symmetric range, so the
/// result negates without overflow — and rounds half to even.
pub fn to_cells(v: f64) -> i32 {
    if v.is_nan() {
        0
    } else {
        let max = f64::from(i32::MAX);
        v.clamp(-max, max).round_ties_even() as i32
    }
}

/// `v` floored onto the grid — the used value of a length that cannot draw
/// a part of a cell (`line-height`, `letter-spacing`, `word-spacing`) — a
/// value within a millionth below a whole cell taking that cell, so `f32`
/// and math-function arithmetic (`calc(3 * (1 / 3))`) does not lose one.
pub(crate) fn floor_cells(v: f64) -> f64 {
    (v + 1e-6).floor()
}

#[cfg(test)]
mod semantics_tests;
#[cfg(test)]
mod tests;
