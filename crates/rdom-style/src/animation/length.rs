//! `<length-percentage>` interpolation (CSS Values 4 §3.4.3) for the
//! length-bearing computed values: cells and percentages mix apart, any
//! other math function as `calc(from * (1 - p) + to * p)` — and the grid
//! track lists built of them (CSS Grid 2 §7.2.6).

use std::sync::Arc;

use super::value::discrete;
use super::value::{Animate, Cx, cells_u16, lerp};
use crate::calc::{CalcExpr, CalcOp, to_cells};
use crate::layout::{
    FlexBasis, GapValue, GridTemplate, Length, MarginValue, MaxSize, MinSize, PaddingValue,
    PaintLength, Size, TrackBreadth, TrackList, TrackListItem, TrackSize,
};

// ── Lengths and percentages ───────────────────────────────────────

/// A `<length-percentage>` mixed at `p` (CSS Values 4 §3.4.3): cells and
/// percentages interpolate apart when both values are linear in their
/// percentage; otherwise the result is the math function
/// `calc(from * (1 - p) + to * p)`, which layout resolves.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Mixed {
    Cells(f64),
    Percent(f64),
    Calc(CalcExpr),
}

pub(crate) fn mix(a: &CalcExpr, b: &CalcExpr, p: f64) -> Mixed {
    if let (Some((ac, ap)), Some((bc, bp))) = (a.linear_parts(), b.linear_parts()) {
        let (c, pc) = (lerp(ac, bc, p), lerp(ap, bp, p));
        return if pc == 0.0 {
            Mixed::Cells(c)
        } else if c == 0.0 {
            Mixed::Percent(pc)
        } else {
            Mixed::Calc(CalcExpr::binary(
                CalcOp::Add,
                CalcExpr::Number(c),
                CalcExpr::Percent(pc),
            ))
        };
    }
    let scaled =
        |e: &CalcExpr, k: f64| CalcExpr::binary(CalcOp::Mul, e.clone(), CalcExpr::Number(k));
    Mixed::Calc(CalcExpr::binary(
        CalcOp::Add,
        scaled(a, 1.0 - p),
        scaled(b, p),
    ))
}

impl Mixed {
    fn into_expr(self) -> CalcExpr {
        match self {
            Mixed::Cells(c) => CalcExpr::Number(c),
            Mixed::Percent(p) => CalcExpr::Percent(p),
            Mixed::Calc(e) => e,
        }
    }
}

/// A value with a `<length-percentage>` reading.
pub(crate) trait LengthPercentage: Sized {
    fn expr(&self) -> Option<CalcExpr>;
    fn from_mixed(m: Mixed) -> Self;
}

pub(crate) fn animate_lp<T: LengthPercentage>(a: &T, b: &T, p: f64) -> Option<T> {
    Some(T::from_mixed(mix(&a.expr()?, &b.expr()?, p)))
}

macro_rules! lp_animate {
    ($($t:ty),+) => {$(
        impl Animate for $t {
            fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
                animate_lp(self, to, p)
            }
        }
    )+};
}

fn arc_expr(e: &Arc<CalcExpr>) -> CalcExpr {
    (**e).clone()
}

impl LengthPercentage for Size {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            Size::Fixed(n) => Some(CalcExpr::Length(i32::from(*n))),
            Size::Percent(p) => Some(CalcExpr::Percent(f64::from(*p))),
            Size::Calc(e) => Some(arc_expr(e)),
            _ => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => Size::Fixed(cells_u16(c)),
            Mixed::Percent(p) => Size::Percent(p.max(0.0) as f32),
            Mixed::Calc(e) => Size::calc(e),
        }
    }
}

/// `width` / `height`: a length-percentage pair mixes; rdom's flex weight
/// (`Size::Flex`) is a number.
impl Animate for Size {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (Size::Flex(a), Size::Flex(b)) => Some(Size::Flex(a.animate(b, p, cx)?.max(0.0))),
            _ => animate_lp(self, to, p),
        }
    }
}

impl LengthPercentage for Length {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            Length::Cells(n) => Some(CalcExpr::Length(*n)),
            Length::Calc(e) => Some(arc_expr(e)),
            Length::Auto => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => Length::Cells(to_cells(c)),
            other => Length::calc(other.into_expr()),
        }
    }
}

impl LengthPercentage for MinSize {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            MinSize::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            MinSize::Percent(p) => Some(CalcExpr::Percent(f64::from(*p))),
            MinSize::Calc(e) => Some(arc_expr(e)),
            _ => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => MinSize::Cells(cells_u16(c)),
            Mixed::Percent(p) => MinSize::Percent(p.max(0.0) as f32),
            Mixed::Calc(e) => MinSize::Calc(Arc::new(e)),
        }
    }
}

impl LengthPercentage for MaxSize {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            MaxSize::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            MaxSize::Percent(p) => Some(CalcExpr::Percent(f64::from(*p))),
            MaxSize::Calc(e) => Some(arc_expr(e)),
            _ => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => MaxSize::Cells(cells_u16(c)),
            Mixed::Percent(p) => MaxSize::Percent(p.max(0.0) as f32),
            Mixed::Calc(e) => MaxSize::Calc(Arc::new(e)),
        }
    }
}

impl LengthPercentage for PaddingValue {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            PaddingValue::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            PaddingValue::Calc(e) => Some(arc_expr(e)),
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => PaddingValue::Cells(cells_u16(c)),
            other => PaddingValue::calc(other.into_expr()),
        }
    }
}

