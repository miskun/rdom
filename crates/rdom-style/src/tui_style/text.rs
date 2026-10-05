//! [`TextDeclarations`]: a style block's declarations of the CSS Text
//! properties, the specified side of [`TextStyle`](crate::layout::TextStyle).

use crate::Value;
use crate::layout::{
    Hyphens, LineBreak, OverflowWrap, TabSize, TextWrapMode, WhiteSpaceCollapse, WordBreak,
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
}
