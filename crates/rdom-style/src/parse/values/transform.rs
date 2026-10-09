//! The transform properties (CSS Transforms 1 §5–§7, Transforms 2
//! §6–§7): `translate`, `transform`, `rotate`, `scale`,
//! `transform-origin`, `transform-box`.
//!
//! Translations are geometry: their offsets are `<length-percentage>`s in
//! cells, and a pixel length is rejected (DESIGN, "Pixel lengths select,
//! cells measure"). Everything else a transform says — rotation, scaling,
//! skews, matrices, perspective, a z offset, the origin — moves nothing in
//! a cell grid; it is checked against its grammar (pixels included) and
//! kept, inert.

use std::sync::Arc;

use super::border::paint_length;
use super::numeric::{
    LengthPercentage, Range, cells_i32, components, length_percentage, number, parse_angle,
    split_commas,
};
use crate::layout::{
    Length, PaintLength, Rotate, Scale, TransformBox, TransformFunction, TransformList,
    TransformOrigin, Translate, TranslateFunction,
};
use crate::parse::token::Token;

fn is_keyword(value: &[Token], keyword: &str) -> bool {
    matches!(value, [Token::Ident(k)] if k.eq_ignore_ascii_case(keyword))
}

/// A translation offset: a signed `<length-percentage>` in cells.
fn offset(component: &[Token]) -> Option<Length> {
    Some(match length_percentage(component, Range::Any)? {
        LengthPercentage::Integer(n) => Length::Cells(n),
        LengthPercentage::Cells(v) => Length::Cells(cells_i32(v)),
        LengthPercentage::Expr(e) => Length::calc(e),
    })
}

/// A z offset: any `<length>`, pixels included — it moves nothing.
fn depth(component: &[Token]) -> Option<PaintLength> {
    paint_length(component, false, Range::Any)
}

/// `translate` (CSS Transforms 2 §6.1): `none | <length-percentage>
/// [<length-percentage> <length>?]?`. `Some(None)` is `none`.
pub fn parse_translate(value: &[Token]) -> Option<Option<Translate>> {
    if is_keyword(value, "none") {
        return Some(None);
    }
    let parts = components(value)?;
    let (x, y, z) = match parts.as_slice() {
        [x] => (offset(x)?, Length::Cells(0), PaintLength::Cells(0.0)),
        [x, y] => (offset(x)?, offset(y)?, PaintLength::Cells(0.0)),
        [x, y, z] => (offset(x)?, offset(y)?, depth(z)?),
        _ => return None,
    };
    Some(Some(Translate { x, y, z }))
}

/// `transform` (CSS Transforms 1 §5): `none | <transform-function>+`.
pub fn parse_transform(value: &[Token]) -> Option<TransformList> {
    if is_keyword(value, "none") {
        return Some(TransformList::none());
    }
    let functions = components(value)?
        .into_iter()
        .map(transform_function)
        .collect::<Option<Vec<_>>>()?;
    (!functions.is_empty()).then(|| TransformList::new(functions))
}

/// The functions other than the translations, by their CSS spelling, each
/// with the grammar its arguments must match (Transforms 1 §12,
/// Transforms 2 §13).
const INERT: &[(&str, Args)] = &[
    ("matrix", Args::Numbers(6)),
    ("matrix3d", Args::Numbers(16)),
    ("scale", Args::Factors(1, 2)),
    ("scale3d", Args::Factors(3, 3)),
    ("scaleX", Args::Factors(1, 1)),
    ("scaleY", Args::Factors(1, 1)),
    ("scaleZ", Args::Factors(1, 1)),
    ("rotate", Args::Angles(1, 1)),
    ("rotateX", Args::Angles(1, 1)),
    ("rotateY", Args::Angles(1, 1)),
    ("rotateZ", Args::Angles(1, 1)),
    ("rotate3d", Args::Rotate3d),
    ("skew", Args::Angles(1, 2)),
    ("skewX", Args::Angles(1, 1)),
    ("skewY", Args::Angles(1, 1)),
    ("perspective", Args::Perspective),
];

