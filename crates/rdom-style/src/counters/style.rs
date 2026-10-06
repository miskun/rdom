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

/// A counter style's name: a predefined one (lowercase, `'static` — no
/// allocation, so the initial `list-style-type: disc` costs nothing per
/// element) or an author's.
#[derive(Debug, Clone)]
pub struct CounterStyleName(NameRepr);

#[derive(Debug, Clone)]
enum NameRepr {
    Static(&'static str),
    Shared(Arc<str>),
}

impl CounterStyleName {
    /// The name.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            NameRepr::Static(s) => s,
            NameRepr::Shared(s) => s,
        }
    }
}

impl std::ops::Deref for CounterStyleName {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for CounterStyleName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CounterStyleName {}

impl std::hash::Hash for CounterStyleName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

/// A `<counter-style>`: a counter style name (§3: `<counter-style-name>`,
/// a `<custom-ident>`) or `symbols()` (§5). A name no rule defines
/// formats as `decimal`; the name `none` formats as nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterStyle {
    /// A `<counter-style-name>`. Predefined names are stored lowercase.
    Name(CounterStyleName),
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
    pub const fn decimal() -> Self {
        Self::predefined_static("decimal")
    }

    /// `disc`, `list-style-type`'s initial value.
    pub const fn disc() -> Self {
        Self::predefined_static("disc")
    }

    const fn predefined_static(name: &'static str) -> Self {
        CounterStyle::Name(CounterStyleName(NameRepr::Static(name)))
    }

    /// The style named `name`. A predefined name matches ASCII
    /// case-insensitively and is lowercased (§3: "predefined counter
    /// styles … are matched case-insensitively"), without allocating;
    /// any other name keeps its case.
    pub fn named(name: &str) -> Self {
        let predefined = super::predefined_names()
            .iter()
            .chain(std::iter::once(&"none"))
            .find(|p| p.eq_ignore_ascii_case(name));
        match predefined {
            Some(p) => Self::predefined_static(p),
            None => CounterStyle::Name(CounterStyleName(NameRepr::Shared(Arc::from(name)))),
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
            CounterStyle::Name(name) => Some(name.as_str()),
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
            CounterStyle::Name(name) => name.as_str().to_string(),
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
                    parts.push(rdom_core::css_syntax::serialize_string(s));
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
