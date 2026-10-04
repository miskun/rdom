//! Units a math expression or a length value can carry besides rdom's
//! unitless cell and the percentage (CSS Values 4 §6 – §7), each with its
//! terminal meaning.

use super::{CalcKind, ResolveCtx};

/// A dimension's unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CalcUnit {
    /// `ch` — the advance of "0": one column on a monospaced grid
    /// (Values 4 §6.1.1).
    Ch,
    /// `lh` — the element's line height: one row, as rdom's line is
    /// until `line-height` lands (C9-LINE-HEIGHT) (Values 4 §6.1.1).
    Lh,
    /// `rlh` — the root's line height: one row (Values 4 §6.1.1).
    Rlh,
}

impl CalcUnit {
    /// The unit of a dimension token, ASCII case-insensitive (CSS
    /// Values 4 §6: unit identifiers are case-insensitive). `None` for a
    /// unit rdom does not take (`px`, `em`, … — DIVERGENCES §1).
    pub fn parse(unit: &str) -> Option<CalcUnit> {
        const TABLE: &[(&str, CalcUnit)] = &[
            ("ch", CalcUnit::Ch),
            ("lh", CalcUnit::Lh),
            ("rlh", CalcUnit::Rlh),
        ];
        TABLE
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(unit))
            .map(|(_, u)| *u)
    }

    /// The unit's CSS spelling.
    pub fn css_name(self) -> &'static str {
        match self {
            CalcUnit::Ch => "ch",
            CalcUnit::Lh => "lh",
            CalcUnit::Rlh => "rlh",
        }
    }

    /// The type of a value in this unit.
    pub fn kind(self) -> CalcKind {
        match self {
            CalcUnit::Ch | CalcUnit::Lh | CalcUnit::Rlh => CalcKind::Length,
        }
    }

    /// `true` when the value depends on something only known after
    /// parsing (the viewport); such a value stays symbolic until then.
    pub fn needs_context(self) -> bool {
        match self {
            CalcUnit::Ch | CalcUnit::Lh | CalcUnit::Rlh => false,
        }
    }

    /// `value` in this unit, in the evaluator's canonical unit — cells
    /// for lengths.
    pub(super) fn canonical(self, value: f64, _cx: &ResolveCtx) -> f64 {
        match self {
            // One column; one row (the fixed line height) — a cell
            // either way.
            CalcUnit::Ch | CalcUnit::Lh | CalcUnit::Rlh => value,
        }
    }
}
