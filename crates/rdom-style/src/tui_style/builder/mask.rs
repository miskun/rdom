//! The mask setters of the `TuiStyle` builder (CSS Masking 1 §6–§7): each
//! takes the value's CSS text, kept as written.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;

/// A setter for one [`MaskDeclarations`](crate::MaskDeclarations) field
/// and its `!important` twin.
macro_rules! mask_setter {
    ($css:literal, $field:ident, $important_setter:ident, $mask:ident) => {
        #[doc = concat!("Set the `", $css, "` property to the CSS text `v` (unchecked: the property draws nothing). Chainable.")]
        pub fn $field(mut self, v: impl Into<String>) -> Self {
            self.masks.$field = Some(Value::Specified(v.into()));
            self
        }

        #[doc = concat!("Like `", stringify!($field), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: impl Into<String>) -> Self {
            self.important |= ImportantMask::$mask;
            self.$field(v)
        }
    };
}

impl TuiStyle {
    mask_setter!("mask-image", mask_image, mask_image_important, MASK_IMAGE);
    mask_setter!("mask-mode", mask_mode, mask_mode_important, MASK_MODE);
    mask_setter!(
        "mask-repeat",
        mask_repeat,
        mask_repeat_important,
        MASK_REPEAT
    );
    mask_setter!(
        "mask-position",
        mask_position,
        mask_position_important,
        MASK_POSITION
    );
    mask_setter!("mask-clip", mask_clip, mask_clip_important, MASK_CLIP);
    mask_setter!(
        "mask-origin",
        mask_origin,
        mask_origin_important,
        MASK_ORIGIN
    );
    mask_setter!("mask-size", mask_size, mask_size_important, MASK_SIZE);
    mask_setter!(
        "mask-composite",
        mask_composite,
        mask_composite_important,
        MASK_COMPOSITE
    );
    mask_setter!("mask-type", mask_type, mask_type_important, MASK_TYPE);
    mask_setter!(
        "mask-border-source",
        mask_border_source,
        mask_border_source_important,
        MASK_BORDER_SOURCE
    );
    mask_setter!(
        "mask-border-slice",
        mask_border_slice,
        mask_border_slice_important,
        MASK_BORDER_SLICE
    );
    mask_setter!(
        "mask-border-width",
        mask_border_width,
        mask_border_width_important,
        MASK_BORDER_WIDTH
    );
    mask_setter!(
        "mask-border-outset",
        mask_border_outset,
        mask_border_outset_important,
        MASK_BORDER_OUTSET
    );
    mask_setter!(
        "mask-border-repeat",
        mask_border_repeat,
        mask_border_repeat_important,
        MASK_BORDER_REPEAT
    );
    mask_setter!(
        "mask-border-mode",
        mask_border_mode,
        mask_border_mode_important,
        MASK_BORDER_MODE
    );
}
