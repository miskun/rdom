//! The pieces `resolve_context_units` works with: the context a
//! resolution reads ([`Reading`]), the values that hold a math expression
//! ([`HasExpr`]) and their rewrite ([`absolutize`]), the list-valued
//! properties' walks, and the cell conversions.

use crate::calc::{CalcExpr, UnitContext, UnitReads};
use crate::layout::{
    BorderWidth, FlexBasis, GapValue, Length, MarginValue, MaxSize, MinSize, PaddingValue,
    PaintLength, Size,
};

/// A [`UnitContext`] and what the values resolved against it so far read.
pub(super) struct Reading<'a> {
    cx: &'a UnitContext,
    pub(super) reads: std::cell::Cell<UnitReads>,
}

impl<'a> Reading<'a> {
    pub(super) fn new(cx: &'a UnitContext) -> Self {
        Reading {
            cx,
            reads: std::cell::Cell::new(UnitReads::NONE),
        }
    }

    /// `expr` made absolute ([`CalcExpr::absolutize_in`]), its reads kept.
    pub(super) fn absolutize(&self, expr: &CalcExpr) -> CalcExpr {
        let (expr, reads) = expr.absolutize_in(self.cx);
        self.reads.set(self.reads.get() | reads);
        expr
    }
}

/// Every breadth of a track list, its viewport units absolute: a
/// percentage-bearing `calc()` stays one, any other is whole cells.
#[deny(clippy::wildcard_enum_match_arm)]
pub(super) fn absolutize_template(template: &mut crate::layout::GridTemplate, vp: &Reading<'_>) {
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

/// A translation's x and y offsets.
pub(super) fn absolutize_translate(t: &mut crate::layout::Translate, vp: &Reading<'_>) {
    for offset in [&mut t.x, &mut t.y] {
        absolutize(offset, vp, Length::Calc, |v| Length::Cells(cells_i32(v)));
    }
}

/// The translate functions of a `transform` list, the list rebuilt only
/// when one holds a math expression.
/// Whether a `transform` list has a `calc()` translation offset — what
/// [`absolutize_transform`] rewrites.
pub(super) fn transform_has_calc(list: &crate::layout::TransformList) -> bool {
    use crate::layout::TransformFunction;
    let calc = |l: &Length| matches!(l, Length::Calc(_));
    list.functions().iter().any(|f| {
        matches!(f, TransformFunction::Translate { offset, .. } if calc(&offset.x) || calc(&offset.y))
    })
}

pub(super) fn absolutize_transform(list: &mut crate::layout::TransformList, vp: &Reading<'_>) {
    use crate::layout::{TransformFunction, TransformList};
    if !transform_has_calc(list) {
        return;
    }
    let mut functions = list.functions().to_vec();
    for f in &mut functions {
        if let TransformFunction::Translate { offset, .. } = f {
            absolutize_translate(offset, vp);
        }
    }
    *list = TransformList::new(functions);
}

/// [`absolutize_sizes`] for a `grid-auto-*` list, made owned only when it
/// holds a math expression (the initial, borrowed `auto` never does).
pub(super) fn absolutize_list(
    list: &mut std::borrow::Cow<'static, [crate::layout::TrackSize]>,
    vp: &Reading<'_>,
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
    vp: &Reading<'_>,
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
pub(super) trait HasExpr {
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

impl HasExpr for crate::layout::ColumnWidth {
    fn expr(&self) -> Option<&CalcExpr> {
        match self {
            crate::layout::ColumnWidth::Calc(e) => Some(e),
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
/// Whether `value` holds an expression [`absolutize`] would rewrite.
pub(super) fn needs<T: HasExpr>(value: &T) -> bool {
    value.expr().is_some_and(|e| e.needs_context())
}

pub(super) fn absolutize<T: HasExpr>(
    value: &mut T,
    cx: &Reading<'_>,
    calc: fn(std::sync::Arc<CalcExpr>) -> T,
    fixed: impl FnOnce(f64) -> T,
) {
    let Some(expr) = value.expr().filter(|e| e.needs_context()) else {
        return;
    };
    let expr = cx.absolutize(expr);
    *value = if expr.contains_percent() {
        calc(std::sync::Arc::new(expr))
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
