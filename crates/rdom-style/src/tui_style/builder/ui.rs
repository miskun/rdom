//! The CSS UI 4 setters of the `TuiStyle` builder: the outline (§5),
//! `cursor` and `resize` (§4), the caret (§6.2), `accent-color` (§6.3),
//! `appearance` and `field-sizing` (§7).

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{
    AccentColor, Appearance, BorderWidth, CaretAnimation, CaretShape, Cursor, FieldSizing,
    OutlineColor, OutlineStyle, PaintLength, Resize,
};

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
    ui_setter!("cursor", cursor, cursor_important, CURSOR, Cursor);
    ui_setter!(
        "caret-shape",
        caret_shape,
        caret_shape_important,
        CARET_SHAPE,
        CaretShape
    );
    ui_setter!(
        "caret-animation",
        caret_animation,
        caret_animation_important,
        CARET_ANIMATION,
        CaretAnimation
    );
    ui_setter!(
        "accent-color",
        accent_color,
        accent_color_important,
        ACCENT_COLOR,
        AccentColor
    );
    ui_setter!(
        "appearance",
        appearance,
        appearance_important,
        APPEARANCE,
        Appearance
    );
    ui_setter!(
        "field-sizing",
        field_sizing,
        field_sizing_important,
        FIELD_SIZING,
        FieldSizing
    );
    ui_setter!("resize", resize, resize_important, RESIZE, Resize);
}
