//! `clip-path` (CSS Masking 1 §5.1) and its `<basic-shape>`s (CSS Shapes
//! 1 §3.1): lengths in cells and percentages — a clip is geometry, so a
//! pixel length is rejected (DESIGN, "Pixel lengths select, cells
//! measure").

use std::sync::Arc;

use super::numeric::{
    LengthPercentage, Range, cells_i32, components, length_percentage, split_commas,
};
use crate::calc::CalcExpr;
use crate::layout::{
    BasicShape, ClipPath, ClipRect, GeometryBox, Length, PaintLength, ShapeRadius,
};
use crate::parse::token::Token;

fn keyword(c: &[Token]) -> Option<String> {
    match c {
        [Token::Ident(k)] => Some(k.to_ascii_lowercase()),
        _ => None,
    }
}

/// A `<length-percentage>` in `range`, as a [`Length`].
fn lp(c: &[Token], range: Range) -> Option<Length> {
    Some(match length_percentage(c, range)? {
        LengthPercentage::Integer(n) => Length::Cells(n),
        LengthPercentage::Cells(v) => Length::Cells(cells_i32(v)),
        LengthPercentage::Expr(e) => Length::calc(e),
    })
}

fn percent(p: f64) -> Length {
    Length::calc(CalcExpr::Percent(p))
}

/// `clip-path`: `none | <clip-source> | [<basic-shape> || <geometry-box>]`.
pub fn parse_clip_path(value: &[Token]) -> Option<ClipPath> {
    match value {
        [Token::Ident(k)] if k.eq_ignore_ascii_case("none") => return Some(ClipPath::None),
        [Token::Url(u)] => return Some(ClipPath::Url(Arc::from(u.as_str()))),
        [Token::Function(f), Token::String(u), Token::RParen] if f.eq_ignore_ascii_case("url") => {
            return Some(ClipPath::Url(Arc::from(u.as_str())));
        }
        _ => {}
    }
    let mut shape = None;
    let mut reference = None;
    for part in components(value)? {
        if let Some(b) = keyword(part).and_then(|k| {
            GeometryBox::KEYWORDS
                .iter()
                .find(|(name, _)| *name == k)
                .map(|(_, b)| *b)
        }) {
            if reference.replace(b).is_some() {
                return None;
            }
        } else if shape.replace(basic_shape(part)?).is_some() {
            return None;
        }
    }
    if shape.is_none() && reference.is_none() {
        return None;
    }
    Some(ClipPath::Shape {
        shape: shape.map(Arc::new),
        reference: reference.unwrap_or_default(),
    })
}

