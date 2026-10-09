//! [`EffectsStyle`]: the computed group of the transform, filter and
//! compositing properties.

use super::{
    BlendMode, ClipPath, FilterList, Isolation, Rotate, Scale, TransformBox, TransformList,
    TransformOrigin, Translate,
};
use crate::Color;

/// The computed transform, filter and compositing properties of an
/// element ([`ComputedStyle::effects`](crate::ComputedStyle::effects)):
/// none inherit.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, PartialEq)]
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
    /// `filter` (Filter Effects 1 §5), its drop shadows' colors computed.
    pub filter: FilterList<Color>,
    /// `backdrop-filter` (Filter Effects 2 §3), likewise.
    pub backdrop_filter: FilterList<Color>,
    /// `mix-blend-mode` (Compositing 1 §3.2).
    pub mix_blend_mode: BlendMode,
    /// `isolation` (§5.2).
    pub isolation: Isolation,
    /// `background-blend-mode` (§3.4), one per background layer. Inert:
    /// a cell's background is one color.
    pub background_blend_mode: std::borrow::Cow<'static, [BlendMode]>,
    /// `clip-path` (CSS Masking 1 §5.1).
    pub clip_path: ClipPath,
}

impl Default for EffectsStyle {
    /// The initial values; nothing allocated.
    fn default() -> Self {
        EffectsStyle {
            translate: None,
            rotate: None,
            scale: None,
            transform: TransformList::default(),
            transform_origin: TransformOrigin::default(),
            transform_box: TransformBox::default(),
            filter: FilterList::default(),
            backdrop_filter: FilterList::default(),
            mix_blend_mode: BlendMode::Normal,
            isolation: Isolation::Auto,
            background_blend_mode: std::borrow::Cow::Borrowed(&[BlendMode::Normal]),
            clip_path: ClipPath::None,
        }
    }
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

    /// Whether `self` and `other` differ in what layout reads of them: of
    /// the transforms (CSS Transforms 1 §2, §7), whether the box is
    /// transformed, its translation and the reference box of their
    /// percentages — not the inert rotations, scales and skews; of the
    /// filters (Filter Effects 1 §5, 2 §3), whether there is one — a
    /// containing block — not what it does to colors.
    pub fn layout_differs(&self, other: &EffectsStyle) -> bool {
        self.is_transformed() != other.is_transformed()
            || self.translate != other.translate
            || !self.transform.same_translations(&other.transform)
            || self.transform_box != other.transform_box
            || self.filter.is_none() != other.filter.is_none()
            || self.backdrop_filter.is_none() != other.backdrop_filter.is_none()
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
