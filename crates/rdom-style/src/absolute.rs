//! Computed values made absolute: the viewport-percentage lengths of a
//! computed style resolved against the terminal (CSS Values 4 §6.1.2 —
//! they are absolute lengths at computed-value time). Percentages stay
//! for layout, which alone knows their basis; an expression left with
//! none folds to whole cells.

use crate::ComputedStyle;
use crate::calc::{CalcExpr, Viewport};
use crate::layout::{
    BorderWidth, FlexBasis, GapValue, IntrinsicSize, Length, MarginValue, MaxSize, MinSize,
    PaddingValue, PaintLength, Size,
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
        // A `fit-content()` limit stays an expression (layout resolves it
        // against the containing block), with its viewport units absolute.
        let limits = [
            match &mut self.width {
                Size::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.height {
                Size::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.min_width {
                MinSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.min_height {
                MinSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.max_width {
                MaxSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.max_height {
                MaxSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.flex_basis {
                FlexBasis::Intrinsic(k) => Some(k),
                _ => None,
            },
        ];
        for size in [
            &mut self.contain_intrinsic_width,
            &mut self.contain_intrinsic_height,
        ] {
            if let Some(expr) = size.length.as_mut().filter(|e| e.needs_context()) {
                let absolute = expr.absolutize(vp);
                *expr = CalcExpr::Length(cells_i32(
                    absolute.resolve_f64(&crate::calc::ResolveCtx::new(0)),
                ));
            }
        }
        for limit in limits.into_iter().flatten() {
            if let IntrinsicSize::FitContentLimit(expr) = limit
                && expr.needs_context()
            {
                **expr = expr.absolutize(vp);
            }
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
        for gap in [
            &mut self.row_gap,
            &mut self.column_gap,
            &mut self.border_spacing.horizontal,
            &mut self.border_spacing.vertical,
        ] {
            absolutize(gap, vp, GapValue::Calc, |v| GapValue::Cells(cells_u16(v)));
        }
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
        for template in [
            &mut self.grid_template_columns,
            &mut self.grid_template_rows,
        ] {
            absolutize_template(template, vp);
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

/// Every breadth of a track list, its viewport units absolute: a
/// percentage-bearing `calc()` stays one, any other is whole cells.
fn absolutize_template(template: &mut crate::layout::GridTemplate, vp: Viewport) {
    use crate::layout::{TrackBreadth, TrackListItem, TrackSize};
    let Some(list) = (match template {
        crate::layout::GridTemplate::Tracks(list) => Some(list),
        _ => None,
    }) else {
        return;
    };
    let sizes = list.items.iter_mut().flat_map(|item| match item {
        TrackListItem::Size(s) => std::slice::from_mut(s).iter_mut(),
        TrackListItem::Repeat(r) => r.sizes.iter_mut(),
    });
    for size in sizes {
        let breadths: [Option<&mut TrackBreadth>; 2] = match size {
            TrackSize::Breadth(b) | TrackSize::FitContent(b) => [Some(b), None],
            TrackSize::MinMax(min, max) => [Some(min), Some(max)],
        };
        for b in breadths.into_iter().flatten() {
            absolutize(b, vp, TrackBreadth::Calc, |v| {
                TrackBreadth::Cells(cells_u16(v))
            });
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

impl HasExpr for crate::layout::TrackBreadth {
    fn expr(&self) -> Option<&CalcExpr> {
        match self {
            crate::layout::TrackBreadth::Calc(e) => Some(e),
            _ => None,
        }
    }
}

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
