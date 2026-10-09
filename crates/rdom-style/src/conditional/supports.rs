//! Feature queries (CSS Conditional 3 §6, Conditional 4 §6–§7,
//! Conditional 5 §5): the `<supports-condition>` of `@supports`,
//! `@import … supports(…)` and `CSS.supports()`, evaluated against what
//! rdom parses — a declaration through the property dispatch table, with
//! the real value parser, a `selector()` through the selector parser.
//!
//! What rdom supports does not change while it runs, so a condition is
//! evaluated once, when parsed ([`SupportsCondition::matches`]).

use crate::parse::Token;

use super::syntax::{Cv, Prelude};
use super::{Condition, Truth};

/// A parsed, evaluated `<supports-condition>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportsCondition {
    condition: Condition<SupportsFeature>,
    text: String,
    holds: bool,
}

/// One `<supports-feature>`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SupportsFeature {
    /// `(<property>: <value>)` — Conditional 3 §6.1 `<supports-decl>`.
    Declaration { property: String, value: String },
    /// `selector(<complex-selector>)` — Conditional 4 §6.1.
    Selector(String),
    /// `font-tech(…)` — Conditional 5 §5.1: rdom draws no fonts.
    FontTech(String),
    /// `font-format(…)` — Conditional 5 §5.2: rdom draws no fonts.
    FontFormat(String),
}

impl SupportsCondition {
    /// Parse and evaluate a `<supports-condition>`; `None` when `text`
    /// does not match the grammar (an `@supports` rule with such a
    /// prelude is invalid, Conditional 3 §6).
    pub fn parse(text: &str) -> Option<Self> {
        let prelude = Prelude::parse(text)?;
        let values = prelude.values();
        let condition = prelude.condition(&values, true, &feature)?;
        // CSS Conditional 3 §6.1: a `<general-enclosed>` is false.
        let holds = condition
            .evaluate_enclosed(&mut |f| f.evaluate(), Truth::False)
            .holds();
        Some(SupportsCondition {
            text: text.trim().to_string(),
            condition,
            holds,
        })
    }

    /// Whether rdom supports what the condition asks (an unknown result
    /// is false, Conditional 4 §6.1).
    pub fn matches(&self) -> bool {
        self.holds
    }

    /// The parsed condition.
    pub fn condition(&self) -> &Condition<SupportsFeature> {
        &self.condition
    }
}

/// The condition as written (CSSOM `CSSSupportsRule.conditionText`).
impl std::fmt::Display for SupportsCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl SupportsFeature {
    fn evaluate(&self) -> Truth {
        match self {
            SupportsFeature::Declaration { property, value } => {
                Truth::from_bool(declaration_supported(property, value))
            }
            SupportsFeature::Selector(text) => Truth::from_bool(selector_supported(text)),
            SupportsFeature::FontTech(_) | SupportsFeature::FontFormat(_) => Truth::False,
        }
    }
}

/// A `( … )` block or a function as a supports feature, if it is one.
fn feature(prelude: &Prelude<'_>, cv: &Cv) -> Option<SupportsFeature> {
    match cv {
        Cv::Paren {
            inner,
            close: Some(_),
            ..
        } => {
            // `<supports-decl>`: `( <declaration> )`.
            let [name, colon, value @ ..] = inner.as_slice() else {
                return None;
            };
            let property = prelude.ident(name)?;
            if !matches!(colon, Cv::Token(i) if *prelude.token(*i) == Token::Colon) {
                return None;
            }
            Some(SupportsFeature::Declaration {
                property: property.to_string(),
                value: prelude.text_of_all(value).to_string(),
            })
        }
        Cv::Function {
            at,
            inner,
            close: Some(_),
        } => {
            let Token::Function(name) = prelude.token(*at) else {
                return None;
            };
            let args = prelude.text_of_all(inner).to_string();
            if name.eq_ignore_ascii_case("selector") {
                Some(SupportsFeature::Selector(args))
            } else if name.eq_ignore_ascii_case("font-tech") {
                Some(SupportsFeature::FontTech(args))
            } else if name.eq_ignore_ascii_case("font-format") {
                Some(SupportsFeature::FontFormat(args))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Whether `property: value` is a declaration rdom accepts (Conditional
/// 3 §6.1): a custom property with any value, or a property the dispatch
/// table knows whose value its parser takes — `!important` allowed, as in
/// a block.
fn declaration_supported(property: &str, value: &str) -> bool {
    let value = value.trim();
    let value = strip_important(value).unwrap_or(value);
    if property.starts_with("--") {
        return true;
    }
    !value.is_empty()
        && crate::property_dispatch::set(property, value, &mut crate::TuiStyle::new()).is_ok()
}

/// `value` without a trailing `! important`, if it has one.
fn strip_important(value: &str) -> Option<&str> {
    let bang = value.rfind('!')?;
    let after = value[bang + 1..].trim();
    after
        .eq_ignore_ascii_case("important")
        .then(|| value[..bang].trim_end())
}

/// Whether rdom parses `text` as one `<complex-selector>` (Conditional 4
/// §6.1): a list is not one.
fn selector_supported(text: &str) -> bool {
    match crate::StyleSelector::parse(text) {
        Ok(selector) => selector.len() == 1,
        Err(_) => false,
    }
}

/// `CSS.supports(property, value)` (CSS Conditional 3 §7.1): whether
/// `property: value` is a declaration rdom accepts. `!important` is not
/// part of a value, so a value carrying it is not supported.
pub fn supports(property: &str, value: &str) -> bool {
    strip_important(value.trim()).is_none() && declaration_supported(property.trim(), value)
}

/// `CSS.supports(conditionText)` (CSS Conditional 3 §7.1): whether the
/// `<supports-condition>` holds — or, when the text is not one, the
/// declaration it is, parenthesized (`CSS.supports("display: grid")`).
pub fn supports_condition(text: &str) -> bool {
    match SupportsCondition::parse(text) {
        Some(c) => c.matches(),
        None => SupportsCondition::parse(&format!("({text})")).is_some_and(|c| c.matches()),
    }
}
