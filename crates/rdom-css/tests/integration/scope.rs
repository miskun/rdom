//! `@scope` in the parser (CSS Cascade 6 §2.5): the prelude, the scope
//! records, which rules sit in which scope, and invalid preludes. The
//! cascade behavior is tested in rdom-tui's `cascade/scope_tests.rs`.

use rdom_css::{WarningKind, parse};

/// `@scope (start) to (end) { … }` declares a scope holding both
/// boundaries; its rules (and its declarations' rule) sit in it.
#[test]
fn scope_rule_declares_a_scope() {
    let r = parse("@scope (.card) to (.content) { color: red; p { width: 1 } } q { width: 2 }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    assert_eq!(sheet.scopes().len(), 1);
    let scope = &sheet.scopes()[0];
    assert!(scope.start.is_some() && scope.end.is_some() && scope.parent.is_none());
    let rules = sheet.rules();
    assert_eq!(rules.len(), 3);
    assert!(rules[0].scope.is_some() && rules[1].scope.is_some());
    assert_eq!(rules[2].scope, None);
}

/// The boundaries are optional; nested `@scope` records its parent;
/// `@layer` inside `@scope` keeps the scope.
#[test]
fn scope_preludes_and_nesting() {
    let r = parse(
        "@scope { p { width: 1 } } @scope to (.x) { p { width: 1 } } \
         @scope (.a) { @scope (.b) { p { width: 1 } } @layer l { p { width: 2 } } }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let scopes = r.stylesheet.scopes();
    assert_eq!(scopes.len(), 4);
    assert!(scopes[0].start.is_none() && scopes[0].end.is_none());
    assert!(scopes[1].start.is_none() && scopes[1].end.is_some());
    assert!(scopes[3].parent.is_some());
    let last = r.stylesheet.rules().last().unwrap();
    assert!(last.layer.is_some() && last.scope.is_some());
}

/// An invalid prelude drops the rule and its block.
#[test]
fn invalid_scope_prelude_drops_the_rule() {
    for css in [
        "@scope .a { p { width: 1 } } q { width: 2 }",
        "@scope (.a) too (.b) { p { width: 1 } } q { width: 2 }",
        "@scope (%%) { p { width: 1 } } q { width: 2 }",
    ] {
        let r = parse(css);
        assert_eq!(r.warnings.len(), 1, "{css}: {:?}", r.warnings);
        assert!(
            matches!(&r.warnings[0].kind, WarningKind::InvalidAtRulePrelude { name, .. } if name == "scope"),
            "{css}"
        );
        assert_eq!(r.stylesheet.rules().len(), 1, "{css}");
    }
}
