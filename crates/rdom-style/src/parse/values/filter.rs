//! `filter` and `backdrop-filter` (Filter Effects 1 §5–§6, Filter Effects
//! 2 §3): `none | <filter-value-list>`.

use std::sync::Arc;

use super::border::paint_length;
use super::numeric::{Range, components, number, parse_angle, split_commas};
use super::shadow::parse_shadow;
use crate::layout::{FilterFunction, FilterList, PaintLength};
use crate::parse::token::Token;

/// `filter` / `backdrop-filter`: `none`, or one or more filter functions
/// and `url()`s, space-separated.
pub fn parse_filter(value: &[Token]) -> Option<FilterList> {
    if matches!(value, [Token::Ident(k)] if k.eq_ignore_ascii_case("none")) {
        return Some(FilterList::none());
    }
    let functions = components(value)?
        .into_iter()
        .map(filter_function)
        .collect::<Option<Vec<_>>>()?;
    (!functions.is_empty()).then(|| FilterList::new(functions))
}

/// `<number [0,∞]> | <percentage [0,∞]>`, as a number; `default` when
/// omitted, and at most `max`.
fn amount(args: &[&[Token]], default: f64, max: f64) -> Option<f64> {
    let v = match args {
        [] => default,
        [[Token::Percentage(p)]] => p / 100.0,
        [a] => {
            if matches!(a.first(), Some(Token::Delim('-'))) {
                return None;
            }
            number(a, Range::NonNegative)?
        }
        _ => return None,
    };
    (v >= 0.0).then_some(v.min(max))
}

/// One `<filter-function>` or `url()` (§6).
fn filter_function(part: &[Token]) -> Option<FilterFunction> {
    use FilterFunction as F;
    match part {
        [Token::Url(u)] => return Some(F::Url(Arc::from(u.as_str()))),
        [Token::Function(f), Token::String(u), Token::RParen] if f.eq_ignore_ascii_case("url") => {
            return Some(F::Url(Arc::from(u.as_str())));
        }
        _ => {}
    }
    let [Token::Function(name), inner @ .., Token::RParen] = part else {
        return None;
    };
    let args = if inner.is_empty() {
        Vec::new()
    } else {
        split_commas(inner)?
    };
    let one = f64::INFINITY;
    Some(match name.to_ascii_lowercase().as_str() {
        "brightness" => F::Brightness(amount(&args, 1.0, one)?),
        "contrast" => F::Contrast(amount(&args, 1.0, one)?),
        "saturate" => F::Saturate(amount(&args, 1.0, one)?),
        // §6: values over 100% are clamped to 1.
        "grayscale" => F::Grayscale(amount(&args, 1.0, 1.0)?),
        "sepia" => F::Sepia(amount(&args, 1.0, 1.0)?),
        "invert" => F::Invert(amount(&args, 1.0, 1.0)?),
        "opacity" => F::Opacity(amount(&args, 1.0, 1.0)?),
        "hue-rotate" => F::HueRotate(match args.as_slice() {
            [] => 0.0,
            [[Token::Number(0)]] => 0.0,
            [a] => parse_angle(a)?,
            _ => return None,
        }),
        "blur" => F::Blur(match args.as_slice() {
            [] => PaintLength::Cells(0.0),
            [a] => paint_length(a, false, Range::NonNegative)?,
            _ => return None,
        }),
        "drop-shadow" => {
            let lengths = components(inner)?
                .into_iter()
                .filter(|c| paint_length(c, false, Range::Any).is_some())
                .count();
            let shadow = parse_shadow(inner).filter(|s| !s.inset && lengths <= 3)?;
            F::DropShadow(shadow)
        }
        _ => return None,
    })
}
