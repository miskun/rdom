//! [`EffectsDeclarations`]: a style block's declarations of the transform,
//! filter and compositing properties, the specified side of
//! [`EffectsStyle`](crate::layout::EffectsStyle).

use crate::Value;
use crate::layout::{Rotate, Scale, TransformBox, TransformList, TransformOrigin, Translate};

/// The transform, filter and compositing properties a
/// [`TuiStyle`](crate::TuiStyle) declares
/// ([`TuiStyle::effects`](crate::TuiStyle::effects)), one field per
/// longhand, `None` where the block does not declare it.
///
/// Closed (DESIGN), as the other declaration groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectsDeclarations {
    /// `translate` (CSS Transforms 2 §6.1); `Specified(None)` is `none`.
    pub translate: Option<Value<Option<Translate>>>,
    /// `rotate` (§6.2); `Specified(None)` is `none`.
    pub rotate: Option<Value<Option<Rotate>>>,
    /// `scale` (§6.3); `Specified(None)` is `none`.
    pub scale: Option<Value<Option<Scale>>>,
    /// `transform` (CSS Transforms 1 §5).
    pub transform: Option<Value<TransformList>>,
    /// `transform-origin` (§6).
    pub transform_origin: Option<Value<TransformOrigin>>,
    /// `transform-box` (§7).
    pub transform_box: Option<Value<TransformBox>>,
}
