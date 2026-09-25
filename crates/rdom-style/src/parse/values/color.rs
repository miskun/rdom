//! Color values: named / hex literals, `rgb()`, `rgba()` (alpha
//! dropped) and `var(--name [, fallback])` with recursive fallbacks.

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
/// `value[start]`. Returns `(color, tokens_consumed)` so `var()`
/// fallbacks (which are themselves colors) can recurse.
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
                "var" => parse_var_args(value, after).map(|(c, n)| (c, n + 1)),
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

/// Consume `--name [, fallback])`. Returns the constructed
/// `TuiColor::Var` and tokens consumed including the closing `)`.
pub fn parse_var_args(value: &[Token], start: usize) -> Option<(TuiColor, usize)> {
    let raw_name = match value.get(start)? {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    let stripped = raw_name.strip_prefix("--")?;
    let name = stripped.to_string();
    let after_name = start + 1;

    match value.get(after_name)? {
        Token::RParen => Some((
            TuiColor::Var {
                name,
                fallback: None,
            },
            2, // ident + RParen
        )),
        Token::Comma => {
            let (fallback, consumed) = parse_color_at(value, after_name + 1)?;
            let after_fb = after_name + 1 + consumed;
            if value.get(after_fb) != Some(&Token::RParen) {
                return None;
            }
            Some((
                TuiColor::Var {
                    name,
                    fallback: Some(Box::new(fallback)),
                },
                // ident + comma + fallback tokens + RParen
                1 + 1 + consumed + 1,
            ))
        }
        _ => None,
    }
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
