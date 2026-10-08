//! C14-SUPPORTS — `@supports` (CSS Conditional 3 §6): the parser
//! declares the condition — evaluated once, against what rdom parses —
//! and parses the body under it, at the top level, nested in a style rule
//! and on `@import`.

use rdom_css::{WarningKind, parse};
use rdom_style::ConditionKind;

/// The rules inside record the condition; its text and result are kept.
#[test]
fn supports_rules_record_their_condition() {
    let r = parse("@supports (display: grid) { .a { color: red } } .b { color: blue }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let c = sheet.rules()[0].condition.expect("in @supports");
    assert_eq!(sheet.rules()[1].condition, None);
    let ConditionKind::Supports(cond) = &sheet.conditions()[c.index()].kind else {
        panic!("a supports condition")
    };
    assert_eq!(cond.to_string(), "(display: grid)");
    assert!(cond.matches());
}

/// Nested in a style rule (CSS Nesting 1 §3.2) and in `@media`.
#[test]
fn supports_nests() {
    let r = parse(
        ".a { @supports (display: grid) { color: red } } @media screen { @supports not (x: y) { .b { color: red } } }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.rules();
    assert_eq!(rules.len(), 3);
    assert!(rules[1].condition.is_some(), "the nested block");
    let chain: Vec<_> = r.stylesheet.condition_chain(rules[2].condition).collect();
    assert_eq!(chain.len(), 2);
}

/// A prelude that is not a `<supports-condition>` makes the rule invalid:
/// reported, its block skipped.
#[test]
fn an_invalid_prelude_drops_the_rule() {
    let r = parse("@supports display: grid { .a { color: red } } .b { color: blue }");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert!(matches!(
        &r.warnings[0].kind,
        WarningKind::InvalidAtRulePrelude { name, .. } if name == "supports"
    ));
}

/// CSS Cascade 5 §3: `supports()` on an `@import` conditions the imported
/// rules.
#[test]
fn an_import_supports_condition_conditions_its_rules() {
    let loader = |_: &str| -> Result<String, String> { Ok(".a { color: red }".to_string()) };
    let r = rdom_css::parse_with_loader("@import 'x.css' supports(display: frob);", &loader);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let c = sheet.rules()[0].condition.expect("conditional");
    let ConditionKind::Supports(cond) = &sheet.conditions()[c.index()].kind else {
        panic!("a supports condition")
    };
    assert!(!cond.matches());
}
