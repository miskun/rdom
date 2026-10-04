//! Conversions between the color spaces (CSS Color 4 §7.1, §8.1;
//! sample code in §19). Components are in each space's own reference
//! range: sRGB `0..=1`, HSL / HWB hue in degrees and the other two
//! channels on a 0–100 scale.

use super::absolute::{AbsoluteColor, ColorSpace};

/// `color` converted to sRGB. Missing components count as zero; the
/// alpha (missing or not) is carried over.
pub(crate) fn to_srgb(color: AbsoluteColor) -> AbsoluteColor {
    let [a, b, c] = color.values();
    let rgb = match color.space {
        ColorSpace::Srgb => return color,
        ColorSpace::Hsl => hsl_to_srgb(a, b, c),
        ColorSpace::Hwb => hwb_to_srgb(a, b, c),
    };
    AbsoluteColor {
        space: ColorSpace::Srgb,
        coords: rgb.map(Some),
        alpha: color.alpha,
    }
}

/// A hue in `[0, 360)`.
pub(crate) fn normalize_hue(degrees: f64) -> f64 {
    let h = degrees.rem_euclid(360.0);
    if h.is_finite() { h } else { 0.0 }
}

/// CSS Color 4 §7.2 `hslToRgb`.
fn hsl_to_srgb(hue: f64, saturation: f64, lightness: f64) -> [f64; 3] {
    let hue = normalize_hue(hue);
    let (s, l) = (saturation / 100.0, lightness / 100.0);
    let f = |n: f64| {
        let k = (n + hue / 30.0) % 12.0;
        let a = s * l.min(1.0 - l);
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    [f(0.0), f(8.0), f(4.0)]
}

/// CSS Color 4 §8.2 `hwbToRgb`.
fn hwb_to_srgb(hue: f64, whiteness: f64, blackness: f64) -> [f64; 3] {
    let (w, b) = (whiteness / 100.0, blackness / 100.0);
    if w + b >= 1.0 {
        let gray = w / (w + b);
        return [gray; 3];
    }
    hsl_to_srgb(hue, 100.0, 50.0).map(|c| c * (1.0 - w - b) + w)
}
