//! `letter-spacing` and `word-spacing` (CSS Text 3 §9.1, §9.2): blank
//! cells added after a typographic character unit, on a terminal grid in
//! whole cells.

use crate::calc::CalcExpr;

/// `letter-spacing` / `word-spacing: normal | <length>` (CSS Text 3 §9.2,
/// §9.1). Inherited; initial `normal`. A length is in cells — a bare number,
/// `ch`, or a viewport or line-height unit, which computes to cells; a
/// pixel or font-relative length is invalid (DESIGN "Pixel lengths select,
/// cells measure": spacing is geometry). [`cells`](Self::cells) is the
/// used spacing.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Spacing {
    /// No additional spacing.
    #[default]
    Normal,
    /// A length in cells: one known at parse time (serialized in `ch`,
    /// rdom's cell), or any length once computed.
    Cells(f32),
    /// A length in a unit that needs the context (a viewport unit, `lh`,
    /// `rlh`), as specified; it computes to [`Cells`](Self::Cells)
    /// (`ComputedStyle::resolve_context_units`).
    Calc(Box<CalcExpr>),
}

impl Spacing {
    /// The used spacing in whole cells: the length floored onto the grid,
    /// as a fractional `line-height` is (a part of a cell cannot be drawn),
    /// and never negative — glyphs cannot be drawn closer than a cell
    /// apart (DIVERGENCES §1). `normal` is none.
    pub fn cells(&self) -> u16 {
        let value = match self {
            Spacing::Normal => return 0,
            Spacing::Cells(n) => f64::from(*n),
            Spacing::Calc(expr) => expr.resolve_f64(&crate::calc::ResolveCtx::new(0)),
        };
        if value.is_nan() {
            return 0;
        }
        crate::calc::floor_cells(value).clamp(0.0, f64::from(u16::MAX)) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whole cells: floored, nothing negative, `normal` none.
    #[test]
    fn spacing_is_whole_non_negative_cells() {
        assert_eq!(Spacing::Normal.cells(), 0);
        assert_eq!(Spacing::Cells(1.0).cells(), 1);
        assert_eq!(Spacing::Cells(1.9).cells(), 1);
        assert_eq!(Spacing::Cells(0.5).cells(), 0);
        assert_eq!(Spacing::Cells(-2.0).cells(), 0);
        assert_eq!(Spacing::Cells(f32::NAN).cells(), 0);
        assert_eq!(Spacing::Cells(3.0 - 1e-7).cells(), 3);
    }
}
