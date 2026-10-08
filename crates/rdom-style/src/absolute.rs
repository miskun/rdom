//! Computed values made absolute: the viewport-percentage lengths of a
//! computed style resolved against the terminal (CSS Values 4 §6.1.2),
//! and the line-height units (`lh`, `rlh`, §6.1.1) against the line
//! heights — they are absolute lengths at computed-value time.
//! Percentages stay for layout, which alone knows their basis; an
//! expression left with none folds to whole cells.

use crate::ComputedStyle;
use crate::calc::{CalcExpr, UnitContext, Viewport};
use crate::layout::{
    BorderWidth, FlexBasis, GapValue, IntrinsicSize, Length, MarginValue, MaxSize, MinSize,
    PaddingValue, PaintLength, Size,
};

impl ComputedStyle {
    /// Resolve every viewport-percentage length in the style's
    /// length-bearing values against `viewport`, and the line-height
    /// units as one row each ([`Self::resolve_context_units`] with
    /// [`UnitContext::new`]).
    pub fn resolve_viewport_units(&mut self, viewport: Viewport) {
        self.resolve_context_units(&UnitContext::new(viewport));
    }

    /// Resolve every unit that needs a context — the viewport-percentage
    /// lengths, `lh` and `rlh` — in the style's length-bearing values
    /// against `cx`. The cascade calls this once per computed style, once
    /// `line-height` is computed (`lh` reads it); values without such a
    /// unit are untouched.
    pub fn resolve_context_units(&mut self, cx: &UnitContext) {
        let vp = cx;
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
                let absolute = expr.absolutize_in(vp);
                *expr = CalcExpr::Length(cells_i32(
                    absolute.resolve_f64(&crate::calc::ResolveCtx::new(0)),
                ));
            }
        }
        for limit in limits.into_iter().flatten() {
            if let IntrinsicSize::FitContentLimit(expr) = limit
                && expr.needs_context()
            {
                **expr = expr.absolutize_in(vp);
            }
        }
        let p = &mut self.padding;
        for side in [&mut p.top, &mut p.right, &mut p.bottom, &mut p.left] {
            absolutize(side, vp, PaddingValue::Calc, |v| {
                PaddingValue::Cells(cells_u16(v))
            });
        }
        let sp = &mut self.scroll_padding;
        for side in [&mut sp.top, &mut sp.right, &mut sp.bottom, &mut sp.left] {
            if let crate::layout::ScrollPadding::Length(v) = side {
                absolutize(v, vp, PaddingValue::Calc, |v| {
                    PaddingValue::Cells(cells_u16(v))
                });
            }
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
        for list in [&mut self.grid_auto_columns, &mut self.grid_auto_rows] {
            absolutize_list(list, vp);
        }
        for inset in [
            &mut self.top,
            &mut self.right,
            &mut self.bottom,
            &mut self.left,
        ] {
            absolutize(
                inset,
                vp,
                |e| Length::Calc(e.into()),
                |v| Length::Cells(cells_i32(v)),
            );
        }
        // CSS Text Decoration 4 §2.5 / §4.2: the decoration lengths
        // (parsed and kept, not drawn).
        if let crate::layout::TextDecorationThickness::Length(length) =
            &mut self.text_decoration.thickness
        {
            absolutize(length, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        if let crate::layout::TextUnderlineOffset::Length(length) =
            &mut self.text.text_underline_offset
        {
            absolutize(length, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        // CSS Fonts 4 §2.5: `font-size` (parsed and kept, not drawn).
        if let crate::layout::FontSize::Length(length) = &mut self.font.size {
            absolutize(length, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        // CSS Text 3 §9.1 / §9.2: the spacing lengths.
        for spacing in [&mut self.text.letter_spacing, &mut self.text.word_spacing] {
            absolutize(spacing, vp, crate::layout::Spacing::Calc, |v| {
                crate::layout::Spacing::Cells(v as f32)
            });
        }
        // CSS Text 3 §8.1: `text-indent` is a length-percentage.
        absolutize(
            &mut self.text.text_indent.length,
            vp,
            |e| Length::Calc(e.into()),
            |v| Length::Cells(cells_i32(v)),
        );
    }
}

/// Every breadth of a track list, its viewport units absolute: a
/// percentage-bearing `calc()` stays one, any other is whole cells.
#[deny(clippy::wildcard_enum_match_arm)]
fn absolutize_template(template: &mut crate::layout::GridTemplate, vp: &UnitContext) {
    use crate::layout::TrackListItem;
    let Some(list) = (match template {
        crate::layout::GridTemplate::Tracks(list) => Some(list),
        crate::layout::GridTemplate::None | crate::layout::GridTemplate::Subgrid(_) => None,
    }) else {
        return;
    };
    let sizes = list.items.iter_mut().flat_map(|item| match item {
        TrackListItem::Size(s) => std::slice::from_mut(s).iter_mut(),
        TrackListItem::Repeat(r) => r.sizes.iter_mut(),
    });
    absolutize_sizes(sizes, vp);
}

/// [`absolutize_sizes`] for a `grid-auto-*` list, made owned only when it
/// holds a math expression (the initial, borrowed `auto` never does).
fn absolutize_list(
    list: &mut std::borrow::Cow<'static, [crate::layout::TrackSize]>,
    vp: &UnitContext,
) {
    use crate::layout::{TrackBreadth, TrackSize};
    let calc = |b: &TrackBreadth| matches!(b, TrackBreadth::Calc(_));
    let has_calc = list.iter().any(|s| match s {
        TrackSize::Breadth(b) | TrackSize::FitContent(b) => calc(b),
        TrackSize::MinMax(min, max) => calc(min) || calc(max),
    });
    if has_calc {
        absolutize_sizes(list.to_mut().iter_mut(), vp);
    }
}

/// [`absolutize_template`] for each of `sizes`.
fn absolutize_sizes<'a>(
    sizes: impl Iterator<Item = &'a mut crate::layout::TrackSize>,
    vp: &UnitContext,
) {
    use crate::layout::{TrackBreadth, TrackSize};
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

impl HasExpr for crate::layout::Spacing {
    fn expr(&self) -> Option<&CalcExpr> {
        match self {
            crate::layout::Spacing::Calc(e) => Some(e),
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

/// Replace `value`'s expression, when it has a context unit, by the
/// absolute one: `calc` keeps a percent-bearing expression, `fixed`
/// takes the cells of one left without a percentage.
fn absolutize<T: HasExpr>(
    value: &mut T,
    cx: &UnitContext,
    calc: fn(Box<CalcExpr>) -> T,
    fixed: impl FnOnce(f64) -> T,
) {
    let Some(expr) = value.expr().filter(|e| e.needs_context()) else {
        return;
    };
    let expr = expr.absolutize_in(cx);
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
