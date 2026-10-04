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

use super::selector_text::{extract_pseudo_suffix, split_top_level_commas};
use super::{LayerId, PseudoElementTarget, Rule, RuleOrigin, StyleError, Stylesheet};
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
    text: String,
}

impl StyleSelector {
    /// Parse a top-level rule's selector list.
    pub fn parse(text: &str) -> Result<Self, StyleError> {
        Self::parse_in(text, None)
    }

    /// Parse the selector list of a rule nested in a rule whose
    /// selector is `parent` (CSS Nesting 1 §2): `&` is the parent's
    /// elements, and a selector without `&` is relative to them.
    pub fn parse_nested(text: &str, parent: &StyleSelector) -> Result<Self, StyleError> {
        Self::parse_in(text, Some(&parent.nesting_list()))
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

    fn parse_in(text: &str, nest: Option<&SelectorList>) -> Result<Self, StyleError> {
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
            // A nested `::before` is relative: `& ::before`, i.e.
            // `& *::before`.
            let owned;
            let source = if nest.is_some() && trimmed.starts_with("::") {
                owned = format!("*{trimmed}");
                owned.as_str()
            } else {
                trimmed
            };
            let (core, pseudo) = extract_pseudo_suffix(source).map_err(error)?;
            let parsed = match nest {
                Some(parent) => selectors::parse_nested(core, parent),
                None => selectors::parse(core),
            }
            .map_err(|e| StyleError::from((text, e)))?;
            for complex in parsed.0 {
                items.push(Item {
                    complex,
                    pseudo,
                    text: trimmed.to_string(),
                });
            }
        }
        Ok(StyleSelector { items })
    }
}

impl Stylesheet {
    /// Add an author style rule for a parsed selector (one [`Rule`] per
    /// item) in `layer` (`None`: unlayered).
    pub fn add_style_rule(
        &mut self,
        selector: &StyleSelector,
        style: TuiStyle,
        layer: Option<LayerId>,
    ) {
        let mut rules = self.rules_for(selector, style, RuleOrigin::Author);
        for rule in &mut rules {
            rule.layer = layer;
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
        selector
            .items
            .iter()
            .map(|item| {
                let pseudo_count = u16::from(item.pseudo != PseudoElementTarget::None);
                let source_idx = self.next_source_idx;
                self.next_source_idx += 1;
                // CSS Pseudo-Elements 4 §4.3: only the `::first-line`
                // properties apply to `::placeholder`.
                let style = if item.pseudo == PseudoElementTarget::Placeholder {
                    style.first_line_subset()
                } else {
                    style.clone()
                };
                Rule {
                    selector: SelectorList(vec![item.complex.clone()]),
                    pseudo: item.pseudo,
                    style,
                    specificity: Specificity::of_complex(&item.complex, pseudo_count),
                    origin,
                    source_idx,
                    source_text: item.text.clone(),
                    layer: None,
                }
            })
            .collect()
    }
}
