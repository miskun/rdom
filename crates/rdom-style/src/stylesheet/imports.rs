//! `@import` records (CSS Cascade 5 §3, CSSOM `CSSImportRule`).
//!
//! The parser resolves an `@import` through a host-provided loader and
//! inserts the imported rules at the import's position; the sheet keeps
//! one [`Import`] per import that loaded, with its layer and its
//! conditions. The `supports()` and media conditions are recorded as
//! written: rdom evaluates no conditional rules yet (C14), so an
//! imported sheet always applies.

use super::{LayerId, Stylesheet};

/// One `@import` that loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Import {
    /// The URL as written (`url(…)` or a string), handed to the loader.
    pub url: String,
    /// The layer the imported rules sit in (`layer` / `layer(name)`).
    pub layer: Option<LayerId>,
    /// The `supports(…)` condition's text.
    pub supports: Option<String>,
    /// The media query list's text.
    pub media: Option<String>,
}

impl Import {
    pub fn new(
        url: String,
        layer: Option<LayerId>,
        supports: Option<String>,
        media: Option<String>,
    ) -> Self {
        Import {
            url,
            layer,
            supports,
            media,
        }
    }
}

impl Stylesheet {
    /// The `@import`s that loaded, in source order (nested imports
    /// after the import that holds them).
    pub fn imports(&self) -> &[Import] {
        &self.imports
    }

    /// Record an `@import` (the parser does this).
    pub fn record_import(&mut self, import: Import) {
        self.touch();
        self.imports.push(import);
    }
}
