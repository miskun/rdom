//! The `@position-try` rules a sheet defines (CSS Anchor Positioning 1
//! §4.1): named position options, their declarations the inset, margin,
//! sizing and self-alignment properties, `position-anchor` and
//! `position-area`. A backend lets the last of a name win, in cascade
//! layer order (CSS Cascade 5 §6.4.3: unlayered last), then sheet order,
//! then source order — as `@keyframes`.

use std::sync::Arc;

use super::Stylesheet;
use crate::TuiStyle;

/// One `@position-try --name { … }` rule.
///
/// `#[non_exhaustive]`: built by [`PositionTryRule::new`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PositionTryRule {
    /// Its `<dashed-ident>`.
    pub name: Arc<str>,
    /// The cascade layer it sits in.
    pub layer: Option<crate::LayerId>,
    /// The conditional group rule it sits in.
    pub condition: Option<crate::ConditionId>,
    declarations: Arc<TuiStyle>,
}

impl PositionTryRule {
    /// The rule `name` with `declarations` (the parser keeps only the
    /// descriptors §4.1 accepts).
    pub fn new(name: impl Into<Arc<str>>, declarations: TuiStyle) -> Self {
        PositionTryRule {
            name: name.into(),
            layer: None,
            condition: None,
            declarations: Arc::new(declarations),
        }
    }

    /// The rule in the cascade layer `layer`.
    pub fn in_layer(mut self, layer: Option<crate::LayerId>) -> Self {
        self.layer = layer;
        self
    }

    /// The rule under the conditional group rule `condition`.
    pub fn in_condition(mut self, condition: Option<crate::ConditionId>) -> Self {
        self.condition = condition;
        self
    }

    /// Its declarations.
    pub fn declarations(&self) -> &Arc<TuiStyle> {
        &self.declarations
    }

    /// Whether `name` is a descriptor a `@position-try` rule takes
    /// (§4.1): the inset, margin and sizing properties (their logical
    /// forms too), the self-alignment properties, `position-anchor` and
    /// `position-area`.
    pub fn accepts(name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        let n = name.as_str();
        matches!(
            n,
            "top"
                | "right"
                | "bottom"
                | "left"
                | "inset"
                | "width"
                | "height"
                | "min-width"
                | "min-height"
                | "max-width"
                | "max-height"
                | "inline-size"
                | "block-size"
                | "min-inline-size"
                | "min-block-size"
                | "max-inline-size"
                | "max-block-size"
                | "margin"
                | "justify-self"
                | "align-self"
                | "place-self"
                | "position-anchor"
                | "position-area"
        ) || n.starts_with("inset-")
            || n.starts_with("margin-")
    }
}

impl Stylesheet {
    /// The `@position-try` rules this sheet defines, in source order.
    pub fn position_try_rules(&self) -> &[PositionTryRule] {
        &self.position_try
    }

    /// Define a `@position-try` rule (parsed, or a Rust-built sheet's).
    pub fn define_position_try(&mut self, rule: PositionTryRule) {
        self.touch();
        self.position_try.push(rule);
    }
}
