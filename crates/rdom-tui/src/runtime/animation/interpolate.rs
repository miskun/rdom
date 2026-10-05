//! Interpolation primitives for the animated value types.

use super::AnimatedValue;
use crate::layout::{Length, Size, ZIndex};
use crate::style::Color;

// ── Interpolation primitives ──────────────────────────────────────

/// Interpolate `from` → `to` at `t`; a color endpoint that is `reset`
/// stands for `reset` ([`lerp_color`]).
pub(super) fn interpolate(
    from: &AnimatedValue,
    to: &AnimatedValue,
    t: f32,
    reset: Color,
) -> AnimatedValue {
    match (from, to) {
        (AnimatedValue::Color(a), AnimatedValue::Color(b)) => {
            AnimatedValue::Color(lerp_color(*a, *b, t, reset))
        }
        (AnimatedValue::Colors(a), AnimatedValue::Colors(b)) => {
            AnimatedValue::Colors(a.zip(*b).map(|(a, b)| lerp_color(a, b, t, reset)))
        }
        (AnimatedValue::Size(a), AnimatedValue::Size(b)) => AnimatedValue::Size(lerp_size(a, b, t)),
        (AnimatedValue::Length(a), AnimatedValue::Length(b)) => {
            AnimatedValue::Length(lerp_length(a, b, t))
        }
        (AnimatedValue::U16(a), AnimatedValue::U16(b)) => AnimatedValue::U16(lerp_u16(*a, *b, t)),
        (AnimatedValue::Padding(a), AnimatedValue::Padding(b)) => {
            AnimatedValue::Padding(crate::layout::Padding {
                top: lerp_padding_value(&a.top, &b.top, t),
                right: lerp_padding_value(&a.right, &b.right, t),
                bottom: lerp_padding_value(&a.bottom, &b.bottom, t),
                left: lerp_padding_value(&a.left, &b.left, t),
            })
        }
        (AnimatedValue::ZIndex(a), AnimatedValue::ZIndex(b)) => {
            AnimatedValue::ZIndex(lerp_zindex(*a, *b, t))
        }
        (AnimatedValue::Gaps(ar, ac), AnimatedValue::Gaps(br, bc)) => {
            AnimatedValue::Gaps(lerp_u16(*ar, *br, t), lerp_u16(*ac, *bc, t))
        }
        (AnimatedValue::Visibility(a), AnimatedValue::Visibility(b)) => {
            AnimatedValue::Visibility(lerp_visibility(*a, *b, t))
        }
        // Type mismatch — snap at midpoint.
        _ => {
            if t < 0.5 {
                from.clone()
            } else {
                to.clone()
            }
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
pub(super) fn lerp_color(a: Color, b: Color, t: f32, reset: Color) -> Color {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let definite = |c: Color| if c == Color::Reset { reset } else { c };
    match rdom_style::color::interpolate_oklab(definite(a), definite(b), f64::from(t)) {
        Some(c) => c,
        None if t < 0.5 => a,
        None => b,
    }
}

/// CSS Display 3 §4 (`visibility`'s animation type, after CSS
/// Transitions 1 §2.1): with a `visible` end, every progress strictly
/// between 0 and 1 is `visible`, the ends their own values; with
/// neither end `visible`, a discrete step at the midpoint. A progress
/// past an end (an overshooting easing) takes the nearer end.
pub(super) fn lerp_visibility(
    a: crate::layout::Visibility,
    b: crate::layout::Visibility,
    t: f32,
) -> crate::layout::Visibility {
    if t <= 0.0 {
        a
    } else if t >= 1.0 {
        b
    } else if a.is_visible() || b.is_visible() {
        crate::layout::Visibility::Visible
    } else if t < 0.5 {
        a
    } else {
        b
    }
}

#[inline]
fn lerp_u16(a: u16, b: u16, t: f32) -> u16 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round().clamp(0.0, u16::MAX as f32) as u16
}

/// Interpolate two `PaddingValue` sides. `Cells ↔ Cells` lerps the
/// cell count linearly. Anything involving `Calc` (whose value
/// depends on a containing-block width unknown at animation time)
/// snaps at midpoint — matches the policy already used for
/// `Size::Flex|Auto|Calc` lerps below.
fn lerp_padding_value(
    a: &crate::layout::PaddingValue,
    b: &crate::layout::PaddingValue,
    t: f32,
) -> crate::layout::PaddingValue {
    use crate::layout::PaddingValue;
    match (a, b) {
        (PaddingValue::Cells(x), PaddingValue::Cells(y)) => {
            PaddingValue::Cells(lerp_u16(*x, *y, t))
        }
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// An `<integer>` interpolated as a real number and rounded (CSS Values
/// 4 §3.2), in `f64`, which holds every `i32` exactly (the cast
/// saturates).
#[inline]
fn lerp_i32(a: i32, b: i32, t: f32) -> i32 {
    let v = f64::from(a) + (f64::from(b) - f64::from(a)) * f64::from(t);
    v.round() as i32
}

fn lerp_size(a: &Size, b: &Size, t: f32) -> Size {
    match (a, b) {
        (Size::Fixed(x), Size::Fixed(y)) => Size::Fixed(lerp_u16(*x, *y, t)),
        // Fixed ↔ Flex / Auto / Calc don't lerp meaningfully; snap.
        // Calc-bearing transitions snap because we don't have layout
        // context at interpolation time to resolve the percent
        // basis. Documented divergence; pay down by snapshotting
        // computed-pixel values at transition start.
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

fn lerp_length(a: &Length, b: &Length, t: f32) -> Length {
    match (a, b) {
        (Length::Cells(x), Length::Cells(y)) => Length::Cells(lerp_i32(*x, *y, t)),
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

fn lerp_zindex(a: ZIndex, b: ZIndex, t: f32) -> ZIndex {
    match (a, b) {
        (ZIndex::Value(x), ZIndex::Value(y)) => ZIndex::Value(lerp_i32(x, y, t)),
        _ => {
            if t < 0.5 {
                a
            } else {
                b
            }
        }
    }
}
