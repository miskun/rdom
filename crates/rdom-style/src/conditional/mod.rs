//! The conditional rules' data model: `@media` (Media Queries 4 / 5),
//! `@supports` (CSS Conditional 3 §6) and `@container` (CSS Conditional 5
//! §6, the container queries of CSS Containment 3).
//!
//! The three share one boolean grammar — `not X`, `X and X …`,
//! `X or X …` over parenthesized leaves, with `<general-enclosed>` for a
//! leaf no grammar knows (Media Queries 4 §3, Conditional 3 §6.1,
//! Conditional 5 §6.1) — evaluated in three-valued (Kleene) logic, where
//! a `<general-enclosed>` or an unknown feature is *unknown*: `not` of
//! unknown is unknown, and a whole condition that ends unknown is false
//! (Media Queries 4 §3.1). [`Condition`] is that tree, generic over its
//! leaf; [`Truth`] the logic.
//!
//! The rules themselves are kept as rule context, as `@layer` and
//! `@scope` are: a [`Stylesheet`](crate::Stylesheet) declares each
//! conditional group rule ([`ConditionRule`](crate::ConditionRule)) and
//! every rule inside one records the innermost it sits under; the
//! backend's cascade evaluates them.

mod media;
mod media_env;
mod media_feature;
mod syntax;

pub use media::{MediaList, MediaQuery, MediaType};
pub use media_env::{Contrast, MediaEnvironment, MediaPreferences, PointerAccuracy};
pub use media_feature::MediaFeature;

/// A three-valued truth (Kleene logic, Media Queries 4 §3.1): what a
/// condition evaluates to when a leaf of it is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Truth {
    True,
    False,
    /// Neither: a `<general-enclosed>`, or a feature with an unknown name
    /// or value.
    Unknown,
}

impl Truth {
    /// `true` → [`Truth::True`], `false` → [`Truth::False`].
    pub fn from_bool(b: bool) -> Self {
        if b { Truth::True } else { Truth::False }
    }

    /// The condition holds: unknown is false (Media Queries 4 §3.1,
    /// CSS Conditional 3 §6.1).
    pub fn holds(self) -> bool {
        self == Truth::True
    }

    /// Kleene negation: unknown stays unknown.
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Self {
        match self {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        }
    }

    /// Kleene conjunction: false wins, then unknown.
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Truth::False, _) | (_, Truth::False) => Truth::False,
            (Truth::True, Truth::True) => Truth::True,
            _ => Truth::Unknown,
        }
    }

    /// Kleene disjunction: true wins, then unknown.
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Truth::True, _) | (_, Truth::True) => Truth::True,
            (Truth::False, Truth::False) => Truth::False,
            _ => Truth::Unknown,
        }
    }
}

/// A boolean condition over leaves `L` — the shared grammar of
/// `<media-condition>`, `<supports-condition>` and `<container-query>`.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition<L> {
    /// A leaf: a media feature, a supports feature, a size or style
    /// query.
    Leaf(L),
    /// `not <in-parens>`.
    Not(Box<Condition<L>>),
    /// `<in-parens> [and <in-parens>]+`.
    And(Vec<Condition<L>>),
    /// `<in-parens> [or <in-parens>]+`.
    Or(Vec<Condition<L>>),
    /// `<general-enclosed>`: a function or parenthesized block no grammar
    /// knows, kept as written. Always unknown.
    Unknown(String),
}

impl<L> Condition<L> {
    /// Evaluate in Kleene logic, each leaf by `leaf`.
    pub fn evaluate(&self, leaf: &mut impl FnMut(&L) -> Truth) -> Truth {
        match self {
            Condition::Leaf(l) => leaf(l),
            Condition::Not(c) => c.evaluate(leaf).not(),
            Condition::And(cs) => cs
                .iter()
                .fold(Truth::True, |acc, c| acc.and(c.evaluate(leaf))),
            Condition::Or(cs) => cs
                .iter()
                .fold(Truth::False, |acc, c| acc.or(c.evaluate(leaf))),
            Condition::Unknown(_) => Truth::Unknown,
        }
    }

    /// Every leaf, depth first.
    pub fn leaves(&self) -> Vec<&L> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves<'a>(&'a self, out: &mut Vec<&'a L>) {
        match self {
            Condition::Leaf(l) => out.push(l),
            Condition::Not(c) => c.collect_leaves(out),
            Condition::And(cs) | Condition::Or(cs) => {
                for c in cs {
                    c.collect_leaves(out);
                }
            }
            Condition::Unknown(_) => {}
        }
    }

    /// Serialize, each leaf by `leaf` (which writes its parentheses).
    pub(crate) fn write(&self, out: &mut String, leaf: &impl Fn(&L, &mut String), top: bool) {
        match self {
            Condition::Leaf(l) => leaf(l, out),
            Condition::Not(c) => {
                if !top {
                    out.push('(');
                }
                out.push_str("not ");
                c.write(out, leaf, false);
                if !top {
                    out.push(')');
                }
            }
            Condition::And(cs) | Condition::Or(cs) => {
                let joiner = if matches!(self, Condition::And(_)) {
                    " and "
                } else {
                    " or "
                };
                if !top {
                    out.push('(');
                }
                for (i, c) in cs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(joiner);
                    }
                    c.write(out, leaf, false);
                }
                if !top {
                    out.push(')');
                }
            }
            Condition::Unknown(text) => out.push_str(text),
        }
    }
}

#[cfg(test)]
mod tests;
