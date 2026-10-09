//! Container queries (CSS Conditional 5 §6.4–§6.5): the prelude of
//! `@container` — `<container-condition>#`, each `[ <container-name> ]?
//! <container-query>?` — and its evaluation against one query container.
//!
//! A `<container-query>` is the shared `not` / `and` / `or` grammar over
//! size features (Media Queries 4's range syntax: `width`, `height`,
//! `inline-size`, `block-size`, `aspect-ratio`, `orientation`), style
//! queries (`style(--x: y)`) and `scroll-state()` queries. Which element
//! is the query container, and its size, is the backend's: it finds the
//! nearest ancestor this condition can query ([`ContainerCondition::name`],
//! [`ContainerCondition::needs_size`], [`ContainerCondition::needs_scroll_state`])
//! and describes it as a [`QueryContainer`].

use std::sync::Arc;

use crate::parse::Token;

use super::media_feature::MediaFeature;
use super::syntax::{Cv, Prelude, split_commas};
use super::{Condition, Truth};

/// A parsed `@container` prelude: it matches when any of its conditions
/// does.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerQuery {
    conditions: Vec<ContainerCondition>,
    text: String,
}

/// One `<container-condition>`: an optional name and query.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerCondition {
    name: Option<Arc<str>>,
    query: Option<Condition<ContainerFeature>>,
}

/// A leaf of a `<container-query>`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ContainerFeature {
    /// A size feature (§6.5), in Media Queries 4's feature syntax.
    Size(MediaFeature),
    /// `style( <style-query> )` (§6.4).
    Style(Condition<StyleFeature>),
    /// `scroll-state( … )`, kept as written: rdom does not evaluate it.
    ScrollState(String),
}

/// One `<style-feature>`: `(--name: value)`, or `(--name)` (the property
/// has a value). A standard property is kept, and unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleFeature {
    /// The property name, as written.
    pub name: String,
    /// The value, as written and trimmed; `None` for `(--name)`.
    pub value: Option<String>,
}

/// The query container a condition is evaluated against: its content-box
/// size on the axes it answers size queries on (cells; `None` on an axis
/// it does not, or before it is laid out), and its custom properties'
/// computed values.
#[non_exhaustive]
pub struct QueryContainer<'a> {
    pub width: Option<f64>,
    pub height: Option<f64>,
    /// A custom property's computed value, as text (`None`: the
    /// guaranteed-invalid value).
    pub custom: &'a dyn Fn(&str) -> Option<String>,
}

impl<'a> QueryContainer<'a> {
    /// A container of `width` × `height` cells (each `None` on an axis it
    /// does not answer size queries on) whose custom properties `custom`
    /// reads.
    pub fn new(
        width: Option<f64>,
        height: Option<f64>,
        custom: &'a dyn Fn(&str) -> Option<String>,
    ) -> Self {
        QueryContainer {
            width,
            height,
            custom,
        }
    }
}

impl ContainerQuery {
    /// Parse an `@container` prelude; `None` when it is not a
    /// `<container-condition>#` (the rule is invalid).
    pub fn parse(text: &str) -> Option<Self> {
        let prelude = Prelude::parse(text)?;
        let values = prelude.values();
        if values.is_empty() {
            return None;
        }
        let conditions = split_commas(&prelude, &values)
            .into_iter()
            .map(|c| ContainerCondition::parse(&prelude, c))
            .collect::<Option<Vec<_>>>()?;
        Some(ContainerQuery {
            conditions,
            text: text.trim().to_string(),
        })
    }

    /// The conditions, in order.
    pub fn conditions(&self) -> &[ContainerCondition] {
        &self.conditions
    }
}

/// The prelude as written (CSSOM `CSSContainerRule.conditionText`).
impl std::fmt::Display for ContainerQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl ContainerCondition {
    fn parse(prelude: &Prelude<'_>, values: &[Cv]) -> Option<Self> {
        let (name, rest) = match values.split_first() {
            Some((first, rest))
                if prelude.ident(first).is_some_and(|n| {
                    !["not", "and", "or", "none"]
                        .iter()
                        .any(|k| n.eq_ignore_ascii_case(k))
                }) =>
            {
                (prelude.ident(first).map(Arc::<str>::from), rest)
            }
            _ => (None, values),
        };
        let query = if rest.is_empty() {
            None
        } else {
            Some(prelude.condition(rest, true, &feature)?)
        };
        // §6.4: a condition is a name, a query or both.
        (name.is_some() || query.is_some()).then_some(ContainerCondition { name, query })
    }

