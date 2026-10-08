//! [`StyleSelector`] — a style rule's selector list, parsed once: each
//! comma-separated item split off, its pseudo-element suffix stripped
//! (`selector_text.rs`) and its structural part parsed by rdom-core —
//! resolved against the parent rule's list when the rule is nested
//! (CSS Nesting 1 §2, `rdom_core::selectors::parse_nested`).
//!
//! The CSS parser parses a rule's selector before its block, because a
//! nested rule inside the block needs it as its parent; it then adds
//! the block's declarations with [`Stylesheet::add_style_rule`].

use rdom_core::selectors::{self, ComplexSelector, SelectorList};

use super::selector_text::{extract_pseudo_chain, split_top_level_commas};
use super::{
    LayerId, PseudoElementTarget, Rule, RuleOrigin, ScopeId, StyleError, Stylesheet,
    UserActionState,
};
use crate::{Specificity, TuiStyle};

/// A style rule's parsed selector list.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSelector {
    items: Vec<Item>,
}

/// One comma-separated item: its structural selector, its
/// pseudo-element and its source text.
#[derive(Debug, Clone, PartialEq)]
struct Item {
    complex: ComplexSelector,
    pseudo: PseudoElementTarget,
    /// The user-action pseudo-classes after the pseudo-element.
    pseudo_state: UserActionState,
    text: String,
}

impl StyleSelector {
    /// Parse a top-level rule's selector list.
    pub fn parse(text: &str) -> Result<Self, StyleError> {
        Self::parse_in(text, Mode::Top)
    }

    /// Parse the selector list of a scoped style rule — one directly in
    /// `@scope` (CSS Cascade 6 §2.5.2): relative to `:where(:scope)`,
    /// `&` is `:where(:scope)`.
    pub fn parse_scoped(text: &str) -> Result<Self, StyleError> {
        Self::parse_in(text, Mode::Scoped)
    }

    /// Parse the selector list of a rule nested in a rule whose
    /// selector is `parent` (CSS Nesting 1 §2): `&` is the parent's
    /// elements, and a selector without `&` is relative to them.
    pub fn parse_nested(text: &str, parent: &StyleSelector) -> Result<Self, StyleError> {
        Self::parse_in(text, Mode::Nested(&parent.nesting_list()))
    }

    /// The list `&` stands for in a rule nested in this one: the items
    /// without a pseudo-element, which `&` cannot represent (CSS
    /// Nesting 1 §2).
    pub fn nesting_list(&self) -> SelectorList {
        SelectorList(
            self.items
                .iter()
                .filter(|i| i.pseudo == PseudoElementTarget::None)
                .map(|i| i.complex.clone())
                .collect(),
        )
    }

    fn parse_in(text: &str, mode: Mode<'_>) -> Result<Self, StyleError> {
        let error = |msg: String| StyleError {
            msg,
            pos: None,
            source: text.to_string(),
        };
        let raw = split_top_level_commas(text);
        if raw.is_empty() {
            return Err(error("empty selector".to_string()));
        }
        let mut items = Vec::with_capacity(raw.len());
        for item in raw {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return Err(error("empty selector in list".to_string()));
            }
            // A bare `::before` is `*::before` (Selectors 4 §5.2); nested,
            // it is relative: `& *::before`.
            let (core, pseudo, pseudo_state) = extract_pseudo_chain(trimmed).map_err(error)?;
            let parsed = match mode {
                Mode::Top => selectors::parse(&core),
                Mode::Nested(parent) => selectors::parse_nested(&core, parent),
                Mode::Scoped => selectors::parse_scoped(&core),
            }
            .map_err(|e| StyleError::from((text, e)))?;
            for complex in parsed.0 {
                items.push(Item {
                    complex,
                    pseudo: pseudo.clone(),
                    pseudo_state,
                    text: trimmed.to_string(),
                });
            }
        }
        Ok(StyleSelector { items })
    }
}

/// What a selector is relative to.
#[derive(Clone, Copy)]
enum Mode<'a> {
    Top,
    Nested(&'a SelectorList),
    Scoped,
}

