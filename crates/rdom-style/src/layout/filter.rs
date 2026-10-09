//! The filter values (Filter Effects 1 §5–§6; Filter Effects 2 §3):
//! the `<filter-function>`s `filter` and `backdrop-filter` list, and the
//! color math of the color-matrix functions.
//!
//! A cell has one foreground and one background color, so a filter is a
//! function of a color: the color-matrix functions (`grayscale()`,
//! `sepia()`, `saturate()`, `hue-rotate()`, `invert()`, `brightness()`,
//! `contrast()`) map each color the filtered element paints, in sRGB as §5
//! requires of the filter functions; `opacity()` is group opacity;
//! `drop-shadow()` a whole-cell shade (as `box-shadow`'s); `blur()` and
//! `url()` are kept and draw nothing (DIVERGENCES §1).

use std::sync::Arc;

use super::{BoxShadow, PaintLength};
use crate::Color;

/// One `<filter-function>` or `url()` (Filter Effects 1 §6). The amounts
/// are computed numbers (a percentage divided by 100; `grayscale`,
/// `sepia`, `invert` and `opacity` clamped to 1), `hue-rotate()` in
/// degrees. `C` is the drop shadow's color: [`TuiColor`](crate::TuiColor)
/// as declared, [`Color`] once computed.
///
/// Open (DESIGN, `#[non_exhaustive]`): Filter Effects 2 adds functions,
/// and a reader that meets one it does not know leaves the colors as they
/// are — as rdom does `blur()` and `url()`. rdom-tui reads a list only
/// through [`FilterList`]'s methods.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FilterFunction<C = crate::TuiColor> {
    /// `blur(<length>)`: kept, inert — a cell cannot blur.
    Blur(PaintLength),
    Brightness(f64),
    Contrast(f64),
    Grayscale(f64),
    /// `hue-rotate(<angle>)`, in degrees.
    HueRotate(f64),
    Invert(f64),
    /// `opacity()`: group opacity.
    Opacity(f64),
    Saturate(f64),
    Sepia(f64),
    /// `drop-shadow(<color>? <length>{2,3})`: offsets, blur (inert) and
    /// color; never `inset`, spread 0.
    DropShadow(BoxShadow<C>),
    /// `url(…)`: an SVG filter reference — kept, inert.
    Url(Arc<str>),
}

impl<C> FilterFunction<C> {
    /// The function with its drop shadow's color mapped by `f` (`None`
    /// when `f` gives none).
    pub fn map_color<D>(self, f: impl FnOnce(C) -> Option<D>) -> Option<FilterFunction<D>> {
        use FilterFunction as F;
        Some(match self {
            F::Blur(l) => F::Blur(l),
            F::Brightness(a) => F::Brightness(a),
            F::Contrast(a) => F::Contrast(a),
            F::Grayscale(a) => F::Grayscale(a),
            F::HueRotate(a) => F::HueRotate(a),
            F::Invert(a) => F::Invert(a),
            F::Opacity(a) => F::Opacity(a),
            F::Saturate(a) => F::Saturate(a),
            F::Sepia(a) => F::Sepia(a),
            F::DropShadow(s) => {
                let mut color = None;
                let shadow = s.with_color(|c| color = f(c));
                let color = color?;
                F::DropShadow(shadow.with_color(|_| color))
            }
            F::Url(u) => F::Url(u),
        })
    }
}

/// A `filter` / `backdrop-filter` value: `none`, or the functions applied
/// in order (Filter Effects 1 §5).
///
/// Closed (DESIGN): a value record read through its accessors; the
/// functions behind an `Arc`, so a style clones it without allocating.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterList<C = crate::TuiColor>(Option<Arc<[FilterFunction<C>]>>);

impl<C> Default for FilterList<C> {
    fn default() -> Self {
        FilterList(None)
    }
}

impl<C> FilterList<C> {
    /// `none`.
    pub fn none() -> Self {
        FilterList(None)
    }

    /// The list of `functions`; `none` when there are none.
    pub fn new(functions: Vec<FilterFunction<C>>) -> Self {
        if functions.is_empty() {
            FilterList(None)
        } else {
            FilterList(Some(functions.into()))
        }
    }

    /// Whether it is `none`.
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// The functions, in order (empty for `none`).
    pub fn functions(&self) -> &[FilterFunction<C>] {
        self.0.as_deref().unwrap_or(&[])
    }
}

impl<C: Clone> FilterList<C> {
    /// The list with its drop shadows' colors mapped by `f`; `None` when
    /// `f` resolves one to none.
    pub fn map_colors<D>(&self, f: impl Fn(C) -> Option<D>) -> Option<FilterList<D>> {
        let functions = self
            .functions()
            .iter()
            .map(|g| g.clone().map_color(&f))
            .collect::<Option<Vec<_>>>()?;
        Some(FilterList::new(functions))
    }
}

/// A color-matrix step of a filter, on sRGB channels in `[0, 1]`.
#[derive(Debug, Clone, Copy)]
enum Step {
    Matrix([[f64; 3]; 3]),
    /// Each channel `slope · c + intercept` (`feComponentTransfer`'s
    /// linear function).
    Linear(f64, f64),
}

