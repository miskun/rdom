//! Conversions between the color spaces (CSS Color 4 §7.2, §8.2, §9,
//! §10; the sample code of §19). Every space converts through CIE XYZ
//! with the D65 white point; Lab, ProPhoto RGB and XYZ-D50 are D50
//! spaces, chromatically adapted with the Bradford transform.
//!
//! Components are in each space's own reference range: the RGB spaces
//! and XYZ `0..=1`, HSL / HWB hue in degrees and the other two channels
//! `0..=100`, Lab / LCH lightness `0..=100`, Oklab / Oklch lightness
//! `0..=1`, the polar hues in degrees.

use super::absolute::{AbsoluteColor, ColorSpace};

type Vec3 = [f64; 3];
pub(super) type Mat3 = [Vec3; 3];

use super::matrices::*;

fn mul(m: &Mat3, v: Vec3) -> Vec3 {
    [0, 1, 2].map(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}

/// `color` converted to `to`. Missing components count as zero (the
/// interpolation code carries them over itself); the alpha is kept.
pub(crate) fn convert(color: AbsoluteColor, to: ColorSpace) -> AbsoluteColor {
    if color.space == to {
        return color;
    }
    let coords = from_xyz_d65(to, to_xyz_d65(color.space, color.values()));
    AbsoluteColor {
        space: to,
        coords: coords.map(Some),
        alpha: color.alpha,
    }
}

/// A hue in `[0, 360)`.
pub(crate) fn normalize_hue(degrees: f64) -> f64 {
    let h = degrees.rem_euclid(360.0);
    if h.is_finite() { h } else { 0.0 }
}

/// Coordinates in `space` to CIE XYZ (D65).
pub(crate) fn to_xyz_d65(space: ColorSpace, c: Vec3) -> Vec3 {
    match space {
        ColorSpace::Srgb => mul(&SRGB_TO_XYZ, c.map(srgb_to_linear)),
        ColorSpace::SrgbLinear => mul(&SRGB_TO_XYZ, c),
        ColorSpace::Hsl => to_xyz_d65(ColorSpace::Srgb, hsl_to_srgb(c)),
        ColorSpace::Hwb => to_xyz_d65(ColorSpace::Srgb, hwb_to_srgb(c)),
        ColorSpace::DisplayP3 => mul(&P3_TO_XYZ, c.map(srgb_to_linear)),
        ColorSpace::A98Rgb => mul(&A98_TO_XYZ, c.map(a98_to_linear)),
        ColorSpace::ProphotoRgb => d50_to_d65(mul(&PROPHOTO_TO_XYZ_D50, c.map(prophoto_to_linear))),
        ColorSpace::Rec2020 => mul(&REC2020_TO_XYZ, c.map(rec2020_to_linear)),
        ColorSpace::XyzD50 => d50_to_d65(c),
        ColorSpace::XyzD65 => c,
        ColorSpace::Lab => d50_to_d65(lab_to_xyz_d50(c)),
        ColorSpace::Lch => d50_to_d65(lab_to_xyz_d50(polar_to_rect(c))),
        ColorSpace::Oklab => oklab_to_xyz(c),
        ColorSpace::Oklch => oklab_to_xyz(polar_to_rect(c)),
    }
}

/// CIE XYZ (D65) to coordinates in `space`.
pub(crate) fn from_xyz_d65(space: ColorSpace, xyz: Vec3) -> Vec3 {
    match space {
        ColorSpace::Srgb => mul(&XYZ_TO_SRGB, xyz).map(linear_to_srgb),
        ColorSpace::SrgbLinear => mul(&XYZ_TO_SRGB, xyz),
        ColorSpace::Hsl => srgb_to_hsl(from_xyz_d65(ColorSpace::Srgb, xyz)),
        ColorSpace::Hwb => srgb_to_hwb(from_xyz_d65(ColorSpace::Srgb, xyz)),
        ColorSpace::DisplayP3 => mul(&XYZ_TO_P3, xyz).map(linear_to_srgb),
        ColorSpace::A98Rgb => mul(&XYZ_TO_A98, xyz).map(linear_to_a98),
        ColorSpace::ProphotoRgb => {
            mul(&XYZ_D50_TO_PROPHOTO, d65_to_d50(xyz)).map(linear_to_prophoto)
        }
        ColorSpace::Rec2020 => mul(&XYZ_TO_REC2020, xyz).map(linear_to_rec2020),
        ColorSpace::XyzD50 => d65_to_d50(xyz),
        ColorSpace::XyzD65 => xyz,
        ColorSpace::Lab => xyz_d50_to_lab(d65_to_d50(xyz)),
        ColorSpace::Lch => rect_to_polar(xyz_d50_to_lab(d65_to_d50(xyz))),
        ColorSpace::Oklab => xyz_to_oklab(xyz),
        ColorSpace::Oklch => rect_to_polar(xyz_to_oklab(xyz)),
    }
}

// ── HSL / HWB (§7.2, §8.2) ──────────────────────────────────────

/// CSS Color 4 §7.2 `hslToRgb`.
fn hsl_to_srgb([hue, saturation, lightness]: Vec3) -> Vec3 {
    let hue = normalize_hue(hue);
    let (s, l) = (saturation / 100.0, lightness / 100.0);
    let f = |n: f64| {
        let k = (n + hue / 30.0) % 12.0;
        let a = s * l.min(1.0 - l);
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    [f(0.0), f(8.0), f(4.0)]
}

/// CSS Color 4 §7.3 `rgbToHsl` (hue 0 where it is powerless).
fn srgb_to_hsl([r, g, b]: Vec3) -> Vec3 {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (min + max) / 2.0;
    let d = max - min;
    let (mut h, mut s) = (0.0, 0.0);
    if d != 0.0 {
        s = if l == 0.0 || l == 1.0 {
            0.0
        } else {
            (max - l) / l.min(1.0 - l)
        };
        h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        } * 60.0;
    }
    // A negative saturation (out-of-gamut input) flips the hue.
    if s < 0.0 {
        h += 180.0;
        s = -s;
    }
    [normalize_hue(h), s * 100.0, l * 100.0]
}

/// CSS Color 4 §8.2 `hwbToRgb`.
fn hwb_to_srgb([hue, whiteness, blackness]: Vec3) -> Vec3 {
    let (w, b) = (whiteness / 100.0, blackness / 100.0);
    if w + b >= 1.0 {
        let gray = w / (w + b);
        return [gray; 3];
    }
    hsl_to_srgb([hue, 100.0, 50.0]).map(|c| c * (1.0 - w - b) + w)
}

/// CSS Color 4 §8.3 `rgbToHwb`.
fn srgb_to_hwb(rgb: Vec3) -> Vec3 {
    let [h, _, _] = srgb_to_hsl(rgb);
    let white = rgb[0].min(rgb[1]).min(rgb[2]);
    let black = 1.0 - rgb[0].max(rgb[1]).max(rgb[2]);
    [h, white * 100.0, black * 100.0]
}

// ── Transfer functions (§10) ─────────────────────────────────────

/// sRGB (and Display P3) gamma to linear light, extended to negative
/// values by symmetry.
fn srgb_to_linear(c: f64) -> f64 {
    let a = c.abs();
    if a <= 0.04045 {
        c / 12.92
    } else {
        c.signum() * ((a + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    let a = c.abs();
    if a > 0.0031308 {
        c.signum() * (1.055 * a.powf(1.0 / 2.4) - 0.055)
    } else {
        12.92 * c
    }
}

fn a98_to_linear(c: f64) -> f64 {
    c.signum() * c.abs().powf(563.0 / 256.0)
}

fn linear_to_a98(c: f64) -> f64 {
    c.signum() * c.abs().powf(256.0 / 563.0)
}

fn prophoto_to_linear(c: f64) -> f64 {
    const ET2: f64 = 16.0 / 512.0;
    let a = c.abs();
    if a <= ET2 {
        c / 16.0
    } else {
        c.signum() * a.powf(1.8)
    }
}

fn linear_to_prophoto(c: f64) -> f64 {
    const ET: f64 = 1.0 / 512.0;
    let a = c.abs();
    if a >= ET {
        c.signum() * a.powf(1.0 / 1.8)
    } else {
        16.0 * c
    }
}

const REC2020_ALPHA: f64 = 1.099_296_826_809_44;
const REC2020_BETA: f64 = 0.018_053_968_510_807;

fn rec2020_to_linear(c: f64) -> f64 {
    let a = c.abs();
    if a < REC2020_BETA * 4.5 {
        c / 4.5
    } else {
        c.signum() * ((a + REC2020_ALPHA - 1.0) / REC2020_ALPHA).powf(1.0 / 0.45)
    }
}

fn linear_to_rec2020(c: f64) -> f64 {
    let a = c.abs();
    if a > REC2020_BETA {
        c.signum() * (REC2020_ALPHA * a.powf(0.45) - (REC2020_ALPHA - 1.0))
    } else {
        4.5 * c
    }
}

// ── Lab / LCH (§9.2, §9.3) ───────────────────────────────────────

/// The D50 white point, as XYZ.
const D50_WHITE: Vec3 = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];
const LAB_KAPPA: f64 = 24389.0 / 27.0;
const LAB_EPSILON: f64 = 216.0 / 24389.0;

fn xyz_d50_to_lab(xyz: Vec3) -> Vec3 {
    let f = [0, 1, 2].map(|i| {
        let v = xyz[i] / D50_WHITE[i];
        if v > LAB_EPSILON {
            v.cbrt()
        } else {
            (LAB_KAPPA * v + 16.0) / 116.0
        }
    });
    [
        116.0 * f[1] - 16.0,
        500.0 * (f[0] - f[1]),
        200.0 * (f[1] - f[2]),
    ]
}

fn lab_to_xyz_d50([l, a, b]: Vec3) -> Vec3 {
    let f1 = (l + 16.0) / 116.0;
    let f0 = a / 500.0 + f1;
    let f2 = f1 - b / 200.0;
    let cube_or_linear = |f: f64| {
        if f.powi(3) > LAB_EPSILON {
            f.powi(3)
        } else {
            (116.0 * f - 16.0) / LAB_KAPPA
        }
    };
    let y = if l > LAB_KAPPA * LAB_EPSILON {
        f1.powi(3)
    } else {
        l / LAB_KAPPA
    };
    [
        cube_or_linear(f0) * D50_WHITE[0],
        y * D50_WHITE[1],
        cube_or_linear(f2) * D50_WHITE[2],
    ]
}

/// `[L, a, b]` to `[L, C, h]` (h in degrees, `[0, 360)`).
fn rect_to_polar([l, a, b]: Vec3) -> Vec3 {
    let c = (a * a + b * b).sqrt();
    [l, c, normalize_hue(b.atan2(a).to_degrees())]
}

/// `[L, C, h]` to `[L, a, b]`; a negative chroma counts as zero.
fn polar_to_rect([l, c, h]: Vec3) -> Vec3 {
    let c = c.max(0.0);
    let h = h.to_radians();
    [l, c * h.cos(), c * h.sin()]
}

// ── Oklab / Oklch (§9.4) ─────────────────────────────────────────

fn xyz_to_oklab(xyz: Vec3) -> Vec3 {
    mul(&LMS_TO_OKLAB, mul(&XYZ_TO_LMS, xyz).map(f64::cbrt))
}

fn oklab_to_xyz(lab: Vec3) -> Vec3 {
    mul(&LMS_TO_XYZ, mul(&OKLAB_TO_LMS, lab).map(|v| v * v * v))
}

fn d65_to_d50(xyz: Vec3) -> Vec3 {
    mul(&D65_TO_D50, xyz)
}

fn d50_to_d65(xyz: Vec3) -> Vec3 {
    mul(&D50_TO_D65, xyz)
}
