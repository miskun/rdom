//! The `<color>` grammar (CSS Color 4 §4.1): named colors, hex,
//! `rgb()` / `rgba()`, `hsl()` / `hsla()`, `hwb()`, `lab()` / `lch()`
//! / `oklab()` / `oklch()`, `color()`, `currentcolor`, and rdom's `reset` and
//! palette-index forms.
//! `var()` is not part of this grammar: a declaration holding one is
//! substituted by the cascade before it is parsed (`crate::var`).
//!
//! - `channel` — channel and alpha arguments, the modern / legacy
//!   argument split.
//! - `rgb` — `rgb()` / `rgba()`.
//! - `hsl` — `hsl()` / `hsla()` / `hwb()`.
//! - `lab` — `lab()` / `lch()` / `oklab()` / `oklch()` / `color()`.

mod channel;
mod hsl;
mod lab;
mod rgb;

use crate::TuiColor;
use crate::color::{AbsoluteColor, ColorSpace};
use crate::parse::token::Token;

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
            let (color, used) = parse_function(value, start)?;
            Some((TuiColor::Literal(color.to_color()), used))
        }
        _ => None,
    }
}

/// Parse the color function whose token is `value[start]`: the color
/// and the tokens used, its `)` included.
fn parse_function(value: &[Token], start: usize) -> Option<(AbsoluteColor, usize)> {
    let Token::Function(name) = value.get(start)? else {
        return None;
    };
    let close = closing_paren(value, start)?;
    let args = &value[start + 1..close];
    let color = match name.to_ascii_lowercase().as_str() {
        "rgb" | "rgba" => rgb::parse(args)?,
        "hsl" | "hsla" => hsl::parse_hsl(args)?,
        "hwb" => hsl::parse_hwb(args)?,
        "lab" => lab::parse_lab(args, ColorSpace::Lab)?,
        "lch" => lab::parse_lab(args, ColorSpace::Lch)?,
        "oklab" => lab::parse_lab(args, ColorSpace::Oklab)?,
        "oklch" => lab::parse_lab(args, ColorSpace::Oklch)?,
        "color" => lab::parse_color_function(args)?,
        _ => return None,
    };
    Some((color, close + 1 - start))
}

/// Index of the `)` closing the function token at `open`.
fn closing_paren(value: &[Token], open: usize) -> Option<usize> {
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
