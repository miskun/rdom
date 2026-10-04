//! `var()` substitution (CSS Variables 1 §3).
//!
//! A declaration whose value contains `var()` cannot be parsed against
//! its property's grammar until the element's custom properties are
//! known, so the declaration parser keeps it as tokens — a
//! [`PendingDeclaration`] on [`TuiStyle::pending`] — and the cascade
//! substitutes it per element ([`TuiStyle::substituted`]) at
//! computed-value time:
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
//! A style without `var()` has an empty `pending` list and costs the
//! cascade nothing.

use std::collections::{HashMap, HashSet};

use crate::TuiStyle;
use crate::parse::token::{Token, tokenize};
use crate::parse::values::render_value;

/// A declaration kept as tokens until the cascade substitutes `var()`.
///
/// Once a block holds a `var()` declaration, every later declaration of
/// the block is recorded here too (with `has_var: false`) so the
/// cascade replays them in source order: `padding: var(--p);
/// padding-left: 1` keeps the later longhand on top.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PendingDeclaration {
    /// The canonical property name.
    pub name: String,
    /// The value's tokens, `!important` stripped.
    pub value: Vec<Token>,
    /// The value contains `var()`.
    pub has_var: bool,
}

impl PendingDeclaration {
    pub fn new(name: &str, value: &[Token], has_var: bool) -> Self {
        PendingDeclaration {
            name: name.to_string(),
            value: value.to_vec(),
            has_var,
        }
    }
}

/// Does `tokens` contain a `var()` function?
pub fn contains_var(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t, Token::Function(f) if f.eq_ignore_ascii_case("var")))
}

/// Is every `var()` in `tokens` syntactically valid — `var(` a custom
/// property name, then nothing or `,` and any fallback, `)`? A
/// declaration with an invalid `var()` is invalid at parse time (§3).
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
        } else {
            i += 1;
        }
    }
    true
}

/// Substitute every `var()` in `tokens` (§3): `lookup` gives a custom
/// property's value tokens (name without the dashes), `None` when it is
/// not defined. `None` when a `var()` has neither value nor fallback.
pub fn substitute(
    tokens: &[Token],
    lookup: &mut dyn FnMut(&str) -> Option<Vec<Token>>,
) -> Option<Vec<Token>> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if !is_var(&tokens[i]) {
            out.push(tokens[i].clone());
            i += 1;
            continue;
        }
        let end = matching_paren(tokens, i)?;
        let (name, fallback) = match &tokens[i + 1..end] {
            [Token::Ident(name)] => (name, None),
            [Token::Ident(name), Token::Comma, fallback @ ..] => (name, Some(fallback)),
            _ => return None,
        };
        let name = name.strip_prefix("--")?;
        match lookup(name) {
            Some(value) => out.extend(value),
            None => out.extend(substitute(fallback?, lookup)?),
        }
        i = end + 1;
    }
    Some(out)
}

/// The value tokens of custom property `name` in `vars`.
pub fn lookup_in(vars: &HashMap<String, String>, name: &str) -> Option<Vec<Token>> {
    tokenize(vars.get(name)?).ok()
}

impl TuiStyle {
    /// Does this style hold declarations waiting for `var()`
    /// substitution?
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// This style with its pending declarations substituted from `vars`
    /// (the element's custom properties) and parsed, in source order. A
    /// declaration invalid at computed-value time sets its property to
    /// `unset` (§3.1).
    pub fn substituted(&self, vars: &HashMap<String, String>) -> TuiStyle {
        let mut out = self.clone();
        out.pending.clear();
        for decl in &self.pending {
            let tokens = if decl.has_var {
                substitute(&decl.value, &mut |n| lookup_in(vars, n))
            } else {
                Some(decl.value.clone())
            };
            let parsed = tokens.is_some_and(|t| {
                crate::property_dispatch::set_parsed(&decl.name, &t, &mut out).is_ok()
            });
            if !parsed {
                crate::property_dispatch::set_unset(&decl.name, &mut out);
            }
        }
        out
    }
}

/// Substitute the custom properties `names` of `vars` in place (§2.3,
/// §3): each one's `var()`s resolve against `vars` — another of `names`
/// resolved first — and one that cannot (no value and no fallback, or
/// a dependency cycle, which takes every property on it) is removed:
/// the guaranteed-invalid value.
pub fn resolve_custom_properties<'n>(
    vars: &mut HashMap<String, String>,
    names: impl IntoIterator<Item = &'n str>,
) {
    let pending: Vec<String> = names
        .into_iter()
        .filter(|n| vars.get(*n).is_some_and(|v| has_var_text(v)))
        .map(str::to_string)
        .collect();
    resolve(vars, pending, None);
}

/// [`resolve_custom_properties`] with a computed-value step: `computed`
/// maps each of `names`' substituted value (`None`: guaranteed-invalid)
/// to its computed value, and a property's dependents substitute that
/// — every one of `names` goes through it, `var()` or not. The cascade
/// passes the registered-property check here (CSS Properties and Values
/// API 1 §2.4: a value not matching the syntax is invalid at
/// computed-value time, and a `var()` reading the property sees the
/// result).
pub fn resolve_custom_properties_with<'n>(
    vars: &mut HashMap<String, String>,
    names: impl IntoIterator<Item = &'n str>,
    computed: &mut dyn FnMut(&str, Option<String>) -> Option<String>,
) {
    let pending: Vec<String> = names.into_iter().map(str::to_string).collect();
    resolve(vars, pending, Some(computed));
}