impl Step {
    fn apply(self, c: [f64; 3]) -> [f64; 3] {
        let out = match self {
            Step::Matrix(m) => [0, 1, 2].map(|i| m[i][0] * c[0] + m[i][1] * c[1] + m[i][2] * c[2]),
            Step::Linear(slope, intercept) => c.map(|v| slope * v + intercept),
        };
        // Each primitive's result is clamped to the color range.
        out.map(|v| v.clamp(0.0, 1.0))
    }
}

/// `feColorMatrix type="saturate"` primitive with `s`.
fn saturate(s: f64) -> [[f64; 3]; 3] {
    [
        [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
    ]
}

impl<C> FilterFunction<C> {
    /// The function as a color step (§6's SVG equivalents), `None` for the
    /// functions that do not map colors.
    fn step(&self) -> Option<Step> {
        use FilterFunction as F;
        Some(match *self {
            F::Grayscale(a) => {
                let k = 1.0 - a;
                Step::Matrix([
                    [
                        0.2126 + 0.7874 * k,
                        0.7152 - 0.7152 * k,
                        0.0722 - 0.0722 * k,
                    ],
                    [
                        0.2126 - 0.2126 * k,
                        0.7152 + 0.2848 * k,
                        0.0722 - 0.0722 * k,
                    ],
                    [
                        0.2126 - 0.2126 * k,
                        0.7152 - 0.7152 * k,
                        0.0722 + 0.9278 * k,
                    ],
                ])
            }
            F::Sepia(a) => {
                let k = 1.0 - a;
                Step::Matrix([
                    [0.393 + 0.607 * k, 0.769 - 0.769 * k, 0.189 - 0.189 * k],
                    [0.349 - 0.349 * k, 0.686 + 0.314 * k, 0.168 - 0.168 * k],
                    [0.272 - 0.272 * k, 0.534 - 0.534 * k, 0.131 + 0.869 * k],
                ])
            }
            F::Saturate(a) => Step::Matrix(saturate(a)),
            F::HueRotate(deg) => {
                // `feColorMatrix type="hueRotate"` primitive.
                let (s, c) = deg.to_radians().sin_cos();
                Step::Matrix([
                    [
                        0.213 + c * 0.787 - s * 0.213,
                        0.715 - c * 0.715 - s * 0.715,
                        0.072 - c * 0.072 + s * 0.928,
                    ],
                    [
                        0.213 - c * 0.213 + s * 0.143,
                        0.715 + c * 0.285 + s * 0.140,
                        0.072 - c * 0.072 - s * 0.283,
                    ],
                    [
                        0.213 - c * 0.213 - s * 0.787,
                        0.715 - c * 0.715 + s * 0.715,
                        0.072 + c * 0.928 + s * 0.072,
                    ],
                ])
            }
            F::Invert(a) => Step::Linear(1.0 - 2.0 * a, a),
            F::Brightness(a) => Step::Linear(a, 0.0),
            F::Contrast(a) => Step::Linear(a, 0.5 - 0.5 * a),
            F::Blur(_) | F::Opacity(_) | F::DropShadow(_) | F::Url(_) => return None,
        })
    }
}

impl<C> FilterList<C> {
    /// Whether a function maps colors (§6's color-matrix functions): the
    /// filter has a color effect to apply.
    pub fn maps_colors(&self) -> bool {
        self.functions().iter().any(|f| f.step().is_some())
    }

    /// The product of its `opacity()` amounts: the group opacity it adds.
    pub fn opacity(&self) -> f32 {
        self.functions()
            .iter()
            .map(|f| match f {
                FilterFunction::Opacity(a) => *a as f32,
                _ => 1.0,
            })
            .product()
    }

    /// The opaque sRGB color `rgb` through every color-matrix function
    /// from the `from`-th on, in order, rounded once to 8-bit channels.
    pub fn filter_rgb_from(&self, from: usize, rgb: (u8, u8, u8)) -> (u8, u8, u8) {
        let mut c = [rgb.0, rgb.1, rgb.2].map(|v| f64::from(v) / 255.0);
        for step in self.functions().iter().skip(from).filter_map(|f| f.step()) {
            c = step.apply(c);
        }
        let [r, g, b] = c.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8);
        (r, g, b)
    }

    /// [`Self::filter_rgb_from`] the first function on.
    pub fn filter_rgb(&self, rgb: (u8, u8, u8)) -> (u8, u8, u8) {
        self.filter_rgb_from(0, rgb)
    }
}

impl FilterList<Color> {
    /// Its drop shadows, each with its index in the list (the functions
    /// after it filter the shadow too, §5).
    pub fn drop_shadows(&self) -> impl Iterator<Item = (usize, &BoxShadow<Color>)> {
        self.functions()
            .iter()
            .enumerate()
            .filter_map(|(i, f)| match f {
                FilterFunction::DropShadow(s) => Some((i, s)),
                _ => None,
            })
    }
}
