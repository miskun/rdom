//! The font setters of the `TuiStyle` builder (CSS Fonts 4): the font
//! longhands, and `bold` / `italic`, their two-state forms.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::{FontFamily, FontSize, FontStretch, FontStyle, FontVariant, FontWeight};

/// A setter for one [`FontDeclarations`](crate::FontDeclarations) field
/// and its `!important` twin.
macro_rules! font_setter {
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $setter(mut self, v: $ty) -> Self {
            self.font.$field = Some(Value::Specified(v));
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
    /// Set `font-weight` to `bold` (`true`) or `normal`. Chainable.
    pub fn bold(self, on: bool) -> Self {
        self.font_weight(if on {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        })
    }

    /// Like `bold` but also marks the declaration `!important`.
    pub fn bold_important(self, on: bool) -> Self {
        self.font_weight_important(if on {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        })
    }

    /// Set `font-style` to `italic` (`true`) or `normal`. Chainable.
    pub fn italic(self, on: bool) -> Self {
        self.font_style(if on {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        })
    }

    /// Like `italic` but also marks the declaration `!important`.
    pub fn italic_important(self, on: bool) -> Self {
        self.font_style_important(if on {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        })
    }

    font_setter!(
        "font-weight",
        weight,
        font_weight,
        font_weight_important,
        FONT_WEIGHT,
        FontWeight
    );
    font_setter!(
        "font-style",
        style,
        font_style,
        font_style_important,
        FONT_STYLE,
        FontStyle
    );
    font_setter!(
        "font-size",
        size,
        font_size,
        font_size_important,
        FONT_SIZE,
        FontSize
    );
    font_setter!(
        "font-family",
        family,
        font_family,
        font_family_important,
        FONT_FAMILY,
        FontFamily
    );
    font_setter!(
        "font-stretch",
        stretch,
        font_stretch,
        font_stretch_important,
        FONT_STRETCH,
        FontStretch
    );
    font_setter!(
        "font-variant",
        variant,
        font_variant,
        font_variant_important,
        FONT_VARIANT,
        FontVariant
    );
}