/// An inert function's argument grammar.
#[derive(Clone, Copy)]
enum Args {
    /// Exactly `n` `<number>`s.
    Numbers(usize),
    /// `min..=max` `<number> | <percentage>`s.
    Factors(usize, usize),
    /// `min..=max` `<angle> | <zero>`s.
    Angles(usize, usize),
    /// `<number>, <number>, <number>, <angle> | <zero>`.
    Rotate3d,
    /// `<length [0,∞]> | none`.
    Perspective,
}

/// `<angle> | <zero>` (Transforms 1 §12: the functions take a unitless 0).
fn angle_or_zero(c: &[Token]) -> bool {
    matches!(c, [Token::Number(0)]) || parse_angle(c).is_some()
}

/// `<number> | <percentage>`, as a number.
fn factor(c: &[Token]) -> Option<f64> {
    match c {
        [Token::Percentage(p)] => Some(p / 100.0),
        [Token::Delim('-'), Token::Percentage(p)] => Some(-p / 100.0),
        _ => number(c, Range::Any),
    }
}

impl Args {
    fn accepts(self, args: &[&[Token]]) -> bool {
        let n = args.len();
        match self {
            Args::Numbers(k) => n == k && args.iter().all(|a| number(a, Range::Any).is_some()),
            Args::Factors(lo, hi) => {
                (lo..=hi).contains(&n) && args.iter().all(|a| factor(a).is_some())
            }
            Args::Angles(lo, hi) => (lo..=hi).contains(&n) && args.iter().all(|a| angle_or_zero(a)),
            Args::Rotate3d => {
                n == 4
                    && args[..3].iter().all(|a| number(a, Range::Any).is_some())
                    && angle_or_zero(args[3])
            }
            Args::Perspective => {
                n == 1
                    && (is_keyword(args[0], "none")
                        || paint_length(args[0], false, Range::NonNegative).is_some())
            }
        }
    }
}

/// One `<transform-function>`.
fn transform_function(part: &[Token]) -> Option<TransformFunction> {
    let [Token::Function(name), inner @ .., Token::RParen] = part else {
        return None;
    };
    let args = if inner.is_empty() {
        Vec::new()
    } else {
        split_commas(inner)?
    };
    let translate = |function, offset| Some(TransformFunction::Translate { function, offset });
    let zero = || Length::Cells(0);
    let lower = name.to_ascii_lowercase();
    match (lower.as_str(), args.as_slice()) {
        ("translate", [x]) => translate(
            TranslateFunction::Translate,
            Translate::new(offset(x)?, zero()),
        ),
        ("translate", [x, y]) => translate(
            TranslateFunction::Translate,
            Translate::new(offset(x)?, offset(y)?),
        ),
        ("translatex", [x]) => translate(
            TranslateFunction::TranslateX,
            Translate::new(offset(x)?, zero()),
        ),
        ("translatey", [y]) => translate(
            TranslateFunction::TranslateY,
            Translate::new(zero(), offset(y)?),
        ),
        ("translate3d", [x, y, z]) => translate(
            TranslateFunction::Translate3d,
            Translate {
                x: offset(x)?,
                y: offset(y)?,
                z: depth(z)?,
            },
        ),
        ("translatez", [z]) => translate(
            TranslateFunction::TranslateZ,
            Translate {
                z: depth(z)?,
                ..Translate::default()
            },
        ),
        _ => {
            let (spelling, grammar) = INERT.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
            if !grammar.accepts(&args) {
                return None;
            }
            let css = format!("{spelling}({})", super::render_value(inner));
            Some(TransformFunction::Inert {
                name: Arc::from(lower),
                css: Arc::from(css),
            })
        }
    }
}

