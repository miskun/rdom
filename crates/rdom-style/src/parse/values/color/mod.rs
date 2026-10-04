//! The `<color>` grammar (CSS Color 4 §4.1): named colors, hex,
//! `rgb()` / `rgba()`, `hsl()` / `hsla()`, `hwb()`, `lab()` / `lch()`
//! / `oklab()` / `oklch()`, `color()`, `color-mix()`, relative colors,
//! `light-dark()`, `currentcolor`, the system colors, and rdom's `reset` and
//! palette-index forms.
//! `var()` is not part of this grammar: a declaration holding one is
//! substituted by the cascade before it is parsed (`crate::var`).
//!
//! - `channel` — channel and alpha arguments, the modern / legacy
//!   argument split.
//! - `rgb` — `rgb()` / `rgba()`.
//! - `hsl` — `hsl()` / `hsla()` / `hwb()`.
//! - `lab` — `lab()` / `lch()` / `oklab()` / `oklch()` / `color()`.
//! - `mix` — `color-mix()`.
//! - `relative` — relative color syntax (`rgb(from <color> r g b)`).
//! - `context` — what a color inside a function resolves against
//!   (`currentcolor`), and the rule that defers a function holding one
//!   to computed-value time.

mod channel;
mod context;
mod hsl;
mod lab;
mod mix;
mod relative;
mod rgb;

use crate::color::{AbsoluteColor, ColorSpace, SystemColor};
use crate::parse::token::Token;
use crate::{Color, ColorContext, TuiColor};
use context::ColorCx;

/// How many color functions may nest in one value
/// (`color-mix(in srgb, color-mix(…), …)`, a relative color's origin,
/// `light-dark()`'s arms).
///
/// CSS Color 4 / 5 set no limit, but color functions take attribute
/// data (`attr()` with `type(<color>)`, Values 5 §8.7), and the parser
/// recurses once per level: a hostile attribute could exhaust the stack
/// and abort the process. 32 levels is far past any hand-written value;
/// a deeper one is invalid, like any other parse failure (as
/// [`MAX_CALC_NESTING`](crate::parse::values::MAX_CALC_NESTING) is for
/// math functions).
pub const MAX_COLOR_NESTING: usize = 32;

/// Parse a whole value as a `<color>`.
pub fn parse_color(value: &[Token]) -> Option<TuiColor> {
    parse_color_at(value, 0).and_then(|(c, consumed)| {
        if consumed == value.len() {
            Some(c)
        } else {
            None
        }
    })
}

/// Recursive entrypoint for parsing a color value starting at
/// `value[start]`. Returns `(color, tokens_consumed)` so a caller can
/// parse a color inside a longer value.
pub fn parse_color_at(value: &[Token], start: usize) -> Option<(TuiColor, usize)> {
    let tok = value.get(start)?;
    match tok {
        Token::Ident(name) if name.eq_ignore_ascii_case("currentcolor") => {
            Some((TuiColor::CurrentColor, 1))
        }
        Token::Ident(name) if SystemColor::from_keyword(name).is_some() => {
            Some((TuiColor::System(SystemColor::from_keyword(name)?), 1))
        }
        Token::Ident(name) => {
            // Use the simple-cases fast path directly — the public
            // `parse_color(&str)` dispatches through this same
            // grammar and would recurse otherwise.
            let c = crate::tui_color::parse_simple_color(name)?;
            Some((TuiColor::Literal(c), 1))
        }
        Token::HexColor(hex) => {
            let with_hash = format!("#{hex}");
            let c = crate::tui_color::parse_simple_color(&with_hash)?;
            Some((TuiColor::Literal(c), 1))
        }
        Token::Function(_) => {
            // Parsed without an element: a color function that needs one
            // (`currentcolor` inside) is kept, as written, for the
            // cascade to compute.
            let cx = ColorCx::parse_time();
            let (color, used) = parse_function(value, start, &cx)?;
            Some(if cx.needs_element() {
                let text = context::render(&value[start..start + used]);
                (TuiColor::Function(crate::ColorFunction::new(text)), used)
            } else {
                (TuiColor::Literal(color.to_color()), used)
            })
        }
        _ => None,
    }
}

/// Compute a color function kept for computed-value time
/// ([`TuiColor::Function`]) against `context`. `None` when the text
/// does not parse as one color function.
pub(crate) fn compute_function(text: &str, context: &ColorContext) -> Option<Color> {
    let tokens = crate::parse::tokenize(text).ok()?;
    let cx = ColorCx::computed(context);
    let (color, used) = parse_function(&tokens, 0, &cx)?;
    (used == tokens.len()).then(|| color.to_color())
}

