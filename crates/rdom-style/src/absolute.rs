//! Computed values made absolute: the viewport-percentage lengths of a
//! computed style resolved against the terminal (CSS Values 4 §6.1.2 —
//! they are absolute lengths at computed-value time). Percentages stay
//! for layout, which alone knows their basis; an expression left with
//! none folds to whole cells.

use crate::ComputedStyle;
use crate::calc::{CalcExpr, Viewport};
use crate::layout::{
    BorderWidth, FlexBasis, GapValue, Length, MarginValue, MaxSize, MinSize, PaddingValue,
    PaintLength, Size,
};

impl ComputedStyle {
    /// Resolve every viewport-percentage length in the style's
    /// length-bearing values against `viewport`. The cascade calls this
    /// once per computed style; values without one are untouched.
    pub fn resolve_viewport_units(&mut self, viewport: Viewport) {
        let vp = viewport;
        absolutize(&mut self.width, vp, Size::Calc, |v| {
            Size::Fixed(cells_u16(v))
        });
        absolutize(&mut self.height, vp, Size::Calc, |v| {
            Size::Fixed(cells_u16(v))
        });
        for min in [&mut self.min_width, &mut self.min_height] {
            absolutize(min, vp, MinSize::Calc, |v| MinSize::Cells(cells_u16(v)));
        }
        for max in [&mut self.max_width, &mut self.max_height] {
            absolutize(max, vp, MaxSize::Calc, |v| MaxSize::Cells(cells_u16(v)));
        }
        let p = &mut self.padding;
        for side in [&mut p.top, &mut p.right, &mut p.bottom, &mut p.left] {
            absolutize(side, vp, PaddingValue::Calc, |v| {
                PaddingValue::Cells(cells_u16(v))
            });
        }
        let m = &mut self.margin;
        for side in [&mut m.top, &mut m.right, &mut m.bottom, &mut m.left] {
            absolutize(side, vp, MarginValue::Calc, |v| {
                MarginValue::Cells(
                    cells_i32(v).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
                )
            });
        }
        absolutize(&mut self.gap, vp, GapValue::Calc, |v| {
            GapValue::Cells(cells_u16(v))
        });
        absolutize(&mut self.flex_basis, vp, FlexBasis::Calc, |v| {
            FlexBasis::Cells(cells_u16(v))
        });
        for width in self.border_width.each_mut() {
            absolutize(
                width,
                vp,
                |e| BorderWidth::Length(PaintLength::Calc(e)),
                |v| BorderWidth::Length(PaintLength::Cells(v as f32)),
            );
        }
        for shadow in &mut self.box_shadow {
            for length in [
                &mut shadow.offset_x,
                &mut shadow.offset_y,
                &mut shadow.blur,
                &mut shadow.spread,
            ] {
                absolutize(length, vp, PaintLength::Calc, |v| {
                    PaintLength::Cells(v as f32)
                });
            }
        }
        for radius in self.border_radius.each_mut() {
            for axis in [&mut radius.horizontal, &mut radius.vertical] {
                absolutize(axis, vp, PaintLength::Calc, |v| {
                    PaintLength::Cells(v as f32)
                });
            }
        }
        for inset in [
            &mut self.top,
            &mut self.right,
            &mut self.bottom,
            &mut self.left,
        ] {
            absolutize(inset, vp, Length::Calc, |v| Length::Cells(cells_i32(v)));
        }
    }
}

/// A value that may hold a math expression.
trait HasExpr {
    fn expr(&self) -> Option<&CalcExpr>;
}

macro_rules! has_expr {
    ($($t:ident),*) => {$(
        impl HasExpr for $t {
            fn expr(&self) -> Option<&CalcExpr> {
                match self {
                    $t::Calc(e) => Some(e),
                    _ => None,
                }
            }
        }
    )*};
}
has_expr!(
    Size,
    FlexBasis,
    MinSize,
    MaxSize,
    PaddingValue,
    MarginValue,
    GapValue,
    Length
);

impl HasExpr for PaintLength {
    fn expr(&self) -> Option<&CalcExpr> {
        match self {
            PaintLength::Calc(e) => Some(e),
            _ => None,
        }
    }
}

impl HasExpr for BorderWidth {
    fn expr(&self) -> Option<&CalcExpr> {
        match self {
            BorderWidth::Length(PaintLength::Calc(e)) => Some(e),
            _ => None,
        }
    }
}

/// Replace `value`'s expression, when it has a viewport unit, by the
/// absolute one: `calc` keeps a percent-bearing expression, `fixed`
/// takes the cells of one left without a percentage.
fn absolutize<T: HasExpr>(
    value: &mut T,
    viewport: Viewport,
    calc: fn(Box<CalcExpr>) -> T,
    fixed: impl FnOnce(f64) -> T,
) {
    let Some(expr) = value.expr().filter(|e| e.needs_context()) else {
        return;
    };
    let expr = expr.absolutize(viewport);
    *value = if expr.contains_percent() {
        calc(Box::new(expr))
    } else {
        fixed(expr.resolve_f64(&crate::calc::ResolveCtx::new(0)))
    };
}

/// Cells rounded onto the grid (ties to even, as `calc()` rounds),
/// clamped to `u16`; NaN is 0.
pub(crate) fn cells_u16(v: f64) -> u16 {
    cells_i32(v).clamp(0, i32::from(u16::MAX)) as u16
}

/// Cells rounded onto the grid; NaN is 0, infinities clamp to
/// `±i32::MAX` (CSS Values 4 §10.9).
pub(crate) fn cells_i32(v: f64) -> i32 {
    crate::calc::to_cells(v)
}
