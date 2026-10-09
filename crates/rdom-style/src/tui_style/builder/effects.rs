//! The transform, filter and compositing setters of the `TuiStyle`
//! builder.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{
    BlendMode, FilterList, Isolation, Rotate, Scale, TransformBox, TransformList, TransformOrigin,
    Translate,
};

/// A setter for one [`EffectsDeclarations`](crate::EffectsDeclarations)
/// field and its `!important` twin, as `setter!` is for a `TuiStyle` field.
macro_rules! effects_setter {
    ($css:literal, $field:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $field(mut self, v: $ty) -> Self {
            self.effects.$field = Some(Value::Specified(v));
            self
        }

        #[doc = concat!("Like `", stringify!($field), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: $ty) -> Self {
            self.important |= ImportantMask::$mask;
            self.$field(v)
        }
    };
}

impl TuiStyle {
    effects_setter!(
        "translate",
        translate,
        translate_important,
        TRANSLATE,
        Option<Translate>
    );
    effects_setter!("rotate", rotate, rotate_important, ROTATE, Option<Rotate>);
    effects_setter!("scale", scale, scale_important, SCALE, Option<Scale>);
    effects_setter!(
        "transform",
        transform,
        transform_important,
        TRANSFORM,
        TransformList
    );
    effects_setter!(
        "transform-origin",
        transform_origin,
        transform_origin_important,
        TRANSFORM_ORIGIN,
        TransformOrigin
    );
    effects_setter!(
        "transform-box",
        transform_box,
        transform_box_important,
        TRANSFORM_BOX,
        TransformBox
    );
    effects_setter!("filter", filter, filter_important, FILTER, FilterList);
    effects_setter!(
        "backdrop-filter",
        backdrop_filter,
        backdrop_filter_important,
        BACKDROP_FILTER,
        FilterList
    );
    effects_setter!(
        "mix-blend-mode",
        mix_blend_mode,
        mix_blend_mode_important,
        MIX_BLEND_MODE,
        BlendMode
    );
    effects_setter!(
        "isolation",
        isolation,
        isolation_important,
        ISOLATION,
        Isolation
    );
    effects_setter!(
        "background-blend-mode",
        background_blend_mode,
        background_blend_mode_important,
        BACKGROUND_BLEND_MODE,
        std::borrow::Cow<'static, [BlendMode]>
    );
}
