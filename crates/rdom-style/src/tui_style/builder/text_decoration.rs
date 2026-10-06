//! The text decoration setters of the `TuiStyle` builder (CSS Text
//! Decoration 3 / 4): the `text-decoration` shorthand and its longhands,
//! and the inherited underline placement properties.

use super::super::{ImportantMask, TuiStyle};
use crate::layout::{
    TextDecoration, TextDecorationLine, TextDecorationSkipInk, TextDecorationStyle,
    TextDecorationThickness, TextUnderlineOffset, TextUnderlinePosition,
};
use crate::{TuiColor, Value};

/// A setter for one field of a declaration group (`$group`) and its
/// `!important` twin.
macro_rules! group_setter {
    ($css:literal, $group:ident . $field:ident, $setter:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $setter(mut self, v: $ty) -> Self {
            self.$group.$field = Some(Value::Specified(v));
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: $ty) -> Self {
            self.important |= ImportantMask::$mask;
            self.$setter(v)
        }
    };
}

impl TuiStyle {
    /// Set the `text-decoration` shorthand to one line, `d` (CSS Text
    /// Decoration 4 §2.6): its line, with `text-decoration-style`,
    /// `-color` and `-thickness` at their initial values. Chainable.
    pub fn text_decoration(self, d: TextDecoration) -> Self {
        self.text_decoration_line(TextDecorationLine::from(d))
            .text_decoration_style(TextDecorationStyle::Solid)
            .text_decoration_color(TuiColor::CurrentColor)
            .text_decoration_thickness(TextDecorationThickness::Auto)
    }

    /// Like `text_decoration` but also marks the declaration `!important`.
    pub fn text_decoration_important(mut self, d: TextDecoration) -> Self {
        self.important |= ImportantMask::TEXT_DECORATION;
        self.text_decoration(d)
    }

    group_setter!(
        "text-decoration-line",
        text_decoration.line,
        text_decoration_line,
        text_decoration_line_important,
        TEXT_DECORATION_LINE,
        TextDecorationLine
    );
    group_setter!(
        "text-decoration-style",
        text_decoration.style,
        text_decoration_style,
        text_decoration_style_important,
        TEXT_DECORATION_STYLE,
        TextDecorationStyle
    );
    group_setter!(
        "text-decoration-color",
        text_decoration.color,
        text_decoration_color,
        text_decoration_color_important,
        TEXT_DECORATION_COLOR,
        TuiColor
    );
    group_setter!(
        "text-decoration-thickness",
        text_decoration.thickness,
        text_decoration_thickness,
        text_decoration_thickness_important,
        TEXT_DECORATION_THICKNESS,
        TextDecorationThickness
    );
    group_setter!(
        "text-underline-offset",
        text.text_underline_offset,
        text_underline_offset,
        text_underline_offset_important,
        TEXT_UNDERLINE_OFFSET,
        TextUnderlineOffset
    );
    group_setter!(
        "text-underline-position",
        text.text_underline_position,
        text_underline_position,
        text_underline_position_important,
        TEXT_UNDERLINE_POSITION,
        TextUnderlinePosition
    );
    group_setter!(
        "text-decoration-skip-ink",
        text.text_decoration_skip_ink,
        text_decoration_skip_ink,
        text_decoration_skip_ink_important,
        TEXT_DECORATION_SKIP_INK,
        TextDecorationSkipInk
    );
}
