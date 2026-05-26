//! Per-property value parsers — the leaves of the CSS dispatch
//! table.
//!
//! Each `parse_*` function takes a `&[Token]` (whitespace already
//! eaten by the tokenizer) and returns `Option<T>` where `T` is the
//! value type being parsed. `None` means "invalid input"; the caller
//! turns that into a warning / `DispatchError::InvalidValue`.
//!
//! Lives in `rdom-style` so the block parser (in `rdom-css`) and
//! `property_dispatch::set_from_tokens` (in this crate) can both
//! consume the same per-property parsing.

use crate::calc::{CalcExpr, CalcOp};
use crate::layout::{Border, Length, Overflow, Padding, Position, Size, ZIndex};
use crate::parse::token::Token;
use crate::transition::{AnimatableProperty, TimingFunction, TransitionProperty};
use crate::{Content, TuiColor, TuiStyle, Value};

/// Render a `&[Token]` slice back to its source-like string form.
/// Used by the block parser's `InvalidValue` warning path.
pub fn render_value(value: &[Token]) -> String {
    let mut out = String::new();
    for (i, t) in value.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match t {
            Token::Ident(s) => out.push_str(s),
            Token::Number(n) => out.push_str(&n.to_string()),
            Token::Percentage(n) => {
                out.push_str(&n.to_string());
                out.push('%');
            }
            Token::String(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            Token::HexColor(h) => {
                out.push('#');
                out.push_str(h);
            }
            Token::Function(name) => {
                out.push_str(name);
                out.push('(');
            }
            Token::Colon => out.push(':'),
            Token::Semicolon => out.push(';'),
            Token::Comma => out.push(','),
            Token::Bang => out.push('!'),
            Token::LParen => out.push('('),
            Token::RParen => out.push(')'),
            Token::Delim(c) => out.push(*c),
        }
    }
    out
}

// ── Color ─────────────────────────────────────────────────────────

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

// ── Generic helpers ───────────────────────────────────────────────

