//! CSS gamut mapping into sRGB (CSS Color 4 §13.2): a color outside
//! sRGB keeps its OKLCh lightness and hue and loses chroma, by binary
//! search, until clipping it is within a just-noticeable difference of
//! it — rather than each channel clipping on its own, which shifts the
//! hue and lightness.

use super::absolute::{AbsoluteColor, ColorSpace};
use super::convert::{convert, from_xyz_d65, to_xyz_d65};

/// The just-noticeable difference, in deltaEOK (§13.2.2).
const JND: f64 = 0.02;
/// The chroma search's resolution (§13.2.2).
const EPSILON: f64 = 0.0001;
/// How far past `0..=1` a channel may be and count as in gamut: the
/// rounding of the conversions themselves (the spec's sample code uses
/// the same tolerance).
const GAMUT_TOLERANCE: f64 = 0.000_075;

type Vec3 = [f64; 3];

/// `color` as sRGB components within `0..=1` (§13.2.2 "CSS gamut
/// mapping to an RGB destination").
pub(crate) fn map_to_srgb(color: AbsoluteColor) -> Vec3 {
    // `convert` takes sRGB, HSL and HWB directly, without the round
    // trip through XYZ, whose rounding would move a channel off an
    // exact half (CSS rounds 127.5 up).
    let rgb = convert(color, ColorSpace::Srgb).values();
    if in_gamut(rgb) {
        return rgb.map(clamp_unit);
    }
    let xyz = to_xyz_d65(color.space, color.values());
    let [l, c, h] = from_xyz_d65(ColorSpace::Oklch, xyz);
    if l.is_nan() || l <= 0.0 {
        return [0.0; 3];
    }
    if l >= 1.0 {
        return [1.0; 3];
    }
    let srgb_of = |chroma: f64| {
        from_xyz_d65(
            ColorSpace::Srgb,
            to_xyz_d65(ColorSpace::Oklch, [l, chroma, h]),
        )
    };
    let clip = |rgb: Vec3| rgb.map(clamp_unit);
    let delta = |clipped: Vec3, chroma: f64| {
        delta_eok(
            from_xyz_d65(ColorSpace::Oklab, to_xyz_d65(ColorSpace::Srgb, clipped)),
            from_xyz_d65(
                ColorSpace::Oklab,
                to_xyz_d65(ColorSpace::Oklch, [l, chroma, h]),
            ),
        )
    };
    let mut clipped = clip(rgb);
    if delta(clipped, c) < JND {
        return clipped;
    }
    let (mut min, mut max, mut min_in_gamut) = (0.0, c, true);
    while max - min > EPSILON {
        let chroma = (min + max) / 2.0;
        let current = srgb_of(chroma);
        if min_in_gamut && in_gamut(current) {
            min = chroma;
            continue;
        }
        clipped = clip(current);
        let e = delta(clipped, chroma);
        if e < JND {
            if JND - e < EPSILON {
                break;
            }
            min_in_gamut = false;
            min = chroma;
        } else {
            max = chroma;
        }
    }
    clipped
}

fn in_gamut(rgb: Vec3) -> bool {
    rgb.iter()
        .all(|c| (-GAMUT_TOLERANCE..=1.0 + GAMUT_TOLERANCE).contains(c))
}

fn clamp_unit(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

/// deltaEOK (§13.2.1): Euclidean distance in Oklab.
fn delta_eok(a: Vec3, b: Vec3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