/// A computed-value step ([`resolve_custom_properties_with`]).
type ComputedStep<'c> = &'c mut dyn FnMut(&str, Option<String>) -> Option<String>;

fn resolve(
    vars: &mut HashMap<String, String>,
    pending: Vec<String>,
    computed: Option<ComputedStep<'_>>,
) {
    if pending.is_empty() {
        return;
    }
    let mut resolver = Resolver {
        pending: pending.iter().cloned().collect(),
        done: HashMap::new(),
        stack: Vec::new(),
        cyclic: HashSet::new(),
        computed,
    };
    for name in &pending {
        resolver.resolve(name, vars);
    }
    for (name, value) in resolver.done {
        match value {
            Some(v) => {
                vars.insert(name, v);
            }
            None => {
                vars.remove(&name);
            }
        }
    }
}

struct Resolver<'c> {
    /// The names still to substitute.
    pending: HashSet<String>,
    /// Substituted values (`None`: invalid).
    done: HashMap<String, Option<String>>,
    /// The names being substituted, outermost first.
    stack: Vec<String>,
    /// Names found on a dependency cycle.
    cyclic: HashSet<String>,
    /// The computed-value step, if any
    /// ([`resolve_custom_properties_with`]).
    computed: Option<ComputedStep<'c>>,
}

impl Resolver<'_> {
    fn resolve(&mut self, name: &str, vars: &HashMap<String, String>) -> Option<String> {
        if let Some(done) = self.done.get(name) {
            return done.clone();
        }
        if let Some(at) = self.stack.iter().position(|n| n == name) {
            // A cycle: every property from `name` up is on it.
            self.cyclic.extend(self.stack[at..].iter().cloned());
            return None;
        }
        self.stack.push(name.to_string());
        let result = match vars.get(name) {
            // No `var()`: the value as declared, untouched.
            Some(v) if !has_var_text(v) => Some(v.clone()),
            v => v.and_then(|v| tokenize(v).ok()).and_then(|tokens| {
                substitute(&tokens, &mut |n| {
                    if self.pending.contains(n) {
                        self.resolve(n, vars).and_then(|v| tokenize(&v).ok())
                    } else {
                        lookup_in(vars, n)
                    }
                })
                .map(|t| render_value(&t))
            }),
        };
        self.stack.pop();
        let value = if self.cyclic.contains(name) {
            None
        } else {
            result
        };
        let value = match self.computed.as_mut() {
            Some(computed) => computed(name, value),
            None => value,
        };
        self.done.insert(name.to_string(), value.clone());
        value
    }
}

fn has_var_text(value: &str) -> bool {
    value.to_ascii_lowercase().contains("var(")
}

fn is_var(token: &Token) -> bool {
    matches!(token, Token::Function(f) if f.eq_ignore_ascii_case("var"))
}

fn is_custom(name: &str) -> bool {
    name.len() > 2 && name.starts_with("--")
}

/// The index of the `)` closing the function or `(` at `open`.
fn matching_paren(tokens: &[Token], open: usize) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Token> {
        tokenize(s).unwrap()
    }

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// §3: a defined property substitutes; an undefined one takes the
    /// fallback (substituted in turn); neither is a failure.
    #[test]
    fn substitution_and_fallbacks() {
        let v = vars(&[("a", "1 2"), ("b", "3")]);
        let mut look = |n: &str| lookup_in(&v, n);
        assert_eq!(
            substitute(&toks("var(--a) 4"), &mut look),
            Some(toks("1 2 4"))
        );
        assert_eq!(
            substitute(&toks("var(--x, var(--b, 9), 5)"), &mut look),
            Some(toks("3, 5"))
        );
        assert_eq!(substitute(&toks("var(--x,)"), &mut look), Some(vec![]));
        assert_eq!(substitute(&toks("var(--x)"), &mut look), None);
    }

    /// §3: `var()` must name a custom property.
    #[test]
    fn var_syntax() {
        assert!(valid_var_syntax(&toks("var(--a)")));
        assert!(valid_var_syntax(&toks("rgb(var(--r), 0, var(--b, 1))")));
        assert!(!valid_var_syntax(&toks("var(a)")));
        assert!(!valid_var_syntax(&toks("var()")));
        assert!(!valid_var_syntax(&toks("var(--a 1)")));
    }

    /// §2.3: custom properties resolve against each other; a cycle
    /// removes every property on it, one that refers to it falls back.
    #[test]
    fn custom_property_resolution_and_cycles() {
        let mut v = vars(&[
            ("a", "var(--b)"),
            ("b", "2"),
            ("c", "var(--d, 1)"),
            ("d", "var(--c)"),
            ("e", "var(--c, 7)"),
        ]);
        resolve_custom_properties(&mut v, ["a", "b", "c", "d", "e"]);
        assert_eq!(v.get("a").map(String::as_str), Some("2"));
        assert!(!v.contains_key("c") && !v.contains_key("d"));
        assert_eq!(v.get("e").map(String::as_str), Some("7"));
    }
}