/// `rotate` (CSS Transforms 2 §6.2): `none | <angle> | [x | y | z |
/// <number>{3}] && <angle>`. `Some(None)` is `none`.
pub fn parse_rotate(value: &[Token]) -> Option<Option<Rotate>> {
    if is_keyword(value, "none") {
        return Some(None);
    }
    let parts = components(value)?;
    let axis_keyword = |c: &[Token]| match c {
        [Token::Ident(k)] => match k.to_ascii_lowercase().as_str() {
            "x" => Some([1.0, 0.0, 0.0]),
            "y" => Some([0.0, 1.0, 0.0]),
            "z" => Some([0.0, 0.0, 1.0]),
            _ => None,
        },
        _ => None,
    };
    let vector = |cs: &[&[Token]]| -> Option<[f64; 3]> {
        Some([
            number(cs[0], Range::Any)?,
            number(cs[1], Range::Any)?,
            number(cs[2], Range::Any)?,
        ])
    };
    let rotate = |axis, degrees| Some(Some(Rotate { axis, degrees }));
    match parts.as_slice() {
        [a] => rotate(None, parse_angle(a)?),
        [a, b] => match (parse_angle(a), parse_angle(b)) {
            (Some(d), None) => rotate(Some(axis_keyword(b)?), d),
            (None, Some(d)) => rotate(Some(axis_keyword(a)?), d),
            _ => None,
        },
        [a, rest @ ..] if rest.len() == 3 && parse_angle(a).is_some() => {
            rotate(Some(vector(rest)?), parse_angle(a)?)
        }
        [first @ .., d] if first.len() == 3 => rotate(Some(vector(first)?), parse_angle(d)?),
        _ => None,
    }
}

/// `scale` (CSS Transforms 2 §6.3): `none | [<number> | <percentage>]{1,3}`
/// — y defaults to x, z to 1. `Some(None)` is `none`.
pub fn parse_scale(value: &[Token]) -> Option<Option<Scale>> {
    if is_keyword(value, "none") {
        return Some(None);
    }
    let factors = components(value)?
        .into_iter()
        .map(factor)
        .collect::<Option<Vec<_>>>()?;
    let (x, y, z) = match factors.as_slice() {
        [x] => (*x, *x, 1.0),
        [x, y] => (*x, *y, 1.0),
        [x, y, z] => (*x, *y, *z),
        _ => return None,
    };
    Some(Some(Scale { x, y, z }))
}

/// `transform-box` (CSS Transforms 1 §7).
pub fn parse_transform_box(value: &[Token]) -> Option<TransformBox> {
    let [Token::Ident(k)] = value else {
        return None;
    };
    TransformBox::KEYWORDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(k))
        .map(|(_, b)| *b)
}

/// `transform-origin` (CSS Transforms 1 §6): one position keyword or
/// length; or an x and a y, keywords in either order when both are
/// keywords, then an optional z `<length>`. A keyword computes to its
/// percentage.
pub fn parse_transform_origin(value: &[Token]) -> Option<TransformOrigin> {
    use crate::calc::CalcExpr;
    let parts = components(value)?;
    let percent = |p: f64| PaintLength::calc(CalcExpr::Percent(p));
    // A component as (is it a vertical keyword, is it a horizontal one,
    // its length).
    let read = |c: &[Token]| -> Option<(bool, bool, PaintLength)> {
        let [Token::Ident(k)] = c else {
            return paint_length(c, true, Range::Any).map(|l| (true, true, l));
        };
        Some(match k.to_ascii_lowercase().as_str() {
            "left" => (false, true, percent(0.0)),
            "right" => (false, true, percent(100.0)),
            "top" => (true, false, percent(0.0)),
            "bottom" => (true, false, percent(100.0)),
            "center" => (true, true, percent(50.0)),
            _ => return None,
        })
    };
    let pair = |a: &[Token], b: &[Token]| -> Option<(PaintLength, PaintLength)> {
        let (a_v, a_h, a) = read(a)?;
        let (b_v, b_h, b) = read(b)?;
        if a_h && b_v {
            Some((a, b))
        } else if a_v && b_h {
            // Two keywords the other way round (`top left`): a length is
            // both horizontal and vertical, so it never lands here.
            Some((b, a))
        } else {
            None
        }
    };
    let (x, y, z) = match parts.as_slice() {
        [a] => {
            let (vertical, horizontal, l) = read(a)?;
            if horizontal {
                (l, percent(50.0), PaintLength::Cells(0.0))
            } else {
                debug_assert!(vertical);
                (percent(50.0), l, PaintLength::Cells(0.0))
            }
        }
        [a, b] => {
            let (x, y) = pair(a, b)?;
            (x, y, PaintLength::Cells(0.0))
        }
        [a, b, z] => {
            let (x, y) = pair(a, b)?;
            (x, y, paint_length(z, false, Range::Any)?)
        }
        _ => return None,
    };
    Some(TransformOrigin::new(x, y, z))
}
