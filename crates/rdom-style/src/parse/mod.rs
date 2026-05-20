//! Token-level CSS parsing primitives.
//!
//! Used internally by [`crate::property_dispatch`] and re-exported
//! for `rdom-css`'s block parser (which still owns top-level
//! stylesheet + declaration-list parsing on top of these tokens).

pub mod cursor;
pub mod token;
pub mod values;

pub use cursor::Cursor;
pub use token::{Token, TokenizerError, TokenizerErrorKind, tokenize};
