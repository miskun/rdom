//! Interpolation of computed values (CSS Values 4 §3 "Combining
//! Values", Web Animations 1 §5.3 "Animation types"): the per-type
//! `Animate` rule the longhand table (`table.rs`) folds over.
//!
//! A pair `Animate::animate` returns `None` for is not interpolable and
//! changes discretely (Web Animations 1 §5.3.1: from below 50 %
//! progress, to from 50 % on). A length that layout reads in cells
//! rounds onto the grid half to even, as a `calc()` result does
//! ([`to_cells`]); a length kept fractional for paint (`PaintLength`,
//! `letter-spacing`, …) stays fractional until its used value is taken.

use crate::Color;
use crate::calc::to_cells;
use crate::layout::{
    BorderRadius, BorderSpacing, BorderWidth, BoxShadow, CaretColor, CaretTextColor,
    ContainIntrinsicSize, OverflowClipMargin, PaintLength, ScrollPadding, ScrollbarColor,
    Visibility, ZIndex,
};

/// What an interpolation needs besides the two values: the color a
/// `reset` endpoint stands for (the canvas model's color for the
/// property's role, CSS Color Adjust 1 §2.1).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cx {
    pub reset: Color,
}

/// A computed value that interpolates.
pub(crate) trait Animate: Clone {
    /// The value `progress` of the way from `self` to `to`; `None` when
    /// the pair does not interpolate.
    fn animate(&self, to: &Self, progress: f64, cx: &Cx) -> Option<Self>;

    /// `self + other` (Web Animations 1 §5.4.4, CSS Values 4 §3.1): the
    /// composite of an `add` / `accumulate` effect value onto the
    /// underlying value; `None` where the type defines no addition (a
    /// discrete value, a keyword) — the effect value then replaces.
    fn add(&self, _other: &Self, _cx: &Cx) -> Option<Self> {
        None
    }
}

/// Web Animations 1 §5.3.1: the discrete step — `from` below 50 %
/// progress, `to` from it on.
pub(crate) fn discrete<T: Clone>(from: &T, to: &T, progress: f64) -> T {
    if progress < 0.5 {
        from.clone()
    } else {
        to.clone()
    }
}

/// `from` → `to` at `progress`, discretely where the pair does not
/// interpolate.
pub(crate) fn blend<T: Animate>(from: &T, to: &T, progress: f64, cx: &Cx) -> T {
    from.animate(to, progress, cx)
        .unwrap_or_else(|| discrete(from, to, progress))
}

/// Whether the pair interpolates (rather than stepping).
pub(crate) fn interpolable<T: Animate>(from: &T, to: &T) -> bool {
    let cx = Cx {
        reset: Color::Reset,
    };
    from.animate(to, 0.5, &cx).is_some()
}

#[inline]
pub(crate) fn lerp(a: f64, b: f64, p: f64) -> f64 {
    a + (b - a) * p
}

// ── Numbers ───────────────────────────────────────────────────────

impl Animate for f32 {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        Some(lerp(f64::from(*self), f64::from(*to), p) as f32)
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(self + other)
    }
}

/// An `<integer>` interpolates as a real number and rounds (CSS Values 4
/// §3.2).
impl Animate for i32 {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        Some(to_cells(lerp(f64::from(*self), f64::from(*to), p)))
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(self.saturating_add(*other))
    }
}

/// Whole cells, never negative.
impl Animate for u16 {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        Some(cells_u16(lerp(f64::from(*self), f64::from(*to), p)))
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(self.saturating_add(*other))
    }
}

impl Animate for i16 {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let v = to_cells(lerp(f64::from(*self), f64::from(*to), p));
        Some(v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16)
    }
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        Some(self.saturating_add(*other))
    }
}

impl Animate for Option<u32> {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let (a, b) = ((*self)?, (*to)?);
        let v = to_cells(lerp(f64::from(a), f64::from(b), p));
        Some(Some(v.max(0) as u32))
    }
}

pub(crate) fn cells_u16(v: f64) -> u16 {
    to_cells(v).clamp(0, i32::from(u16::MAX)) as u16
}

// ── Colors ────────────────────────────────────────────────────────

impl Animate for Color {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(lerp_color(*self, *to, p as f32, cx.reset))
    }
    fn add(&self, other: &Self, cx: &Cx) -> Option<Self> {
        Some(add_color(*self, *other, cx.reset))
    }
}

