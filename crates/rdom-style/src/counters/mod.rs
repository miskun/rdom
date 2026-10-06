//! CSS counters (CSS Lists 3 §4) and counter styles (CSS Counter
//! Styles 3): the `counter-reset` / `counter-increment` declarations,
//! the `<counter-style>` value `counter()` / `counters()` and the list
//! markers name, and the generation of a counter's representation. The
//! cascade in `rdom-tui` keeps the per-element counter state; this module
//! owns the data model and the formatting.
//!
//! Counter styles are table-driven: every style — the predefined ones
//! of §6 ([`predefined_names`]) and, through the same descriptors, an
//! author's `@counter-style` — is a [`CounterStyleRule`], and one
//! algorithm (`generate`) turns a rule and a value into text.

mod generate;
mod predefined;
mod rule;
mod style;
#[cfg(test)]
mod tests;

pub use generate::{MAX_FALLBACK_DEPTH, MAX_REPRESENTATION_CHARS};
pub use predefined::{predefined, predefined_names};
pub use rule::{CounterRange, CounterStyleRule, SpeakAs, System};
pub use style::{CounterStyle, CounterStyleLookup, Predefined};

/// One `name [<integer>]` item of `counter-reset` / `counter-increment`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CounterOp {
    pub name: String,
    /// The reset value, or the increment delta.
    pub value: i32,
}
