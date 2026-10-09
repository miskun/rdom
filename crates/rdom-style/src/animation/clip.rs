//! Interpolation of `clip-path` (CSS Masking 1 §5.1, CSS Shapes 1 §3.2
//! "Interpolation of Basic Shapes"): two basic shapes of one kind and one
//! reference box interpolate their lengths — an `inset()`'s insets and
//! radii, a `circle()` / `ellipse()`'s length radii and centre, a
//! `polygon()`'s points when the fill rule and the number of points
//! match; anything else is discrete.

use std::sync::Arc;

use super::value::{Animate, Cx};
use crate::layout::{BasicShape, ClipPath, ClipRect, Length, ShapeRadius};

fn lengths<const N: usize>(
    a: &[Length; N],
    b: &[Length; N],
    p: f64,
    cx: &Cx,
) -> Option<[Length; N]> {
    let v: Vec<Length> = a
        .iter()
        .zip(b)
        .map(|(x, y)| x.animate(y, p, cx))
        .collect::<Option<_>>()?;
    v.try_into().ok()
}

fn radius(a: &ShapeRadius, b: &ShapeRadius, p: f64, cx: &Cx) -> Option<ShapeRadius> {
    match (a, b) {
        (ShapeRadius::Length(x), ShapeRadius::Length(y)) => {
            Some(ShapeRadius::Length(x.animate(y, p, cx)?))
        }
        (x, y) if x == y => Some(x.clone()),
        _ => None,
    }
}

fn shape(a: &BasicShape, b: &BasicShape, p: f64, cx: &Cx) -> Option<BasicShape> {
    use BasicShape as S;
    Some(match (a, b) {
        (
            S::Inset {
                insets: i,
                round: r,
            },
            S::Inset {
                insets: j,
                round: q,
            },
        ) => S::Inset {
            insets: lengths(i, j, p, cx)?,
            round: lengths(r, q, p, cx)?,
        },
        (S::Circle { radius: r, at: c }, S::Circle { radius: q, at: d }) => S::Circle {
            radius: radius(r, q, p, cx)?,
            at: lengths(c, d, p, cx)?,
        },
        (
            S::Ellipse { rx, ry, at: c },
            S::Ellipse {
                rx: sx,
                ry: sy,
                at: d,
            },
        ) => S::Ellipse {
            rx: radius(rx, sx, p, cx)?,
            ry: radius(ry, sy, p, cx)?,
            at: lengths(c, d, p, cx)?,
        },
        (
            S::Polygon { evenodd: e, points },
            S::Polygon {
                evenodd: f,
                points: qs,
            },
        ) if e == f && points.len() == qs.len() => S::Polygon {
            evenodd: *e,
            points: points
                .iter()
                .zip(qs)
                .map(|(x, y)| lengths(x, y, p, cx))
                .collect::<Option<_>>()?,
        },
        (x, y) if x == y => x.clone(),
        _ => return None,
    })
}

impl Animate for ClipPath {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        match (self, to) {
            (
                ClipPath::Shape {
                    shape: Some(a),
                    reference: r,
                },
                ClipPath::Shape {
                    shape: Some(b),
                    reference: q,
                },
            ) if r == q => Some(ClipPath::Shape {
                shape: Some(Arc::new(shape(a, b, p, cx)?)),
                reference: *r,
            }),
            (x, y) if x == y => Some(x.clone()),
            _ => None,
        }
    }
}

/// CSS 2.1 §11.1.2 / CSS Masking 1 §6.1 `clip`: by computed value, as a
/// rectangle — edge by edge between two `rect()`s whose `auto` edges
/// match; anything else is discrete (C15G-LEGACY-CLIP).
impl Animate for ClipRect {
    fn animate(&self, to: &Self, p: f64, cx: &Cx) -> Option<Self> {
        let edge = |a: Option<i32>, b: Option<i32>| -> Option<Option<i32>> {
            match (a, b) {
                (Some(a), Some(b)) => Some(Some(a.animate(&b, p, cx)?)),
                (None, None) => Some(None),
                _ => None,
            }
        };
        match (*self, *to) {
            (
                ClipRect::Rect {
                    top: t0,
                    right: r0,
                    bottom: b0,
                    left: l0,
                },
                ClipRect::Rect {
                    top: t1,
                    right: r1,
                    bottom: b1,
                    left: l1,
                },
            ) => Some(ClipRect::Rect {
                top: edge(t0, t1)?,
                right: edge(r0, r1)?,
                bottom: edge(b0, b1)?,
                left: edge(l0, l1)?,
            }),
            (x, y) if x == y => Some(x),
            _ => None,
        }
    }
}
