//! `AbsoluteColor` — a color as CSS Color 4 computes it before rdom
//! stores it: floating-point components in one color space, any of
//! which may be *missing* (`none`, §4.4), and an alpha.
//!
//! The color functions parse into one of these; [`AbsoluteColor::to_color`]
//! turns it into the 8-bit sRGB [`Color`] a terminal cell takes.

use super::Color;

/// The color spaces a color function names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorSpace {
    /// sRGB (CSS Color 4 §10.2), components `0..=1`.
    Srgb,
}

/// A color in `space`: three components and an alpha (`0..=1`), each
/// `None` when missing (CSS Color 4 §4.4: `none`). A missing component
/// is zero wherever the color is used directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AbsoluteColor {
    pub space: ColorSpace,
    pub coords: [Option<f64>; 3],
    pub alpha: Option<f64>,
}

impl AbsoluteColor {
    /// An sRGB color, every component present.
    pub fn srgb(r: f64, g: f64, b: f64, alpha: f64) -> Self {
        AbsoluteColor {
            space: ColorSpace::Srgb,
            coords: [Some(r), Some(g), Some(b)],
            alpha: Some(alpha),
        }
    }

    /// The components with missing ones as zero.
    pub fn values(self) -> [f64; 3] {
        self.coords.map(|c| c.unwrap_or(0.0))
    }

    /// The alpha, missing as zero, clamped to `0..=1`.
    pub fn alpha_value(self) -> f64 {
        clamp_unit(self.alpha.unwrap_or(0.0))
    }

    /// The 8-bit sRGB color: each component clamped to the gamut and
    /// rounded to the nearest byte, ties toward +∞ (CSS Color 4 §5.1);
    /// alpha likewise.
    pub fn to_color(self) -> Color {
        let ColorSpace::Srgb = self.space;
        let [r, g, b] = self.values();
        Color::rgba(
            to_byte(r),
            to_byte(g),
            to_byte(b),
            to_byte(self.alpha_value()),
        )
    }
}

/// `v` clamped to `0..=1`; NaN is 0.
fn clamp_unit(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

/// A `0..=1` component as a byte, ties rounded up.
fn to_byte(v: f64) -> u8 {
    (clamp_unit(v) * 255.0 + 0.5).floor() as u8
}