/// Parse one `<color>` that is the whole of `component` — one component
/// value, as `components` splits them (a color inside a color function)
/// — as an absolute color.
fn parse_absolute(component: &[Token], cx: &ColorCx) -> Option<AbsoluteColor> {
    match component {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("currentcolor") => cx.current_color(),
        [Token::Ident(name)] if SystemColor::from_keyword(name).is_some() => {
            cx.system(SystemColor::from_keyword(name)?)
        }
        [Token::Ident(name)] => cx.absolute(crate::tui_color::parse_simple_color(name)?),
        [Token::HexColor(hex)] => {
            cx.absolute(crate::tui_color::parse_simple_color(&format!("#{hex}"))?)
        }
        // One component value (`components`): the function runs to the
        // `)` that ends it, so its arguments need no second scan.
        [Token::Function(name), args @ .., Token::RParen] => function(name, args, cx),
        _ => None,
    }
}

/// Parse the color function whose token is `value[start]`: the color
/// and the tokens used, its `)` included.
fn parse_function(value: &[Token], start: usize, cx: &ColorCx) -> Option<(AbsoluteColor, usize)> {
    let Token::Function(name) = value.get(start)? else {
        return None;
    };
    let close = closing_paren(value, start)?;
    let color = function(name, &value[start + 1..close], cx)?;
    Some((color, close + 1 - start))
}

/// The color function `name` applied to `args` (the tokens between the
/// function token and its `)`), one nesting level deeper than the caller
/// — `None` past [`MAX_COLOR_NESTING`].
fn function(name: &str, args: &[Token], cx: &ColorCx) -> Option<AbsoluteColor> {
    cx.nested(|| {
        let name = name.to_ascii_lowercase();
        if matches!(args.first(), Some(Token::Ident(from)) if from.eq_ignore_ascii_case("from")) {
            return relative::parse(&name, args, cx);
        }
        match name.as_str() {
            "color-mix" => mix::parse(args, cx),
            "light-dark" => light_dark(args, cx),
            "rgb" | "rgba" => rgb::parse(args),
            "hsl" | "hsla" => hsl::parse_hsl(args),
            "hwb" => hsl::parse_hwb(args),
            "lab" => lab::parse_lab(args, ColorSpace::Lab),
            "lch" => lab::parse_lab(args, ColorSpace::Lch),
            "oklab" => lab::parse_lab(args, ColorSpace::Oklab),
            "oklch" => lab::parse_lab(args, ColorSpace::Oklch),
            "color" => lab::parse_color_function(args),
            _ => None,
        }
    })
}

/// `light-dark(<color>, <color>)` (CSS Color 5 §5.1): the first under a
/// light color scheme, the second otherwise. Both must parse.
fn light_dark(args: &[Token], cx: &ColorCx) -> Option<AbsoluteColor> {
    let [light, dark] = channel::split_top_level(args, &Token::Comma)[..] else {
        return None;
    };
    let one = |tokens: &[Token]| match crate::parse::values::numeric::components(tokens)?.as_slice()
    {
        [color] => parse_absolute(color, cx),
        _ => None,
    };
    let (light, dark) = (one(light)?, one(dark)?);
    Some(match cx.scheme() {
        crate::color::ColorScheme::Light => light,
        crate::color::ColorScheme::Dark => dark,
    })
}

/// Index of the `)` closing the function token at `open`.
pub(super) fn closing_paren(value: &[Token], open: usize) -> Option<usize> {
    close_from(value, open + 1)
}

/// Index of the `)` closing a function whose arguments start at
/// `start`.
fn close_from(value: &[Token], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (i, t) in value.iter().enumerate().skip(start) {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Consume the arguments of `rgb()` from `start` (just past the
/// function token) through its `)`. Returns the color and the tokens
/// consumed, the `)` included.
pub fn parse_rgb_args(value: &[Token], start: usize) -> Option<(crate::Color, usize)> {
    let close = close_from(value, start)?;
    let color = rgb::parse(&value[start..close])?;
    Some((color.to_color(), close + 1 - start))
}

/// [`parse_rgb_args`] for `rgba()`, the same function (CSS Color 4
/// §5.1).
pub fn parse_rgba_args(value: &[Token], start: usize) -> Option<(crate::Color, usize)> {
    parse_rgb_args(value, start)
}

#[cfg(test)]
mod tests;
