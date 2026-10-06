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

mod descriptors;
mod generate;
mod predefined;
mod registry;
mod rule;
mod style;
#[cfg(test)]
mod tests;

pub use descriptors::{apply_descriptor, check_rule, parse_counter_style};
pub use generate::{MAX_FALLBACK_DEPTH, MAX_REPRESENTATION_CHARS};
pub use predefined::{predefined, predefined_names};
pub use registry::{CounterStyleDefinition, CounterStyleRegistry};
pub use rule::{CounterRange, CounterStyleRule, SpeakAs, System};
pub(crate) use style::css_string;
pub use style::{CounterStyle, CounterStyleLookup, Predefined, is_counter_style_name};

/// One item of `counter-reset`, `counter-increment` or `counter-set`
/// (CSS Lists 3 §4.2–§4.3): `<counter-name> <integer>?`, or — reset
/// only — `reversed(<counter-name>) <integer>?`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CounterOp {
    pub name: String,
    /// The reset or set value, or the increment delta. For a reversed
    /// counter whose value was not given, the initial value the cascade
    /// computed from the increments in its scope (§4.2) — 0 as declared.
    pub value: i32,
    /// `reversed(<counter-name>)`: a reversed counter (§4.2), which the
    /// implicit `list-item` increment counts down (§4.6).
    pub reversed: bool,
    /// The integer was written; `false` only for a reversed reset
    /// without one, whose initial value is computed.
    pub value_given: bool,
}

impl CounterOp {
    /// `name value` — a plain reset, increment or set.
    pub fn new(name: impl Into<String>, value: i32) -> Self {
        CounterOp {
            name: name.into(),
            value,
            reversed: false,
            value_given: true,
        }
    }

    /// `reversed(name) value?` — a reversed counter's reset; `None`
    /// lets the cascade compute its initial value (§4.2).
    pub fn reversed(name: impl Into<String>, value: Option<i32>) -> Self {
        CounterOp {
            name: name.into(),
            value: value.unwrap_or(0),
            reversed: true,
            value_given: value.is_some(),
        }
    }

    /// Whether the cascade computes this op's value: a reversed reset
    /// without an integer.
    pub fn is_auto_reversed(&self) -> bool {
        self.reversed && !self.value_given
    }
}
