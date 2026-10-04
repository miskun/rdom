//! CSS text to a concrete [`Color`]: the full-grammar entry point for
//! stored values (custom properties, registered `<color>`s) and the
//! fast path for single keywords and hex literals.

use super::TuiColor;
use crate::Color;

/// Parse a string into a concrete `Color` using the full CSS color
/// grammar — named keywords, hex (3/4/6/8 digit, with or without
/// `#`), indexed (0–255), `rgb()`, and `rgba()`. Used by the cascade
/// to resolve custom-property (`var(--*)`) string values stored on
/// the stylesheet.
///
/// Tokenizes input and dispatches through `parse::values::parse_color`
/// — the single canonical color grammar in `rdom-style`. Returns
/// `None` for unparseable input *or* for parse results that aren't
/// a `TuiColor::Literal` (e.g. a nested `var(--*)` inside a stored
/// var value — vars-in-vars stay unsupported in v0.1.0).
///
/// Simple cases (single named-ident or hex token) take the fast
/// path via `parse_simple_color` directly; the full grammar handles
/// the function-call cases (`rgb()`, `rgba()`).
pub fn parse_color(input: &str) -> Option<Color> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    // Fast path: simple inputs (named / hex with `#` / indexed)
    // that don't need a tokenizer.
    if let Some(c) = parse_simple_color(s) {
        return Some(c);
    }
    // Full grammar via tokenizer — handles `rgb()`, `rgba()`, and
    // any future grammar additions.
    let tokens = crate::parse::tokenize(s).ok()?;
    match crate::parse::values::parse_color(&tokens)? {
        TuiColor::Literal(c) => Some(c),
        TuiColor::System(s) => Some(s.color()),
        // Not a color on its own: it needs the element
        // (`TuiColor::parse` keeps it).
        TuiColor::CurrentColor | TuiColor::Function(_) | TuiColor::Var { .. } => None,
    }
}

/// Parse the *simple* color subset — named keywords, hex (`#` or
/// bare nibbles), indexed 0–255. Returns `None` for inputs the
/// simple parser can't handle (anything with parentheses, commas,
/// or `var()` syntax — those need the full grammar via
/// [`parse_color`]).
///
/// `pub(crate)` so `parse::values::parse_color_at` can use it for
/// Ident/HexColor tokens without recursing back through the
/// full-grammar dispatch in [`parse_color`].
pub(crate) fn parse_simple_color(input: &str) -> Option<Color> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    // Hex literal with `#` prefix.
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    // Decimal 0..=255 → indexed.
    if s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse::<u8>().ok().map(Color::Indexed);
    }
    // Named.
    //
    // - `reset` → terminal default fg/bg, an rdom-specific keyword.
    // - `transparent` → transparent black (CSS Color 4 §6.3), which
    //   paint composites away: what lies beneath shows through.
    // - Everything else falls through to the 148 CSS named-color
    //   table (`color::named`), which now owns every keyword that
    //   the pre-T6 ANSI match used to claim. `lightblue` resolves
    //   to CSS `#ADD8E6` (pale), `red` to `#FF0000` (full), etc.
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "reset" => return Some(Color::Reset),
        "transparent" => return Some(Color::TRANSPARENT),
        _ => {}
    }
    crate::color::named::lookup(&lower)
}

fn parse_hex(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            // `#rgb` — double each nibble.
            let r = hex_digit(hex.as_bytes()[0])?;
            let g = hex_digit(hex.as_bytes()[1])?;
            let b = hex_digit(hex.as_bytes()[2])?;
            Some(Color::Rgb(r * 17, g * 17, b * 17))
        }
        4 => {
            // `#rgba` — short form with alpha (CSS Color 4 §5.2).
            let r = hex_digit(hex.as_bytes()[0])?;
            let g = hex_digit(hex.as_bytes()[1])?;
            let b = hex_digit(hex.as_bytes()[2])?;
            let a = hex_digit(hex.as_bytes()[3])?;
            Some(Color::rgba(r * 17, g * 17, b * 17, a * 17))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        8 => {
            // `#rrggbbaa` — long form with alpha (CSS Color 4 §5.2).
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color::rgba(r, g, b, a))
        }
        _ => None,
    }
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
