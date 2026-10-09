//! `vertical-align` (CSS 2.1 §10.8.1, CSS Inline 3 §4): where an inline
//! box sits in its line, in whole rows on a terminal grid.

use crate::calc::{CalcExpr, ResolveCtx, UnitContext, UnitReads, to_cells};

/// `vertical-align: baseline | sub | super | text-top | text-bottom |
/// middle | top | bottom | <length-percentage>` (CSS 2.1 §10.8.1). Not
/// inherited; initial `baseline`. Applies to inline-level boxes (and to
/// table cells, whose alignment is C13-TABLE-PROPS).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum VerticalAlign {
    /// The box's baseline on its parent's.
    #[default]
    Baseline,
    /// Lowered to the parent's subscript position: one row.
    Sub,
    /// Raised to the parent's superscript position: one row.
    Super,
    /// The box's top on the parent's content area's top (its glyph row).
    TextTop,
    /// The box's bottom on the parent's content area's bottom.
    TextBottom,
    /// The box's middle row on the parent's baseline row.
    Middle,
    /// The box's aligned subtree's top on the line box's top.
    Top,
    /// The box's aligned subtree's bottom on the line box's bottom.
    Bottom,
    /// Raised by this many rows (a negative value lowers): a length, or
    /// any length or percentage once computed.
    Rows(f32),
    /// A percentage of the element's line height, or a length in a unit
    /// that needs the context, as specified; it computes to
    /// [`Rows`](Self::Rows).
    Calc(std::sync::Arc<CalcExpr>),
}

impl VerticalAlign {
    /// A `calc()` value (CSS Values 4 §10): the expression behind an `Arc`,
    /// shared by every style holding the value — an inherited or copied
    /// one clones without allocating.
    pub fn calc(expr: crate::calc::CalcExpr) -> Self {
        VerticalAlign::Calc(std::sync::Arc::new(expr))
    }
}

impl VerticalAlign {
    /// The computed value: a percentage — "refer to the line-height of
    /// the element itself" (CSS 2.1 §10.8.1) — against `line_height` rows,
    /// a context unit against `cx`; every other value unchanged. Returns
    /// the context sizes that read beside it.
    pub fn computed(&self, line_height: u16, cx: &UnitContext) -> (VerticalAlign, UnitReads) {
        match self {
            VerticalAlign::Calc(expr) => {
                let (expr, reads) = expr.absolutize_in(cx);
                let rows = expr.resolve_f64(&ResolveCtx::new(i32::from(line_height)));
                let rows = if rows.is_finite() { rows as f32 } else { 0.0 };
                (VerticalAlign::Rows(rows), reads)
            }
            other => (other.clone(), UnitReads::NONE),
        }
    }

    /// The rows a length or percentage raises the box by, on the grid
    /// (ties to even); `None` for a keyword.
    pub fn raise(&self) -> Option<i32> {
        match self {
            VerticalAlign::Rows(n) => Some(to_cells(f64::from(*n))),
            VerticalAlign::Calc(expr) => Some(expr.resolve(&ResolveCtx::new(1))),
            _ => None,
        }
    }

    /// The keyword's CSS spelling; `None` for a length.
    pub const fn keyword(&self) -> Option<&'static str> {
        Some(match self {
            VerticalAlign::Baseline => "baseline",
            VerticalAlign::Sub => "sub",
            VerticalAlign::Super => "super",
            VerticalAlign::TextTop => "text-top",
            VerticalAlign::TextBottom => "text-bottom",
            VerticalAlign::Middle => "middle",
            VerticalAlign::Top => "top",
            VerticalAlign::Bottom => "bottom",
            VerticalAlign::Rows(_) | VerticalAlign::Calc(_) => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::Viewport;

    /// CSS 2.1 §10.8.1: a percentage is of the element's own line height.
    #[test]
    fn a_percentage_computes_against_the_line_height() {
        let cx = UnitContext::new(Viewport::new(80, 20));
        let half = VerticalAlign::calc(CalcExpr::Percent(50.0));
        assert_eq!(half.computed(4, &cx).0, VerticalAlign::Rows(2.0));
        assert_eq!(half.computed(4, &cx).0.raise(), Some(2));
        assert_eq!(VerticalAlign::Rows(-1.0).raise(), Some(-1));
        assert_eq!(VerticalAlign::Super.raise(), None);
        assert_eq!(VerticalAlign::TextTop.keyword(), Some("text-top"));
    }
}
