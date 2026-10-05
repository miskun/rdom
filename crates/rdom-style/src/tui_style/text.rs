//! [`TextDeclarations`]: a style block's declarations of the CSS Text
//! properties, the specified side of [`TextStyle`](crate::layout::TextStyle).

use crate::Value;
use crate::layout::{TextWrapMode, WhiteSpaceCollapse};

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
}
