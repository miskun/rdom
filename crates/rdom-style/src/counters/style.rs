//! `<counter-style>` (CSS Counter Styles 3 §3, §5; CSS Lists 3 §4.3):
//! the style a `counter()`, `counters()` or list marker names, and where
//! names are looked up.

use std::sync::Arc;

use super::generate::{Representation, generate};
use super::rule::{CounterStyleRule, System};

/// Where counter style names resolve: the predefined styles, and — in
/// the cascade — the author's `@counter-style` rules over them
/// ([`CounterStyleRegistry`](super::CounterStyleRegistry)).
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
/// a `<custom-ident>`) or `symbols()` (§5). A name no rule defines
/// formats as `decimal`; the name `none` formats as nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterStyle {
    /// A `<counter-style-name>`. Predefined names are stored lowercase.
    Name(Arc<str>),
    /// `symbols( <symbols-type>? <string>+ )` (§5): an anonymous style
    /// of that system (`symbolic` by default) over the strings, with
    /// the suffix `" "`.
    Symbols(Arc<CounterStyleRule>),
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
        is_counter_style_name(ident).then(|| Self::named(ident))
    }

    /// `symbols(system "s"…)` (§5): `system` is `cyclic`, `numeric`,
    /// `alphabetic`, `symbolic` or `fixed` (first symbol value 1);
    /// `None` for another system, or too few symbols for it (two for
    /// alphabetic and numeric, one otherwise).
    pub fn symbols(system: System, symbols: &[&str]) -> Option<Self> {
        if matches!(system, System::Additive | System::Extends(_)) {
            return None;
        }
        let rule = CounterStyleRule::new(system, symbols).with_suffix(" ");
        rule.is_valid()
            .then(|| CounterStyle::Symbols(Arc::new(rule)))
    }

    /// The style's name; `None` for `symbols()`.
    pub fn name(&self) -> Option<&str> {
        match self {
            CounterStyle::Name(name) => Some(name),
            CounterStyle::Symbols(_) => None,
        }
    }

    /// Whether this is `decimal`.
    pub fn is_decimal(&self) -> bool {
        self.name() == Some("decimal")
    }

    /// The CSS text: the name, or `symbols(<system> "…" …)` — the system
    /// left out when it is `symbolic`.
    pub fn to_css(&self) -> String {
        match self {
            CounterStyle::Name(name) => name.to_string(),
            CounterStyle::Symbols(rule) => {
                let mut out = String::from("symbols(");
                let system = match rule.system.as_ref() {
                    Some(System::Cyclic) => Some("cyclic"),
                    Some(System::Numeric) => Some("numeric"),
                    Some(System::Alphabetic) => Some("alphabetic"),
                    Some(System::Fixed(_)) => Some("fixed"),
                    _ => None,
                };
                let mut parts: Vec<String> = system.into_iter().map(str::to_string).collect();
                for s in rule.symbols.iter().flat_map(|s| s.iter()) {
                    parts.push(css_string(s));
                }
                out.push_str(&parts.join(" "));
                out.push(')');
                out
            }
        }
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
        generate(lookup, self, n, rtl, Representation::Counter)
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
        generate(lookup, self, n, rtl, Representation::Marker)
    }
}

/// Whether `ident` may name a counter style: a `<custom-ident>` other
/// than the CSS-wide keywords and `default` (CSS Values 4 §4.2).
pub fn is_counter_style_name(ident: &str) -> bool {
    let reserved = [
        "inherit",
        "initial",
        "unset",
        "revert",
        "revert-layer",
        "default",
    ];
    !ident.is_empty() && !reserved.iter().any(|r| ident.eq_ignore_ascii_case(r))
}

/// A CSS string (CSSOM §2.1 "serialize a string"): double-quoted, `"`
/// and `\\` escaped, a control character as its code point.
pub(crate) fn css_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push_str(&format!("\\{:x} ", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
