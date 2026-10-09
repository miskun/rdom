//! `box-shadow` (CSS Backgrounds 3 §6.1): `none | <shadow>#`, each
//! `<shadow>` a color, two to four lengths and `inset`, in any order
//! with the lengths together.

use super::border::paint_length;
use super::color::parse_color;
use super::numeric::{Range, components, split_commas};
use crate::TuiColor;
use crate::layout::{BoxShadow, PaintLength};
use crate::parse::token::Token;

/// `box-shadow`: `none` (no shadow: an empty list) or one or more
/// comma-separated `<shadow>`s, front to back.
pub fn parse_box_shadow(value: &[Token]) -> Option<Vec<BoxShadow>> {
    if matches!(value, [Token::Ident(k)] if k.eq_ignore_ascii_case("none")) {
        return Some(Vec::new());
    }
    split_commas(value)?.into_iter().map(parse_shadow).collect()
}

/// One `<shadow>`: `<color>? && [ <length>{2} <length [0,∞]>?
/// <length>? ] && inset?` — offset-x, offset-y, blur radius, spread
/// distance; an omitted color is `currentcolor`, an omitted blur or
/// spread 0.
pub(crate) fn parse_shadow(tokens: &[Token]) -> Option<BoxShadow> {
    let parts = components(tokens)?;
    let mut lengths: Option<Vec<PaintLength>> = None;
    let mut color = None;
    let mut inset = false;
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if !inset && matches!(part, [Token::Ident(k)] if k.eq_ignore_ascii_case("inset")) {
            inset = true;
            i += 1;
        } else if color.is_none()
            && let Some(c) = parse_color(part)
        {
            color = Some(c);
            i += 1;
        } else if lengths.is_none() && paint_length(part, false, Range::Any).is_some() {
            // The lengths run together: the blur radius is non-negative.
            let mut run = Vec::with_capacity(4);
            while run.len() < 4
                && let Some(l) = parts.get(i).and_then(|p| {
                    let range = if run.len() == 2 {
                        Range::NonNegative
                    } else {
                        Range::Any
                    };
                    paint_length(p, false, range)
                })
            {
                run.push(l);
                i += 1;
            }
            lengths = Some(run);
        } else {
            return None;
        }
    }
    let mut lengths = lengths.filter(|l| l.len() >= 2)?.into_iter();
    let zero = || PaintLength::Cells(0.0);
    Some(BoxShadow {
        inset,
        offset_x: lengths.next()?,
        offset_y: lengths.next()?,
        blur: lengths.next().unwrap_or_else(zero),
        spread: lengths.next().unwrap_or_else(zero),
        color: color.unwrap_or(TuiColor::CurrentColor),
    })
}
