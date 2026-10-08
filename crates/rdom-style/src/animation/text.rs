//! Interpolation of the text and font values (CSS Text 3 / 4, CSS
//! Inline 3, CSS Fonts 4, CSS Text Decoration 4).

use super::value::{Animate, Cx, lerp};
use crate::layout::{
    FontSize, FontStretch, FontStyle, FontWeight, LineHeight, Spacing, TabSize,
    TextDecorationThickness, TextIndent, TextUnderlineOffset, VerticalAlign,
};

/// CSS Text 4 §9: `normal` spacing computes to zero.
impl Animate for Spacing {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let cells = |s: &Spacing| match s {
            Spacing::Normal => Some(0.0),
            Spacing::Cells(c) => Some(f64::from(*c)),
            Spacing::Calc(_) => None,
        };
        Some(Spacing::Cells(lerp(cells(self)?, cells(to)?, p) as f32))
    }
}

/// CSS Inline 3 §5.1: a number with a number, a length with a length;
/// `normal` is discrete.
impl Animate for LineHeight {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(match (self, to) {
            (LineHeight::Number(a), LineHeight::Number(b)) => {
                LineHeight::Number(a.animate(b, p, cx)?.max(0.0))
            }
            (LineHeight::Rows(a), LineHeight::Rows(b)) => {
                LineHeight::Rows(a.animate(b, p, cx)?.max(0.0))
            }
            _ => return None,
        })
    }
}

/// The length interpolates; the keywords must agree.
impl Animate for TextIndent {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        if (self.hanging, self.each_line) != (to.hanging, to.each_line) {
            return None;
        }
        let mut out = self.clone();
        out.length = self.length.animate(&to.length, p, cx)?;
        Some(out)
    }
}

impl Animate for TabSize {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        Some(match (self, to) {
            (TabSize::Number(a), TabSize::Number(b)) => TabSize::Number(a.animate(b, p, cx)?),
            (TabSize::Length(a), TabSize::Length(b)) => TabSize::Length(a.animate(b, p, cx)?),
            _ => return None,
        })
    }
}

/// CSS Fonts 4 §2.2: a computed weight is a number in `[1, 1000]`.
impl Animate for FontWeight {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let (a, b) = (self.value(400.0), to.value(400.0));
        Some(FontWeight::Number(
            lerp(f64::from(a), f64::from(b), p).clamp(1.0, 1000.0) as f32,
        ))
    }
}

/// CSS Fonts 4 §2.3: the width computes to a percentage.
impl Animate for FontStretch {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let percent = |s: &FontStretch| match s {
            FontStretch::Normal => 100.0,
            FontStretch::Keyword(k) => f64::from(k.percent()),
            FontStretch::Percent(v) => f64::from(*v),
        };
        Some(FontStretch::Percent(
            lerp(percent(self), percent(to), p).max(0.0) as f32,
        ))
    }
}

/// CSS Fonts 4 §2.4: `normal` animates as `oblique 0deg`; `italic` only
/// with `italic`.
impl Animate for FontStyle {
    fn animate(&self, to: &Self, p: f64, _: &Cx) -> Option<Self> {
        let angle = |s: &FontStyle| match s {
            FontStyle::Normal => Some(0.0),
            FontStyle::Oblique(a) => Some(f64::from(a.unwrap_or(14.0))),
            FontStyle::Italic => None,
        };
        let (a, b) = (angle(self)?, angle(to)?);
        Some(FontStyle::Oblique(Some(
            lerp(a, b, p).clamp(-90.0, 90.0) as f32
        )))
    }
}

impl Animate for TextDecorationThickness {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (TextDecorationThickness::Length(a), TextDecorationThickness::Length(b)) => {
                Some(TextDecorationThickness::Length(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}

impl Animate for TextUnderlineOffset {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (TextUnderlineOffset::Length(a), TextUnderlineOffset::Length(b)) => {
                Some(TextUnderlineOffset::Length(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}

impl Animate for FontSize {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (FontSize::Length(a), FontSize::Length(b)) => {
                Some(FontSize::Length(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}

/// CSS Inline 3 §3.3 (`baseline-shift`): a raise in rows interpolates;
/// the keywords are discrete.
impl Animate for VerticalAlign {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (VerticalAlign::Rows(a), VerticalAlign::Rows(b)) => {
                Some(VerticalAlign::Rows(a.animate(b, p, cx)?))
            }
            _ => None,
        }
    }
}
