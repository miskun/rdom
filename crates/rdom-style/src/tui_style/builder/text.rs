//! The CSS Text setters of the `TuiStyle` builder (CSS Text 3 / 4):
//! `white-space` and its longhands.

use super::super::{ImportantMask, TuiStyle};
use crate::Value;
use crate::layout::WhiteSpace;

/// A setter for one [`TextDeclarations`](crate::TextDeclarations) field
/// and its `!important` twin, as `setter!` is for a `TuiStyle` field.
macro_rules! text_setter {
    ($css:literal, $field:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $field(mut self, v: $ty) -> Self {
            self.text.$field = Some(Value::Specified(v));
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
    /// Set the `white-space` shorthand to `v`: its two longhands,
    /// `white-space-collapse` and `text-wrap-mode` (CSS Text 4 §3).
    /// Chainable.
    pub fn white_space(self, v: WhiteSpace) -> Self {
        let (collapse, mode) = v.longhands();
        self.white_space_collapse(collapse).text_wrap_mode(mode)
    }

    /// Like `white_space` but also marks the declaration `!important`.
    pub fn white_space_important(self, v: WhiteSpace) -> Self {
        let (collapse, mode) = v.longhands();
        self.white_space_collapse_important(collapse)
            .text_wrap_mode_important(mode)
    }

    text_setter!(
        "white-space-collapse",
        white_space_collapse,
        white_space_collapse_important,
        WHITE_SPACE_COLLAPSE,
        crate::layout::WhiteSpaceCollapse
    );
    text_setter!(
        "text-wrap-mode",
        text_wrap_mode,
        text_wrap_mode_important,
        TEXT_WRAP_MODE,
        crate::layout::TextWrapMode
    );
}