/// Two colors added channel by channel in sRGB, each clamped (Web
/// Animations 1 §5.4.4 for `<color>`), alpha included; a `reset`
/// endpoint counts as its canvas color, a palette index as its xterm
/// color.
fn add_color(a: Color, b: Color, reset: Color) -> Color {
    let rgba = |c: Color| match if c == Color::Reset { reset } else { c } {
        Color::Rgb(r, g, b) => Some((r, g, b, u8::MAX)),
        Color::Rgba(r, g, b, a) => Some((r, g, b, a)),
        Color::Indexed(n) => {
            let (r, g, b) = crate::color::palette::xterm_rgb(n);
            Some((r, g, b, u8::MAX))
        }
        _ => None,
    };
    match (rgba(a), rgba(b)) {
        (Some(x), Some(y)) => Color::rgba(
            x.0.saturating_add(y.0),
            x.1.saturating_add(y.1),
            x.2.saturating_add(y.2),
            x.3.saturating_add(y.3),
        ),
        _ => b,
    }
}

impl Animate for crate::TuiColor {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (crate::TuiColor::Literal(a), crate::TuiColor::Literal(b)) => Some(
                crate::TuiColor::Literal(lerp_color(*a, *b, p as f32, cx.reset)),
            ),
            _ => None,
        }
    }
    fn add(&self, other: &Self, cx: &Cx) -> Option<Self> {
        match (self, other) {
            (crate::TuiColor::Literal(a), crate::TuiColor::Literal(b)) => {
                Some(crate::TuiColor::Literal(add_color(*a, *b, cx.reset)))
            }
            _ => None,
        }
    }
}

/// Interpolate two colors in Oklab with premultiplied alpha (CSS Color
/// 4 §12.1, §12.3), so a fade from `transparent` does not pass through
/// black; a palette index counts as its xterm color. An endpoint that is
/// the terminal default (`reset`) has no sRGB value of its own: it
/// interpolates as `reset` — the canvas model's color for the
/// property's role in the element's color scheme. Given `Color::Reset`
/// there (no role), such a pair changes discretely at the midpoint, as
/// a value that does not interpolate does (CSS Transitions 1 §2). The
/// endpoints themselves are returned as they are.
pub fn lerp_color(a: Color, b: Color, t: f32, reset: Color) -> Color {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let definite = |c: Color| if c == Color::Reset { reset } else { c };
    match crate::color::interpolate_oklab(definite(a), definite(b), f64::from(t)) {
        Some(c) => c,
        None if t < 0.5 => a,
        None => b,
    }
}

// ── Visibility ────────────────────────────────────────────────────

/// CSS Display 3 §4 (`visibility`'s animation type, after CSS
/// Transitions 1 §2.1 and Web Animations 1 §5.3.2): with a `visible`
/// end, every progress strictly between 0 and 1 is `visible`, the ends
/// their own values; with neither end `visible`, a discrete step at the
/// midpoint. A progress past an end (an overshooting easing) takes the
/// nearer end.
pub fn lerp_visibility(a: Visibility, b: Visibility, t: f32) -> Visibility {
    if t <= 0.0 {
        a
    } else if t >= 1.0 {
        b
    } else if a.is_visible() || b.is_visible() {
        Visibility::Visible
    } else if t < 0.5 {
        a
    } else {
        b
    }
}

// ── Borders and shadows ───────────────────────────────────────────

/// A width keyword is a glyph weight in rdom, not a length: only two
/// lengths interpolate.
impl Animate for BorderWidth {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (BorderWidth::Length(a), BorderWidth::Length(b)) => {
                Some(BorderWidth::Length(a.animate(b, p, cx)?))
            }
            (a, b) if a == b => Some(a.clone()),
            _ => None,
        }
    }
    fn add(&self, other: &Self, cx: &Cx) -> Option<Self> {
        match (self, other) {
            (BorderWidth::Length(a), BorderWidth::Length(b)) => {
                Some(BorderWidth::Length(a.add(b, cx)?))
            }
            _ => None,
        }
    }
}

impl Animate for BorderRadius {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(BorderRadius {
            horizontal: self.horizontal.animate(&to.horizontal, p, cx)?,
            vertical: self.vertical.animate(&to.vertical, p, cx)?,
        })
    }
}

impl Animate for BorderSpacing {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(BorderSpacing {
            horizontal: self.horizontal.animate(&to.horizontal, p, cx)?,
            vertical: self.vertical.animate(&to.vertical, p, cx)?,
        })
    }
}

