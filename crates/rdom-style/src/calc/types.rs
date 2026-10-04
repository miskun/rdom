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
    /// `hypot()`, …): a number or a percentage goes with a length (a
    /// `<length-percentage>`; rdom's number is a cell); an angle only
    /// with an angle.
    fn unify(self, other: CalcKind) -> Option<CalcKind> {
        use CalcKind::*;
        match (self, other) {
            (a, b) if a == b => Some(a),
            (Number | Percent, Length)
            | (Length, Number | Percent)
            | (Number, Percent)
            | (Percent, Number) => Some(Length),
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

impl CalcExpr {
    /// The expression's type, `None` when it does not type-check (CSS
    /// Values 4 §10.9): a sum of an angle and a length, a product of two
    /// typed values, a division by a typed value, a function argument
    /// of the wrong type. A percentage is «percent» — a calculation of
    /// percentages alone is [`CalcKind::Percent`] — and joins a length
    /// or a number as a [`CalcKind::Length`] (the percentage resolves
    /// against a length; rdom's number is a cell).
    pub fn kind(&self) -> Option<CalcKind> {
        self.kind_with(CalcKind::Percent)
    }

    /// The expression's type where percentages are numbers — `opacity`
    /// (CSS Color 4 §11.1: `<number> | <percentage>`, 50% is 0.5), whose
    /// `min(1, 50%)` is a `<number>`.
    pub fn kind_as_number(&self) -> Option<CalcKind> {
        self.kind_with(CalcKind::Number)
    }

    /// The type, with a percentage typed `percent`.
    fn kind_with(&self, percent: CalcKind) -> Option<CalcKind> {
        use CalcKind::*;
        match self {
            CalcExpr::Number(_) | CalcExpr::NoBound => Some(Number),
            CalcExpr::Length(_) => Some(Length),
            CalcExpr::Percent(_) => Some(percent),
            CalcExpr::Dimension { unit, .. } => Some(unit.kind()),
            CalcExpr::Binary { op, lhs, rhs } => {
                let (l, r) = (lhs.kind_with(percent)?, rhs.kind_with(percent)?);
                match op {
                    CalcOp::Add | CalcOp::Sub => l.unify(r),
                    CalcOp::Mul => match (l, r) {
                        (Number, k) | (k, Number) => Some(k),
                        _ => None,
                    },
                    CalcOp::Div => (r == Number).then_some(l),
                }
            }
            CalcExpr::Function { func, args } => function_kind(*func, args, percent),
        }
    }
}

/// A function's result type from its arguments' (CSS Values 4 §10.2 –
/// §10.7).
fn function_kind(func: MathFunction, args: &[CalcExpr], percent: CalcKind) -> Option<CalcKind> {
    use CalcKind::*;
    let kinds = args
        .iter()
        .filter(|a| !matches!(a, CalcExpr::NoBound))
        .map(|a| a.kind_with(percent))
        .collect::<Option<Vec<_>>>()?;
    let agreed = || {
        kinds
            .iter()
            .try_fold(None, |acc: Option<CalcKind>, k| match acc {
                None => Some(Some(*k)),
                Some(a) => a.unify(*k).map(Some),
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
