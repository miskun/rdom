//! Color values: named / hex literals, `rgb()`, `rgba()` (alpha
//! dropped). `var()` is not part of this grammar: a declaration holding
//! one is substituted by the cascade before it is parsed (`crate::var`).

use crate::TuiColor;
use crate::parse::token::Token;

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
        Token::Function(name) => {
            let lname = name.to_ascii_lowercase();
            let after = start + 1;
            match lname.as_str() {
                "rgb" => parse_rgb_args(value, after).map(|(c, n)| (TuiColor::Literal(c), n + 1)),
                "rgba" => parse_rgba_args(value, after).map(|(c, n)| (TuiColor::Literal(c), n + 1)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Consume `r, g, b)` starting at `start`. Returns
/// `(Color::Rgb(r,g,b), tokens_consumed_including_RParen)`.
pub fn parse_rgb_args(value: &[Token], start: usize) -> Option<(crate::Color, usize)> {
    let (r, n1) = expect_byte(value, start)?;
    let n2 = expect_comma(value, start + n1)?;
    let (g, n3) = expect_byte(value, start + n1 + n2)?;
    let n4 = expect_comma(value, start + n1 + n2 + n3)?;
    let (b, n5) = expect_byte(value, start + n1 + n2 + n3 + n4)?;
    let total = n1 + n2 + n3 + n4 + n5;
    if value.get(start + total) != Some(&Token::RParen) {
        return None;
    }
    Some((crate::Color::Rgb(r, g, b), total + 1))
}

/// Consume `r, g, b, <anything>)`. Alpha is dropped — we just walk
/// tokens until the matching `)`.
pub fn parse_rgba_args(value: &[Token], start: usize) -> Option<(crate::Color, usize)> {
    let (r, n1) = expect_byte(value, start)?;
    let n2 = expect_comma(value, start + n1)?;
    let (g, n3) = expect_byte(value, start + n1 + n2)?;
    let n4 = expect_comma(value, start + n1 + n2 + n3)?;
    let (b, n5) = expect_byte(value, start + n1 + n2 + n3 + n4)?;
    let after_b = start + n1 + n2 + n3 + n4 + n5;
    // Expect a comma before the alpha; everything up to the next
    // top-level `)` is alpha and discarded.
    if value.get(after_b) != Some(&Token::Comma) {
        return None;
    }
    let mut i = after_b + 1;
    let mut depth = 0usize;
    while let Some(t) = value.get(i) {
        match t {
            Token::LParen | Token::Function(_) => {
                depth += 1;
                i += 1;
            }
            Token::RParen if depth > 0 => {
                depth -= 1;
                i += 1;
            }
            Token::RParen => {
                return Some((crate::Color::Rgb(r, g, b), i + 1 - start));
            }
            _ => i += 1,
        }
    }
    None
}

fn expect_byte(value: &[Token], at: usize) -> Option<(u8, usize)> {
    match value.get(at)? {
        Token::Number(n) if (0..=255).contains(n) => Some((*n as u8, 1)),
        _ => None,
    }
}

fn expect_comma(value: &[Token], at: usize) -> Option<usize> {
    match value.get(at)? {
        Token::Comma => Some(1),
        _ => None,
    }
}
