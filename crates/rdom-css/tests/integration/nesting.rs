//! CSS Nesting 1 in the parser: which block items are declarations and
//! which are nested rules, the order of the rules a block produces, and
//! error recovery inside a block. The cascade behavior (`&`,
//! specificity) is tested in rdom-tui's `cascade/nesting_tests.rs`.

use rdom_css::{WarningKind, parse};

fn texts(css: &str) -> Vec<String> {
    let r = parse(css);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    r.stylesheet
        .rules()
        .iter()
        .map(|r| r.source_text.clone())
        .collect()
}

/// Nesting 1 §3.2: the rule's own declarations, then each nested rule,
/// then each later run of declarations as a nested declarations rule
/// with the parent's selector.
#[test]
fn a_block_yields_rules_in_order_of_appearance() {
    assert_eq!(
        texts(".a { color: red; .b { color: blue } width: 3; > .c { width: 2 } }"),
        vec![".a", ".b", ".a", "> .c"]
    );
}

/// A rule with only nested rules still has its own (empty) rule first.
#[test]
fn the_parent_rule_comes_first_even_when_empty() {
    assert_eq!(texts(".a { .b { color: blue } }"), vec![".a", ".b"]);
}

/// CSS Syntax 3 "consume a block's contents": `<ident>:` followed by a
/// `{}` block before the `;` is a nested rule (`p:hover { … }`); a
/// custom property may hold a block and stays a declaration.
#[test]
fn ident_colon_items_are_rules_when_they_hold_a_block() {
    let r = parse(".a { p:hover { color: red } --x: { a: b }; }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.rules();
    // `.a` (empty), `p:hover`, then `.a` holding the custom property.
    assert_eq!(rules.len(), 3);
    assert_eq!(rules[1].source_text, "p:hover");
    assert_eq!(rules[2].style.custom_properties.len(), 1);
}

/// A malformed item does not end the run of declarations: `width`
/// after it is still the rule's own.
#[test]
fn a_malformed_item_keeps_the_declaration_run() {
    let r = parse(".a { color red; width: 5 }");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert!(r.stylesheet.rules()[0].style.width.is_some());
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(&w.kind, WarningKind::MalformedDeclaration(t) if t == "color red")),
        "{:?}",
        r.warnings
    );
}

/// An unsupported nested at-rule is reported and skipped whole; the
/// block carries on after it.
#[test]
fn an_unsupported_nested_at_rule_is_skipped() {
    let r = parse(".a { @font-face { src: x } color: red; @charset \"x\"; .b { width: 1 } }");
    assert_eq!(r.warnings.len(), 2, "{:?}", r.warnings);
    assert!(
        r.warnings
            .iter()
            .all(|w| matches!(w.kind, WarningKind::UnsupportedAtRule(_)))
    );
    let rules = r.stylesheet.rules();
    assert_eq!(rules.len(), 2);
    assert!(rules[0].style.fg.is_some());
}

/// Nesting 1 §3.2: a nested `@layer` puts its declarations (a nested
/// declarations rule with the parent's selector) and rules in the
/// layer, nested under the enclosing layer.
#[test]
fn nested_layer_rules_sit_in_the_layer() {
    let r = parse("@layer outer { .a { @layer inner { color: red; .b { width: 1 } } } }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let rules = sheet.rules();
    assert_eq!(rules.len(), 3);
    let outer = rules[0].layer.expect("outer");
    let inner = rules[1].layer.expect("inner");
    assert_eq!(rules[2].layer, Some(inner));
    assert_eq!(sheet.layers()[inner.index()].parent, Some(outer));
    assert_eq!(rules[1].source_text, ".a");
}

/// A nested selector that does not parse drops that rule and its block
/// only.
#[test]
fn an_invalid_nested_selector_drops_its_rule() {
    let r = parse(".a { color: red; .b!! { width: 1 } .c { width: 2 } }");
    assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
    assert!(matches!(
        r.warnings[0].kind,
        WarningKind::InvalidSelector(_)
    ));
    let texts: Vec<_> = r
        .stylesheet
        .rules()
        .iter()
        .map(|r| r.source_text.as_str())
        .collect();
    assert_eq!(texts, vec![".a", ".c"]);
}