pub fn parse_keyword<T: Clone>(value: &[Token], table: &[(&str, T)]) -> Option<T> {
    if value.len() != 1 {
        return None;
    }
    let name = match &value[0] {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    for (k, v) in table {
        if name.eq_ignore_ascii_case(k) {
            return Some(v.clone());
        }
    }
    None
}

/// Parse CSS `opacity: <number>`. Accepts integer (`0`, `1`) and
/// decimal (`0.5`, `0.25`) values. Clamps to `[0.0, 1.0]`. Per
/// CSS, percentage syntax (`50%`) is also valid; not yet wired
/// because rdom's tokenizer doesn't represent `%` and no other
/// property currently needs percentage handling.
///
/// Decimal tokenization: `0.5` arrives as three tokens —
/// `Number(0) Delim('.') Number(5)` — matching the
/// `parse_time_ms` precedent for sub-second durations.
pub fn parse_opacity(value: &[Token]) -> Option<f32> {
    match value {
        // Integer: 0 or 1.
        [Token::Number(n)] if *n >= 0 => Some((*n as f32).clamp(0.0, 1.0)),
        // Decimal: <int>.<frac> pattern.
        [Token::Number(int), Token::Delim('.'), Token::Number(frac)] if *int >= 0 && *frac >= 0 => {
            let frac_str = frac.to_string();
            let denom = 10f32.powi(frac_str.len() as i32);
            Some(((*int as f32) + (*frac as f32) / denom).clamp(0.0, 1.0))
        }
        // Leading-dot form: .5
        [Token::Delim('.'), Token::Number(frac)] if *frac >= 0 => {
            let frac_str = frac.to_string();
            let denom = 10f32.powi(frac_str.len() as i32);
            Some(((*frac as f32) / denom).clamp(0.0, 1.0))
        }
        _ => None,
    }
}

pub fn parse_text_decoration(value: &[Token]) -> Option<(bool, bool)> {
    // M1: only single keyword. `underline | line-through | none`.
    if value.len() != 1 {
        return None;
    }
    let name = match &value[0] {
        Token::Ident(s) => s.as_str(),
        _ => return None,
    };
    match name {
        "underline" => Some((true, false)),
        "line-through" => Some((false, true)),
        "none" => Some((false, false)),
        _ => None,
    }
}

pub fn parse_overflow(value: &[Token]) -> Option<Overflow> {
    parse_keyword(
        value,
        &[
            ("hidden", Overflow::Hidden),
            ("scroll", Overflow::Scroll),
            ("auto", Overflow::Auto),
            ("visible", Overflow::Visible),
        ],
    )
}

pub fn parse_scrollbar_gutter(value: &[Token]) -> Option<crate::layout::ScrollbarGutter> {
    use crate::layout::ScrollbarGutter;
    parse_keyword(
        value,
        &[
            ("auto", ScrollbarGutter::Auto),
            ("stable", ScrollbarGutter::Stable),
        ],
    )
}

pub fn parse_unsigned(value: &[Token]) -> Option<u16> {
    if value.len() == 1
        && let Token::Number(n) = &value[0]
        && *n >= 0
    {
        return Some(*n as u16);
    }
    // Constant `calc(...)` — a percent-bearing form has no
    // sensible static basis here (padding/margin/gap don't carry
    // a Calc-bearing type), so we reject it. The block parser's
    // warning channel surfaces the rejection.
    if looks_like_calc(value) {
        let expr = parse_calc(value)?;
        if expr.contains_percent() {
            return None;
        }
        let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
        return Some(cells.max(0).min(u16::MAX as i32) as u16);
    }
    None
}

/// Padding shorthand expansion. Accepts 1..=4 unsigned integers.
/// Order matches CSS: top, right, bottom, left (clockwise from top).
pub fn parse_padding_shorthand(value: &[Token]) -> Option<Padding> {
    // Split on `calc(...)` function boundaries + bare numbers.
    // Each value-position must be either a bare unsigned number
    // OR a `calc(...)` that resolves to a constant integer (no
    // percent — padding/margin's u16 field type can't carry a
    // Calc AST; see DIVERGENCES.md).
    let nums = split_padding_values(value)?;
    let p = match nums.as_slice() {
        [a] => Padding {
            top: a.clone(),
            right: a.clone(),
            bottom: a.clone(),
            left: a.clone(),
        },
        [a, b] => Padding {
            top: a.clone(),
            right: b.clone(),
            bottom: a.clone(),
            left: b.clone(),
        },
        [a, b, c] => Padding {
            top: a.clone(),
            right: b.clone(),
            bottom: c.clone(),
            left: b.clone(),
        },
        [a, b, c, d] => Padding {
            top: a.clone(),
            right: b.clone(),
            bottom: c.clone(),
            left: d.clone(),
        },
        _ => return None,
    };
    Some(p)
}

/// Split a value-token slice into 1..=4 padding values. Each
/// position accepts:
/// - bare `Token::Number(n)` with `n >= 0` → `PaddingValue::Cells(n)`.
/// - `calc(<expr>)` — constant calcs resolve immediately to
///   `Cells`; percent-bearing calcs are preserved as
///   `PaddingValue::Calc` for layout-time resolution against the
///   containing-block width (closes `CALC-PADMARG-1`).
fn split_padding_values(value: &[Token]) -> Option<Vec<crate::layout::PaddingValue>> {
    use crate::layout::PaddingValue;
    let mut vals = Vec::with_capacity(4);
    let mut i = 0;
    while i < value.len() {
        match &value[i] {
            Token::Number(n) if *n >= 0 => {
                vals.push(PaddingValue::Cells(*n as u16));
                i += 1;
            }
            Token::Function(name) if name.eq_ignore_ascii_case("calc") => {
                // Scan forward for the matching `)`.
                let mut depth = 1usize;
                let mut j = i + 1;
                while j < value.len() && depth > 0 {
                    match &value[j] {
                        Token::LParen | Token::Function(_) => depth += 1,
                        Token::RParen => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth != 0 {
                    return None;
                }
                let expr = parse_calc(&value[i..j])?;
                if expr.contains_percent() {
                    // Keep the AST around — layout resolves against
                    // containing-block width per CSS 2.1 §8.4.
                    vals.push(PaddingValue::Calc(Box::new(expr)));
                } else {
                    let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
                    vals.push(PaddingValue::Cells(cells.max(0).min(u16::MAX as i32) as u16));
                }
                i = j;
            }
            _ => return None,
        }
        if vals.len() > 4 {
            return None;
        }
    }
    if vals.is_empty() {
        return None;
    }
    Some(vals)
}

/// Read the current padding from `style`, defaulting to all-zero
/// when nothing is set or when the existing value is `Inherit` /
/// `Initial`. Used by the per-side longhands so consecutive
/// declarations combine instead of overwriting.
/// Parse a single padding value (used by `padding-*` longhands).
/// Accepts `<number>` (cells) or `calc(<expr>)` — constant calc
/// resolves at parse time; percent-bearing calc is preserved as
/// `PaddingValue::Calc` for layout-time resolution.
pub fn parse_padding_value(value: &[Token]) -> Option<crate::layout::PaddingValue> {
    let vals = split_padding_values(value)?;
    if vals.len() == 1 {
        Some(vals.into_iter().next().unwrap())
    } else {
        None
    }
}

pub fn current_padding(style: &TuiStyle) -> Padding {
    match &style.padding {
        Some(Value::Specified(p)) => p.clone(),
        _ => Padding::default(),
    }
}

/// Read the current border from `style`, defaulting to all-sides-off
/// when nothing is set. Used by the `border-top` / `border-right` /
/// `border-bottom` / `border-left` longhands so consecutive
/// declarations combine instead of overwriting.
pub fn current_border(style: &TuiStyle) -> Border {
    match style.border {
        Some(Value::Specified(b)) => b,
        _ => Border::none(),
    }
}

/// Parse a per-side `border-<side>` value into a boolean: any
/// recognized "draw" keyword (`solid`, `single`, `rounded`) → true;
/// `none` → false. Width / color / style triples aren't modeled
/// (rdom borders are binary per side). Unknown values → `None` so
/// the caller emits a warning.
pub fn parse_border_side(value: &[Token]) -> Option<bool> {
    parse_keyword(
        value,
        &[
            ("solid", true),
            ("single", true),
            ("rounded", true),
            ("none", false),
        ],
    )
}

pub fn parse_size(value: &[Token]) -> Option<Size> {
    // `auto` | `<n>` | `<n>fr` | `<n>%` | `calc(<expr>)`
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Size::Auto),
        [Token::Number(n)] if *n >= 0 => Some(Size::Fixed(*n as u16)),
        [Token::Number(n), Token::Ident(unit)] if *n >= 0 && unit.eq_ignore_ascii_case("fr") => {
            Some(Size::Flex(*n as u16))
        }
        [Token::Percentage(n)] if *n >= 0 => Some(Size::Percent(*n as u16)),
        // calc(...) — parse to a CalcExpr. If the expression has
        // no percentages, constant-fold at parse time to Fixed.
        // Otherwise carry the AST through to layout via Size::Calc.
        _ if looks_like_calc(value) => parse_calc_to_size(value),
        _ => None,
    }
}

/// Parse a `calc(...)` value in `Size` position. Constant-fold to
/// `Size::Fixed` when the expression contains no percentages
/// (saves layout-time work for the common arithmetic-only case).
/// Otherwise carry the AST through as `Size::Calc` for layout
/// resolution.
fn parse_calc_to_size(value: &[Token]) -> Option<Size> {
    let expr = parse_calc(value)?;
    if expr.contains_percent() {
        Some(Size::Calc(Box::new(expr)))
    } else {
        let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
        let clamped = cells.max(0).min(u16::MAX as i32) as u16;
        Some(Size::Fixed(clamped))
    }
}

/// Parse the CSS `flex` shorthand. Models the main-axis sizing
/// of a flex child. Returns the `Size` that should be applied to
/// the child's width AND height (cross-axis `Size::Flex` already
/// means "stretch to container" in our layout, matching CSS
/// default `align-items: stretch` behavior).
///
/// Supported value shapes:
/// - `flex: auto`   → `Size::Flex(1)` (grow as `flex: 1 1 auto`)
/// - `flex: none`   → `Size::Auto`     (don't grow as `flex: 0 0 auto`)
/// - `flex: <n>`    → `n > 0` → `Size::Flex(n)`; `n == 0` → `Size::Auto`
/// - `flex: <n> <m> <basis>` → use `<n>` as the grow value; `<m>`
///   (shrink) and `<basis>` are parsed-and-accepted but ignored
///   until full flex-grow / flex-shrink / flex-basis tracking lands
///   in the substrate.
pub fn parse_flex_shorthand(value: &[Token]) -> Option<Size> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Size::Flex(1)),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(Size::Auto),
        [Token::Number(n)] if *n >= 0 => {
            if *n == 0 {
                Some(Size::Auto)
            } else {
                Some(Size::Flex(*n as u16))
            }
        }
        // Two-value form: `<grow> <shrink>` (basis defaults to 0).
        // Three-value form: `<grow> <shrink> <basis>`. Both ignore
        // shrink and basis for now; the grow value drives the Size.
        [Token::Number(n), _rest @ ..] if *n >= 0 && !value.is_empty() => {
            // Validate the remaining tokens look like a valid
            // flex shorthand tail (1 or 2 more numeric/auto values).
            // If not, reject so authors get a warning rather than a
            // silent partial-apply.
            let tail = &value[1..];
            let tail_ok = match tail.len() {
                1 => {
                    matches!(&tail[0], Token::Number(_))
                        || matches!(&tail[0], Token::Ident(s) if s.eq_ignore_ascii_case("auto"))
                }
                2 => {
                    matches!(&tail[0], Token::Number(_))
                        && (matches!(&tail[1], Token::Number(_))
                            || matches!(&tail[1], Token::Ident(s) if s.eq_ignore_ascii_case("auto")))
                }
                _ => false,
            };
            if !tail_ok {
                return None;
            }
            if *n == 0 {
                Some(Size::Auto)
            } else {
                Some(Size::Flex(*n as u16))
            }
        }
        _ => None,
    }
}

