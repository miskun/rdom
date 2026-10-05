//! The CSS Text setters of the `TuiStyle` builder (CSS Text 3 / 4):
//! `white-space` and its longhands, `word-break`, `overflow-wrap`,
//! `line-break`, `hyphens`, `tab-size`, `text-transform`.

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
    text_setter!(
        "word-break",
        word_break,
        word_break_important,
        WORD_BREAK,
        crate::layout::WordBreak
    );
    text_setter!(
        "overflow-wrap",
        overflow_wrap,
        overflow_wrap_important,
        OVERFLOW_WRAP,
        crate::layout::OverflowWrap
    );
    text_setter!(
        "line-break",
        line_break,
        line_break_important,
        LINE_BREAK,
        crate::layout::LineBreak
    );
    text_setter!(
        "text-transform",
        text_transform,
        text_transform_important,
        TEXT_TRANSFORM,
        crate::layout::TextTransform
    );
    text_setter!(
        "tab-size",
        tab_size,
        tab_size_important,
        TAB_SIZE,
        crate::layout::TabSize
    );
    text_setter!(
        "hyphens",
        hyphens,
        hyphens_important,
        HYPHENS,
        crate::layout::Hyphens
    );
}