/// One `<basic-shape>` function.
fn basic_shape(part: &[Token]) -> Option<BasicShape> {
    let [Token::Function(name), inner @ .., Token::RParen] = part else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "inset" => inset(&components(inner)?),
        "circle" => {
            let (radii, at) = radii_and_position(inner)?;
            let radius = match radii.as_slice() {
                [] => ShapeRadius::ClosestSide,
                [r] => radius(r)?,
                _ => return None,
            };
            Some(BasicShape::Circle { radius, at })
        }
        "ellipse" => {
            let (radii, at) = radii_and_position(inner)?;
            let (rx, ry) = match radii.as_slice() {
                [] => (ShapeRadius::ClosestSide, ShapeRadius::ClosestSide),
                [a, b] => (radius(a)?, radius(b)?),
                _ => return None,
            };
            Some(BasicShape::Ellipse { rx, ry, at })
        }
        "polygon" => {
            let mut parts = split_commas(inner)?.into_iter().peekable();
            let evenodd = fill_rule(parts.peek().copied())
                .inspect(|_| {
                    parts.next();
                })
                .unwrap_or(false);
            let points = parts
                .map(|p| match components(p)?.as_slice() {
                    [x, y] => Some([lp(x, Range::Any)?, lp(y, Range::Any)?]),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            (!points.is_empty()).then_some(BasicShape::Polygon { evenodd, points })
        }
        "path" => {
            let parts = split_commas(inner)?;
            let (evenodd, data) = match parts.as_slice() {
                [rule, data] => (fill_rule(Some(rule))?, *data),
                [data] => (false, *data),
                _ => return None,
            };
            let [Token::String(s)] = data else {
                return None;
            };
            Some(BasicShape::Path {
                evenodd,
                data: Arc::from(s.as_str()),
            })
        }
        _ => None,
    }
}

/// `nonzero` (false) or `evenodd` (true).
fn fill_rule(c: Option<&[Token]>) -> Option<bool> {
    match keyword(c?)?.as_str() {
        "nonzero" => Some(false),
        "evenodd" => Some(true),
        _ => None,
    }
}

/// `inset(<length-percentage>{1,4} [round <length-percentage>{1,4}]?)`.
fn inset(parts: &[&[Token]]) -> Option<BasicShape> {
    let split = parts
        .iter()
        .position(|p| keyword(p).as_deref() == Some("round"))
        .unwrap_or(parts.len());
    let four = |values: &[&[Token]], range: Range| -> Option<[Length; 4]> {
        let v = values
            .iter()
            .map(|c| lp(c, range))
            .collect::<Option<Vec<_>>>()?;
        Some(match v.as_slice() {
            [a] => [a.clone(), a.clone(), a.clone(), a.clone()],
            [a, b] => [a.clone(), b.clone(), a.clone(), b.clone()],
            [a, b, c] => [a.clone(), b.clone(), c.clone(), b.clone()],
            [a, b, c, d] => [a.clone(), b.clone(), c.clone(), d.clone()],
            _ => return None,
        })
    };
    let insets = four(&parts[..split], Range::Any)?;
    let round = if split < parts.len() {
        four(&parts[split + 1..], Range::NonNegative)?
    } else {
        [
            Length::Cells(0),
            Length::Cells(0),
            Length::Cells(0),
            Length::Cells(0),
        ]
    };
    Some(BasicShape::Inset { insets, round })
}

/// A `<shape-radius>`: a non-negative length or a side keyword.
fn radius(c: &[Token]) -> Option<ShapeRadius> {
    match keyword(c).as_deref() {
        Some("closest-side") => Some(ShapeRadius::ClosestSide),
        Some("farthest-side") => Some(ShapeRadius::FarthestSide),
        Some(_) => None,
        None => lp(c, Range::NonNegative).map(ShapeRadius::Length),
    }
}

/// The radii before `at`, and the position after it (the centre by
/// default).
fn radii_and_position(inner: &[Token]) -> Option<(Vec<&[Token]>, [Length; 2])> {
    let parts = components(inner)?;
    let at = parts
        .iter()
        .position(|p| keyword(p).as_deref() == Some("at"));
    let (radii, position) = match at {
        Some(i) => (parts[..i].to_vec(), position(&parts[i + 1..])?),
        None => (parts, [percent(50.0), percent(50.0)]),
    };
    Some((radii, position))
}

/// A `<position>` of one or two values — keywords in either order when
/// both are keywords — each a keyword's percentage or a length.
fn position(parts: &[&[Token]]) -> Option<[Length; 2]> {
    // (vertical, horizontal, value)
    let read = |c: &[Token]| -> Option<(bool, bool, Length)> {
        Some(match keyword(c).as_deref() {
            Some("left") => (false, true, percent(0.0)),
            Some("right") => (false, true, percent(100.0)),
            Some("top") => (true, false, percent(0.0)),
            Some("bottom") => (true, false, percent(100.0)),
            Some("center") => (true, true, percent(50.0)),
            Some(_) => return None,
            None => (true, true, lp(c, Range::Any)?),
        })
    };
    match parts {
        [a] => {
            let (v, h, l) = read(a)?;
            Some(if h {
                [l, percent(50.0)]
            } else {
                debug_assert!(v);
                [percent(50.0), l]
            })
        }
        [a, b] => {
            let (a_v, a_h, a) = read(a)?;
            let (b_v, b_h, b) = read(b)?;
            if a_h && b_v {
                Some([a, b])
            } else if a_v && b_h {
                Some([b, a])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `clip` (CSS 2.1 §11.1.2): `auto | rect(<top>, <right>, <bottom>,
/// <left>)`, each edge a cell length or `auto`; the legacy space-separated
/// form too (§11.1.2: "user agents may support separation of offsets with
/// whitespace"). A clip is geometry, so a percentage, or a pixel length,
/// is rejected (DESIGN) (C15G-LEGACY-CLIP) — except a rectangle empty
/// along an axis whose two edges are pixel lengths (or `0`): it is empty
/// at any pixel-to-cell mapping, so it is the empty rect, `rect(0, 0, 0,
/// 0)` — WordPress's `rect(1px, 1px, 1px, 1px)` (C16G-LEGACY-CLIP-PX).
pub fn parse_clip_rect(value: &[Token]) -> Option<ClipRect> {
    if keyword(value).as_deref() == Some("auto") {
        return Some(ClipRect::Auto);
    }
    let [Token::Function(name), inner @ .., Token::RParen] = value else {
        return None;
    };
    if !name.eq_ignore_ascii_case("rect") {
        return None;
    }
    let commas = split_commas(inner)?;
    let parts: Vec<&[Token]> = if commas.len() == 4 {
        commas
    } else {
        components(inner)?
    };
    let [top, right, bottom, left] = parts.as_slice() else {
        return None;
    };
    let edges = [edge(top)?, edge(right)?, edge(bottom)?, edge(left)?];
    let cells = |e: Edge| match e {
        Edge::Auto => Some(None),
        Edge::Cells(n) => Some(Some(n)),
        Edge::Px(_) => None,
    };
    if !edges.iter().any(|e| matches!(e, Edge::Px(_))) {
        let [top, right, bottom, left] = edges.map(cells);
        return Some(ClipRect::Rect {
            top: top?,
            right: right?,
            bottom: bottom?,
            left: left?,
        });
    }
    let [top, right, bottom, left] = edges.map(Edge::px);
    let empty =
        |start: Option<f32>, end: Option<f32>| matches!((start, end), (Some(s), Some(e)) if e <= s);
    (empty(top, bottom) || empty(left, right)).then_some(ClipRect::Rect {
        top: Some(0),
        right: Some(0),
        bottom: Some(0),
        left: Some(0),
    })
}

/// One `rect()` edge.
#[derive(Clone, Copy)]
enum Edge {
    Auto,
    Cells(i32),
    /// A pixel-family length, in CSS pixels.
    Px(f32),
}

impl Edge {
    /// The edge in pixels, when it has one: a pixel length, or a zero.
    fn px(self) -> Option<f32> {
        match self {
            Edge::Px(p) => Some(p),
            Edge::Cells(0) => Some(0.0),
            Edge::Cells(_) | Edge::Auto => None,
        }
    }
}

fn edge(c: &[Token]) -> Option<Edge> {
    if keyword(c).as_deref() == Some("auto") {
        return Some(Edge::Auto);
    }
    match super::border::paint_length(c, false, Range::Any)? {
        PaintLength::Px(p) => Some(Edge::Px(p)),
        _ => match lp(c, Range::Any)? {
            Length::Cells(n) => Some(Edge::Cells(n)),
            _ => None,
        },
    }
}
