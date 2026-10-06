//! [`TextDeclarations`]: a style block's declarations of the CSS Text
//! properties, the specified side of [`TextStyle`](crate::layout::TextStyle).

use crate::Value;
use crate::layout::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextAlign, TextAlignLast, TextIndent, TextJustify,
    TextTransform, TextWrapMode, TextWrapStyle, WhiteSpaceCollapse, WordBreak,
};

/// The CSS Text properties a [`TuiStyle`](crate::TuiStyle) declares
/// ([`TuiStyle::text`](crate::TuiStyle::text)), one field per longhand,
/// `None` where the block does not declare it. The `white-space`
/// shorthand writes `white_space_collapse` and `text_wrap_mode`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextDeclarations {
    /// `white-space-collapse` (CSS Text 4 §4.1).
    pub white_space_collapse: Option<Value<WhiteSpaceCollapse>>,
    /// `text-wrap-mode` (CSS Text 4 §6.1).
    pub text_wrap_mode: Option<Value<TextWrapMode>>,
    /// `word-break` (CSS Text 3 §5.2).
    pub word_break: Option<Value<WordBreak>>,
    /// `overflow-wrap` and its legacy name `word-wrap` (CSS Text 3 §5.5).
    pub overflow_wrap: Option<Value<OverflowWrap>>,
    /// `line-break` (CSS Text 3 §5.3).
    pub line_break: Option<Value<LineBreak>>,
    /// `hyphens` (CSS Text 3 §6.1).
    pub hyphens: Option<Value<Hyphens>>,
    /// `tab-size` (CSS Text 3 §4.2).
    pub tab_size: Option<Value<TabSize>>,
    /// `text-transform` (CSS Text 3 §2.1).
    pub text_transform: Option<Value<TextTransform>>,
    /// `text-indent` (CSS Text 3 §8.1).
    pub text_indent: Option<Value<TextIndent>>,
    /// `text-align-all` (CSS Text 3 §6.2); the `text-align` shorthand
    /// writes it and `text_align_last`.
    pub text_align_all: Option<Value<TextAlign>>,
    /// `text-align-last` (CSS Text 3 §6.3).
    pub text_align_last: Option<Value<TextAlignLast>>,
    /// `text-justify` (CSS Text 3 §6.4).
    pub text_justify: Option<Value<TextJustify>>,
    /// `text-wrap-style` (CSS Text 4); the `text-wrap` shorthand writes it
    /// and `text_wrap_mode`.
    pub text_wrap_style: Option<Value<TextWrapStyle>>,
}
