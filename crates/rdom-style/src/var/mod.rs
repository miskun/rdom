//! `var()` and `attr()` substitution (CSS Variables 1 §3, CSS Values 5
//! §8.7 — the arbitrary substitution functions).
//!
//! A declaration whose value contains `var()` cannot be parsed against
//! its property's grammar until the element's custom properties are
//! known, so the declaration parser keeps it as tokens — a
//! [`PendingDeclaration`] on [`TuiStyle::pending`](crate::TuiStyle::pending)
//! — and the cascade substitutes it per element
//! ([`TuiStyle::substituted_pending`](crate::TuiStyle::substituted_pending))
//! at computed-value time:
//!
//! - each `var(--name)` is replaced by the custom property's value, or
//!   by the fallback (any token sequence, itself substituted) when the
//!   property has none;
//! - the result is parsed with the property's normal grammar; a failure
//!   anywhere makes the declaration *invalid at computed-value time*,
//!   and the property is `unset`.
//!
//! Custom properties are substituted where they are declared
//! ([`resolve_custom_properties`]), so descendants inherit the
//! substituted value; a dependency cycle makes every custom property in
//! it guaranteed-invalid (§2.3), i.e. not defined.
//!
//! What a substitution reads besides the custom properties — the
//! element's attributes, a computed-value step — is one
//! [`SubstitutionContext`]; why one fails is a [`SubstitutionError`].
//!
//! A style without `var()` has an empty `pending` list and costs the
//! cascade nothing.
//!
//! Module layout: this file holds the substitution itself; `pending`
//! the declarations a style keeps for it; `resolve` custom-property
//! resolution.

mod pending;
mod resolve;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use crate::custom_value::CustomValue;
use crate::parse::token::Token;
pub use pending::PendingDeclaration;
pub use resolve::{ComputedStep, resolve_custom_properties};

/// The most tokens one `var()` substitution may produce (CSS Variables
/// 1 §3.3 "safely handling overly-long variables": a UA-defined limit,
/// past which the property is invalid at computed-value time). 65 536
/// tokens is far beyond any real value and stops a doubling chain
/// (`--b: var(--a) var(--a)`, `--c: var(--b) var(--b)`, …) after 16
/// links: a custom property's value is never longer, and each one is
/// resolved once, so a cascade's substitution work is bounded by the
/// number of `var()` references times this limit.
pub const MAX_SUBSTITUTED_TOKENS: usize = 1 << 16;

/// Why a substitution failed — the declaration (or custom property)
/// holding it is invalid at computed-value time (CSS Variables 1 §3,
/// CSS Values 5 §8.7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubstitutionError {
    /// `var(--name)` with no fallback names a custom property that has
    /// no value: never declared, `initial`, or itself invalid. The name
    /// is without the dashes.
    Undefined(String),
    /// `var(--name)` with no fallback names a custom property on a
    /// dependency cycle (§2.3), which makes it guaranteed-invalid.
    Cycle(String),
    /// A typed `attr(name …)` with no fallback, whose attribute is
    /// missing or does not parse as its type.
    InvalidAttr(String),
    /// The result would exceed [`MAX_SUBSTITUTED_TOKENS`] (§3.3).
    TooLong,
    /// A substitution function that does not parse: unbalanced, or a
    /// `var()` without a custom property name.
    Syntax,
}

impl std::fmt::Display for SubstitutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubstitutionError::Undefined(n) => write!(f, "`--{n}` has no value and no fallback"),
            SubstitutionError::Cycle(n) => write!(f, "`--{n}` is on a dependency cycle"),
            SubstitutionError::InvalidAttr(n) => {
                write!(
                    f,
                    "attribute `{n}` is missing or not of its type, and no fallback"
                )
            }
            SubstitutionError::TooLong => write!(
                f,
                "the substitution exceeds {MAX_SUBSTITUTED_TOKENS} tokens"
            ),
            SubstitutionError::Syntax => f.write_str("a substitution function does not parse"),
        }
    }
}

impl std::error::Error for SubstitutionError {}