/// `min-width` / `min-height` value: `auto` | `<unsigned-int>`. The
/// `auto` keyword opts a flex item into intrinsic min-content
/// protection (decision 4 from the M5 pre-prep, M5.1.b).
pub fn parse_min_size(value: &[Token]) -> Option<crate::layout::MinSize> {
    use crate::layout::MinSize;
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(MinSize::Auto),
        [Token::Number(n)] if *n >= 0 => Some(MinSize::Cells(*n as u16)),
        _ => None,
    }
}

/// Parse one margin token-group: either `auto` or a signed integer
/// (positive `Number`, or `Delim('-')` followed by `Number`).
/// Returns the value and the number of tokens consumed.
fn parse_margin_value_at(
    value: &[Token],
    start: usize,
) -> Option<(crate::layout::MarginValue, usize)> {
    use crate::layout::MarginValue;
    match value.get(start) {
        Some(Token::Ident(s)) if s.eq_ignore_ascii_case("auto") => Some((MarginValue::Auto, 1)),
        Some(Token::Number(n)) => {
            if *n > i16::MAX as i32 || *n < i16::MIN as i32 {
                return None;
            }
            Some((MarginValue::Cells(*n as i16), 1))
        }
        Some(Token::Delim('-')) => match value.get(start + 1) {
            Some(Token::Number(n)) if -(*n) >= i16::MIN as i32 => {
                Some((MarginValue::Cells(-(*n) as i16), 2))
            }
            _ => None,
        },
        // `calc(<expr>)` — constant resolves immediately; percent-
        // bearing preserved as `MarginValue::Calc` for layout-time
        // resolution against containing-block width.
        Some(Token::Function(name)) if name.eq_ignore_ascii_case("calc") => {
            let mut depth = 1usize;
            let mut j = start + 1;
            while j < value.len() && depth > 0 {
                match &value[j] {
                    Token::LParen | Token::Function(_) => depth += 1,
                    Token::RParen => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth != 0 {
                return None;
            }
            let expr = parse_calc(&value[start..j])?;
            let consumed = j - start;
            if expr.contains_percent() {
                Some((MarginValue::Calc(Box::new(expr)), consumed))
            } else {
                let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
                if !(i16::MIN as i32..=i16::MAX as i32).contains(&cells) {
                    return None;
                }
                Some((MarginValue::Cells(cells as i16), consumed))
            }
        }
        _ => None,
    }
}

/// `margin: <v>` | `<v> <v>` | `<v> <v> <v>` | `<v> <v> <v> <v>`
/// where each `<v>` is `auto` or a signed integer. CSS expansion:
/// - 1 value → all four sides
/// - 2 values → top/bottom = a, right/left = b
/// - 3 values → top = a, right/left = b, bottom = c
/// - 4 values → top, right, bottom, left
pub fn parse_margin_shorthand(value: &[Token]) -> Option<crate::layout::Margin> {
    use crate::layout::{Margin, MarginValue};
    let mut vals: Vec<MarginValue> = Vec::with_capacity(4);
    let mut i = 0;
    while i < value.len() {
        let (v, consumed) = parse_margin_value_at(value, i)?;
        vals.push(v);
        i += consumed;
        if vals.len() > 4 {
            return None;
        }
    }
    let m = match vals.as_slice() {
        [a] => Margin::new(a.clone(), a.clone(), a.clone(), a.clone()),
        [a, b] => Margin::new(a.clone(), b.clone(), a.clone(), b.clone()),
        [a, b, c] => Margin::new(a.clone(), b.clone(), c.clone(), b.clone()),
        [a, b, c, d] => Margin::new(a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return None,
    };
    Some(m)
}

/// Parse a single margin longhand (`margin-top`, etc.).
pub fn parse_margin_longhand(value: &[Token]) -> Option<crate::layout::MarginValue> {
    let (v, consumed) = parse_margin_value_at(value, 0)?;
    if consumed != value.len() {
        return None;
    }
    Some(v)
}

/// Read the current margin from `style`, defaulting to all-zero when
/// nothing is set. Used by per-side longhands so consecutive
/// declarations combine instead of overwriting.
pub fn current_margin(style: &TuiStyle) -> crate::layout::Margin {
    match &style.margin {
        Some(Value::Specified(m)) => m.clone(),
        _ => crate::layout::Margin::default(),
    }
}

/// `aspect-ratio: <w> / <h>`. v1 surface: `<positive-int>/<positive-int>`
/// (e.g. `16/9`, `4/3`, `1/1`). Stored as the integer pair so the
/// round-trip recovers the original form. `auto` keyword and the
/// single-number CSS form are deferred polish.
pub fn parse_aspect_ratio(value: &[Token]) -> Option<crate::layout::AspectRatio> {
    match value {
        [Token::Number(w), Token::Delim('/'), Token::Number(h)] if *w > 0 && *h > 0 => {
            crate::layout::AspectRatio::new(*w as u16, *h as u16)
        }
        _ => None,
    }
}

pub fn parse_border(value: &[Token]) -> Option<Border> {
    // Border keyword maps to the existing Border enum. v1 surface
    // is intentionally small — width / per-side color shorthand
    // arrives in M5.
    parse_keyword(
        value,
        &[
            ("none", Border::none()),
            ("solid", Border::single()),
            ("single", Border::single()),
            ("rounded", Border::rounded()),
            ("top", Border::top()),
            ("bottom", Border::bottom()),
            ("left", Border::left()),
            ("right", Border::right()),
        ],
    )
}

pub fn parse_content(value: &[Token]) -> Option<Content> {
    // Either a single string or `attr(<ident>)`.
    match value {
        [Token::String(s)] => Some(Content::Str(s.clone())),
        [Token::Function(name), Token::Ident(arg), Token::RParen]
            if name.eq_ignore_ascii_case("attr") =>
        {
            Some(Content::Attr(arg.clone()))
        }
        _ => None,
    }
}

// ── Positioning value parsers (M2) ────────────────────────────────

pub fn parse_position(value: &[Token]) -> Option<Position> {
    parse_keyword(
        value,
        &[
            ("static", Position::Static),
            ("relative", Position::Relative),
            ("absolute", Position::Absolute),
            ("fixed", Position::Fixed),
            ("sticky", Position::Sticky),
        ],
    )
}

/// `auto` keyword | signed integer in cells | `calc(<expr>)`.
pub fn parse_length(value: &[Token]) -> Option<Length> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(Length::Auto),
        [Token::Number(n)] => i16::try_from(*n).ok().map(Length::Cells),
        [Token::Delim('-'), Token::Number(n)] => i16::try_from(-*n).ok().map(Length::Cells),
        _ if looks_like_calc(value) => parse_calc_to_length(value),
        _ => None,
    }
}

/// Parse a `calc(...)` value in `Length` position. Constant-fold
/// to `Length::Cells` when the expression has no percentages;
/// carry the AST through as `Length::Calc` otherwise.
fn parse_calc_to_length(value: &[Token]) -> Option<Length> {
    let expr = parse_calc(value)?;
    if expr.contains_percent() {
        Some(Length::Calc(Box::new(expr)))
    } else {
        let cells = expr.resolve(&crate::calc::ResolveCtx::new(0));
        let clamped = cells.max(i16::MIN as i32).min(i16::MAX as i32) as i16;
        Some(Length::Cells(clamped))
    }
}

/// `auto` keyword | signed integer.
pub fn parse_z_index(value: &[Token]) -> Option<ZIndex> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(ZIndex::Auto),
        [Token::Number(n)] => i16::try_from(*n).ok().map(ZIndex::Value),
        [Token::Delim('-'), Token::Number(n)] => i16::try_from(-*n).ok().map(ZIndex::Value),
        _ => None,
    }
}

