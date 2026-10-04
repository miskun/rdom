//! `lab()` / `lch()` / `oklab()` / `oklch()` (CSS Color 4 §9) and
//! `color()` (§10).
//!
//! ```text
//! lab()   = lab( [<percentage> | <number> | none]{3} [ / [<alpha-value> | none] ]? )
//! lch()   = lch( [<percentage> | <number> | none]{2} [<hue> | none] [ / … ]? )
//! oklab() / oklch() likewise
//! color() = color( <colorspace> [<number> | <percentage> | none]{3} [ / … ]? )
//! ```
//!
//! No legacy (comma) syntax. Lightness clamps to `0..=100` (Lab) or
//! `0..=1` (Oklab) and chroma to `≥ 0` at parsed-value time; the
//! other channels are unbounded.

use super::channel::{Arguments, Channel, alpha, hue, split_arguments};
use crate::color::{AbsoluteColor, ColorSpace};
use crate::parse::token::Token;

/// Parse the arguments of `lab()` / `lch()` / `oklab()` / `oklch()`;
/// `space` is the function's.
pub(super) fn parse_lab(args: &[Token], space: ColorSpace) -> Option<AbsoluteColor> {
    let Arguments::Modern { channels, alpha } = split_arguments(args)? else {
        return None;
    };
    let [l, a, b] = channels.as_slice() else {
        return None;
    };
    // What 100% is, per channel (§9.1, §9.4).
    let (l_max, ab_ref, c_ref) = match space {
        ColorSpace::Lab | ColorSpace::Lch => (100.0, 125.0, 150.0),
        _ => (1.0, 0.4, 0.4),
    };
    let l = Channel::parse(l)?
        .resolve(l_max)
        .map(|l| l.clamp(0.0, l_max));
    let coords = match space {
        ColorSpace::Lch | ColorSpace::Oklch => [
            l,
            Channel::parse(a)?.resolve(c_ref).map(|c| c.max(0.0)),
            hue(b, true)?,
        ],
        _ => [
            l,
            Channel::parse(a)?.resolve(ab_ref),
            Channel::parse(b)?.resolve(ab_ref),
        ],
    };
    Some(AbsoluteColor {
        space,
        coords,
        alpha: alpha_of(alpha)?,
    })
}

/// Parse the arguments of `color()`: a predefined space (§10.1) and
/// its three channels, `100%` being 1.
pub(super) fn parse_color_function(args: &[Token]) -> Option<AbsoluteColor> {
    let Arguments::Modern { channels, alpha } = split_arguments(args)? else {
        return None;
    };
    let [[Token::Ident(name)], c0, c1, c2] = channels.as_slice() else {
        return None;
    };
    let space = ColorSpace::predefined(name)?;
    let coords = [
        Channel::parse(c0)?.resolve(1.0),
        Channel::parse(c1)?.resolve(1.0),
        Channel::parse(c2)?.resolve(1.0),
    ];
    Some(AbsoluteColor {
        space,
        coords,
        alpha: alpha_of(alpha)?,
    })
}

/// The `/ alpha` component (opaque when absent), clamped to `0..=1`.
fn alpha_of(component: Option<&[Token]>) -> Option<Option<f64>> {
    Some(match component {
        Some(c) => alpha(c, true)?.map(|a| a.clamp(0.0, 1.0)),
        None => Some(1.0),
    })
}