/// What a substitution reads besides the custom properties: the
/// element's attributes, which `attr()` reads (CSS Values 5 §8.7.1;
/// `None`: no element, every attribute absent), and — for
/// [`resolve_custom_properties`] — a computed-value step each resolved
/// custom property goes through. One argument for every backend hook, so
/// an input added later (a container's size, a line height) is a field,
/// not another `_with` / `_on` variant.
///
/// Two lifetimes: the attributes are borrowed from the element (`'a`),
/// the computed-value step is usually a closure local to the caller
/// (`'c`), and an [`AttrLookup`](crate::attr::AttrLookup) cannot be
/// shortened to the closure's lifetime.
#[derive(Default)]
#[non_exhaustive]
pub struct SubstitutionContext<'a, 'c> {
    /// The element's attributes.
    pub attrs: Option<crate::attr::AttrLookup<'a>>,
    /// The computed-value step ([`ComputedStep`]); only
    /// [`resolve_custom_properties`] reads it.
    pub computed: Option<ComputedStep<'c>>,
}

impl<'a, 'c> SubstitutionContext<'a, 'c> {
    /// No element, no computed-value step.
    pub fn new() -> Self {
        Self::default()
    }

    /// This context for the element whose attributes `attrs` gives.
    pub fn with_attrs(mut self, attrs: crate::attr::AttrLookup<'a>) -> Self {
        self.attrs = Some(attrs);
        self
    }

    /// This context with a computed-value step for custom properties.
    pub fn with_computed(mut self, computed: ComputedStep<'c>) -> Self {
        self.computed = Some(computed);
        self
    }
}

/// Does `tokens` contain a `var()` function?
pub fn contains_var(tokens: &[Token]) -> bool {
    tokens.iter().any(is_var)
}

/// Does `tokens` contain an arbitrary substitution function — `var()` or
/// `attr()` (CSS Values 5 §8.7)? Such a declaration is substituted at
/// computed-value time.
pub fn contains_substitution(tokens: &[Token]) -> bool {
    tokens.iter().any(|t| is_var(t) || crate::attr::is_attr(t))
}

