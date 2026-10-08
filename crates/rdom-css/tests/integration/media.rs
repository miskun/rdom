//! C14-MEDIA — `@media` (CSS Conditional 3 §3, Media Queries 4): the
//! parser declares each rule's condition and parses its body under it —
//! at the top level, in `@layer`, nested in a style rule (CSS Nesting 1
//! §3.2) and as an `@import`'s media list (CSS Cascade 5 §3). The queries
//! are evaluated by the backend's cascade, not here.

use rdom_css::parse;
use rdom_style::ConditionKind;

/// The rules inside `@media` record its condition; the rules after it do
/// not; nothing is reported.
#[test]
fn media_rules_record_their_condition() {
    let r = parse(
        "@media screen and (width > 40) { .a { color: red } .b { color: blue } } .c { color: green }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let rules = sheet.rules();
    assert_eq!(rules.len(), 3);
    let c = rules[0].condition.expect("in @media");
    assert_eq!(rules[1].condition, Some(c));
    assert_eq!(rules[2].condition, None);
    let ConditionKind::Media(list) = &sheet.conditions()[c.index()].kind else {
        panic!("a media condition")
    };
    assert_eq!(list.to_string(), "screen and (width > 40)");
}

/// Conditional rules nest: the inner one's parent is the outer.
#[test]
fn media_nests_in_media_and_layers() {
    let r = parse("@media print { @layer x { @media (hover) { .a { color: red } } } }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let rule = &sheet.rules()[0];
    assert!(rule.layer.is_some());
    let inner = rule.condition.expect("conditional");
    let chain: Vec<String> = sheet
        .condition_chain(Some(inner))
        .map(|c| match &c.kind {
            ConditionKind::Media(l) => l.to_string(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(chain, ["(hover)", "print"]);
}

/// CSS Nesting 1 §3.2: nested in a style rule, its declarations are the
/// parent's under the condition, and a declaration after it is a nested
/// declarations rule, unconditional.
#[test]
fn media_nests_in_a_style_rule() {
    let r =
        parse(".a { color: red; @media (width < 9) { color: blue; .b & { width: 1 } } height: 2 }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.rules();
    let summary: Vec<(&str, bool)> = rules
        .iter()
        .map(|r| (r.source_text.as_str(), r.condition.is_some()))
        .collect();
    assert_eq!(
        summary,
        [(".a", false), (".a", true), (".b &", true), (".a", false)]
    );
}

/// A `@media` without a block is invalid and reported; the next rule
/// stands.
#[test]
fn media_without_a_block_is_invalid() {
    let r = parse("@media screen; .a { color: red }");
    assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].condition, None);
}

/// CSS Conditional 3 §2: `@keyframes` and `@counter-style` inside
/// `@media` record the condition — they define their name only while it
/// holds.
#[test]
fn definitions_inside_media_record_the_condition() {
    let r = parse(
        "@media (width > 50) { @keyframes k { to { color: red } } @counter-style c { system: cyclic; symbols: x } }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(r.stylesheet.keyframes()[0].condition.is_some());
    assert!(r.stylesheet.counter_styles()[0].condition.is_some());
}

/// `Stylesheet::append` carries the conditions, remapped.
#[test]
fn append_carries_conditions() {
    let mut base = parse("@media print { .x { color: red } }").stylesheet;
    let other = parse("@media screen { @media (hover) { .a { color: red } } }").stylesheet;
    base.append(&other);
    assert_eq!(base.conditions().len(), 3);
    let last = base.rules().last().unwrap();
    let chain: Vec<_> = base.condition_chain(last.condition).collect();
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[1].parent, None);
}