/// Where an author style rule sits: its cascade layer and `@scope`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RuleContext {
    /// The cascade layer (`None`: unlayered).
    pub layer: Option<LayerId>,
    /// The innermost `@scope` (`None`: unscoped).
    pub scope: Option<ScopeId>,
    /// Inside `@starting-style` (CSS Transitions 2 §3): the rule applies
    /// only to an element's starting style.
    pub starting_style: bool,
}

impl RuleContext {
    /// In cascade layer `layer`.
    pub fn in_layer(self, layer: Option<LayerId>) -> Self {
        RuleContext { layer, ..self }
    }

    /// In `@scope` `scope`.
    pub fn in_scope(self, scope: Option<ScopeId>) -> Self {
        RuleContext { scope, ..self }
    }

    /// Inside `@starting-style`.
    pub fn in_starting_style(self) -> Self {
        RuleContext {
            starting_style: true,
            ..self
        }
    }
}

impl Stylesheet {
    /// Add an author style rule for a parsed selector (one [`Rule`] per
    /// item) at `ctx`'s layer and scope.
    pub fn add_style_rule(&mut self, selector: &StyleSelector, style: TuiStyle, ctx: RuleContext) {
        let mut rules = self.rules_for(selector, style, RuleOrigin::Author);
        for rule in &mut rules {
            rule.layer = ctx.layer;
            rule.scope = ctx.scope;
            rule.starting_style = ctx.starting_style;
        }
        self.push_rules(rules);
    }

    /// The rules of `selector`'s items, each with its specificity and
    /// the next source index.
    pub(super) fn rules_for(
        &mut self,
        selector: &StyleSelector,
        style: TuiStyle,
        origin: RuleOrigin,
    ) -> Vec<Rule> {
        // The direction-mapped forms of the block, built once for every
        // item (and once for the `::placeholder` subset, if any).
        let directional = style.directional_overlays().map(std::sync::Arc::new);
        selector
            .items
            .iter()
            .map(|item| {
                // Selectors 4 §17: each pseudo-element counts as a type
                // selector — two in a nested `::before::marker`.
                let pseudo_count = match item.pseudo {
                    PseudoElementTarget::None => 0,
                    PseudoElementTarget::BeforeMarker | PseudoElementTarget::AfterMarker => 2,
                    _ => 1,
                };
                let mut specificity = Specificity::of_complex(&item.complex, pseudo_count);
                // Selectors 4 §17: a pseudo-class after a pseudo-element
                // counts as any pseudo-class does.
                specificity.class_attr_pseudo += item.pseudo_state.len();
                let source_idx = self.next_source_idx;
                self.next_source_idx += 1;
                // CSS Pseudo-Elements 4 §2.2.1, §4.3: only the
                // `::first-line` properties apply to `::first-line` and
                // `::placeholder`. None of them is flow-relative, and the
                // subset keeps declarations only while one waits for
                // substitution, so it has no direction-mapped forms to
                // precompute.
                let (style, directional) = match item.pseudo {
                    PseudoElementTarget::Placeholder | PseudoElementTarget::FirstLine => {
                        (style.first_line_subset(), None)
                    }
                    // §2.3.1: `::first-letter` takes box properties too,
                    // flow-relative ones among them, mapped by direction.
                    PseudoElementTarget::FirstLetter => {
                        let subset = style.first_letter_subset();
                        let directional = subset.directional_overlays().map(std::sync::Arc::new);
                        (subset, directional)
                    }
                    // CSS Lists 3 §3.2: only some properties apply to
                    // `::marker`; none of them is flow-relative.
                    PseudoElementTarget::Marker
                    | PseudoElementTarget::BeforeMarker
                    | PseudoElementTarget::AfterMarker => (style.marker_subset(), None),
                    // CSS Pseudo-Elements 4 §3.2: the highlight
                    // pseudo-elements take a few paint properties.
                    PseudoElementTarget::Selection | PseudoElementTarget::Highlight(_) => {
                        (style.highlight_subset(), None)
                    }
                    _ => (style.clone(), directional.clone()),
                };
                Rule {
                    selector: SelectorList(vec![item.complex.clone()]),
                    pseudo: item.pseudo.clone(),
                    pseudo_state: item.pseudo_state,
                    style,
                    specificity,
                    origin,
                    source_idx,
                    source_text: item.text.clone(),
                    layer: None,
                    scope: None,
                    starting_style: false,
                    directional,
                }
            })
            .collect()
    }
}
