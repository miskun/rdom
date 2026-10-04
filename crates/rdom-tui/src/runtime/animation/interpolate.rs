//! Interpolation primitives for the animated value types.

use super::AnimatedValue;
use crate::layout::{Length, Size, ZIndex};
use crate::style::Color;

// ── Interpolation primitives ──────────────────────────────────────

pub(super) fn interpolate(from: &AnimatedValue, to: &AnimatedValue, t: f32) -> AnimatedValue {
    match (from, to) {
        (AnimatedValue::Color(a), AnimatedValue::Color(b)) => {
            AnimatedValue::Color(lerp_color(*a, *b, t))
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
/// black. An endpoint that is the terminal default or a palette index
/// has no sRGB value: those interpolate in sRGB from an approximation.
pub(super) fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    if let Some(c) = rdom_style::color::interpolate_oklab(a, b, f64::from(t)) {
        return c;
    }
    let (ar, ag, ab) = color_to_rgb_approx(a);
    let (br, bg, bb) = color_to_rgb_approx(b);
    let (aa, ba) = (f32::from(a.alpha()) / 255.0, f32::from(b.alpha()) / 255.0);
    let alpha = aa + (ba - aa) * t;
    if alpha <= 0.0 {
        return Color::TRANSPARENT;
    }
    let channel = |x: u8, y: u8| {
        let v = (f32::from(x) * aa * (1.0 - t) + f32::from(y) * ba * t) / alpha;
        v.round().clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        channel(ar, br),
        channel(ag, bg),
        channel(ab, bb),
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
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

#[inline]
fn lerp_i16(a: i16, b: i16, t: f32) -> i16 {
    let v = a as f32 + (b as f32 - a as f32) * t;
    v.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

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
        (ZIndex::Value(x), ZIndex::Value(y)) => ZIndex::Value(lerp_i16(x, y, t)),
        _ => {
            if t < 0.5 {
                a
            } else {
                b
            }
        }
    }
}

/// Map a `Color` to an approximate sRGB triple for interpolation.
/// Named colors use the canonical ANSI-16 palette values.
/// `Reset` and `Indexed(_)` fall back to mid-gray since we don't
/// know the terminal's actual palette — interpolation through
/// these is best-effort and apps that want exact lerps should
/// use `Color::Rgb` endpoints.
/// Resolve a `Color` to its (r, g, b) for interpolation. Now
/// trivial in the truecolor-only world: `Rgb` returns its
/// channels, `Indexed` falls back to a neutral midgray placeholder
/// (a future commit could read the xterm-256 RGB table), and
/// `Reset` returns a neutral light-gray since the actual terminal
/// default is unknowable from inside the engine.
fn color_to_rgb_approx(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Reset => (192, 192, 192),
        Color::Indexed(_) => (128, 128, 128),
        Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => (r, g, b),
    }
}
