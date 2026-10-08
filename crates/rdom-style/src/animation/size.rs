//! Sizing-property interpolation with sizing keywords (CSS Values 5
//! §11): under `interpolate-size: allow-keywords` a keyword of `width` /
//! `height`, `min-*`, `max-*` or `flex-basis` interpolates with a length
//! through `calc-size()`, and a `calc-size()` interpolates with a length
//! or a `calc-size()` of the same basis whatever it says.

use std::sync::Arc;

use super::length::{LengthPercentage, mix};
use super::value::{Animate, Cx, discrete, lerp};
use crate::calc::CalcExpr;
use crate::layout::{CalcSize, CalcSizeBasis, FlexBasis, InterpolateSize, MaxSize, MinSize, Size};

/// A sizing property's value as `calc-size()` sees it (§10–§11).
pub(crate) trait Sizing: Animate + LengthPercentage {
    /// The sizing keyword it is, as a `calc-size()` basis.
    fn keyword(&self) -> Option<CalcSizeBasis>;
    /// Its `calc-size()`, when it is one.
    fn calc_size(&self) -> Option<&CalcSize>;
    /// The value `c` is: `c` itself, or its basis keyword when the sum is
    /// `size` alone.
    fn from_calc_size(c: CalcSize) -> Self;
}

macro_rules! sizing {
    ($t:ident, $basis:ident, { $($kw:pat => $b:expr),* $(,)? }) => {
        impl Sizing for $t {
            fn keyword(&self) -> Option<CalcSizeBasis> {
                match self {
                    $($kw => Some($b),)*
                    _ => None,
                }
            }
            fn calc_size(&self) -> Option<&CalcSize> {
                match self {
                    $t::CalcSize(c) => Some(c),
                    _ => None,
                }
            }
            fn from_calc_size(c: CalcSize) -> Self {
                if c.factor == 1.0 && c.offset.linear_parts() == Some((0.0, 0.0)) {
                    c.$basis()
                } else {
                    $t::CalcSize(Arc::new(c))
                }
            }
        }
    };
}

sizing!(Size, basis_size, {
    Size::Auto => CalcSizeBasis::Auto,
    Size::Intrinsic(k) => CalcSizeBasis::Intrinsic(k.clone()),
});
sizing!(MinSize, basis_min_size, {
    MinSize::Auto => CalcSizeBasis::Auto,
    MinSize::Intrinsic(k) => CalcSizeBasis::Intrinsic(k.clone()),
});
sizing!(MaxSize, basis_max_size, {
    MaxSize::Intrinsic(k) => CalcSizeBasis::Intrinsic(k.clone()),
});
sizing!(FlexBasis, basis_flex_basis, {
    FlexBasis::Auto => CalcSizeBasis::Auto,
    FlexBasis::Content => CalcSizeBasis::Content,
    FlexBasis::Intrinsic(k) => CalcSizeBasis::Intrinsic(k.clone()),
});

/// A size as `calc-size(basis, size * factor + offset)`, `basis` `None`
/// for `any` (a plain length or percentage).
struct Form {
    basis: Option<CalcSizeBasis>,
    factor: f64,
    offset: CalcExpr,
}

fn form<T: Sizing>(s: &T, keywords: bool) -> Option<Form> {
    if let Some(c) = s.calc_size() {
        return Some(Form {
            basis: Some(c.basis.clone()),
            factor: c.factor,
            offset: c.offset.clone(),
        });
    }
    match s.keyword() {
        Some(basis) if keywords => Some(Form {
            basis: Some(basis),
            factor: 1.0,
            offset: CalcExpr::Number(0.0),
        }),
        Some(_) => None,
        None => Some(Form {
            basis: None,
            factor: 0.0,
            offset: s.expr()?,
        }),
    }
}

/// `from` → `to` at `p`, or `None` when the pair does not interpolate.
pub(crate) fn animate_size<T: Sizing>(
    from: &T,
    to: &T,
    p: f64,
    interpolate_size: InterpolateSize,
    cx: &Cx,
) -> Option<T> {
    let calc_size = from.calc_size().is_some() || to.calc_size().is_some();
    let keywords = interpolate_size == InterpolateSize::AllowKeywords;
    if !calc_size && !(keywords && (from.keyword().is_some() || to.keyword().is_some())) {
        return from.animate(to, p, cx);
    }
    let (a, b) = (form(from, keywords)?, form(to, keywords)?);
    let basis = match (a.basis, b.basis) {
        (Some(x), Some(y)) if x == y => x,
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => return from.animate(to, p, cx),
        _ => return None,
    };
    let factor = lerp(a.factor, b.factor, p);
    let offset = mix(&a.offset, &b.offset, p);
    if factor == 0.0 {
        return Some(T::from_mixed(offset));
    }
    Some(T::from_calc_size(CalcSize::new(
        basis,
        factor,
        offset.into_expr(),
    )))
}

/// [`animate_size`], discretely where the pair does not interpolate.
pub(crate) fn blend_size<T: Sizing>(
    from: &T,
    to: &T,
    p: f64,
    interpolate_size: InterpolateSize,
    cx: &Cx,
) -> T {
    animate_size(from, to, p, interpolate_size, cx).unwrap_or_else(|| discrete(from, to, p))
}

/// Whether the pair interpolates.
pub(crate) fn size_interpolable<T: Sizing>(
    from: &T,
    to: &T,
    interpolate_size: InterpolateSize,
) -> bool {
    let cx = Cx {
        reset: crate::Color::Reset,
    };
    animate_size(from, to, 0.5, interpolate_size, &cx).is_some()
}
