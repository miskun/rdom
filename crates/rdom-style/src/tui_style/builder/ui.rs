//! The CSS UI 4 setters of the `TuiStyle` builder: the outline (§5).

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{BorderWidth, OutlineColor, OutlineStyle, PaintLength};

/// A setter for one [`UiDeclarations`](crate::UiDeclarations) field and
/// its `!important` twin, as `setter!` is for a `TuiStyle` field.
macro_rules! ui_setter {
    ($css:literal, $field:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $field(mut self, v: $ty) -> Self {
            self.ui.$field = Some(Value::Specified(v));
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
    ui_setter!(
        "outline-style",
        outline_style,
        outline_style_important,
        OUTLINE_STYLE,
        OutlineStyle
    );
    ui_setter!(
        "outline-width",
        outline_width,
        outline_width_important,
        OUTLINE_WIDTH,
        BorderWidth
    );
    ui_setter!(
        "outline-color",
        outline_color,
        outline_color_important,
        OUTLINE_COLOR,
        OutlineColor
    );
    ui_setter!(
        "outline-offset",
        outline_offset,
        outline_offset_important,
        OUTLINE_OFFSET,
        PaintLength
    );
}