    /// The container name it selects by, if any (case-sensitive).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Whether it asks a size feature: its container must be a size
    /// container (`container-type: size | inline-size`).
    pub fn needs_size(&self) -> bool {
        self.leaves()
            .any(|f| matches!(f, ContainerFeature::Size(_)))
    }

    /// Whether it asks a scroll-state feature: its container must be a
    /// `scroll-state` container.
    pub fn needs_scroll_state(&self) -> bool {
        self.leaves()
            .any(|f| matches!(f, ContainerFeature::ScrollState(_)))
    }

    fn leaves(&self) -> impl Iterator<Item = &ContainerFeature> {
        self.query.iter().flat_map(Condition::leaves)
    }

    /// Evaluate against `container` (§6.4); a condition with a name only
    /// holds once the container is found.
    pub fn evaluate(&self, container: &QueryContainer<'_>) -> Truth {
        let Some(query) = &self.query else {
            return Truth::True;
        };
        query.evaluate(&mut |f| match f {
            ContainerFeature::Size(feature) => {
                feature.evaluate_size(container.width, container.height)
            }
            ContainerFeature::Style(style) => style.evaluate(&mut |s| s.evaluate(container)),
            ContainerFeature::ScrollState(_) => Truth::Unknown,
        })
    }
}

impl StyleFeature {
    /// §6.4: a custom property's computed value against the given value
    /// (compared as token text, whitespace collapsed), or that it has
    /// one; a standard property is unknown (rdom, as browsers, queries
    /// custom properties only).
    fn evaluate(&self, container: &QueryContainer<'_>) -> Truth {
        if !self.name.starts_with("--") {
            return Truth::Unknown;
        }
        let actual = (container.custom)(&self.name);
        Truth::from_bool(match (&self.value, actual) {
            (None, actual) => actual.is_some(),
            (Some(want), Some(actual)) => normalize(want) == normalize(&actual),
            (Some(_), None) => false,
        })
    }
}

/// Token text with runs of whitespace collapsed and the ends trimmed.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A `( … )` block or a function as a container query leaf.
fn feature(prelude: &Prelude<'_>, cv: &Cv) -> Option<ContainerFeature> {
    match cv {
        Cv::Paren {
            inner,
            close: Some(_),
            ..
        } => MediaFeature::parse(prelude, inner).map(ContainerFeature::Size),
        Cv::Function {
            at,
            inner,
            close: Some(_),
        } => {
            let Token::Function(name) = prelude.token(*at) else {
                return None;
            };
            if name.eq_ignore_ascii_case("style") {
                // `<style-query>`: a condition over `( <style-feature> )`,
                // or one bare `<style-feature>`.
                let query = prelude
                    .condition(inner, true, &style_feature)
                    .or_else(|| style_declaration(prelude, inner).map(Condition::Leaf))?;
                Some(ContainerFeature::Style(query))
            } else if name.eq_ignore_ascii_case("scroll-state") {
                Some(ContainerFeature::ScrollState(
                    prelude.text_of_all(inner).to_string(),
                ))
            } else {
                None
            }
        }
        Cv::Token(_) => None,
        _ => None,
    }
}

/// `( <style-feature> )`.
fn style_feature(prelude: &Prelude<'_>, cv: &Cv) -> Option<StyleFeature> {
    match cv {
        Cv::Paren {
            inner,
            close: Some(_),
            ..
        } => style_declaration(prelude, inner),
        _ => None,
    }
}

/// `<style-feature>` itself: `name: value` or a bare `name`.
fn style_declaration(prelude: &Prelude<'_>, values: &[Cv]) -> Option<StyleFeature> {
    match values {
        [name] => Some(StyleFeature {
            name: prelude.ident(name)?.to_string(),
            value: None,
        }),
        [name, colon, value @ ..] => {
            let name = prelude.ident(name)?.to_string();
            if !matches!(colon, Cv::Token(i) if *prelude.token(*i) == Token::Colon) {
                return None;
            }
            Some(StyleFeature {
                name,
                value: Some(prelude.text_of_all(value).trim().to_string()),
            })
        }
        _ => None,
    }
}