/// `inset: <a> [<b> [<c> [<d>]]]` — same clockwise expansion as
/// `padding`, but each value can be `auto` or signed (negative).
pub fn parse_inset_shorthand(value: &[Token]) -> Option<(Length, Length, Length, Length)> {
    let lengths = split_lengths(value)?;
    let p = match lengths.as_slice() {
        [a] => (a.clone(), a.clone(), a.clone(), a.clone()),
        [a, b] => (a.clone(), b.clone(), a.clone(), b.clone()),
        [a, b, c] => (a.clone(), b.clone(), c.clone(), b.clone()),
        [a, b, c, d] => (a.clone(), b.clone(), c.clone(), d.clone()),
        _ => return None,
    };
    Some(p)
}

/// Split a value-token slice into 1..=4 `Length` values separated
/// by whitespace (already eaten by the tokenizer). Used by the
/// `inset` shorthand.
fn split_lengths(value: &[Token]) -> Option<Vec<Length>> {
    let mut out = Vec::with_capacity(4);
    let mut i = 0usize;
    while i < value.len() {
        // Try the two-token negative pattern first.
        if let (Some(Token::Delim('-')), Some(Token::Number(n))) = (value.get(i), value.get(i + 1))
        {
            let l = i16::try_from(-*n).ok().map(Length::Cells)?;
            out.push(l);
            i += 2;
            continue;
        }
        let l = match value.get(i)? {
            Token::Ident(s) if s.eq_ignore_ascii_case("auto") => Length::Auto,
            Token::Number(n) => i16::try_from(*n).ok().map(Length::Cells)?,
            _ => return None,
        };
        out.push(l);
        i += 1;
        if out.len() > 4 {
            return None;
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

// ── Transition value parsers (M3) ─────────────────────────────────

/// Map a CSS property name to its `AnimatableProperty` slot.
/// Returns `None` for non-animatable / unknown names so the
/// caller can warn.
pub fn parse_animatable_property(name: &str) -> Option<AnimatableProperty> {
    Some(match name.to_ascii_lowercase().as_str() {
        "color" => AnimatableProperty::Color,
        "background-color" => AnimatableProperty::BackgroundColor,
        "border-color" => AnimatableProperty::BorderColor,
        "width" => AnimatableProperty::Width,
        "height" => AnimatableProperty::Height,
        "padding" => AnimatableProperty::Padding,
        "gap" => AnimatableProperty::Gap,
        "top" => AnimatableProperty::Top,
        "right" => AnimatableProperty::Right,
        "bottom" => AnimatableProperty::Bottom,
        "left" => AnimatableProperty::Left,
        "z-index" => AnimatableProperty::ZIndex,
        _ => return None,
    })
}

/// Parse a single property keyword (`all` / `none` / named).
pub fn parse_transition_property_keyword(name: &str) -> Option<TransitionProperty> {
    match name.to_ascii_lowercase().as_str() {
        "all" => Some(TransitionProperty::All),
        "none" => Some(TransitionProperty::None),
        other => parse_animatable_property(other).map(TransitionProperty::Named),
    }
}

/// Parse a single timing-function keyword.
pub fn parse_timing_function_keyword(name: &str) -> Option<TimingFunction> {
    match name.to_ascii_lowercase().as_str() {
        "linear" => Some(TimingFunction::Linear),
        "ease" => Some(TimingFunction::Ease),
        "ease-in" => Some(TimingFunction::EaseIn),
        "ease-out" => Some(TimingFunction::EaseOut),
        "ease-in-out" => Some(TimingFunction::EaseInOut),
        _ => None,
    }
}

/// Parse a comma-separated list of property keywords.
pub fn parse_transition_property_list(value: &[Token]) -> Option<Vec<TransitionProperty>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        let name = match seg {
            [Token::Ident(s)] => s.as_str(),
            _ => return None,
        };
        out.push(parse_transition_property_keyword(name)?);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a comma-separated list of `<time>` values into ms.
pub fn parse_time_list(value: &[Token]) -> Option<Vec<u32>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        out.push(parse_time_ms(seg)?);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a comma-separated list of timing-function keywords.
pub fn parse_timing_function_list(value: &[Token]) -> Option<Vec<TimingFunction>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        let name = match seg {
            [Token::Ident(s)] => s.as_str(),
            _ => return None,
        };
        out.push(parse_timing_function_keyword(name)?);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse a single `<time>` value (`200ms` or `0.5s` or `0s`).
/// Tokenizer produces:
///   `200ms` → `Number(200)` `Ident("ms")`
///   `0.5s`  → `Number(0)` `Delim('.')` `Number(5)` `Ident("s")`
///   `0s`    → `Number(0)` `Ident("s")`
pub fn parse_time_ms(tokens: &[Token]) -> Option<u32> {
    match tokens {
        [Token::Number(n), Token::Ident(unit)] if *n >= 0 => match unit.as_str() {
            "ms" => Some(*n as u32),
            "s" => Some((*n as u32).checked_mul(1000)?),
            _ => None,
        },
        // 0.5s pattern: integer.integer<unit>. The fractional
        // part is in the second Number token.
        [
            Token::Number(int),
            Token::Delim('.'),
            Token::Number(frac),
            Token::Ident(unit),
        ] if *int >= 0 && *frac >= 0 => {
            let (int_part, frac_str) = (*int as u32, frac.to_string());
            // Determine the fractional digit count: count digits in `frac_str`.
            let denom = 10u32.checked_pow(frac_str.len() as u32)?;
            match unit.as_str() {
                "s" => {
                    // total_ms = (int + frac/denom) * 1000
                    //         = int * 1000 + frac * (1000 / denom)
                    let int_ms = int_part.checked_mul(1000)?;
                    let frac_ms = (*frac as u32).checked_mul(1000)?.checked_div(denom)?;
                    Some(int_ms + frac_ms)
                }
                "ms" => {
                    // 0.5ms — sub-millisecond. Round to int.
                    let int_us = int_part.checked_mul(1000)?;
                    let frac_us = (*frac as u32).checked_mul(1000)?.checked_div(denom)?;
                    Some((int_us + frac_us) / 1000)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Split `value` on commas at depth 0 (parens / function args
/// don't get split). Used by every transition list parser.
fn split_on_top_level_commas(value: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, tok) in value.iter().enumerate() {
        match tok {
            Token::LParen | Token::Function(_) => depth += 1,
            Token::RParen if depth > 0 => depth -= 1,
            Token::Comma if depth == 0 => {
                out.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}

/// Parsed shorthand piece — `transition: <single>` per the CSS L1
/// grammar. Holds all four longhand values for one comma-separated
/// rule.
pub struct TransitionShorthandRule {
    property: TransitionProperty,
    duration: u32,
    timing: TimingFunction,
    delay: u32,
}

pub fn parse_transition_shorthand(value: &[Token]) -> Option<Vec<TransitionShorthandRule>> {
    let segments = split_on_top_level_commas(value);
    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        out.push(parse_transition_shorthand_single(seg)?);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Parse one comma-separated piece of `transition: …`. Pieces
/// can appear in any order; we walk the tokens detecting which
/// kind each is. A second `<time>` token becomes the delay (CSS
/// L1 rule).
pub fn parse_transition_shorthand_single(value: &[Token]) -> Option<TransitionShorthandRule> {
    let mut property: Option<TransitionProperty> = None;
    let mut duration: Option<u32> = None;
    let mut delay: Option<u32> = None;
    let mut timing: Option<TimingFunction> = None;

    let mut i = 0;
    while i < value.len() {
        // Try a `<time>` first — it spans 2..=4 tokens.
        if let Some((ms, consumed)) = try_parse_time_at(value, i) {
            if duration.is_none() {
                duration = Some(ms);
            } else if delay.is_none() {
                delay = Some(ms);
            } else {
                return None; // a third <time> in one piece is invalid
            }
            i += consumed;
            continue;
        }
        // Otherwise — must be an Ident.
        match value.get(i)? {
            Token::Ident(name) => {
                if let Some(t) = parse_timing_function_keyword(name) {
                    if timing.is_some() {
                        return None;
                    }
                    timing = Some(t);
                } else if let Some(p) = parse_transition_property_keyword(name) {
                    if property.is_some() {
                        return None;
                    }
                    property = Some(p);
                } else {
                    return None;
                }
                i += 1;
            }
            _ => return None,
        }
    }

    Some(TransitionShorthandRule {
        property: property.unwrap_or(TransitionProperty::All),
        duration: duration.unwrap_or(0),
        timing: timing.unwrap_or(TimingFunction::Ease),
        delay: delay.unwrap_or(0),
    })
}

/// Try to parse a `<time>` value starting at `value[start]`.
/// Returns `(ms, tokens_consumed)`.
fn try_parse_time_at(value: &[Token], start: usize) -> Option<(u32, usize)> {
    // Try the longest prefix first (4 tokens for `0.5s`).
    if start + 4 <= value.len()
        && let Some(ms) = parse_time_ms(&value[start..start + 4])
    {
        return Some((ms, 4));
    }
    if start + 2 <= value.len()
        && let Some(ms) = parse_time_ms(&value[start..start + 2])
    {
        return Some((ms, 2));
    }
    None
}

pub fn unzip_transition_rules(
    rules: &[TransitionShorthandRule],
) -> (
    Vec<TransitionProperty>,
    Vec<u32>,
    Vec<TimingFunction>,
    Vec<u32>,
) {
    let mut props = Vec::with_capacity(rules.len());
    let mut durs = Vec::with_capacity(rules.len());
    let mut timings = Vec::with_capacity(rules.len());
    let mut delays = Vec::with_capacity(rules.len());
    for r in rules {
        props.push(r.property);
        durs.push(r.duration);
        timings.push(r.timing);
        delays.push(r.delay);
    }
    (props, durs, timings, delays)
}

// ─── calc() expression parser (M6) ───────────────────────────────────
//
// Recursive-descent over the token stream. Grammar:
//
//   calc       = 'calc' '(' sum ')'
//   sum        = product (('+' | '-') product)*
//   product    = factor (('*' | '/') factor)*
//   factor     = leaf | '(' sum ')' | calc
//   leaf       = Number | Length | Percentage
//
// Whitespace is already eaten by the tokenizer. Operator
// precedence follows CSS Values L3 §10.2: * and / bind tighter
// than + and -. Per CSS, `+` and `-` MUST be surrounded by
// whitespace at the source level (`5+5` is invalid; `5 + 5` is
// valid). Our tokenizer doesn't preserve whitespace, so we
// accept both forms — a deliberate relaxation documented in
// DIVERGENCES.md.

/// Parser cursor over a `&[Token]`. Tracks position only.
struct CalcParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> CalcParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Top-level entry: parse a `calc(<sum>)` form. The leading
    /// `Function("calc")` token must already be matched by the
    /// caller (this fn starts after the opening paren).
    fn parse_sum(&mut self) -> Option<CalcExpr> {
        let mut lhs = self.parse_product()?;
        loop {
            let op = match self.peek() {
                Some(Token::Delim('+')) => CalcOp::Add,
                Some(Token::Delim('-')) => CalcOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_product()?;
            lhs = CalcExpr::binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_product(&mut self) -> Option<CalcExpr> {
        let mut lhs = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Some(Token::Delim('*')) => CalcOp::Mul,
                Some(Token::Delim('/')) => CalcOp::Div,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_factor()?;
            lhs = CalcExpr::binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_factor(&mut self) -> Option<CalcExpr> {
        match self.peek()? {
            Token::Number(n) => {
                let n = *n;
                self.advance();
                // A number followed by `fr` is a flex unit and
                // doesn't make sense inside calc(); other unit
                // idents (px / em / rem / ch) would be terminal-
                // incompatible. We accept bare numbers as
                // unitless "Number" leaves; the cell-vs-number
                // distinction is by syntax (bare `5` = number,
                // `5` with explicit cell typing in the property
                // wrapper). For value-position calc operands
                // (e.g., `calc(100% - 4)`) the `4` is a number
                // that resolves as a length because the
                // containing property is a length.
                Some(CalcExpr::Number(n as f64))
            }
            Token::Percentage(n) => {
                let n = *n;
                self.advance();
                Some(CalcExpr::Percent(n as f64))
            }
            Token::Delim('-') => {
                // Unary minus — accept `-5` as a literal.
                self.advance();
                match self.advance()? {
                    Token::Number(n) => Some(CalcExpr::Number(-(*n as f64))),
                    Token::Percentage(n) => Some(CalcExpr::Percent(-(*n as f64))),
                    _ => None,
                }
            }
            Token::Delim('+') => {
                // Unary plus — accept and ignore.
                self.advance();
                self.parse_factor()
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_sum()?;
                match self.advance()? {
                    Token::RParen => Some(inner),
                    _ => None,
                }
            }
            Token::Function(name) if name.eq_ignore_ascii_case("calc") => {
                self.advance();
                let inner = self.parse_sum()?;
                match self.advance()? {
                    Token::RParen => Some(inner),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Parse a `calc(<sum>)` expression starting from the
/// `Function("calc")` token. Returns the AST + the position
/// AFTER the closing `)`. None on parse failure or unbalanced
/// parens.
pub fn parse_calc(tokens: &[Token]) -> Option<CalcExpr> {
    if tokens.is_empty() {
        return None;
    }
    let mut parser = CalcParser::new(tokens);
    // First token must be `calc(`.
    match parser.advance()? {
        Token::Function(name) if name.eq_ignore_ascii_case("calc") => {}
        _ => return None,
    }
    let expr = parser.parse_sum()?;
    match parser.advance()? {
        Token::RParen => {}
        _ => return None,
    }
    // Reject trailing tokens — a calc() must be the entire value.
    if parser.peek().is_some() {
        return None;
    }
    Some(expr)
}

/// `true` iff `tokens` is exactly a single `calc(...)` form.
/// Used by per-property parsers to detect the calc path before
/// trying the bare-value patterns.
pub fn looks_like_calc(tokens: &[Token]) -> bool {
    matches!(tokens.first(), Some(Token::Function(n)) if n.eq_ignore_ascii_case("calc"))
}

#[cfg(test)]
mod calc_parser_tests {
    use super::*;
    use crate::parse::token::Token;

    fn calc_tokens(inner: Vec<Token>) -> Vec<Token> {
        let mut v = vec![Token::Function("calc".to_string())];
        v.extend(inner);
        v.push(Token::RParen);
        v
    }

    #[test]
    fn bare_number() {
        let tokens = calc_tokens(vec![Token::Number(5)]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(e, CalcExpr::Number(5.0));
    }

    #[test]
    fn bare_percent() {
        let tokens = calc_tokens(vec![Token::Percentage(50)]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(e, CalcExpr::Percent(50.0));
    }

    #[test]
    fn add_percent_and_number() {
        let tokens = calc_tokens(vec![
            Token::Percentage(50),
            Token::Delim('+'),
            Token::Number(2),
        ]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(
            e,
            CalcExpr::binary(CalcOp::Add, CalcExpr::Percent(50.0), CalcExpr::Number(2.0))
        );
    }

    #[test]
    fn sub_full_minus_constant() {
        let tokens = calc_tokens(vec![
            Token::Percentage(100),
            Token::Delim('-'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(
            e,
            CalcExpr::binary(CalcOp::Sub, CalcExpr::Percent(100.0), CalcExpr::Number(4.0))
        );
    }

    #[test]
    fn mul_binds_tighter_than_add() {
        // calc(2 + 3 * 4) → Add(2, Mul(3, 4))
        let tokens = calc_tokens(vec![
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::Delim('*'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        let expected = CalcExpr::binary(
            CalcOp::Add,
            CalcExpr::Number(2.0),
            CalcExpr::binary(CalcOp::Mul, CalcExpr::Number(3.0), CalcExpr::Number(4.0)),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn parens_override_precedence() {
        // calc((2 + 3) * 4) → Mul(Add(2,3), 4)
        let tokens = calc_tokens(vec![
            Token::LParen,
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::RParen,
            Token::Delim('*'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        let expected = CalcExpr::binary(
            CalcOp::Mul,
            CalcExpr::binary(CalcOp::Add, CalcExpr::Number(2.0), CalcExpr::Number(3.0)),
            CalcExpr::Number(4.0),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn nested_calc() {
        // calc(calc(2 + 3) * 4) — semantically same as the parens form.
        let tokens = calc_tokens(vec![
            Token::Function("calc".to_string()),
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::RParen,
            Token::Delim('*'),
            Token::Number(4),
        ]);
        let e = parse_calc(&tokens).unwrap();
        let expected = CalcExpr::binary(
            CalcOp::Mul,
            CalcExpr::binary(CalcOp::Add, CalcExpr::Number(2.0), CalcExpr::Number(3.0)),
            CalcExpr::Number(4.0),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn unary_minus() {
        let tokens = calc_tokens(vec![
            Token::Number(5),
            Token::Delim('-'),
            Token::Delim('-'),
            Token::Number(3),
        ]);
        // calc(5 - -3) = Sub(5, -3) — and -3 is a Number(-3.0).
        let e = parse_calc(&tokens).unwrap();
        assert_eq!(
            e,
            CalcExpr::binary(CalcOp::Sub, CalcExpr::Number(5.0), CalcExpr::Number(-3.0))
        );
    }

    #[test]
    fn invalid_form_returns_none() {
        // Missing closing paren.
        let tokens = vec![Token::Function("calc".to_string()), Token::Number(5)];
        assert!(parse_calc(&tokens).is_none());

        // Trailing tokens after the calc.
        let tokens = calc_tokens(vec![Token::Number(5)]);
        let mut with_trail = tokens.clone();
        with_trail.push(Token::Number(99));
        assert!(parse_calc(&with_trail).is_none());

        // Not a calc() at all.
        let tokens = vec![Token::Number(5)];
        assert!(parse_calc(&tokens).is_none());
    }

    #[test]
    fn looks_like_calc_detects_function_token() {
        let yes = vec![Token::Function("calc".to_string())];
        let no = vec![Token::Number(5)];
        assert!(looks_like_calc(&yes));
        assert!(!looks_like_calc(&no));
    }

    // ─── Parse-time constant-eval integration ───────────────────────

    #[test]
    fn parse_size_accepts_constant_calc() {
        // `width: calc(2 + 3)` → `Size::Fixed(5)`.
        let tokens = calc_tokens(vec![Token::Number(2), Token::Delim('+'), Token::Number(3)]);
        assert_eq!(parse_size(&tokens), Some(Size::Fixed(5)));
    }

    #[test]
    fn parse_size_accepts_constant_calc_with_precedence() {
        // `width: calc(2 + 3 * 4)` → `Size::Fixed(14)`.
        let tokens = calc_tokens(vec![
            Token::Number(2),
            Token::Delim('+'),
            Token::Number(3),
            Token::Delim('*'),
            Token::Number(4),
        ]);
        assert_eq!(parse_size(&tokens), Some(Size::Fixed(14)));
    }

    #[test]
    fn parse_size_carries_percent_bearing_calc_as_calc_variant() {
        // M6 full: percent-bearing calc parses into Size::Calc and
        // resolves at layout time.
        let tokens = calc_tokens(vec![
            Token::Percentage(100),
            Token::Delim('-'),
            Token::Number(4),
        ]);
        match parse_size(&tokens) {
            Some(Size::Calc(expr)) => {
                assert!(expr.contains_percent());
            }
            other => panic!("expected Size::Calc, got {other:?}"),
        }
    }

    #[test]
    fn parse_size_clamps_negative_constant_calc_to_zero() {
        // `width: calc(2 - 10)` → -8 cells → clamped to 0.
        let tokens = calc_tokens(vec![Token::Number(2), Token::Delim('-'), Token::Number(10)]);
        assert_eq!(parse_size(&tokens), Some(Size::Fixed(0)));
    }

    #[test]
    fn parse_length_accepts_constant_calc_negative_result() {
        // `top: calc(-3 * 2)` → -6 → Length::Cells(-6).
        let tokens = calc_tokens(vec![
            Token::Delim('-'),
            Token::Number(3),
            Token::Delim('*'),
            Token::Number(2),
        ]);
        assert_eq!(parse_length(&tokens), Some(Length::Cells(-6)));
    }

    #[test]
    fn parse_length_carries_percent_bearing_calc_as_calc_variant() {
        let tokens = calc_tokens(vec![
            Token::Percentage(50),
            Token::Delim('+'),
            Token::Number(2),
        ]);
        match parse_length(&tokens) {
            Some(Length::Calc(expr)) => {
                assert!(expr.contains_percent());
            }
            other => panic!("expected Length::Calc, got {other:?}"),
        }
    }
}
