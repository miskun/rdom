//! `var()` and `attr()` substitution (CSS Variables 1 §3, CSS Values 5
//! §8.7 — the arbitrary substitution functions).
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
pub use crate::custom_value::CustomValue;
use crate::parse::token::Token;

/// The most tokens one `var()` substitution may produce (CSS Variables
/// 1 §3.3 "safely handling overly-long variables": a UA-defined limit,
/// past which the property is invalid at computed-value time). 65 536
/// tokens is far beyond any real value and stops a doubling chain
/// (`--b: var(--a) var(--a)`, `--c: var(--b) var(--b)`, …) after 16
/// links: a custom property's value is never longer, and each one is
/// resolved once, so a cascade's substitution work is bounded by the
/// number of `var()` references times this limit.
pub const MAX_SUBSTITUTED_TOKENS: usize = 1 << 16;

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
    /// The value contains an arbitrary substitution function: `var()` or
    /// `attr()`.
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

/// Substitute every `var()` in `tokens` (§3): `lookup` gives a custom
/// property's value (name without the dashes), `None` when it is not
/// defined. `None` when a `var()` has neither value nor fallback, or
/// the result would exceed [`MAX_SUBSTITUTED_TOKENS`] (§3.3).
/// Substitution is token-level: a substituted number before an ident
/// stays a number and an ident, never a dimension. An `attr()` reads no
/// element here — see [`substitute_with`].
pub fn substitute(
    tokens: &[Token],
    lookup: &mut dyn FnMut(&str) -> Option<CustomValue>,
) -> Option<Vec<Token>> {
    substitute_with(tokens, lookup, None)
}

/// [`substitute`] for an element: `attrs` gives its attributes, which
/// `attr()` reads (CSS Values 5 §8.7.1); `None` substitutes `attr()` as
/// for an absent attribute. `None` when a `var()` or a typed `attr()`
/// has neither value nor fallback, or the result is too long.
pub fn substitute_with(
    tokens: &[Token],
    lookup: &mut dyn FnMut(&str) -> Option<CustomValue>,
    attrs: Option<crate::attr::AttrLookup<'_>>,
) -> Option<Vec<Token>> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if crate::attr::is_attr(&tokens[i]) {
            let end = matching_paren(tokens, i)?;
            let (head, fallback) = crate::attr::split_args(&tokens[i + 1..end]);
            let head = substitute_with(head, lookup, attrs)?;
            match crate::attr::replace(&head, fallback, attrs) {
                crate::attr::Replacement::Tokens(t) => out.extend(t),
                crate::attr::Replacement::Fallback(f) => {
                    out.extend(substitute_with(f, lookup, attrs)?)
                }
                crate::attr::Replacement::Invalid => return None,
            }
            if out.len() > MAX_SUBSTITUTED_TOKENS {
                return None;
            }
            i = end + 1;
            continue;
        }
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
            Some(value) => out.extend_from_slice(value.tokens()?),
            None => out.extend(substitute_with(fallback?, lookup, attrs)?),
        }
        if out.len() > MAX_SUBSTITUTED_TOKENS {
            return None;
        }
        i = end + 1;
    }
    Some(out)
}

/// Custom property `name` of `vars`, when it is defined: a value whose
/// text does not tokenize substitutes as undefined. A reference-count
/// copy — the value's tokens are not re-made.
pub fn lookup_in(vars: &HashMap<String, CustomValue>, name: &str) -> Option<CustomValue> {
    vars.get(name).filter(|v| v.tokens().is_some()).cloned()
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
    pub fn substituted(&self, vars: &HashMap<String, CustomValue>) -> TuiStyle {
        let mut out = self.clone();
        out.pending.clear();
        self.replay_pending(vars, None, &mut out);
        out
    }

    /// Only the substituted pending declarations of this style, on an
    /// otherwise empty style that keeps this one's `!important` bits:
    /// applying this style and then the result is applying
    /// [`substituted`](Self::substituted) — the pending declarations
    /// come after the rest of the block in source order — without
    /// copying the block. What the cascade uses per element.
    pub fn substituted_pending(&self, vars: &HashMap<String, CustomValue>) -> TuiStyle {
        self.substituted_pending_on(vars, None)
    }

    /// [`substituted_pending`](Self::substituted_pending) for an element
    /// whose attributes `attrs` gives, which `attr()` reads (CSS Values
    /// 5 §8.7).
    pub fn substituted_pending_on(
        &self,
        vars: &HashMap<String, CustomValue>,
        attrs: Option<crate::attr::AttrLookup<'_>>,
    ) -> TuiStyle {
        let mut out = TuiStyle {
            important: self.important,
            ..TuiStyle::default()
        };
        self.replay_pending(vars, attrs, &mut out);
        out
    }

    fn replay_pending(
        &self,
        vars: &HashMap<String, CustomValue>,
        attrs: Option<crate::attr::AttrLookup<'_>>,
        out: &mut TuiStyle,
    ) {
        for decl in &self.pending {
            let parsed = if decl.has_var {
                substitute_with(&decl.value, &mut |n| lookup_in(vars, n), attrs).is_some_and(|t| {
                    crate::property_dispatch::set_parsed(&decl.name, &t, out).is_ok()
                })
            } else {
                crate::property_dispatch::set_parsed(&decl.name, &decl.value, out).is_ok()
            };
            if !parsed {
                crate::property_dispatch::set_unset(&decl.name, out);
            }
        }
    }
}

