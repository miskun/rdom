//! `<counter-style>` (CSS Counter Styles 3 §3, CSS Lists 3 §4.3): the
//! style a `counter()`, `counters()` or list marker names, and where
//! names are looked up.

use std::sync::Arc;

use super::generate::{Representation, generate};
use super::rule::CounterStyleRule;

/// Where counter style names resolve: the predefined styles, and — in
/// the cascade — the author's `@counter-style` rules over them.
pub trait CounterStyleLookup {
    /// The rule defining `name` (a counter style name, compared
    /// case-sensitively; predefined names are lowercase), if any.
    fn rule(&self, name: &str) -> Option<&CounterStyleRule>;
}

/// The predefined counter styles alone (§6).
#[derive(Debug, Clone, Copy, Default)]
pub struct Predefined;

impl CounterStyleLookup for Predefined {
    fn rule(&self, name: &str) -> Option<&CounterStyleRule> {
        super::predefined(name)
    }
}

/// A `<counter-style>`: a counter style name (§3: `<counter-style-name>`,
/// a `<custom-ident>`). A name no rule defines formats as `decimal`; the
/// name `none` formats as nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterStyle {
    /// A `<counter-style-name>`. Predefined names are stored lowercase.
    Name(Arc<str>),
}

impl Default for CounterStyle {
    /// `decimal`, every counter's default style.
    fn default() -> Self {
        Self::decimal()
    }
}

impl CounterStyle {
    /// `decimal`, every counter's default style.
    pub fn decimal() -> Self {
        CounterStyle::Name(Arc::from("decimal"))
    }

    /// The style named `name`. A predefined name matches ASCII
    /// case-insensitively and is lowercased (§3: "predefined counter
    /// styles … are matched case-insensitively"); any other name keeps
    /// its case.
    pub fn named(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        if super::predefined(&lower).is_some() || lower == "none" {
            CounterStyle::Name(Arc::from(lower))
        } else {
            CounterStyle::Name(Arc::from(name))
        }
    }

    /// Parse a `<counter-style-name>` identifier: any `<custom-ident>`
    /// but the CSS-wide keywords and `default` (CSS Values 4 §4.2).
    pub fn parse(ident: &str) -> Option<Self> {
        let reserved = [
            "inherit",
            "initial",
            "unset",
            "revert",
            "revert-layer",
            "default",
        ];
        if ident.is_empty() || reserved.iter().any(|r| ident.eq_ignore_ascii_case(r)) {
            return None;
        }
        Some(Self::named(ident))
    }

    /// The style's name.
    pub fn name(&self) -> &str {
        match self {
            CounterStyle::Name(name) => name,
        }
    }

    /// Whether this is `decimal`.
    pub fn is_decimal(&self) -> bool {
        self.name() == "decimal"
    }

    /// `n` in this style, as `counter()` writes it (no prefix or
    /// suffix), resolved among the predefined styles, left-to-right.
    pub fn format(&self, n: i32) -> String {
        self.format_in(n, false)
    }

    /// [`format`](Self::format) for text of the given direction
    /// (`disclosure-closed` points along it).
    pub fn format_in(&self, n: i32, rtl: bool) -> String {
        self.format_with(n, rtl, &super::Predefined)
    }

    /// `n` as `counter()` writes it, resolving names through `lookup`.
    pub fn format_with(&self, n: i32, rtl: bool, lookup: &impl CounterStyleLookup) -> String {
        generate(lookup, self.name(), n, rtl, Representation::Counter)
    }

    /// A list marker's text for `n` (CSS Lists 3 §3.1): the
    /// representation between the style's `prefix` and `suffix`, among
    /// the predefined styles.
    pub fn marker_text(&self, n: i32, rtl: bool) -> String {
        self.marker_text_with(n, rtl, &super::Predefined)
    }

    /// [`marker_text`](Self::marker_text), resolving names through
    /// `lookup`.
    pub fn marker_text_with(&self, n: i32, rtl: bool, lookup: &impl CounterStyleLookup) -> String {
        generate(lookup, self.name(), n, rtl, Representation::Marker)
    }
}