/// CSS Backgrounds 3 §6.1 "as shadow list" (Web Animations 1 §5.3):
/// the shorter list is padded with transparent zero shadows of the
/// other's `inset`; each pair interpolates its color and lengths, and a
/// pair whose `inset` differs makes the whole list discrete.
impl Animate for Vec<BoxShadow<Color>> {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        let zero = |s: &BoxShadow<Color>| BoxShadow {
            inset: s.inset,
            offset_x: PaintLength::Cells(0.0),
            offset_y: PaintLength::Cells(0.0),
            blur: PaintLength::Cells(0.0),
            spread: PaintLength::Cells(0.0),
            color: Color::TRANSPARENT,
        };
        let n = self.len().max(to.len());
        (0..n)
            .map(|i| {
                let (a, b) = match (self.get(i), to.get(i)) {
                    (Some(a), Some(b)) => (a.clone(), b.clone()),
                    (Some(a), None) => (a.clone(), zero(a)),
                    (None, Some(b)) => (zero(b), b.clone()),
                    (None, None) => unreachable!("i < the longer list's length"),
                };
                if a.inset != b.inset {
                    return None;
                }
                Some(BoxShadow {
                    inset: a.inset,
                    offset_x: a.offset_x.animate(&b.offset_x, p, cx)?,
                    offset_y: a.offset_y.animate(&b.offset_y, p, cx)?,
                    blur: a.blur.animate(&b.blur, p, cx)?,
                    spread: a.spread.animate(&b.spread, p, cx)?,
                    color: a.color.animate(&b.color, p, cx)?,
                })
            })
            .collect()
    }
}

// ── Others ────────────────────────────────────────────────────────

impl Animate for ZIndex {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (ZIndex::Value(a), ZIndex::Value(b)) => Some(ZIndex::Value(a.animate(b, p, cx)?)),
            _ => None,
        }
    }
    fn add(&self, other: &Self, cx: &Cx) -> Option<Self> {
        match (self, other) {
            (ZIndex::Value(a), ZIndex::Value(b)) => Some(ZIndex::Value(a.add(b, cx)?)),
            _ => None,
        }
    }
}

impl Animate for ScrollPadding {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (ScrollPadding::Length(a), ScrollPadding::Length(b)) => {
                Some(ScrollPadding::Length(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}

impl Animate for ScrollbarColor {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (
                ScrollbarColor::Colors { thumb: a, track: c },
                ScrollbarColor::Colors { thumb: b, track: d },
            ) => Some(ScrollbarColor::Colors {
                thumb: a.animate(b, p, cx)?,
                track: c.animate(d, p, cx)?,
            }),
            _ => None,
        }
    }
}

/// CSS Values 4 §5.7: a ratio interpolates through the logarithm of its
/// value; a degenerate ratio, or a pair whose `auto` differs, is
/// discrete.
impl Animate for Option<crate::layout::AspectRatio> {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let (a, b) = ((*self)?, (*to)?);
        if a.auto() != b.auto() {
            return None;
        }
        let (x, y) = (f64::from(a.value()?), f64::from(b.value()?));
        if x <= 0.0 || y <= 0.0 {
            return None;
        }
        let v = lerp(x.ln(), y.ln(), p).exp();
        Some(Some(
            crate::layout::AspectRatio::new(v as f32, 1.0)?.with_auto(a.auto()),
        ))
    }
}

/// CSS Lists 3: two lists naming the same counters in the same order
/// interpolate their integers.
impl Animate for Vec<crate::CounterOp> {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        if self.len() != to.len() {
            return None;
        }
        self.iter()
            .zip(to)
            .map(|(a, b)| {
                (a.name == b.name && a.reversed == b.reversed).then(|| {
                    let mut out = a.clone();
                    out.value = a.value.animate(&b.value, p, cx).unwrap_or(b.value);
                    out
                })
            })
            .collect()
    }
}

/// CSS Overflow 4 §3.2: the margin interpolates; the box keyword must
/// agree.
impl Animate for OverflowClipMargin {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        (self.visual_box == to.visual_box).then(|| {
            OverflowClipMargin::new(
                self.visual_box,
                self.margin.animate(&to.margin, p, cx).unwrap_or(to.margin),
            )
        })
    }
}

impl Animate for CaretColor {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (CaretColor::Color(a), CaretColor::Color(b)) => {
                Some(CaretColor::Color(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}

impl Animate for CaretTextColor {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (CaretTextColor::Color(a), CaretTextColor::Color(b)) => {
                Some(CaretTextColor::Color(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}

/// CSS Sizing 4 §6.1: the lengths interpolate; `auto` must agree.
impl Animate for ContainIntrinsicSize {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        if self.auto != to.auto {
            return None;
        }
        let (a, b) = (self.cells()?, to.cells()?);
        Some(ContainIntrinsicSize {
            auto: self.auto,
            length: Some(crate::calc::CalcExpr::Length(i32::from(cells_u16(lerp(
                f64::from(a),
                f64::from(b),
                p,
            ))))),
        })
    }
}