/// Substitute the custom properties `names` of `vars` in place (§2.3,
/// §3): each one's `var()`s resolve against `vars` — another of `names`
/// resolved first — and one that cannot (no value and no fallback, or
/// a dependency cycle, which takes every property on it) is removed:
/// the guaranteed-invalid value.
pub fn resolve_custom_properties<'n>(
    vars: &mut HashMap<String, CustomValue>,
    names: impl IntoIterator<Item = &'n str>,
) {
    let pending: Vec<String> = names
        .into_iter()
        .filter(|n| vars.get(*n).is_some_and(CustomValue::has_var))
        .map(str::to_string)
        .collect();
    resolve(vars, pending, None, None);
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
    vars: &mut HashMap<String, CustomValue>,
    names: impl IntoIterator<Item = &'n str>,
    computed: &mut dyn FnMut(&str, Option<CustomValue>) -> Option<CustomValue>,
) {
    let pending: Vec<String> = names.into_iter().map(str::to_string).collect();
    resolve(vars, pending, Some(computed), None);
}

/// [`resolve_custom_properties`] / [`resolve_custom_properties_with`]
/// (`computed` optional) for an element whose attributes `attrs` gives:
/// a custom property's `attr()`s read them where it is declared, like
/// its `var()`s (CSS Values 5 §8.7).
pub fn resolve_custom_properties_on<'n>(
    vars: &mut HashMap<String, CustomValue>,
    names: impl IntoIterator<Item = &'n str>,
    computed: Option<ComputedStep<'_>>,
    attrs: Option<crate::attr::AttrLookup<'_>>,
) {
    let pending: Vec<String> = match computed {
        Some(_) => names.into_iter().map(str::to_string).collect(),
        None => names
            .into_iter()
            .filter(|n| vars.get(*n).is_some_and(CustomValue::has_var))
            .map(str::to_string)
            .collect(),
    };
    resolve(vars, pending, computed, attrs);
}

/// A computed-value step ([`resolve_custom_properties_with`]).
pub type ComputedStep<'c> = &'c mut dyn FnMut(&str, Option<CustomValue>) -> Option<CustomValue>;

