//! `Stylesheet` tests: selector-text preprocessing, the builder, the
//! rule index, and cross-builder scenarios.

use super::selector_text::{extract_pseudo_suffix, split_top_level_commas};
use super::*;
use crate::{Color, TuiStyle};

// ── split_top_level_commas ───────────────────────────────────────

#[test]
fn split_empty() {
    assert!(split_top_level_commas("").is_empty());
    assert!(split_top_level_commas("   ").is_empty());
}

#[test]
fn split_no_commas() {
    assert_eq!(split_top_level_commas("div.foo"), vec!["div.foo"]);
}

#[test]
fn split_simple_list() {
    assert_eq!(split_top_level_commas("a, b, c"), vec!["a", " b", " c"]);
}

#[test]
fn split_preserves_not_parens() {
    // :not(a, b) has an internal comma that must not split.
    assert_eq!(
        split_top_level_commas("div:not(a, b), span"),
        vec!["div:not(a, b)", " span"]
    );
}

#[test]
fn split_preserves_attribute_brackets() {
    // Attribute values with quoted commas (rare but valid).
    assert_eq!(
        split_top_level_commas(r#"[data-x="a,b"], .foo"#),
        vec![r#"[data-x="a,b"]"#, " .foo"]
    );
}

// ── extract_pseudo_suffix ────────────────────────────────────────

#[test]
fn extract_no_pseudo() {
    assert_eq!(
        extract_pseudo_suffix("div.foo").unwrap(),
        ("div.foo", PseudoElementTarget::None)
    );
}

#[test]
fn extract_before() {
    assert_eq!(
        extract_pseudo_suffix("tree-item::before").unwrap(),
        ("tree-item", PseudoElementTarget::Before)
    );
}

#[test]
fn extract_after() {
    assert_eq!(
        extract_pseudo_suffix("dialog .close::after").unwrap(),
        ("dialog .close", PseudoElementTarget::After)
    );
}

#[test]
fn extract_tolerates_trailing_whitespace() {
    assert_eq!(
        extract_pseudo_suffix("h1::before   ").unwrap(),
        ("h1", PseudoElementTarget::Before)
    );
}

#[test]
fn extract_rejects_bare_pseudo() {
    assert!(extract_pseudo_suffix("::before").is_err());
    assert!(extract_pseudo_suffix("::after").is_err());
}

#[test]
fn extract_rejects_unsupported_pseudo_element() {
    assert!(extract_pseudo_suffix("p::first-line").is_err());
}

#[test]
fn extract_selection() {
    assert_eq!(
        extract_pseudo_suffix("p::selection").unwrap(),
        ("p", PseudoElementTarget::Selection)
    );
    assert_eq!(
        extract_pseudo_suffix("article .body::selection").unwrap(),
        ("article .body", PseudoElementTarget::Selection)
    );
}

#[test]
fn extract_rejects_bare_selection() {
    assert!(extract_pseudo_suffix("::selection").is_err());
}

#[test]
fn extract_rejects_multiple_pseudo_suffixes() {
    assert!(extract_pseudo_suffix("p::before::after").is_err());
}

#[test]
fn extract_scrollbar() {
    // `::scrollbar-thumb` must split before `::scrollbar` — if
    // the parser stripped `::scrollbar` first the leftover
    // would be `-thumb` (invalid). Order matters.
    assert_eq!(
        extract_pseudo_suffix("*::scrollbar-thumb").unwrap(),
        ("*", PseudoElementTarget::ScrollbarThumb)
    );
    assert_eq!(
        extract_pseudo_suffix("*::scrollbar").unwrap(),
        ("*", PseudoElementTarget::Scrollbar)
    );
    assert_eq!(
        extract_pseudo_suffix(".sidebar::scrollbar-thumb").unwrap(),
        (".sidebar", PseudoElementTarget::ScrollbarThumb)
    );
}

#[test]
fn extract_rejects_bare_scrollbar() {
    assert!(extract_pseudo_suffix("::scrollbar").is_err());
    assert!(extract_pseudo_suffix("::scrollbar-thumb").is_err());
}

// ── Stylesheet builder ───────────────────────────────────────────

/// `CASCADE-INITIAL-ALLOC-1`: for every UA rule (plus a few author
/// shapes), an element carrying the subject's tag / id / classes gets
/// that rule back from the index — the index never hides a match.
#[test]
fn rule_index_never_drops_a_matching_rule() {
    use rdom_core::selectors::SimpleSelector;
    let sheet = Stylesheet::new()
        .rule_unchecked("#hero.big span", TuiStyle::new())
        .rule_unchecked("*:hover", TuiStyle::new())
        .rule_unchecked("[role=tree] > li.leaf", TuiStyle::new())
        .rule_unchecked(":not(.x)", TuiStyle::new())
        .rule_unchecked("DIV.Mixed", TuiStyle::new());
    let index = sheet.rule_index();
    let mut out = Vec::new();
    for (i, rule) in sheet.rules().iter().enumerate() {
        let simples = &rule.selector.0[0].subject.simples;
        let tag = simples.iter().find_map(|s| match s {
            SimpleSelector::Type(t) => Some(t.as_str()),
            _ => None,
        });
        let id = simples.iter().find_map(|s| match s {
            SimpleSelector::Id(v) => Some(v.as_str()),
            _ => None,
        });
        let classes: Vec<&str> = simples
            .iter()
            .filter_map(|s| match s {
                SimpleSelector::Class(c) => Some(c.as_str()),
                _ => None,
            })
            .collect();
        index.candidates(tag, id, classes.iter().copied(), &mut out);
        assert!(
            out.contains(&(i as u32)),
            "rule {i} `{}` missing",
            rule.source_text
        );
        assert!(out.windows(2).all(|w| w[0] < w[1]), "sorted, deduplicated");
    }
    // Case is exact on both sides, like the matcher: a `DIV` rule is a
    // candidate for a `DIV` element and not for a `div` one.
    index.candidates(Some("DIV"), None, ["Mixed"].into_iter(), &mut out);
    assert!(
        out.iter()
            .any(|&i| sheet.rules()[i as usize].source_text == "DIV.Mixed")
    );
    index.candidates(Some("div"), None, ["mixed"].into_iter(), &mut out);
    assert!(
        !out.iter()
            .any(|&i| sheet.rules()[i as usize].source_text == "DIV.Mixed")
    );
    // Nothing keyed leaks onto an element it cannot match.
    index.candidates(Some("zzz"), None, std::iter::empty(), &mut out);
    assert!(out.iter().all(|&i| {
        let simples = &sheet.rules()[i as usize].selector.0[0].subject.simples;
        !simples
            .iter()
            .any(|s| matches!(s, SimpleSelector::Id(_) | SimpleSelector::Class(_)))
    }));
}

#[test]
fn bare_has_no_rules() {
    assert_eq!(Stylesheet::bare().rules().len(), 0);
}

#[test]
fn rule_adds_author_rule() {
    let s = Stylesheet::bare()
        .rule("div.hero", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap();
    assert_eq!(s.rules().len(), 1);
    assert_eq!(s.rules()[0].origin, RuleOrigin::Author);
    assert_eq!(s.rules()[0].source_text, "div.hero");
}

#[test]
fn rule_computes_specificity() {
    let s = Stylesheet::bare()
        .rule("#main", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap()
        .rule("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255)))
        .unwrap();
    assert!(s.rules()[0].specificity > s.rules()[1].specificity);
}

#[test]
fn rule_assigns_monotonic_source_idx() {
    let s = Stylesheet::bare()
        .rule("a", TuiStyle::new())
        .unwrap()
        .rule("b", TuiStyle::new())
        .unwrap()
        .rule("c", TuiStyle::new())
        .unwrap();
    let idxs: Vec<u32> = s.rules().iter().map(|r| r.source_idx).collect();
    assert_eq!(idxs, vec![0, 1, 2]);
}

#[test]
fn rule_expands_selector_list() {
    let s = Stylesheet::bare()
        .rule("a, b, c.foo", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap();
    assert_eq!(s.rules().len(), 3);
    assert_eq!(s.rules()[0].source_text, "a");
    assert_eq!(s.rules()[1].source_text, "b");
    assert_eq!(s.rules()[2].source_text, "c.foo");
    // Each one parses as a single complex selector, so specificities differ.
    // a + b are equal (type 1 each); c.foo is higher.
    assert_eq!(s.rules()[0].specificity, s.rules()[1].specificity);
    assert!(s.rules()[2].specificity > s.rules()[0].specificity);
}

#[test]
fn rule_handles_pseudo_element() {
    let s = Stylesheet::bare()
        .rule(
            "tree-item::before",
            TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
        )
        .unwrap();
    assert_eq!(s.rules()[0].pseudo, PseudoElementTarget::Before);
    // source_text preserves the full selector for debug/devtools —
    // the pseudo suffix has already been handled by `pseudo`.
    assert_eq!(s.rules()[0].source_text, "tree-item::before");
}

#[test]
fn rule_handles_mixed_pseudo_list() {
    let s = Stylesheet::bare()
        .rule(
            "a, b::before, c::after",
            TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
        )
        .unwrap();
    assert_eq!(s.rules().len(), 3);
    assert_eq!(s.rules()[0].pseudo, PseudoElementTarget::None);
    assert_eq!(s.rules()[1].pseudo, PseudoElementTarget::Before);
    assert_eq!(s.rules()[2].pseudo, PseudoElementTarget::After);
}

#[test]
fn rule_pseudo_specificity_adds_type_bump() {
    let s = Stylesheet::bare()
        .rule("a", TuiStyle::new())
        .unwrap()
        .rule("a::before", TuiStyle::new())
        .unwrap();
    // Both share the `a` type count; `a::before` has one more.
    assert!(s.rules()[1].specificity > s.rules()[0].specificity);
}

#[test]
fn rule_rejects_invalid_selector() {
    let err = Stylesheet::bare()
        .rule("div..foo", TuiStyle::new())
        .unwrap_err();
    assert!(err.msg.contains("empty class selector") || err.msg.contains("class"));
    assert_eq!(err.source, "div..foo");
}

#[test]
fn rule_rejects_empty_selector_string() {
    assert!(Stylesheet::bare().rule("", TuiStyle::new()).is_err());
    assert!(Stylesheet::bare().rule("   ", TuiStyle::new()).is_err());
}

#[test]
fn rule_rejects_empty_item_in_list() {
    assert!(Stylesheet::bare().rule("a, , b", TuiStyle::new()).is_err());
}

#[test]
fn rule_unchecked_panics_on_error() {
    let result =
        std::panic::catch_unwind(|| Stylesheet::bare().rule_unchecked("div..foo", TuiStyle::new()));
    assert!(result.is_err());
}

#[test]
fn rule_unchecked_succeeds_on_valid() {
    let s = Stylesheet::bare().rule_unchecked("div", TuiStyle::new());
    assert_eq!(s.rules().len(), 1);
}

#[test]
fn define_var_stores_value() {
    let s = Stylesheet::bare()
        .define_var("accent", "#ff0000")
        .define_var("muted", "#888");
    assert_eq!(s.var("accent"), Some("#ff0000"));
    assert_eq!(s.var("muted"), Some("#888"));
    assert!(s.var("nope").is_none());
}

#[test]
fn define_var_overwrites() {
    let s = Stylesheet::bare().define_var("x", "1").define_var("x", "2");
    assert_eq!(s.var("x"), Some("2"));
}

#[test]
fn define_var_mut_accumulates_in_a_loop() {
    // The borrow-by-value fluent `define_var` forces consumers to
    // either chain inline or `mem::take` the sheet to pass it
    // through. `define_var_mut` takes `&mut self` and returns
    // `&mut Self` so accumulation in a loop Just Works.
    let mut s = Stylesheet::bare();
    for (name, value) in [("a", "#111"), ("b", "#222"), ("c", "#333")] {
        s.define_var_mut(name, value);
    }
    assert_eq!(s.var("a"), Some("#111"));
    assert_eq!(s.var("b"), Some("#222"));
    assert_eq!(s.var("c"), Some("#333"));
}

#[test]
fn define_var_mut_chains_via_returned_ref() {
    let mut s = Stylesheet::bare();
    s.define_var_mut("a", "1")
        .define_var_mut("b", "2")
        .define_var_mut("c", "3");
    assert_eq!(s.var("a"), Some("1"));
    assert_eq!(s.var("b"), Some("2"));
    assert_eq!(s.var("c"), Some("3"));
}

#[test]
fn define_var_mut_overwrites_like_define_var() {
    let mut s = Stylesheet::bare();
    s.define_var_mut("x", "1");
    s.define_var_mut("x", "2");
    assert_eq!(s.var("x"), Some("2"));
}

// ── Cross-builder scenarios ──────────────────────────────────────

#[test]
fn ua_rule_comes_before_author_rule_in_source_order() {
    let s = Stylesheet::new()
        .rule(".my-button", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap();
    // All UA rules precede any author rule. Phase E extended the
    // UA default set, so the author rule lands at the tail.
    let last = s.rules().last().unwrap();
    assert_eq!(last.origin, RuleOrigin::Author);
    for r in s.rules().iter().take(s.rules().len() - 1) {
        assert_eq!(r.origin, RuleOrigin::UserAgent);
    }
}

#[test]
fn complex_selector_preserved_in_rule() {
    let s = Stylesheet::bare()
        .rule("div.a > span", TuiStyle::new())
        .unwrap();
    let complex = &s.rules()[0].selector.0[0];
    // Subject is span; one ancestor (div.a with Child combinator)
    assert_eq!(complex.ancestors.len(), 1);
}

#[test]
fn not_with_internal_comma_not_split() {
    let s = Stylesheet::bare()
        .rule(":not(a, b)", TuiStyle::new())
        .unwrap();
    // One rule (not two).
    assert_eq!(s.rules().len(), 1);
}

#[test]
fn source_idx_continues_across_multiple_rule_calls_with_lists() {
    let s = Stylesheet::bare()
        .rule("a, b", TuiStyle::new()) // idxs 0, 1
        .unwrap()
        .rule("c, d, e", TuiStyle::new()) // idxs 2, 3, 4
        .unwrap();
    let idxs: Vec<u32> = s.rules().iter().map(|r| r.source_idx).collect();
    assert_eq!(idxs, vec![0, 1, 2, 3, 4]);
}

#[test]
fn ua_rule_specificity_is_tiny() {
    let s = Stylesheet::new();
    let ua = &s.rules()[0];
    // [disabled] is 1 class-level, 0 ids, 0 types.
    assert_eq!(ua.specificity.id, 0);
    assert_eq!(ua.specificity.class_attr_pseudo, 1);
    assert_eq!(ua.specificity.type_pseudo_el, 0);
}

#[test]
fn default_is_empty() {
    // Default::default is the `bare` constructor, not `new`.
    assert_eq!(Stylesheet::default().rules().len(), 0);
}

#[test]
fn error_display_shows_position() {
    let err = Stylesheet::bare()
        .rule("div..foo", TuiStyle::new())
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("div..foo"));
}
