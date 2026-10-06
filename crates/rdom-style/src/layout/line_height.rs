//! `line-height` (CSS Inline 3 §5.1, CSS 2.1 §10.8.1): how tall an
//! inline box is, in whole rows on a terminal grid.

use crate::calc::{CalcExpr, ResolveCtx, UnitContext, to_cells};

/// `line-height: normal | <number [0,∞]> | <length-percentage [0,∞]>`
/// (CSS Inline 3 §5.1). Inherited; initial `normal`.
///
/// The font is one row tall, so `normal` and `1` are one row, a number
/// is that many rows and a percentage that share of one row. A number
/// inherits as the number, a length or percentage as its computed rows
/// (§5.1 "Computed value: the keyword normal or a number or an absolute
/// length"). [`rows`](Self::rows) is the used line height.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum LineHeight {
    /// `normal`: the font's own height, one row.
    #[default]
    Normal,
    /// `<number>`: the font size (one row) times the number.
    Number(f32),
    /// A length in rows: a specified length known at parse time
    /// (serialized in `ch`, rdom's cell), or any length or percentage
    /// once computed.
    Rows(f32),
    /// A percentage of the font size, or a length in a unit that needs
    /// the context (`lh`, `rlh`, a viewport unit), as specified; it
    /// computes to [`Rows`](Self::Rows).
    Calc(Box<CalcExpr>),
}

impl LineHeight {
    /// The computed value (CSS Inline 3 §5.1): a percentage of the font
    /// size (one row) and a length in context units resolved against
    /// `cx` — whose `lh` is the parent's line height (CSS Values 4 §6.1.1:
    /// "when specified in the line-height property itself, refer to the
    /// parent's") — to rows; `normal` and a number unchanged.
    pub fn computed(&self, cx: &UnitContext) -> LineHeight {
        match self {
            LineHeight::Calc(expr) => {
                let rows = expr.absolutize_in(cx).resolve_f64(&ResolveCtx::new(1));
                LineHeight::Rows(if rows.is_finite() { rows.max(0.0) } else { 0.0 } as f32)
            }
            other => other.clone(),
        }
    }

    /// The used line height in whole rows: the value rounded onto the
    /// grid (ties to even, as every fractional length rounds —
    /// DIVERGENCES §1), at least one — the glyph's own row (DIVERGENCES
    /// §2: a line height below one row would overlap the lines).
    pub fn rows(&self) -> u16 {
        let value = match self {
            LineHeight::Normal => 1.0,
            LineHeight::Number(n) | LineHeight::Rows(n) => f64::from(*n),
            LineHeight::Calc(expr) => expr.resolve_f64(&ResolveCtx::new(1)),
        };
        to_cells(value).clamp(1, i32::from(u16::MAX)) as u16
    }

    /// Rows of leading above and below the glyph row of an inline box
    /// this tall (CSS 2.1 §10.8.1: half the leading above, half below):
    /// the odd row of an odd leading goes below (DIVERGENCES §2).
    pub fn half_leading(&self) -> (u16, u16) {
        let leading = self.rows() - 1;
        (leading / 2, leading - leading / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{CalcUnit, Viewport};

    /// CSS Inline 3 §5.1 on a one-row font: `normal` and `1` are one row,
    /// numbers and lengths round onto the grid, nothing is below one row.
    #[test]
    fn rows_round_onto_the_grid_at_least_one() {
        assert_eq!(LineHeight::Normal.rows(), 1);
        assert_eq!(LineHeight::Number(1.0).rows(), 1);
        assert_eq!(LineHeight::Number(1.5).rows(), 2);
        assert_eq!(LineHeight::Number(2.5).rows(), 2);
        assert_eq!(LineHeight::Rows(3.0).rows(), 3);
        assert_eq!(LineHeight::Number(0.0).rows(), 1);
        assert_eq!(
            LineHeight::Calc(Box::new(CalcExpr::Percent(200.0))).rows(),
            2
        );
    }

    /// CSS 2.1 §10.8.1: half the leading above the glyph row and half
    /// below, the odd row below.
    #[test]
    fn half_leading_splits_the_extra_rows() {
        assert_eq!(LineHeight::Normal.half_leading(), (0, 0));
        assert_eq!(LineHeight::Number(2.0).half_leading(), (0, 1));
        assert_eq!(LineHeight::Number(3.0).half_leading(), (1, 1));
        assert_eq!(LineHeight::Number(4.0).half_leading(), (1, 2));
    }

    /// CSS Values 4 §6.1.1: `lh` in `line-height` is the parent's line
    /// height; a percentage is of the font size, one row.
    #[test]
    fn a_context_length_computes_to_rows() {
        let cx = UnitContext::new(Viewport::new(80, 20)).with_line_heights(2.0, 3.0);
        let lh = |unit| {
            LineHeight::Calc(Box::new(CalcExpr::Dimension { value: 2.0, unit })).computed(&cx)
        };
        assert_eq!(lh(CalcUnit::Lh), LineHeight::Rows(4.0));
        assert_eq!(lh(CalcUnit::Rlh), LineHeight::Rows(6.0));
        assert_eq!(
            LineHeight::Calc(Box::new(CalcExpr::Percent(150.0))).computed(&cx),
            LineHeight::Rows(1.5)
        );
        assert_eq!(
            LineHeight::Number(1.5).computed(&cx),
            LineHeight::Number(1.5)
        );
    }
}