fn resolve(
    vars: &mut HashMap<String, CustomValue>,
    pending: Vec<String>,
    computed: Option<ComputedStep<'_>>,
    attrs: Option<crate::attr::AttrLookup<'_>>,
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
        attrs,
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

struct Resolver<'c, 'a> {
    /// The names still to substitute.
    pending: HashSet<String>,
    /// Substituted values (`None`: invalid).
    done: HashMap<String, Option<CustomValue>>,
    /// The names being substituted, outermost first.
    stack: Vec<String>,
    /// Names found on a dependency cycle.
    cyclic: HashSet<String>,
    /// The computed-value step, if any
    /// ([`resolve_custom_properties_with`]).
    computed: Option<ComputedStep<'c>>,
    /// The element's attributes, for `attr()`.
    attrs: Option<crate::attr::AttrLookup<'a>>,
}

impl Resolver<'_, '_> {
    fn resolve(&mut self, name: &str, vars: &HashMap<String, CustomValue>) -> Option<CustomValue> {
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
            Some(v) if !v.has_var() => Some(v.clone()),
            v => v.and_then(CustomValue::tokens).and_then(|tokens| {
                let attrs = self.attrs;
                substitute_with(
                    tokens,
                    &mut |n| {
                        if self.pending.contains(n) {
                            self.resolve(n, vars)
                        } else {
                            lookup_in(vars, n)
                        }
                    },
                    attrs,
                )
                // Tokenized here, once: the substitution is the tokens.
                .map(CustomValue::from_tokens)
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
    use crate::custom_value::probe;
    use crate::parse::token::tokenize;

    fn toks(s: &str) -> Vec<Token> {
        tokenize(s).unwrap()
    }

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, CustomValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), CustomValue::new(v)))
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
        assert_eq!(v.get("a").map(CustomValue::as_str), Some("2"));
        assert!(!v.contains_key("c") && !v.contains_key("d"));
        assert_eq!(v.get("e").map(CustomValue::as_str), Some("7"));
    }

    fn with(decls: &[(&str, &str)]) -> TuiStyle {
        let mut s = TuiStyle::new();
        for (name, value) in decls {
            crate::property_dispatch::set(name, value, &mut s).unwrap();
        }
        s
    }

    /// `C1G-VAR-TOKENS` — §3: substitution is token-level, so a number
    /// substituted before an ident stays a number and an ident, never
    /// the dimension `1fr` / `300ms`; the property is invalid at
    /// computed-value time (`unset`).
    #[test]
    fn substitution_never_forms_a_dimension() {
        let v = vars(&[("n", "1"), ("t", "300")]);
        let got = with(&[
            ("width", "var(--n)fr"),
            ("transition-duration", "var(--t)ms"),
        ])
        .substituted(&v);
        let unset = with(&[("width", "unset"), ("transition-duration", "unset")]);
        assert_eq!(got.width, unset.width);
        assert_eq!(got.transition_duration, unset.transition_duration);
        // A dimension inside the custom property substitutes whole.
        let v = vars(&[("w", "1fr")]);
        let got = with(&[("width", "var(--w)")]).substituted(&v);
        assert_eq!(got.width, with(&[("width", "1fr")]).width);
    }

    /// `C1G-VAR-TOKENS` — a custom property's value is kept as text: a
    /// string or ident with escaped characters serializes with its
    /// escapes (CSSOM §2.1), so substituting it gives the same tokens.
    #[test]
    fn strings_and_idents_survive_storage_with_their_escapes() {
        let s = with(&[
            ("--q", r#""say \"hi\"""#),
            ("--i", r"a\:b"),
            ("--d", "2\\66 r"),
        ]);
        let stored = |n: &str| tokenize(s.custom_property_value(n).unwrap()).unwrap();
        assert_eq!(stored("q"), vec![Token::String(r#"say "hi""#.to_string())]);
        assert_eq!(stored("i"), vec![Token::Ident("a:b".to_string())]);
        assert_eq!(stored("d"), toks("2fr"));
        let v = vars(&[("q", s.custom_property_value("q").unwrap())]);
        let got = with(&[("content", "var(--q)")]).substituted(&v);
        assert_eq!(got.content, with(&[("content", r#""say \"hi\"""#)]).content);
    }

    /// `C1G-VAR-TOKENS` — §3.3: a `var()` that expands past
    /// [`MAX_SUBSTITUTED_TOKENS`] is invalid at computed-value time, so
    /// a doubling chain (`--b: var(--a) var(--a)`, …) cannot grow
    /// exponentially; the short links still resolve.
    #[test]
    fn overlong_substitutions_are_invalid() {
        let mut pairs = vec![("p0".to_string(), "x".to_string())];
        for i in 1..40 {
            pairs.push((format!("p{i}"), format!("var(--p{0}) var(--p{0})", i - 1)));
        }
        let mut v: HashMap<String, CustomValue> = pairs
            .into_iter()
            .map(|(k, v)| (k, CustomValue::new(&v)))
            .collect();
        let names: Vec<String> = v.keys().cloned().collect();
        resolve_custom_properties(&mut v, names.iter().map(String::as_str));
        assert_eq!(v.get("p2").map(CustomValue::as_str), Some("x x x x"));
        assert!(v.contains_key("p16"), "2^16 tokens is the limit");
        assert!(!v.contains_key("p17"), "2^17 tokens is over it");
        assert!(!v.contains_key("p39"));
    }

    /// `C1G-VAR-COST` — CSS Variables 1 §2: a custom property's value is
    /// a token sequence. It is tokenized once, when the value is made
    /// (declared or substituted); substituting it into any number of
    /// declarations — one per element in a real cascade — reuses those
    /// tokens.
    #[test]
    fn custom_property_values_are_tokenized_once() {
        let mut v = vars(&[("a", "red"), ("b", "1 2"), ("c", "var(--b) 3")]);
        let names: Vec<String> = v.keys().cloned().collect();
        resolve_custom_properties(&mut v, names.iter().map(String::as_str));
        let style = with(&[
            ("color", "var(--a)"),
            ("padding", "var(--c)"),
            ("margin", "var(--b)"),
        ]);
        probe::take();
        for _ in 0..50 {
            let got = style.substituted(&v);
            assert_eq!(got.fg, with(&[("color", "red")]).fg);
            assert_eq!(got.padding, with(&[("padding", "1 2 3")]).padding);
        }
        assert_eq!(probe::take(), 0, "substitution re-tokenized a value");
    }
}
