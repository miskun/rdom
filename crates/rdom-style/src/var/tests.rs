//! Substitution: values, fallbacks, cycles, limits and why one fails.

use std::collections::HashMap;

use super::*;
use crate::TuiStyle;
use crate::custom_value::probe;
use crate::parse::token::{Token, tokenize};

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
    let cx = SubstitutionContext::new();
    assert_eq!(
        substitute(&toks("var(--a) 4"), &mut look, &cx),
        Ok(toks("1 2 4"))
    );
    assert_eq!(
        substitute(&toks("var(--x, var(--b, 9), 5)"), &mut look, &cx),
        Ok(toks("3, 5"))
    );
    assert_eq!(substitute(&toks("var(--x,)"), &mut look, &cx), Ok(vec![]));
    assert_eq!(
        substitute(&toks("var(--x)"), &mut look, &cx),
        Err(SubstitutionError::Undefined("x".into()))
    );
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
    resolve_custom_properties(
        &mut v,
        ["a", "b", "c", "d", "e"],
        SubstitutionContext::new(),
    );
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
    .substituted(&v, &SubstitutionContext::new());
    let unset = with(&[("width", "unset"), ("transition-duration", "unset")]);
    assert_eq!(got.width, unset.width);
    assert_eq!(
        got.motion.transition_duration,
        unset.motion.transition_duration
    );
    // A dimension inside the custom property substitutes whole.
    let v = vars(&[("w", "1fr")]);
    let got = with(&[("width", "var(--w)")]).substituted(&v, &SubstitutionContext::new());
    assert_eq!(got.width, with(&[("width", "1fr")]).width);
}

/// `C5G-INT-CLAMP-SITE` — CSS Syntax 3 §4.3.12: an integer literal keeps
/// its value in the token; only a property that consumes an `<integer>`
/// clamps it to its range (CSS Values 4 §5.1). A custom property holds
/// `99999999999` as written, and `calc(var(--n) / 1e9)` is 100 (not
/// 2.147…, the `i32::MAX` the tokenizer used to clamp it to).
#[test]
fn a_large_integer_survives_a_custom_property() {
    let s = with(&[("--n", "99999999999")]);
    assert_eq!(
        crate::property_dispatch::serialize("--n", &s).as_deref(),
        Some("99999999999")
    );
    let v = vars(&[("n", "99999999999")]);
    let got =
        with(&[("width", "calc(var(--n) / 1e9)")]).substituted(&v, &SubstitutionContext::new());
    assert_eq!(got.width, with(&[("width", "100")]).width);
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
    let got = with(&[("content", "var(--q)")]).substituted(&v, &SubstitutionContext::new());
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
    resolve_custom_properties(
        &mut v,
        names.iter().map(String::as_str),
        SubstitutionContext::new(),
    );
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
    resolve_custom_properties(
        &mut v,
        names.iter().map(String::as_str),
        SubstitutionContext::new(),
    );
    let style = with(&[
        ("color", "var(--a)"),
        ("padding", "var(--c)"),
        ("margin", "var(--b)"),
    ]);
    probe::take();
    for _ in 0..50 {
        let got = style.substituted(&v, &SubstitutionContext::new());
        assert_eq!(got.fg, with(&[("color", "red")]).fg);
        assert_eq!(got.padding, with(&[("padding", "1 2 3")]).padding);
    }
    assert_eq!(probe::take(), 0, "substitution re-tokenized a value");
}

/// `C2G-SUBSTITUTION-ERRORS` — why a substitution fails is reported, not
/// collapsed into one `None`: a missing `var()` (CSS Variables 1 §3), a
/// cycle (§2.3), the token cap (§3.3), a typed `attr()` without a value
/// (CSS Values 5 §8.7.1) and a malformed function are told apart.
#[test]
fn substitution_errors_say_why() {
    let cx = SubstitutionContext::new();
    let v = vars(&[]);
    let sub = |src: &str| substitute(&toks(src), &mut |n| lookup_in(&v, n), &cx);
    assert_eq!(
        sub("1 var(--gone)"),
        Err(SubstitutionError::Undefined("gone".into()))
    );
    assert_eq!(
        sub("attr(data-w type(<length>))"),
        Err(SubstitutionError::InvalidAttr("data-w".into()))
    );
    assert_eq!(sub("var(--a"), Err(SubstitutionError::Syntax));
    let mut huge = |_: &str| Ok(CustomValue::from_tokens(vec![Token::Number(1); 40_000]));
    assert_eq!(
        substitute(&toks("var(--a) var(--a)"), &mut huge, &cx),
        Err(SubstitutionError::TooLong)
    );

    // Custom-property resolution reports each declared property it made
    // invalid, with why: the cycle's members, and a dependent of the
    // cycle without a fallback.
    let mut v = vars(&[
        ("c", "var(--d)"),
        ("d", "var(--c)"),
        ("e", "var(--c)"),
        ("f", "var(--nope)"),
        ("g", "var(--c, 1)"),
    ]);
    let mut failures = resolve_custom_properties(
        &mut v,
        ["c", "d", "e", "f", "g"],
        SubstitutionContext::new(),
    );
    failures.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        failures,
        vec![
            ("c".to_string(), SubstitutionError::Cycle("c".into())),
            ("d".to_string(), SubstitutionError::Cycle("d".into())),
            ("e".to_string(), SubstitutionError::Cycle("c".into())),
            ("f".to_string(), SubstitutionError::Undefined("nope".into())),
        ]
    );
    assert_eq!(v.get("g").map(CustomValue::as_str), Some("1"));
}
