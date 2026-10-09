//! The `clip-path` values (CSS Masking 1 §5, CSS Shapes 1 §3) and their
//! geometry on the cell grid.
//!
//! A cell is clipped whole: it is inside the clip when its centre is
//! inside the shape — simple, deterministic, and the same answer for
//! paint and hit-testing. The geometry is in cells, so a `circle()` is
//! round in cells, which are about twice as tall as wide on screen.

use std::sync::Arc;

use super::Length;
use crate::calc::{CalcExpr, ResolveCtx};

/// `clip-path` (CSS Masking 1 §5.1): `none`, a `url()` reference, or a
/// basic shape and / or a reference box.
///
/// Closed (DESIGN): the property's three forms; paint and hit-testing
/// must answer each.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ClipPath {
    /// The initial value: no clipping.
    #[default]
    None,
    /// `url(…)`: an SVG `<clipPath>` reference — kept, inert (there is no
    /// SVG to reference); still a stacking context.
    Url(Arc<str>),
    /// `<basic-shape> || <geometry-box>`: the shape (none: the box itself)
    /// within the reference box.
    Shape {
        shape: Option<Arc<BasicShape>>,
        reference: GeometryBox,
    },
}

/// A `<geometry-box>` (CSS Masking 1 §5.1): the reference box of a
/// `clip-path` shape.
///
/// Closed (DESIGN): the keywords are the grammar; each is a box layout has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GeometryBox {
    MarginBox,
    /// The initial reference box of a shape.
    #[default]
    BorderBox,
    PaddingBox,
    ContentBox,
    FillBox,
    StrokeBox,
    ViewBox,
}

impl GeometryBox {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, GeometryBox)] = &[
        ("margin-box", GeometryBox::MarginBox),
        ("border-box", GeometryBox::BorderBox),
        ("padding-box", GeometryBox::PaddingBox),
        ("content-box", GeometryBox::ContentBox),
        ("fill-box", GeometryBox::FillBox),
        ("stroke-box", GeometryBox::StrokeBox),
        ("view-box", GeometryBox::ViewBox),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, b)| *b == self)
            .map_or("border-box", |(k, _)| k)
    }
}

/// The radius of a `circle()` / `ellipse()` (CSS Shapes 1 §3.1.1).
///
/// Closed (DESIGN): a length or the two side keywords.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ShapeRadius {
    Length(Length),
    /// The initial radius.
    #[default]
    ClosestSide,
    FarthestSide,
}

/// A `<basic-shape>` (CSS Shapes 1 §3.1) in cells and percentages of the
/// reference box.
///
/// Open (DESIGN, `#[non_exhaustive]`): CSS Shapes keeps adding shape
/// functions (`xywh()`, `rect()`, `shape()`); a reader that meets one it
/// does not know clips nothing, as rdom does `path()`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BasicShape {
    /// `inset(<length-percentage>{1,4} [round <length-percentage>{1,4}]?)`:
    /// the insets top, right, bottom, left; the corner radii top-left,
    /// top-right, bottom-right, bottom-left.
    Inset {
        insets: [Length; 4],
        round: [Length; 4],
    },
    /// `circle(<radius>? [at <position>]?)`.
    Circle {
        radius: ShapeRadius,
        at: [Length; 2],
    },
    /// `ellipse([<radius>{2}]? [at <position>]?)`.
    Ellipse {
        rx: ShapeRadius,
        ry: ShapeRadius,
        at: [Length; 2],
    },
    /// `polygon(<fill-rule>?, [<length-percentage>{2}]#)`; `evenodd` the
    /// fill rule, else `nonzero`.
    Polygon {
        evenodd: bool,
        points: Vec<[Length; 2]>,
    },
    /// `path(<fill-rule>?, <string>)`: kept, inert — rdom draws no SVG
    /// path; it clips nothing.
    Path { evenodd: bool, data: Arc<str> },
}

/// A length's exact value against a whole-cell `basis` (`auto` is 0).
fn exact(l: &Length, basis: f64) -> f64 {
    match l {
        Length::Cells(n) => f64::from(*n),
        Length::Calc(e) => e.resolve_f64(&ResolveCtx::new(basis.round() as i32)),
        Length::Auto => 0.0,
    }
}

