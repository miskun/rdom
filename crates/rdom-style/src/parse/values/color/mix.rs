//! `color-mix()` (CSS Color 5 §2).
//!
//! ```text
//! color-mix() = color-mix( [<color-interpolation-method> ,]?
//!                          [ <color> && <percentage [0,100]>? ]#{2} )
//! <color-interpolation-method> = in [ <rectangular-color-space>
//!                                   | <polar-color-space> <hue-interpolation-method>? ]
//! <hue-interpolation-method> = [ shorter | longer | increasing | decreasing ] hue
//! ```
//!
//! Without a method the colors mix in Oklab. The percentages normalize
//! per §2.2: both omitted are 50% each, one omitted is the other's
//! complement, a sum other than 100% scales them, and a sum below 100%
//! scales the result's alpha too; a zero sum is invalid.

use super::channel::split_top_level;
use super::context::ColorCx;
use super::parse_absolute;
use crate::calc::{CalcKind, ResolveCtx};
use crate::color::{AbsoluteColor, ColorSpace, HueMethod, mix};
use crate::parse::token::Token;
use crate::parse::values::calc::{looks_like_calc, parse_math};
use crate::parse::values::numeric::components;

/// Parse the arguments of `color-mix()`.
pub(super) fn parse(args: &[Token], cx: &ColorCx) -> Option<AbsoluteColor> {
    let parts = split_top_level(args, &Token::Comma);
    let (method, colors) = match parts.as_slice() {
        [method, a, b] => (Some(*method), [*a, *b]),
        [a, b] => (None, [*a, *b]),
        _ => return None,
    };
    let (space, hue) = match method {
        Some(tokens) => interpolation_method(tokens)?,
        None => (ColorSpace::Oklab, HueMethod::Shorter),
    };
    let [(c1, p1), (c2, p2)] = [
        color_and_percentage(colors[0], cx)?,
        color_and_percentage(colors[1], cx)?,
    ];
    let (p1, p2) = match (p1, p2) {
        (None, None) => (50.0, 50.0),
        (Some(p1), None) => (p1, 100.0 - p1),
        (None, Some(p2)) => (100.0 - p2, p2),
        (Some(p1), Some(p2)) => (p1, p2),
    };
    let sum = p1 + p2;
    if sum <= 0.0 {
        return None;
    }
    let mut out = mix(c1, c2, space, hue, p2 / sum);
    if sum < 100.0 {
        out.alpha = out.alpha.map(|a| a * sum / 100.0);
    }
    Some(out)
}

/// `in <space> [<hue-method> hue]?`.
fn interpolation_method(tokens: &[Token]) -> Option<(ColorSpace, HueMethod)> {
    let [Token::Ident(kw), Token::Ident(space), rest @ ..] = tokens else {
        return None;
    };
    if !kw.eq_ignore_ascii_case("in") {
        return None;
    }
    let space = ColorSpace::interpolation(space)?;
    let hue = match rest {
        [] => HueMethod::Shorter,
        [Token::Ident(method), Token::Ident(hue)]
            if space.is_polar() && hue.eq_ignore_ascii_case("hue") =>
        {
            HueMethod::from_keyword(method)?
        }
        _ => return None,
    };
    Some((space, hue))
}

/// `<color> && <percentage [0,100]>?`: the color and the percentage
/// (as written, 50% is 50).
fn color_and_percentage(tokens: &[Token], cx: &ColorCx) -> Option<(AbsoluteColor, Option<f64>)> {
    match components(tokens)?.as_slice() {
        [color] => Some((parse_absolute(color, cx)?, None)),
        [a, b] => match (percentage(a), percentage(b)) {
            (Some(p), None) => Some((parse_absolute(b, cx)?, Some(p?))),
            (None, Some(p)) => Some((parse_absolute(a, cx)?, Some(p?))),
            _ => None,
        },
        _ => None,
    }
}

/// `Some(Some(p))` for a `<percentage>` in `0..=100` (a math function's
/// value clamps into it), `Some(None)` for a literal outside it, `None`
/// for no percentage.
fn percentage(component: &[Token]) -> Option<Option<f64>> {
    match component {
        [Token::Percentage(p)] => Some((0.0..=100.0).contains(p).then_some(*p)),
        [Token::Delim('-'), Token::Percentage(_)] => Some(None),
        _ if looks_like_calc(component) => {
            let expr = parse_math(component)?;
            if expr.kind() != Some(CalcKind::Percent) || expr.needs_context() {
                return None;
            }
            let v = expr.resolve_f64(&ResolveCtx::new(100));
            Some(Some(if v.is_nan() { 0.0 } else { v.clamp(0.0, 100.0) }))
        }
        _ => None,
    }
}
