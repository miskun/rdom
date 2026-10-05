//! The type of a math expression (CSS Values 4 §10.9 "type checking"):
//! what a calculation resolves to, so a property can reject one its
//! grammar does not take (an `<angle>` in `width`), and an operation can
//! reject operands it cannot combine (a length times a length).
//!
//! rdom's bare number doubles as its cell (DIVERGENCES §1), so a
//! `<number>` combines with a `<length>` wherever CSS would need a
//! length — `calc(50% - 4)` — and is itself a valid length. A
//! percentage has its own type, «percent» ([`CalcKind::Percent`]).

use super::{CalcExpr, CalcOp, MathFunction};

/// What a calculation resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CalcKind {
    /// `<number>` (in a length property, cells).
    Number,
    /// `<length>` or `<length-percentage>`: cells, percentages.
    Length,
    /// `<angle>` (radians inside the evaluator).
    Angle,
    /// `<percentage>`: a calculation of percentages alone.
    Percent,
}

impl CalcKind {
    /// The kind of a sum, or of arguments that must agree (`min()`,
    /// `hypot()`, …): a percentage goes with a length (a
    /// `<length-percentage>`), and so does a number under `typing.cells`
    /// (rdom's number is a cell); an angle only with an angle.
    fn unify(self, other: CalcKind, typing: Typing) -> Option<CalcKind> {
        use CalcKind::*;
        match (self, other) {
            (a, b) if a == b => Some(a),
            (Percent, Length) | (Length, Percent) => Some(Length),
            (Number, Length | Percent) | (Length | Percent, Number) if typing.cells => Some(Length),
            _ => None,
        }
    }

    /// `true` for a value a length property takes (a
    /// `<length-percentage>`).
    pub fn is_length(self) -> bool {
        matches!(
            self,
            CalcKind::Number | CalcKind::Length | CalcKind::Percent
        )
    }
}

/// How an expression is typed: what a percentage is, and whether a
/// number is a length (rdom's cell) or only a factor (CSS's own rule).
#[derive(Debug, Clone, Copy)]
struct Typing {
    percent: CalcKind,
    cells: bool,
}

impl CalcExpr {
    /// The expression's type, `None` when it does not type-check (CSS
    /// Values 4 §10.9): a sum of an angle and a length, a product of two
    /// typed values, a division by a typed value, a function argument
    /// of the wrong type. A percentage is «percent» — a calculation of
    /// percentages alone is [`CalcKind::Percent`] — and joins a length
    /// or a number as a [`CalcKind::Length`] (the percentage resolves
    /// against a length; rdom's number is a cell).
    pub fn kind(&self) -> Option<CalcKind> {
        self.kind_with(Typing {
            percent: CalcKind::Percent,
            cells: true,
        })
    }

    /// The expression's type under CSS's own rule (§10.9), where a
    /// `<number>` is a factor and never joins a `<length>` — for a math
    /// function whose lengths are not rdom's cells: the pixel lengths of
    /// a border width, radius or shadow offset (C4G-PX-CALC), where
    /// `calc(2px + 1)` has no meaning.
    pub fn kind_strict(&self) -> Option<CalcKind> {
        self.kind_with(Typing {
            percent: CalcKind::Percent,
            cells: false,
        })
    }

    /// The expression's type where percentages are numbers — `opacity`
    /// (CSS Color 4 §11.1: `<number> | <percentage>`, 50% is 0.5), whose
    /// `min(1, 50%)` is a `<number>`.
    pub fn kind_as_number(&self) -> Option<CalcKind> {
        self.kind_with(Typing {
            percent: CalcKind::Number,
            cells: true,
        })
    }

    /// The type under `typing`.
    fn kind_with(&self, typing: Typing) -> Option<CalcKind> {
        use CalcKind::*;
        match self {
            CalcExpr::Number(_) | CalcExpr::NoBound => Some(Number),
            CalcExpr::Length(_) => Some(Length),
            CalcExpr::Percent(_) => Some(typing.percent),
            CalcExpr::Dimension { unit, .. } => Some(unit.kind()),
            CalcExpr::Binary { op, lhs, rhs } => {
                let (l, r) = (lhs.kind_with(typing)?, rhs.kind_with(typing)?);
                match op {
                    CalcOp::Add | CalcOp::Sub => l.unify(r, typing),
                    CalcOp::Mul => match (l, r) {
                        (Number, k) | (k, Number) => Some(k),
                        _ => None,
                    },
                    CalcOp::Div => (r == Number).then_some(l),
                }
            }
            CalcExpr::Function { func, args } => function_kind(*func, args, typing),
        }
    }
}

/// A function's result type from its arguments' (CSS Values 4 §10.2 –
/// §10.7).
fn function_kind(func: MathFunction, args: &[CalcExpr], typing: Typing) -> Option<CalcKind> {
    use CalcKind::*;
    let kinds = args
        .iter()
        .filter(|a| !matches!(a, CalcExpr::NoBound))
        .map(|a| a.kind_with(typing))
        .collect::<Option<Vec<_>>>()?;
    let agreed = || {
        kinds
            .iter()
            .try_fold(None, |acc: Option<CalcKind>, k| match acc {
                None => Some(Some(*k)),
                Some(a) => a.unify(*k, typing).map(Some),
            })
            .flatten()
    };
    let numbers = || kinds.iter().all(|k| *k == Number);
    match func {
        MathFunction::Min
        | MathFunction::Max
        | MathFunction::Clamp
        | MathFunction::Round(_)
        | MathFunction::Mod
        | MathFunction::Rem
        | MathFunction::Abs
        | MathFunction::Hypot => agreed(),
        MathFunction::Sign => agreed().map(|_| Number),
        MathFunction::Sin | MathFunction::Cos | MathFunction::Tan => {
            matches!(kinds[..], [Number | Angle]).then_some(Number)
        }
        MathFunction::Asin | MathFunction::Acos | MathFunction::Atan => numbers().then_some(Angle),
        MathFunction::Atan2 => agreed().map(|_| Angle),
        MathFunction::Pow | MathFunction::Sqrt | MathFunction::Log | MathFunction::Exp => {
            numbers().then_some(Number)
        }
    }
}