/// Is every `var()` and `attr()` in `tokens` syntactically valid —
/// `var(` a custom property name, then nothing or `,` and any fallback,
/// `)`; `attr(` a non-empty first argument, then nothing or `,` and any
/// fallback, `)` (CSS Values 5 §8.7)? A declaration with an invalid one
/// is invalid at parse time (§3).
pub fn valid_var_syntax(tokens: &[Token]) -> bool {
    let mut i = 0;
    while i < tokens.len() {
        if is_var(&tokens[i]) {
            let Some(end) = matching_paren(tokens, i) else {
                return false;
            };
            let args = &tokens[i + 1..end];
            match args {
                [Token::Ident(name)] if is_custom(name) => {}
                [Token::Ident(name), Token::Comma, fallback @ ..] if is_custom(name) => {
                    if !valid_var_syntax(fallback) {
                        return false;
                    }
                }
                _ => return false,
            }
            i = end + 1;
        } else if crate::attr::is_attr(&tokens[i]) {
            let Some(end) = matching_paren(tokens, i) else {
                return false;
            };
            let args = &tokens[i + 1..end];
            if !crate::attr::valid_args(args) || !valid_var_syntax(args) {
                return false;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    true
}

/// A custom property's value for `var()` (name without the dashes), or
/// why it has none.
pub type Lookup<'l> = &'l mut dyn FnMut(&str) -> Result<CustomValue, SubstitutionError>;

/// Substitute every `var()` and `attr()` in `tokens` (CSS Variables 1
/// §3, CSS Values 5 §8.7.1): `lookup` gives a custom property's value
/// ([`lookup_in`] reads a map), `cx` the element's attributes. A
/// `var()` whose property has no value takes its fallback; without one,
/// the lookup's error is the result. Substitution is token-level: a
/// substituted number before an ident stays a number and an ident,
/// never a dimension.
pub fn substitute(
    tokens: &[Token],
    lookup: Lookup<'_>,
    cx: &SubstitutionContext<'_, '_>,
) -> Result<Vec<Token>, SubstitutionError> {
    substitute_at(tokens, 0, None, lookup, cx.attrs)
}

/// [`substitute`] over `tokens`, which start at index `base` of a stored
/// declaration whose `attr()` heads `heads` holds parsed (`None`: parse
/// each head as it is met).
pub(crate) fn substitute_at(
    tokens: &[Token],
    base: usize,
    heads: Option<&crate::attr::AttrHeads>,
    lookup: Lookup<'_>,
    attrs: Option<crate::attr::AttrLookup<'_>>,
) -> Result<Vec<Token>, SubstitutionError> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if crate::attr::is_attr(&tokens[i]) {
            let end = matching_paren(tokens, i).ok_or(SubstitutionError::Syntax)?;
            let (head, fallback) = crate::attr::split_args(&tokens[i + 1..end]);
            // `attr(` head `,` fallback: where the fallback starts.
            let fallback_base = base + i + 1 + head.len() + 1;
            // A head parsed in advance holds no substitution function.
            let parsed = heads.and_then(|h| h.get(base + i));
            let substituted;
            let head = match parsed {
                Some(_) => head,
                None => {
                    substituted = substitute_at(head, base + i + 1, heads, lookup, attrs)?;
                    &substituted
                }
            };
            match crate::attr::replace(head, parsed, fallback, attrs) {
                crate::attr::Replacement::Tokens(t) => out.extend(t),
                crate::attr::Replacement::Fallback(f) => {
                    out.extend(substitute_at(f, fallback_base, heads, lookup, attrs)?)
                }
                crate::attr::Replacement::Invalid(name) => {
                    return Err(SubstitutionError::InvalidAttr(name));
                }
            }
            if out.len() > MAX_SUBSTITUTED_TOKENS {
                return Err(SubstitutionError::TooLong);
            }
            i = end + 1;
            continue;
        }
        if !is_var(&tokens[i]) {
            out.push(tokens[i].clone());
            i += 1;
            continue;
        }
        let end = matching_paren(tokens, i).ok_or(SubstitutionError::Syntax)?;
        let (name, fallback) = match &tokens[i + 1..end] {
            [Token::Ident(name)] => (name, None),
            [Token::Ident(name), Token::Comma, fallback @ ..] => (name, Some(fallback)),
            _ => return Err(SubstitutionError::Syntax),
        };
        let name = name.strip_prefix("--").ok_or(SubstitutionError::Syntax)?;
        match (lookup(name), fallback) {
            (Ok(value), _) => match value.tokens() {
                Some(t) => out.extend_from_slice(t),
                None => return Err(SubstitutionError::Undefined(name.to_string())),
            },
            // `var(` `--name` `,` fallback: the fallback is at `i + 3`.
            (Err(_), Some(fallback)) => {
                out.extend(substitute_at(fallback, base + i + 3, heads, lookup, attrs)?)
            }
            (Err(e), None) => return Err(e),
        }
        if out.len() > MAX_SUBSTITUTED_TOKENS {
            return Err(SubstitutionError::TooLong);
        }
        i = end + 1;
    }
    Ok(out)
}

/// Custom property `name` of `vars`, when it is defined: a value whose
/// text does not tokenize substitutes as undefined. A reference-count
/// copy — the value's tokens are not re-made.
pub fn lookup_in(
    vars: &HashMap<String, CustomValue>,
    name: &str,
) -> Result<CustomValue, SubstitutionError> {
    vars.get(name)
        .filter(|v| v.tokens().is_some())
        .cloned()
        .ok_or_else(|| SubstitutionError::Undefined(name.to_string()))
}

fn is_var(token: &Token) -> bool {
    matches!(token, Token::Function(f) if f.eq_ignore_ascii_case("var"))
}

fn is_custom(name: &str) -> bool {
    name.len() > 2 && name.starts_with("--")
}

/// The index of the `)` closing the function or `(` at `open`.
pub(crate) fn matching_paren(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate().skip(open) {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
