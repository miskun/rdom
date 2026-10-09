//! Interpolation of `filter` / `backdrop-filter` lists (Filter Effects 1
//! §14 "Animation of Filters"): two lists whose functions pair up — one the
//! other's prefix, a missing function its initial value for interpolation,
//! `none` a list of those initial values — interpolate function by
//! function; any other pair (a `url()` among them) is discrete. Addition
//! concatenates.

use super::value::{Animate, Cx, lerp};
use crate::Color;
use crate::layout::{BoxShadow, FilterFunction, FilterList, PaintLength};

/// The function's initial value for interpolation (§14.1): the identity.
fn identity(f: &FilterFunction<Color>) -> Option<FilterFunction<Color>> {
    use FilterFunction as F;
    Some(match f {
        F::Blur(_) => F::Blur(PaintLength::Cells(0.0)),
        F::Brightness(_) => F::Brightness(1.0),
        F::Contrast(_) => F::Contrast(1.0),
        F::Grayscale(_) => F::Grayscale(0.0),
        F::HueRotate(_) => F::HueRotate(0.0),
        F::Invert(_) => F::Invert(0.0),
        F::Opacity(_) => F::Opacity(1.0),
        F::Saturate(_) => F::Saturate(1.0),
        F::Sepia(_) => F::Sepia(0.0),
        F::DropShadow(_) => F::DropShadow(BoxShadow {
            inset: false,
            offset_x: PaintLength::Cells(0.0),
            offset_y: PaintLength::Cells(0.0),
            blur: PaintLength::Cells(0.0),
            spread: PaintLength::Cells(0.0),
            color: Color::TRANSPARENT,
        }),
        F::Url(_) => return None,
    })
}

/// One pair of functions of the same kind, interpolated (amounts clamped
/// to their ranges: an overshooting easing stays valid).
fn pair(
    a: &FilterFunction<Color>,
    b: &FilterFunction<Color>,
    p: f64,
    cx: &Cx,
) -> Option<FilterFunction<Color>> {
    use FilterFunction as F;
    let amount = |x: f64, y: f64, max: f64| lerp(x, y, p).clamp(0.0, max);
    let inf = f64::INFINITY;
    Some(match (a, b) {
        (F::Blur(x), F::Blur(y)) => F::Blur(x.animate(y, p, cx)?),
        (F::Brightness(x), F::Brightness(y)) => F::Brightness(amount(*x, *y, inf)),
        (F::Contrast(x), F::Contrast(y)) => F::Contrast(amount(*x, *y, inf)),
        (F::Saturate(x), F::Saturate(y)) => F::Saturate(amount(*x, *y, inf)),
        (F::Grayscale(x), F::Grayscale(y)) => F::Grayscale(amount(*x, *y, 1.0)),
        (F::Sepia(x), F::Sepia(y)) => F::Sepia(amount(*x, *y, 1.0)),
        (F::Invert(x), F::Invert(y)) => F::Invert(amount(*x, *y, 1.0)),
        (F::Opacity(x), F::Opacity(y)) => F::Opacity(amount(*x, *y, 1.0)),
        (F::HueRotate(x), F::HueRotate(y)) => F::HueRotate(lerp(*x, *y, p)),
        (F::DropShadow(x), F::DropShadow(y)) => {
            let mut shadows = vec![x.clone()].animate(&vec![y.clone()], p, cx)?;
            F::DropShadow(shadows.pop()?)
        }
        _ => return None,
    })
}

impl Animate for FilterList<Color> {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        let (a, b) = (self.functions(), to.functions());
        if a.is_empty() && b.is_empty() {
            return Some(FilterList::none());
        }
        let n = a.len().max(b.len());
        let functions = (0..n)
            .map(|i| {
                let (x, y) = match (a.get(i), b.get(i)) {
                    (Some(x), Some(y)) => (x.clone(), y.clone()),
                    (Some(x), None) => (x.clone(), identity(x)?),
                    (None, Some(y)) => (identity(y)?, y.clone()),
                    (None, None) => return None,
                };
                pair(&x, &y, p, cx)
            })
            .collect::<Option<Vec<_>>>()?;
        Some(FilterList::new(functions))
    }
    /// Filter Effects 1 §14.2: addition appends the lists.
    fn add(&self, other: &Self, _: &Cx) -> Option<Self> {
        let functions = self
            .functions()
            .iter()
            .chain(other.functions())
            .cloned()
            .collect();
        Some(FilterList::new(functions))
    }
}
