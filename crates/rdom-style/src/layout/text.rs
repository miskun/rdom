//! The computed group of the CSS Text properties, [`TextStyle`]
//! (CSS Text 3 / 4, with `line-height` and the inherited text decoration
//! properties). The values are `white_space` (white space processing,
//! wrapping, line breaking) and `text_align` (transform, indent,
//! alignment).

use super::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextAlign, TextAlignLast, TextIndent, TextJustify,
    TextTransform, TextWrapMode, TextWrapStyle, WhiteSpace, WhiteSpaceCollapse, WordBreak,
};

/// The computed CSS Text properties of an element
/// ([`ComputedStyle::text`](crate::ComputedStyle::text)), with
/// `line-height` and the inherited text decoration properties. All of
/// them inherit, so the cascade copies the group from the parent whole.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    /// `white-space-collapse` (CSS Text 4 §4.1).
    pub white_space_collapse: WhiteSpaceCollapse,
    /// `text-wrap-mode` (CSS Text 4 §6.1).
    pub text_wrap_mode: TextWrapMode,
    /// `word-break` (CSS Text 3 §5.2).
    pub word_break: WordBreak,
    /// `overflow-wrap` / `word-wrap` (CSS Text 3 §5.5).
    pub overflow_wrap: OverflowWrap,
    /// `line-break` (CSS Text 3 §5.3).
    pub line_break: LineBreak,
    /// `hyphens` (CSS Text 3 §6.1).
    pub hyphens: Hyphens,
    /// `tab-size` (CSS Text 3 §4.2).
    pub tab_size: TabSize,
    /// `text-transform` (CSS Text 3 §2.1).
    pub text_transform: TextTransform,
    /// `text-indent` (CSS Text 3 §8.1).
    pub text_indent: TextIndent,
    /// `text-align-all` (CSS Text 3 §6.2), the `text-align` shorthand's
    /// first longhand.
    pub text_align_all: TextAlign,
    /// `text-align-last` (CSS Text 3 §6.3).
    pub text_align_last: TextAlignLast,
    /// `text-justify` (CSS Text 3 §6.4).
    pub text_justify: TextJustify,
    /// `text-wrap-style` (CSS Text 4), the `text-wrap` shorthand's second
    /// longhand.
    pub text_wrap_style: TextWrapStyle,
    /// `line-height` (CSS Inline 3 §5.1), computed: a percentage or a
    /// context length resolved to rows. Inherited, as the CSS Text
    /// properties are.
    pub line_height: super::LineHeight,
    /// `text-underline-offset` (CSS Text Decoration 4 §4.2): parsed, not
    /// drawn.
    pub text_underline_offset: super::TextUnderlineOffset,
    /// `text-underline-position` (§4.1): parsed, not drawn.
    pub text_underline_position: super::TextUnderlinePosition,
    /// `text-decoration-skip-ink` (§3.2): parsed, not drawn.
    pub text_decoration_skip_ink: super::TextDecorationSkipInk,
}

impl TextStyle {
    /// The `white-space` keyword the longhands spell, if one does.
    pub fn white_space(&self) -> Option<WhiteSpace> {
        WhiteSpace::from_longhands(self.white_space_collapse, self.text_wrap_mode)
    }

    /// Whether lines may break at soft wrap opportunities.
    pub fn wraps(&self) -> bool {
        self.text_wrap_mode == TextWrapMode::Wrap
    }
}
