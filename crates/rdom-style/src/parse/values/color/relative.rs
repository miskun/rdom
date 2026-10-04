//! Relative color syntax (CSS Color 5 §4): `rgb(from <color> r g b)`
//! and its equivalent for every color function.
//!
//! The origin color is converted to the function's color space (§4.1)
//! and each of its channels — and its alpha — is bound to a keyword
//! (`r g b`, `h s l`, `h w b`, `l a b`, `l c h`, `x y z`, `alpha`); a
//! keyword resolves to the channel's value as a `<number>` (a missing
//! one as 0), alone or inside a math function. The keywords are then
//! substituted as numbers and the function's own (modern) grammar
//! parses the result, so every channel form it takes works here too.

use super::context::ColorCx;
use super::{hsl, lab, parse_absolute, rgb};
use crate::color::{AbsoluteColor, ColorSpace, convert};
use crate::parse::token::Token;
use crate::parse::values::numeric::components;

/// Parse `args` — `from <color> …` — of the color function `name`
/// (lower case).
pub(super) fn parse(name: &str, args: &[Token], cx: &ColorCx) -> Option<AbsoluteColor> {
    let after_from = args.get(1..)?;
    let origin_tokens = *components(after_from)?.first()?;
    let origin = parse_absolute(origin_tokens, cx)?;
    let rest = &after_from[origin_tokens.len()..];
    if rest.contains(&Token::Comma) {
        return None;
    }
    let (space, keywords, scale) = target(name, rest)?;
    let converted = convert(origin, space);
    let values = converted.values().map(|v| v * scale);
    let alpha = origin.alpha.unwrap_or(0.0);
    let bind = |ident: &str| {
        if ident.eq_ignore_ascii_case("alpha") {
            return Some(alpha);
        }
        (0..3)
            .find(|&i| keywords[i].eq_ignore_ascii_case(ident))
            .map(|i| values[i])
    };
    let substituted: Vec<Token> = rest
        .iter()
        .map(|t| match t {
            Token::Ident(ident) => bind(ident).map_or_else(|| t.clone(), Token::Float),
            _ => t.clone(),
        })
        .collect();
    match name {
        "rgb" | "rgba" => rgb::parse(&substituted),
        "hsl" | "hsla" => hsl::parse_hsl(&substituted),
        "hwb" => hsl::parse_hwb(&substituted),
        "lab" => lab::parse_lab(&substituted, ColorSpace::Lab),
        "lch" => lab::parse_lab(&substituted, ColorSpace::Lch),
        "oklab" => lab::parse_lab(&substituted, ColorSpace::Oklab),
        "oklch" => lab::parse_lab(&substituted, ColorSpace::Oklch),
        "color" => lab::parse_color_function(&substituted),
        _ => None,
    }
}

/// The function's color space, its channel keywords, and the factor
/// from the space's coordinates to the keywords' values (`rgb()`'s
/// channels are 0–255).
fn target(name: &str, rest: &[Token]) -> Option<(ColorSpace, [&'static str; 3], f64)> {
    Some(match name {
        "rgb" | "rgba" => (ColorSpace::Srgb, ["r", "g", "b"], 255.0),
        "hsl" | "hsla" => (ColorSpace::Hsl, ["h", "s", "l"], 1.0),
        "hwb" => (ColorSpace::Hwb, ["h", "w", "b"], 1.0),
        "lab" => (ColorSpace::Lab, ["l", "a", "b"], 1.0),
        "lch" => (ColorSpace::Lch, ["l", "c", "h"], 1.0),
        "oklab" => (ColorSpace::Oklab, ["l", "a", "b"], 1.0),
        "oklch" => (ColorSpace::Oklch, ["l", "c", "h"], 1.0),
        "color" => {
            let Some(Token::Ident(space)) = rest.first() else {
                return None;
            };
            let space = ColorSpace::predefined(space)?;
            let keywords = if matches!(space, ColorSpace::XyzD50 | ColorSpace::XyzD65) {
                ["x", "y", "z"]
            } else {
                ["r", "g", "b"]
            };
            (space, keywords, 1.0)
        }
        _ => return None,
    })
}
