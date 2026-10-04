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
    /// The arc of at least 180°.
    Longer,
    /// Hue only increases.
    Increasing,
    /// Hue only decreases.
    Decreasing,
}

impl HueMethod {
    /// The method a `<hue-interpolation-method>` keyword names, ASCII
    /// case-insensitive.
    pub fn from_keyword(name: &str) -> Option<HueMethod> {
        const TABLE: &[(&str, HueMethod)] = &[
            ("shorter", HueMethod::Shorter),
            ("longer", HueMethod::Longer),
            ("increasing", HueMethod::Increasing),
            ("decreasing", HueMethod::Decreasing),
        ];
        TABLE
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, m)| *m)
    }
}

/// The kinds of component Color 4 §12.2 calls analogous: a missing one
/// stays missing through a conversion to a space with the same kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Analog {
    Red,
    Green,
    Blue,
    Lightness,
    Colorfulness,
    Hue,
    OpponentA,
    OpponentB,
}

impl ColorSpace {
    /// The space a `<color-interpolation-method>` names (CSS Color 4
    /// §12.1), ASCII case-insensitive.
    pub fn interpolation(name: &str) -> Option<ColorSpace> {
        const TABLE: &[(&str, ColorSpace)] = &[
            ("lab", ColorSpace::Lab),
            ("oklab", ColorSpace::Oklab),
            ("hsl", ColorSpace::Hsl),
            ("hwb", ColorSpace::Hwb),
            ("lch", ColorSpace::Lch),
            ("oklch", ColorSpace::Oklch),
        ];
        TABLE
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, s)| *s)
            .or_else(|| ColorSpace::predefined(name))
    }

    /// True for the cylindrical spaces, which have a hue.
    pub fn is_polar(self) -> bool {
        self.hue_index().is_some()
    }

    /// The index of the hue component in a polar space.
    fn hue_index(self) -> Option<usize> {
        match self {
            ColorSpace::Hsl | ColorSpace::Hwb => Some(0),
            ColorSpace::Lch | ColorSpace::Oklch => Some(2),
            _ => None,
        }
    }

    /// What kind each component is (§12.2).
    fn analogs(self) -> [Option<Analog>; 3] {
        use Analog::*;
        match self {
            ColorSpace::Srgb
            | ColorSpace::SrgbLinear
            | ColorSpace::DisplayP3
            | ColorSpace::A98Rgb
            | ColorSpace::ProphotoRgb
            | ColorSpace::Rec2020
            | ColorSpace::XyzD50
            | ColorSpace::XyzD65 => [Some(Red), Some(Green), Some(Blue)],
            ColorSpace::Hsl => [Some(Hue), Some(Colorfulness), Some(Lightness)],
            ColorSpace::Hwb => [Some(Hue), None, None],
            ColorSpace::Lab | ColorSpace::Oklab => {
                [Some(Lightness), Some(OpponentA), Some(OpponentB)]
            }
            ColorSpace::Lch | ColorSpace::Oklch => [Some(Lightness), Some(Colorfulness), Some(Hue)],
        }
    }
}

/// Interpolate from `a` (`t = 0`) to `b` (`t = 1`) in `space`
/// (CSS Color 4 §12.1–§12.4). The result is in `space`.
pub(crate) fn mix(
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

/// `color` in `space`. A component missing in `color` stays missing in
/// its analogous component (§12.2: "carried forward"), and a hue that
/// is powerless after the conversion (an achromatic color) is missing,
/// so it takes the other color's.
fn in_space(color: AbsoluteColor, space: ColorSpace) -> AbsoluteColor {
    if color.space == space {
        return color;
    }
    let mut out = convert(color, space);
    let missing: Vec<Analog> = (0..3)
        .filter(|&i| color.coords[i].is_none())
        .filter_map(|i| color.space.analogs()[i])
        .collect();
    for (i, analog) in space.analogs().into_iter().enumerate() {
        if analog.is_some_and(|a| missing.contains(&a)) {
            out.coords[i] = None;
        }
    }
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
        HueMethod::Longer => {
            if 0.0 < d && d < 180.0 {
                h1 += 360.0;
            } else if -180.0 < d && d <= 0.0 {
                h2 += 360.0;
            }
        }
        HueMethod::Increasing => {
            if h2 < h1 {
                h2 += 360.0;
            }
        }
        HueMethod::Decreasing => {
            if h1 < h2 {
                h1 += 360.0;
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
/// premultiplied alpha; the result is gamut-mapped to 8-bit sRGB. A
/// palette index counts as its xterm color; `None` when either color is
/// the terminal default (`reset`), which has no one sRGB value.
pub fn interpolate_oklab(from: Color, to: Color, t: f64) -> Option<Color> {
    let a = AbsoluteColor::from_color(from)?;
    let b = AbsoluteColor::from_color(to)?;
    Some(mix(a, b, ColorSpace::Oklab, HueMethod::Shorter, t).to_color())
}
