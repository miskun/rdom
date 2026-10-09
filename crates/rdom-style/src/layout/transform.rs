//! The CSS Transforms values: `translate` (CSS Transforms 2 §6.1), the
//! `transform` list (Transforms 1 §5, Transforms 2 §7), `rotate` and
//! `scale` (Transforms 2 §6.2–§6.3), `transform-origin` (Transforms 1 §6)
//! and `transform-box` (§7).
//!
//! A cell grid can move a box by whole cells and do nothing else a
//! transform does: translations take effect (layout moves the box after
//! laying it out — `rdom-tui`'s `style::effects`), every other function
//! and the `rotate` / `scale` properties are kept, serialized and
//! animated, and draw nothing (DIVERGENCES §1). A value other than `none`
//! still makes the box a stacking context and a containing block.

use std::sync::Arc;

use super::{Length, PaintLength};
use crate::calc::ResolveCtx;

/// `translate` (CSS Transforms 2 §6.1): `<length-percentage>` offsets on x
/// and y, percentages of the reference box ([`TransformBox`]), and a z
/// offset that moves nothing in a grid. The property's `none` is
/// [`EffectsStyle::translate`](super::EffectsStyle::translate) `None`.
///
/// Closed (DESIGN): a value record every reader resolves whole.
#[derive(Debug, Clone, PartialEq)]
pub struct Translate {
    /// The x offset: cells or a math function (a percentage of the
    /// reference box's width).
    pub x: Length,
    /// The y offset: cells or a math function (of its height).
    pub y: Length,
    /// The z offset: kept and inert — a grid has no depth.
    pub z: PaintLength,
}

impl Default for Translate {
    fn default() -> Self {
        Translate::new(Length::Cells(0), Length::Cells(0))
    }
}

impl Translate {
    /// The offset `(x, y)` with no z offset.
    pub fn new(x: Length, y: Length) -> Self {
        Translate {
            x,
            y,
            z: PaintLength::Cells(0.0),
        }
    }

    /// The exact offset in (fractional) cells against a reference box of
    /// `width` × `height` — percentages resolved, nothing rounded.
    pub fn offset(&self, width: i32, height: i32) -> (f64, f64) {
        (exact(&self.x, width), exact(&self.y, height))
    }
}

/// A length's exact value against `basis`: cells as they are, a math
/// function unrounded (`auto`, never a translation, is zero).
fn exact(l: &Length, basis: i32) -> f64 {
    match l {
        Length::Cells(n) => f64::from(*n),
        Length::Calc(e) => e.resolve_f64(&ResolveCtx::new(basis)),
        Length::Auto => 0.0,
    }
}

/// Which translate function a [`TransformFunction::Translate`] was written
/// as (CSS Transforms 1 §12, Transforms 2 §13): the offsets mean the same,
/// the spelling serializes back.
///
/// Closed (DESIGN): the five functions are fixed by the two specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslateFunction {
    /// `translate(<lp> [, <lp>]?)`.
    Translate,
    /// `translateX(<lp>)`.
    TranslateX,
    /// `translateY(<lp>)`.
    TranslateY,
    /// `translate3d(<lp>, <lp>, <length>)`.
    Translate3d,
    /// `translateZ(<length>)`.
    TranslateZ,
}

impl TranslateFunction {
    /// The function's name, as CSS spells it.
    pub fn name(self) -> &'static str {
        match self {
            TranslateFunction::Translate => "translate",
            TranslateFunction::TranslateX => "translateX",
            TranslateFunction::TranslateY => "translateY",
            TranslateFunction::Translate3d => "translate3d",
            TranslateFunction::TranslateZ => "translateZ",
        }
    }
}

/// One `<transform-function>` (CSS Transforms 1 §12, Transforms 2 §13).
///
/// Open (DESIGN, `#[non_exhaustive]`): CSS keeps adding transform
/// functions, and a reader that meets one it does not know treats it as
/// the identity — as rdom treats every function but the translations.
/// rdom-tui reads a list only through [`TransformList::offset`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TransformFunction {
    /// A translation, as written ([`TranslateFunction`]): moves the box.
    Translate {
        function: TranslateFunction,
        offset: Translate,
    },
    /// Any other function — `rotate()`, `scale()`, `skew()`, `matrix()`,
    /// `perspective()` and their 3D forms — checked against its grammar
    /// and kept as its CSS text (`rotate(45deg)`). Inert: a cell grid
    /// cannot rotate, scale or skew a glyph.
    Inert {
        /// The function's name, lowercased (`rotatey`), what a list
        /// interpolation pairs functions by.
        name: Arc<str>,
        /// The whole function as CSS text.
        css: Arc<str>,
    },
}

