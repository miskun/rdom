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
    /// HSL (§7): hue in degrees, saturation and lightness `0..=100`.
    Hsl,
    /// HWB (§8): hue in degrees, whiteness and blackness `0..=100`.
    Hwb,
    /// Linear-light sRGB (§10.3).
    SrgbLinear,
    /// Display P3 (§10.4).
    DisplayP3,
    /// A98 RGB (§10.5).
    A98Rgb,
    /// ProPhoto RGB (§10.6), a D50 space.
    ProphotoRgb,
    /// ITU-R BT.2020 (§10.7).
    Rec2020,
    /// CIE XYZ with the D50 white point (§10.8).
    XyzD50,
    /// CIE XYZ with the D65 white point (§10.8).
    XyzD65,
    /// CIE Lab (§9.2): lightness `0..=100`, a / b unbounded.
    Lab,
    /// CIE LCH (§9.3): lightness, chroma, hue in degrees.
    Lch,
    /// Oklab (§9.4): lightness `0..=1`, a / b unbounded.
    Oklab,
    /// Oklch (§9.4): lightness, chroma, hue in degrees.
    Oklch,
}

impl ColorSpace {
    /// The space a `color()` function names (CSS Color 4 §10.1), ASCII
    /// case-insensitive; `xyz` is `xyz-d65`.
    pub fn predefined(name: &str) -> Option<ColorSpace> {
        const TABLE: &[(&str, ColorSpace)] = &[
            ("srgb", ColorSpace::Srgb),
            ("srgb-linear", ColorSpace::SrgbLinear),
            ("display-p3", ColorSpace::DisplayP3),
            ("a98-rgb", ColorSpace::A98Rgb),
            ("prophoto-rgb", ColorSpace::ProphotoRgb),
            ("rec2020", ColorSpace::Rec2020),
            ("xyz", ColorSpace::XyzD65),
            ("xyz-d50", ColorSpace::XyzD50),
            ("xyz-d65", ColorSpace::XyzD65),
        ];
        TABLE
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, s)| *s)
    }
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
    /// An 8-bit color as an absolute sRGB one — a palette index as the
    /// xterm palette's color ([`super::palette::xterm_rgb`]); `None` for
    /// the terminal default, which is a foreground or a background
    /// depending on where it is used.
    pub fn from_color(color: Color) -> Option<AbsoluteColor> {
        let (r, g, b) = match color {
            Color::Rgb(r, g, b) | Color::Rgba(r, g, b, _) => (r, g, b),
            Color::Indexed(n) => super::palette::xterm_rgb(n),
            Color::Reset => return None,
        };
        Some(AbsoluteColor {
            space: ColorSpace::Srgb,
            coords: [r, g, b].map(|c| Some(f64::from(c) / 255.0)),
            alpha: Some(f64::from(color.alpha()) / 255.0),
        })
    }

    /// The components with missing ones as zero.
    pub fn values(self) -> [f64; 3] {
        self.coords.map(|c| c.unwrap_or(0.0))
    }

    /// The alpha, missing as zero, clamped to `0..=1`.
    pub fn alpha_value(self) -> f64 {
        clamp_unit(self.alpha.unwrap_or(0.0))
    }

    /// The 8-bit sRGB color: gamut-mapped into sRGB (CSS Color 4
    /// §13.2) and each component rounded to the nearest byte, ties
    /// toward +∞ (§5.1); alpha likewise.
    pub fn to_color(self) -> Color {
        let [r, g, b] = super::gamut::map_to_srgb(self);
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
