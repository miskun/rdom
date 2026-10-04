//! A custom property's value (CSS Variables 1 §2): a token sequence,
//! kept with its text.
//!
//! The value is tokenized once — when it is declared, or when the
//! cascade substitutes its `var()`s — and shared from then on: every
//! element that inherits it holds the same `Arc`s, and every `var()`
//! that reads it copies its tokens without re-tokenizing the text
//! (`C1G-VAR-COST`). The text is what CSSOM reads back
//! (`getPropertyValue`, `ComputedStyle::vars`) and what a registered
//! property's syntax is matched against.

use std::fmt;
use std::sync::Arc;

use crate::parse::token::{Token, tokenize};
use crate::parse::values::render_value;

/// One custom property value: its text and its tokens. Cheap to clone
/// (two reference counts; atomic, so a `TuiStyle` holding one stays
/// `Send + Sync`). Equality and hashing are by text — the
/// tokens are a function of it.
#[derive(Clone)]
pub struct CustomValue {
    text: Arc<str>,
    /// `None` when the text does not tokenize; such a value substitutes
    /// as if the property were undefined.
    tokens: Option<Arc<[Token]>>,
}

impl CustomValue {
    /// The value written as `text`, tokenized now.
    pub fn new(text: &str) -> Self {
        #[cfg(test)]
        probe::TOKENIZED.with(|c| c.set(c.get() + 1));
        CustomValue {
            text: Arc::from(text),
            tokens: tokenize(text).ok().map(Arc::from),
        }
    }

    /// The value made of `tokens` (a substitution result); its text is
    /// their serialization.
    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        CustomValue {
            text: Arc::from(render_value(&tokens)),
            tokens: Some(Arc::from(tokens)),
        }
    }

    /// The value's text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The value's tokens; `None` when its text does not tokenize.
    pub fn tokens(&self) -> Option<&[Token]> {
        self.tokens.as_deref()
    }

    /// Does the value hold a `var()` reference?
    pub fn has_var(&self) -> bool {
        self.tokens().is_some_and(crate::var::contains_var)
    }
}

impl std::ops::Deref for CustomValue {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl AsRef<str> for CustomValue {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl PartialEq for CustomValue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.text, &other.text) || self.text == other.text
    }
}

impl Eq for CustomValue {}

impl PartialEq<str> for CustomValue {
    fn eq(&self, other: &str) -> bool {
        &*self.text == other
    }
}

impl PartialEq<&str> for CustomValue {
    fn eq(&self, other: &&str) -> bool {
        &*self.text == *other
    }
}

impl std::hash::Hash for CustomValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl fmt::Debug for CustomValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.text, f)
    }
}

impl fmt::Display for CustomValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<&str> for CustomValue {
    fn from(text: &str) -> Self {
        CustomValue::new(text)
    }
}

impl From<String> for CustomValue {
    fn from(text: String) -> Self {
        CustomValue::new(&text)
    }
}

/// Test-only: how many custom-property values were tokenized on this
/// thread.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static TOKENIZED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        TOKENIZED.with(|c| c.replace(0))
    }
}
