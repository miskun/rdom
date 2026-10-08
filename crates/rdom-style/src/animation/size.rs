//! `width` / `height` interpolation with sizing keywords (CSS Values 5
//! §11): under `interpolate-size: allow-keywords` a keyword interpolates
//! with a length through `calc-size()`, and a `calc-size()` interpolates
//! with a length or a `calc-size()` of the same basis whatever it says.

use std::sync::Arc;

use super::length::{LengthPercentage, mix};
use super::value::{Animate, Cx, discrete, lerp};
use crate::calc::CalcExpr;
use crate::layout::{CalcSize, CalcSizeBasis, InterpolateSize, Size};

/// A size as `calc-size(basis, size * factor + offset)`, `basis` `None`
/// for `any` (a plain length or percentage).
struct Form {
    basis: Option<CalcSizeBasis>,
    factor: f64,
    offset: CalcExpr,
}

fn form(s: &Size, keywords: bool) -> Option<Form> {
    let keyword = |basis| Form {
        basis: Some(basis),
        factor: 1.0,
        offset: CalcExpr::Number(0.0),
    };
    Some(match s {
        Size::CalcSize(c) => Form {
            basis: Some(c.basis.clone()),
            factor: c.factor,
            offset: c.offset.clone(),
        },
        Size::Auto if keywords => keyword(CalcSizeBasis::Auto),
        Size::Intrinsic(k) if keywords => keyword(CalcSizeBasis::Intrinsic(k.clone())),
        other => Form {
            basis: None,
            factor: 0.0,
            offset: other.expr()?,
        },
    })
}

/// `from` → `to` at `p`, or `None` when the pair does not interpolate.
pub(crate) fn animate_size(
    from: &Size,
    to: &Size,
    p: f64,
    interpolate_size: InterpolateSize,
    cx: &Cx,
) -> Option<Size> {
    let calc_size = matches!(from, Size::CalcSize(_)) || matches!(to, Size::CalcSize(_));
    let keywords = interpolate_size == InterpolateSize::AllowKeywords;
    if !calc_size && !(keywords && (is_keyword(from) || is_keyword(to))) {
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
        return Some(Size::from_mixed(offset));
    }
    let offset = offset.into_expr();
    if factor == 1.0 && offset.linear_parts() == Some((0.0, 0.0)) {
        return Some(CalcSize::new(basis, 1.0, offset).basis_size());
    }
    Some(Size::CalcSize(Arc::new(CalcSize::new(
        basis, factor, offset,
    ))))
}

fn is_keyword(s: &Size) -> bool {
    matches!(s, Size::Auto | Size::Intrinsic(_))
}

/// [`animate_size`], discretely where the pair does not interpolate.
pub(crate) fn blend_size(
    from: &Size,
    to: &Size,
    p: f64,
    interpolate_size: InterpolateSize,
    cx: &Cx,
) -> Size {
    animate_size(from, to, p, interpolate_size, cx).unwrap_or_else(|| discrete(from, to, p))
}

/// Whether the pair interpolates.
pub(crate) fn size_interpolable(from: &Size, to: &Size, interpolate_size: InterpolateSize) -> bool {
    let cx = Cx {
        reset: crate::Color::Reset,
    };
    animate_size(from, to, 0.5, interpolate_size, &cx).is_some()
}
