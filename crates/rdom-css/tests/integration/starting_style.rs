//! C12-STARTING — CSS Transitions 2 §3 `@starting-style`: a rule block
//! whose style rules apply only to an element's *starting style* (its
//! before-change style when it has none), at the top level and nested in
//! a style rule (CSS Nesting 1 §3.2).

use rdom_css::parse;

/// The top-level form: a list of style rules, each a starting-style rule.
#[test]
fn starting_style_rules_parse_at_the_top_level() {
    let r = parse("@starting-style { .a { color: red } .b { color: blue } } .c { color: green }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.rules();
    let starting: Vec<(&str, bool)> = rules
        .iter()
        .map(|r| (r.source_text.as_str(), r.starting_style))
        .collect();
    assert_eq!(starting, [(".a", true), (".b", true), (".c", false)]);
}

/// Nested in a style rule, its declarations apply to the parent's
/// selector, in starting style only.
#[test]
fn starting_style_nests_in_a_style_rule() {
    let r = parse(".a { color: red; @starting-style { color: blue } }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.rules();
    assert_eq!(rules.len(), 2);
    assert!(!rules[0].starting_style);
    assert!(rules[1].starting_style, "the nested block");
    assert!(rules[1].style.fg.is_some());
}

/// A prelude is invalid: `@starting-style` takes none.
#[test]
fn starting_style_takes_no_prelude() {
    let r = parse("@starting-style x { .a { color: red } }");
    assert!(!r.warnings.is_empty());
    assert!(r.stylesheet.rules().is_empty());
}
