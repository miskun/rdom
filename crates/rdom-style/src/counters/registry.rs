//! The author's counter styles: `@counter-style` definitions as a sheet
//! keeps them, and the registry the cascade resolves names through.

use std::collections::HashMap;
use std::sync::Arc;

use super::rule::CounterStyleRule;
use super::style::CounterStyleLookup;
use crate::LayerId;

/// One `@counter-style` rule of a sheet (CSS Counter Styles 3 §3): the
/// name it defines (a predefined name lowercased, as `CounterStyle::named`
/// does), its descriptors, and the cascade layer it sits in.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CounterStyleDefinition {
    pub name: Arc<str>,
    pub rule: CounterStyleRule,
    pub layer: Option<LayerId>,
}

impl CounterStyleDefinition {
    /// `@counter-style name { rule }`, unlayered.
    pub fn new(name: &str, rule: CounterStyleRule) -> Self {
        let name = super::CounterStyle::named(name);
        CounterStyleDefinition {
            name: Arc::from(name.name().unwrap_or_default()),
            rule,
            layer: None,
        }
    }

    /// In cascade layer `layer`.
    pub fn in_layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = layer;
        self
    }
}

/// The counter styles names resolve to: author definitions over the
/// predefined styles (§3, §6). A later [`define`](Self::define) of a
/// name replaces an earlier one, so defining in cascade order (layer,
/// then sheet, then source) makes the winning rule the last.
#[derive(Debug, Clone, Default)]
pub struct CounterStyleRegistry {
    styles: HashMap<Arc<str>, CounterStyleRule>,
}

impl CounterStyleRegistry {
    /// No author styles: the predefined ones alone.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define (or redefine) `name`.
    pub fn define(&mut self, name: Arc<str>, rule: CounterStyleRule) {
        self.styles.insert(name, rule);
    }

    /// No author style is defined.
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }
}

impl CounterStyleLookup for CounterStyleRegistry {
    fn rule(&self, name: &str) -> Option<&CounterStyleRule> {
        self.styles.get(name).or_else(|| super::predefined(name))
    }
}