impl BasicShape {
    /// Whether the point `(x, y)` — a cell's centre — is inside the shape
    /// laid on the reference box `[left, top, width, height]` (all in
    /// cells). A `path()` contains every point.
    pub fn contains(&self, reference: [f64; 4], x: f64, y: f64) -> bool {
        let [left, top, w, h] = reference;
        let along = |l: &Length, basis: f64| percent_of(l, basis);
        match self {
            BasicShape::Inset { insets, round } => {
                let (t, r) = (along(&insets[0], h), along(&insets[1], w));
                let (b, l) = (along(&insets[2], h), along(&insets[3], w));
                let (x0, y0, x1, y1) = (left + l, top + t, left + w - r, top + h - b);
                if x < x0 || x >= x1 || y < y0 || y >= y1 {
                    return false;
                }
                let corners = [
                    (x0, y0, 1.0, 1.0),
                    (x1, y0, -1.0, 1.0),
                    (x1, y1, -1.0, -1.0),
                    (x0, y1, 1.0, -1.0),
                ];
                corners
                    .iter()
                    .zip(round)
                    .all(|(&(cx, cy, sx, sy), radius)| {
                        let (rx, ry) = (along(radius, w), along(radius, h));
                        if rx <= 0.0 || ry <= 0.0 {
                            return true;
                        }
                        // The corner's circle centre, inside the box.
                        let (ox, oy) = (cx + sx * rx, cy + sy * ry);
                        let inside_corner = (x - ox) * sx < 0.0 && (y - oy) * sy < 0.0;
                        !inside_corner || ((x - ox) / rx).powi(2) + ((y - oy) / ry).powi(2) <= 1.0
                    })
            }
            BasicShape::Circle { radius, at } => {
                let (cx, cy) = (left + along(&at[0], w), top + along(&at[1], h));
                let side = |far: bool| {
                    let d = [cx - left, left + w - cx, cy - top, top + h - cy].map(f64::abs);
                    let pick = if far { f64::max } else { f64::min };
                    d.into_iter().reduce(pick).unwrap_or(0.0)
                };
                let r = match radius {
                    ShapeRadius::ClosestSide => side(false),
                    ShapeRadius::FarthestSide => side(true),
                    // §3.1.1: a percentage of the reference box's diagonal
                    // over √2.
                    ShapeRadius::Length(l) => along(l, (w * w + h * h).sqrt() / 2f64.sqrt()),
                };
                (x - cx).powi(2) + (y - cy).powi(2) <= r * r
            }
            BasicShape::Ellipse { rx, ry, at } => {
                let (cx, cy) = (left + along(&at[0], w), top + along(&at[1], h));
                let r = |radius: &ShapeRadius, lo: f64, hi: f64, c: f64, basis: f64| match radius {
                    ShapeRadius::ClosestSide => (c - lo).abs().min((hi - c).abs()),
                    ShapeRadius::FarthestSide => (c - lo).abs().max((hi - c).abs()),
                    ShapeRadius::Length(l) => along(l, basis),
                };
                let (a, b) = (r(rx, left, left + w, cx, w), r(ry, top, top + h, cy, h));
                a > 0.0 && b > 0.0 && ((x - cx) / a).powi(2) + ((y - cy) / b).powi(2) <= 1.0
            }
            BasicShape::Polygon { evenodd, points } => {
                let pts: Vec<(f64, f64)> = points
                    .iter()
                    .map(|[px, py]| (left + along(px, w), top + along(py, h)))
                    .collect();
                polygon_contains(&pts, *evenodd, x, y)
            }
            BasicShape::Path { .. } => true,
        }
    }
}

/// A length against `basis`: a bare percentage exactly, a math function
/// against the basis rounded to a cell.
fn percent_of(l: &Length, basis: f64) -> f64 {
    match l {
        Length::Calc(e) => match **e {
            CalcExpr::Percent(p) => p / 100.0 * basis,
            _ => exact(l, basis),
        },
        other => exact(other, basis),
    }
}

/// The point-in-polygon test (crossings for `evenodd`, the winding number
/// for `nonzero`).
fn polygon_contains(pts: &[(f64, f64)], evenodd: bool, x: f64, y: f64) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let mut winding = 0i32;
    let mut crossings = 0u32;
    for i in 0..n {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % n];
        if (y0 <= y) != (y1 <= y) {
            let t = (y - y0) / (y1 - y0);
            if x < x0 + t * (x1 - x0) {
                crossings += 1;
                winding += if y1 > y0 { 1 } else { -1 };
            }
        }
    }
    if evenodd {
        crossings % 2 == 1
    } else {
        winding != 0
    }
}
