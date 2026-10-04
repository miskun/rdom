//! Color interpolation (CSS Color 4 §12): both colors converted to the
//! interpolation space, missing components taking the other color's
//! value, premultiplied alpha, and — in a polar space — the hue
//! interpolated along the chosen arc.

use super::Color;
use super::absolute::{AbsoluteColor, ColorSpace};
use super::convert::{convert, normalize_hue};

/// How a polar space interpolates its hue (CSS Color 4 §12.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum HueMethod {
    /// The arc of at most 180°.
    #[default]
    Shorter,
}

impl ColorSpace {
    /// The index of the hue component in a polar space.
    fn hue_index(self) -> Option<usize> {
        match self {
            ColorSpace::Hsl | ColorSpace::Hwb => Some(0),
            ColorSpace::Lch | ColorSpace::Oklch => Some(2),
            _ => None,
        }
    }
}

/// Interpolate from `a` (`t = 0`) to `b` (`t = 1`) in `space`
/// (CSS Color 4 §12.1–§12.4). The result is in `space`.
pub(crate) fn interpolate(
    a: AbsoluteColor,
    b: AbsoluteColor,
    space: ColorSpace,
    hue: HueMethod,
    t: f64,
) -> AbsoluteColor {
    let (a, b) = (in_space(a, space), in_space(b, space));
    let hue_index = space.hue_index();
    // §12.2: a component missing in one color takes the other's value.
    let mut ca = a.coords;
    let mut cb = b.coords;
    for i in 0..3 {
        match (ca[i], cb[i]) {
            (None, Some(v)) => ca[i] = Some(v),
            (Some(v), None) => cb[i] = Some(v),
            _ => {}
        }
    }
    let (alpha_a, alpha_b) = match (a.alpha, b.alpha) {
        (None, None) => (None, None),
        (x, y) => (x.or(y), y.or(x)),
    };
    let alpha = match (alpha_a, alpha_b) {
        (Some(x), Some(y)) => Some(lerp(x, y, t)),
        _ => None,
    };
    // §12.3: premultiply every component but the hue.
    let (pa, pb) = (alpha_a.unwrap_or(1.0), alpha_b.unwrap_or(1.0));
    let mut coords = [None; 3];
    for i in 0..3 {
        let (Some(x), Some(y)) = (ca[i], cb[i]) else {
            continue;
        };
        coords[i] = Some(if Some(i) == hue_index {
            let (x, y) = fix_hues(normalize_hue(x), normalize_hue(y), hue);
            normalize_hue(lerp(x, y, t))
        } else {
            let mixed = lerp(x * pa, y * pb, t);
            match alpha {
                Some(al) if al != 0.0 => mixed / al,
                _ => mixed,
            }
        });
    }
    AbsoluteColor {
        space,
        coords,
        alpha,
    }
}

/// `color` in `space`. A hue that is powerless after the conversion (an
/// achromatic color) is missing (§12.2), so it takes the other color's.
fn in_space(color: AbsoluteColor, space: ColorSpace) -> AbsoluteColor {
    if color.space == space {
        return color;
    }
    let mut out = convert(color, space);
    let [c0, c1, c2] = out.values();
    let powerless = match space {
        ColorSpace::Hsl => c1.abs() < 1e-6 || c2 <= 1e-6 || c2 >= 100.0 - 1e-6,
        ColorSpace::Hwb => c1 + c2 >= 100.0 - 1e-6,
        ColorSpace::Lch => c1 < 0.0015 * 150.0 / 0.4 / 100.0 || c0 <= 0.0,
        ColorSpace::Oklch => c1 < 0.000_4 || c0 <= 0.0,
        _ => false,
    };
    if let (true, Some(i)) = (powerless, space.hue_index()) {
        out.coords[i] = None;
    }
    out
}

/// The two hues adjusted so a linear interpolation follows `method`'s
/// arc (§12.4).
fn fix_hues(mut h1: f64, mut h2: f64, method: HueMethod) -> (f64, f64) {
    let d = h2 - h1;
    match method {
        HueMethod::Shorter => {
            if d > 180.0 {
                h1 += 360.0;
            } else if d < -180.0 {
                h2 += 360.0;
            }
        }
    }
    (h1, h2)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Interpolate two sRGB colors in Oklab (CSS Color 4 §12.1: the
/// default space for interpolating colors, as transitions do), with
/// premultiplied alpha; the result is gamut-mapped to 8-bit sRGB.
/// `None` when either color is the terminal default or a palette
/// index, which have no sRGB value here.
pub fn interpolate_oklab(from: Color, to: Color, t: f64) -> Option<Color> {
    let a = AbsoluteColor::from_color(from)?;
    let b = AbsoluteColor::from_color(to)?;
    Some(interpolate(a, b, ColorSpace::Oklab, HueMethod::Shorter, t).to_color())
}