/// A `transform` value (CSS Transforms 1 §5): `none`, or a list of
/// transform functions applied left to right.
///
/// Closed (DESIGN): a value record read through its accessors; the
/// functions behind an `Arc`, so a style clones it without allocating.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransformList(Option<Arc<[TransformFunction]>>);

impl TransformList {
    /// `none`.
    pub fn none() -> Self {
        TransformList(None)
    }

    /// The list of `functions`; `none` when there are none.
    pub fn new(functions: Vec<TransformFunction>) -> Self {
        if functions.is_empty() {
            TransformList(None)
        } else {
            TransformList(Some(functions.into()))
        }
    }

    /// Whether it is `none`.
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// The functions, in order (empty for `none`).
    pub fn functions(&self) -> &[TransformFunction] {
        self.0.as_deref().unwrap_or(&[])
    }

    /// Whether the two lists translate alike: the same translate functions
    /// in the same order (their inert functions aside).
    pub fn same_translations(&self, other: &TransformList) -> bool {
        fn translations(l: &TransformList) -> impl Iterator<Item = &Translate> {
            l.functions().iter().filter_map(|f| match f {
                TransformFunction::Translate { offset, .. } => Some(offset),
                TransformFunction::Inert { .. } => None,
            })
        }
        translations(self).eq(translations(other))
    }

    /// The exact translation the list applies against a reference box of
    /// `width` × `height`: its translate functions summed, every other
    /// function the identity (a grid rotates, scales and skews nothing,
    /// so a translation after one moves by its own offset).
    pub fn offset(&self, width: i32, height: i32) -> (f64, f64) {
        self.functions()
            .iter()
            .fold((0.0, 0.0), |(x, y), f| match f {
                TransformFunction::Translate { offset, .. } => {
                    let (dx, dy) = offset.offset(width, height);
                    (x + dx, y + dy)
                }
                TransformFunction::Inert { .. } => (x, y),
            })
    }
}

/// `rotate` (CSS Transforms 2 §6.2) other than `none`: an angle about an
/// axis. Kept, serialized and animated; a cell grid rotates nothing.
///
/// Closed (DESIGN): a value record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotate {
    /// The axis: `None` for the `<angle>` alone (about z), else the
    /// vector `x` / `y` / `z` or three numbers name.
    pub axis: Option<[f64; 3]>,
    /// The angle in degrees.
    pub degrees: f64,
}

/// `scale` (CSS Transforms 2 §6.3) other than `none`: the factors on x, y
/// and z, a percentage computed to a number. Kept, serialized and
/// animated; a cell grid scales nothing.
///
/// Closed (DESIGN): a value record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// `transform-origin` (CSS Transforms 1 §6): the point the non-translate
/// functions turn and scale about — a keyword as its percentage (`left`
/// 0%, `center` 50%, `bottom` 100%), a length in cells or pixels. Inert,
/// as those functions are (a translation has no origin).
///
/// Closed (DESIGN): a value record, read through [`x`](Self::x),
/// [`y`](Self::y) and [`z`](Self::z). The initial value holds nothing, so
/// every element's initial style builds it without allocating.
#[derive(Debug, Clone, Default)]
pub struct TransformOrigin(Option<Arc<[PaintLength; 3]>>);

impl PartialEq for TransformOrigin {
    /// The same position, however built (`50% 50%` is the initial value).
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) if Arc::ptr_eq(a, b) => true,
            _ => (0..3).all(|i| self.get(i) == other.get(i)),
        }
    }
}

impl TransformOrigin {
    /// The origin at `x`, `y` and `z`.
    pub fn new(x: PaintLength, y: PaintLength, z: PaintLength) -> Self {
        TransformOrigin(Some(Arc::new([x, y, z])))
    }