impl LengthPercentage for MarginValue {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            MarginValue::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            MarginValue::Calc(e) => Some(arc_expr(e)),
            MarginValue::Auto => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => MarginValue::Cells(
                to_cells(c).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
            ),
            other => MarginValue::Calc(Arc::new(other.into_expr())),
        }
    }
}

impl LengthPercentage for GapValue {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            GapValue::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            GapValue::Calc(e) => Some(arc_expr(e)),
            GapValue::Normal => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => GapValue::Cells(cells_u16(c)),
            other => GapValue::Calc(Arc::new(other.into_expr())),
        }
    }
}

impl LengthPercentage for FlexBasis {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            FlexBasis::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            FlexBasis::Calc(e) => Some(arc_expr(e)),
            _ => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => FlexBasis::Cells(cells_u16(c)),
            other => FlexBasis::Calc(Arc::new(other.into_expr())),
        }
    }
}

impl LengthPercentage for TrackBreadth {
    fn expr(&self) -> Option<CalcExpr> {
        match self {
            TrackBreadth::Cells(n) => Some(CalcExpr::Length(i32::from(*n))),
            TrackBreadth::Percent(p) => Some(CalcExpr::Percent(f64::from(*p))),
            TrackBreadth::Calc(e) => Some(arc_expr(e)),
            _ => None,
        }
    }
    fn from_mixed(m: Mixed) -> Self {
        match m {
            Mixed::Cells(c) => TrackBreadth::Cells(cells_u16(c)),
            Mixed::Percent(p) => TrackBreadth::Percent(p.max(0.0) as f32),
            Mixed::Calc(e) => TrackBreadth::calc(e),
        }
    }
}

lp_animate!(
    Length,
    MinSize,
    MaxSize,
    PaddingValue,
    MarginValue,
    GapValue,
    FlexBasis
);

/// A paint length keeps its fraction; a pixel length interpolates with a
/// pixel length only (cells and CSS pixels have no fixed ratio).
impl Animate for PaintLength {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (PaintLength::Cells(a), PaintLength::Cells(b)) => {
                Some(PaintLength::Cells(a.animate(b, p, cx)?))
            }
            (PaintLength::Px(a), PaintLength::Px(b)) => Some(PaintLength::Px(a.animate(b, p, cx)?)),
            (a, b) => {
                let expr = |v: &PaintLength| match v {
                    PaintLength::Cells(c) => Some(CalcExpr::Number(f64::from(*c))),
                    PaintLength::Calc(e) => Some(arc_expr(e)),
                    PaintLength::Px(_) => None,
                };
                Some(match mix(&expr(a)?, &expr(b)?, p) {
                    Mixed::Cells(c) => PaintLength::Cells(c as f32),
                    other => PaintLength::calc(other.into_expr()),
                })
            }
        }
    }
}

// ── Grid tracks ───────────────────────────────────────────────────

impl Animate for TrackBreadth {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (TrackBreadth::Fr(a), TrackBreadth::Fr(b)) => {
                Some(TrackBreadth::Fr(a.animate(b, p, cx)?.max(0.0)))
            }
            (a, b) if a == b => Some(a.clone()),
            _ => animate_lp(self, to, p),
        }
    }
}

impl Animate for TrackSize {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(match (self, to) {
            (TrackSize::Breadth(a), TrackSize::Breadth(b)) => {
                TrackSize::Breadth(a.animate(b, p, cx)?)
            }
            (TrackSize::MinMax(a1, a2), TrackSize::MinMax(b1, b2)) => {
                TrackSize::MinMax(a1.animate(b1, p, cx)?, a2.animate(b2, p, cx)?)
            }
            (TrackSize::FitContent(a), TrackSize::FitContent(b)) => {
                TrackSize::FitContent(a.animate(b, p, cx)?)
            }
            _ => return None,
        })
    }
}

fn animate_tracks(a: &[TrackSize], b: &[TrackSize], p: f64, cx: &Cx) -> Option<Vec<TrackSize>> {
    if a.len() != b.len() {
        return None;
    }
    a.iter().zip(b).map(|(x, y)| x.animate(y, p, cx)).collect()
}

/// CSS Grid 2 §7.2.6: two track lists of the same length (repeats of
/// the same count) interpolate track by track, line names from the
/// discrete step; any other pair is discrete.
impl Animate for TrackList {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        if self.items.len() != to.items.len() {
            return None;
        }
        let items = self
            .items
            .iter()
            .zip(&to.items)
            .map(|(a, b)| match (a, b) {
                (TrackListItem::Size(a), TrackListItem::Size(b)) => {
                    Some(TrackListItem::Size(a.animate(b, p, cx)?))
                }
                (TrackListItem::Repeat(a), TrackListItem::Repeat(b)) if a.count == b.count => {
                    let mut r = discrete(a, b, p);
                    r.sizes = animate_tracks(&a.sizes, &b.sizes, p, cx)?;
                    Some(TrackListItem::Repeat(r))
                }
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(TrackList {
            line_names: discrete(&self.line_names, &to.line_names, p),
            items,
        })
    }
}

impl Animate for GridTemplate {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (GridTemplate::Tracks(a), GridTemplate::Tracks(b)) => {
                Some(GridTemplate::Tracks(a.animate(b, p, cx)?))
            }
            (a, b) if a == b => Some(a.clone()),
            _ => None,
        }
    }
}

impl Animate for std::borrow::Cow<'static, [TrackSize]> {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        if self == to {
            return Some(self.clone());
        }
        animate_tracks(self, to, p, cx).map(std::borrow::Cow::Owned)
    }
}
