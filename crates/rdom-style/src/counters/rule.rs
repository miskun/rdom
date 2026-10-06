//! A counter style's descriptors (CSS Counter Styles 3 §3): what an
//! `@counter-style` rule declares, and what each predefined style is
//! defined as (§6).

use std::sync::Arc;

/// `system` (§3.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum System {
    /// Cycles through its symbols (§3.1.1).
    Cyclic,
    /// Its symbols are digits of a place-value system (§3.1.4).
    Numeric,
    /// Its symbols are letters of a bijective place-value system
    /// (§3.1.5).
    Alphabetic,
    /// Cycles through its symbols, doubling them, tripling them, … on
    /// each pass (§3.1.3).
    Symbolic,
    /// Sums weighted symbols (§3.1.6).
    Additive,
    /// Its symbols, once, from the first symbol value on (§3.1.2).
    Fixed(i32),
    /// Another style's system and symbols, with these descriptors over
    /// its (§3.1.7).
    Extends(Arc<str>),
}

impl System {
    /// Whether the system writes a negative value with the `negative`
    /// sign (§3.4): symbolic, alphabetic, numeric and additive do.
    pub fn uses_negative(&self) -> bool {
        matches!(
            self,
            System::Symbolic | System::Alphabetic | System::Numeric | System::Additive
        )
    }

    /// The `range: auto` of the system (§3.5): every value for cyclic,
    /// numeric and fixed; 1 and up for alphabetic and symbolic; 0 and up
    /// for additive.
    pub fn auto_range(&self) -> (i64, i64) {
        match self {
            System::Alphabetic | System::Symbolic => (1, i64::MAX),
            System::Additive => (0, i64::MAX),
            _ => (i64::MIN, i64::MAX),
        }
    }
}

/// `range` (§3.5): `auto`, or inclusive bounds, `infinite` as the `i64`
/// extremes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CounterRange {
    /// The system's own range ([`System::auto_range`]).
    #[default]
    Auto,
    /// A union of inclusive ranges.
    Ranges(Arc<[(i64, i64)]>),
}

/// `speak-as` (§3.9) — parsed and kept, inert: rdom speaks nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SpeakAs {
    #[default]
    Auto,
    Bullets,
    Numbers,
    Words,
    SpellOut,
    /// Speak as another counter style.
    Style(Arc<str>),
}

/// A counter style's descriptors (§3) — an `@counter-style` rule, or a
/// predefined style's definition. `None` is "not declared": the initial
/// value, or, under `system: extends`, the extended style's.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct CounterStyleRule {
    /// `system` — `symbolic` when not declared (§3.1).
    pub system: Option<System>,
    /// `symbols` (§3.2).
    pub symbols: Option<Arc<[String]>>,
    /// `additive-symbols` (§3.2): weights descending.
    pub additive_symbols: Option<Arc<[(u32, String)]>>,
    /// `negative` (§3.4): the sign before, and after, a negative value
    /// — `"-"` and nothing by default.
    pub negative: Option<(String, String)>,
    /// `prefix` (§3.3) — nothing by default.
    pub prefix: Option<String>,
    /// `suffix` (§3.3) — `". "` by default.
    pub suffix: Option<String>,
    /// `range` (§3.5).
    pub range: Option<CounterRange>,
    /// `pad` (§3.6): the minimum length and the symbol to pad with.
    pub pad: Option<(u32, String)>,
    /// `fallback` (§3.7) — `decimal` by default.
    pub fallback: Option<Arc<str>>,
    /// `speak-as` (§3.9), inert.
    pub speak_as: Option<SpeakAs>,
}

impl CounterStyleRule {
    /// A rule of `system` over `symbols`.
    pub fn new(system: System, symbols: &[&str]) -> Self {
        CounterStyleRule {
            system: Some(system),
            symbols: Some(symbols.iter().map(|s| (*s).to_string()).collect()),
            ..Self::default()
        }
    }

    /// An additive rule (§3.1.6) over `(weight, symbol)` pairs, weights
    /// descending.
    pub fn additive(symbols: &[(u32, &str)]) -> Self {
        CounterStyleRule {
            system: Some(System::Additive),
            additive_symbols: Some(
                symbols
                    .iter()
                    .map(|(w, s)| (*w, (*s).to_string()))
                    .collect(),
            ),
            ..Self::default()
        }
    }

    /// With `suffix`.
    pub fn with_suffix(mut self, suffix: &str) -> Self {
        self.suffix = Some(suffix.to_string());
        self
    }

    /// With `range` — inclusive bounds.
    pub fn with_range(mut self, lo: i64, hi: i64) -> Self {
        self.range = Some(CounterRange::Ranges(Arc::from([(lo, hi)])));
        self
    }

    /// With `pad`.
    pub fn with_pad(mut self, length: u32, symbol: &str) -> Self {
        self.pad = Some((length, symbol.to_string()));
        self
    }

    /// With `fallback`.
    pub fn with_fallback(mut self, name: &str) -> Self {
        self.fallback = Some(Arc::from(name));
        self
    }

    /// Whether the rule's system has the symbols it needs (§3.2): one
    /// symbol for cyclic, fixed and symbolic; two for alphabetic and
    /// numeric; one additive tuple for additive. An `extends` rule may
    /// not declare `symbols` or `additive-symbols`. A rule failing this
    /// defines no counter style.
    pub fn is_valid(&self) -> bool {
        let symbols = self.symbols.as_ref().map_or(0, |s| s.len());
        let additive = self.additive_symbols.as_ref().map_or(0, |s| s.len());
        match self.system.as_ref().unwrap_or(&System::Symbolic) {
            System::Cyclic | System::Fixed(_) | System::Symbolic => symbols >= 1,
            System::Alphabetic | System::Numeric => symbols >= 2,
            System::Additive => additive >= 1,
            System::Extends(_) => self.symbols.is_none() && self.additive_symbols.is_none(),
        }
    }
}
