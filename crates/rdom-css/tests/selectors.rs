//! §11.2 — Selector integration. The CSS parser passes selector
//! text verbatim to `rdom_core::selectors::parse`. These tests
//! confirm the integration: every selector the existing
//! `Stylesheet::rule` builder accepts also works through
//! `rdom_css::parse`, malformed selectors emit a warning and skip
//! the rule, and other rules in the same input survive.

use rdom_css::{WarningKind, parse};

#[test]
fn type_selector() {
    let r = parse("button {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "button");
}

#[test]
fn class_selector() {
    let r = parse(".primary {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, ".primary");
}

#[test]
fn id_selector() {
    let r = parse("#header {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "#header");
}

#[test]
fn compound_selector_type_plus_class() {
    let r = parse("button.primary {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "button.primary");
}

#[test]
fn descendant_combinator() {
    let r = parse("ul li {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "ul li");
}

#[test]
fn child_combinator() {
    let r = parse("ul > li {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "ul > li");
}

#[test]
fn pseudo_class_focus() {
    let r = parse("button:focus {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "button:focus");
}

#[test]
fn selector_list_expands_to_multiple_rules() {
    // Per Stylesheet::rule: "a, b" expands to two rules with
    // source_text "a" and "b" respectively.
    let r = parse("a, b {}");
    assert_eq!(r.stylesheet.rules().len(), 2);
    let texts: Vec<_> = r
        .stylesheet
        .rules()
        .iter()
        .map(|x| x.source_text.as_str())
        .collect();
    assert_eq!(texts, vec!["a", "b"]);
}

#[test]
fn invalid_selector_emits_warning_and_skips_rule() {
    let r = parse("! {}");
    assert_eq!(r.stylesheet.rules().len(), 0);
    assert_eq!(r.warnings.len(), 1);
    match &r.warnings[0].kind {
        WarningKind::InvalidSelector(s) => assert_eq!(s, "!"),
        other => panic!("expected InvalidSelector, got {other:?}"),
    }
}

#[test]
fn invalid_selector_does_not_drop_other_rules() {
    // The rule with `!` is invalid; the rules around it should
    // still be installed. This is the regression test for the
    // §11.1 follow-up note about Stylesheet::rule's fluent
    // take-by-value signature.
    let r = parse("a {} ! {} b {}");
    let texts: Vec<_> = r
        .stylesheet
        .rules()
        .iter()
        .map(|x| x.source_text.as_str())
        .collect();
    assert_eq!(texts, vec!["a", "b"]);
    assert_eq!(r.warnings.len(), 1);
}