    fn get(&self, i: usize) -> PaintLength {
        match &self.0 {
            Some(p) => p[i].clone(),
            None if i == 2 => PaintLength::Cells(0.0),
            None => PaintLength::calc(crate::calc::CalcExpr::Percent(50.0)),
        }
    }

    /// Whether a length needs the viewport or a line height (`vw`, `lh`):
    /// the cascade makes it absolute. Never the initial value.
    pub(crate) fn needs_context(&self) -> bool {
        self.0.as_ref().is_some_and(|p| {
            p.iter()
                .any(|l| matches!(l, PaintLength::Calc(e) if e.needs_context()))
        })
    }

    /// The horizontal position (initial `50%`).
    pub fn x(&self) -> PaintLength {
        self.get(0)
    }

    /// The vertical position (initial `50%`).
    pub fn y(&self) -> PaintLength {
        self.get(1)
    }

    /// The z offset (initial `0`).
    pub fn z(&self) -> PaintLength {
        self.get(2)
    }
}

/// `transform-box` (CSS Transforms 1 §7): the reference box a transform's
/// percentages resolve against.
///
/// Closed (DESIGN): the five keywords are the property's grammar; each is
/// a box layout must pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TransformBox {
    ContentBox,
    BorderBox,
    FillBox,
    StrokeBox,
    /// The initial value.
    #[default]
    ViewBox,
}

impl TransformBox {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, TransformBox)] = &[
        ("content-box", TransformBox::ContentBox),
        ("border-box", TransformBox::BorderBox),
        ("fill-box", TransformBox::FillBox),
        ("stroke-box", TransformBox::StrokeBox),
        ("view-box", TransformBox::ViewBox),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, b)| *b == self)
            .map_or("view-box", |(k, _)| k)
    }

    /// Whether the reference box of a CSS box is its content box (§7:
    /// `fill-box` is used as `content-box`, `stroke-box` and `view-box` as
    /// `border-box`).
    pub fn is_content_box(self) -> bool {
        matches!(self, TransformBox::ContentBox | TransformBox::FillBox)
    }
}

/// The computed transform, filter and compositing properties of an
/// element ([`ComputedStyle::effects`](crate::ComputedStyle::effects)):
/// none inherit.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EffectsStyle {
    /// `translate` (CSS Transforms 2 §6.1); `None` is `none`.
    pub translate: Option<Translate>,
    /// `rotate` (§6.2); `None` is `none`. Inert.
    pub rotate: Option<Rotate>,
    /// `scale` (§6.3); `None` is `none`. Inert.
    pub scale: Option<Scale>,
    /// `transform` (CSS Transforms 1 §5).
    pub transform: TransformList,
    /// `transform-origin` (§6). Inert.
    pub transform_origin: TransformOrigin,
    /// `transform-box` (§7).
    pub transform_box: TransformBox,
}

impl EffectsStyle {
    /// Whether the element is transformed: `transform`, `translate`,
    /// `rotate` or `scale` is not `none` (CSS Transforms 1 §2, Transforms 2
    /// §6) — even an inert or a zero one — so a transformable box
    /// establishes a stacking context and a containing block.
    pub fn is_transformed(&self) -> bool {
        self.translate.is_some()
            || self.rotate.is_some()
            || self.scale.is_some()
            || !self.transform.is_none()
    }

    /// Whether `self` and `other` differ in what layout reads of the
    /// transforms (CSS Transforms 1 §2, §7): whether the box is
    /// transformed (a stacking context and a containing block), its
    /// translation, and the reference box of their percentages — not the
    /// inert rotations, scales and skews.
    pub fn transform_moves(&self, other: &EffectsStyle) -> bool {
        self.is_transformed() != other.is_transformed()
            || self.translate != other.translate
            || !self.transform.same_translations(&other.transform)
            || self.transform_box != other.transform_box
    }

    /// The exact translation the element's `translate` and `transform`
    /// apply against a reference box of `width` × `height` (Transforms 2
    /// §6: `translate`, then `rotate` and `scale` — the identity here —
    /// then the `transform` list).
    pub fn translation(&self, width: i32, height: i32) -> (f64, f64) {
        let (x, y) = self
            .translate
            .as_ref()
            .map_or((0.0, 0.0), |t| t.offset(width, height));
        let (dx, dy) = self.transform.offset(width, height);
        (x + dx, y + dy)
    }
}
