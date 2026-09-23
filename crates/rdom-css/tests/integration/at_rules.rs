//! CSS Syntax 3 §5.4 error recovery at the top level: at-rules are
//! consumed whole (statement form to `;`, block form through a
//! depth-tracked `{…}`) and reported as `UnsupportedAtRule`; a stray
//! `}` is ignored; EOF inside a block closes it and keeps the rule.
//! Before this, `@import …;` and `@media {…}` were read as the *next*
//! rule's selector text and swallowed that rule.

use rdom_css::{WarningKind, parse};

fn selectors(src: &str) -> Vec<String> {
    parse(src)
        .stylesheet
        .rules()
        .iter()
        .map(|r| r.source_text.clone())
        .collect()
}

#[test]
fn statement_at_rule_is_skipped_and_the_next_rule_survives() {
    let r = parse(r#"@import url("x.css"); button { color: red }"#);
    assert_eq!(
        selectors(r#"@import url("x.css"); button { color: red }"#),
        vec!["button"]
    );
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(&w.kind, WarningKind::UnsupportedAtRule(name) if name == "import")),
        "{:?}",
        r.warnings
    );
}

#[test]
fn block_at_rule_with_nested_braces_is_skipped_whole() {
    let src = "@media (min-width: 40) { a { color: red } b { color: blue } } c { color: green }";
    assert_eq!(selectors(src), vec!["c"]);
    let r = parse(src);
    assert_eq!(
        r.warnings
            .iter()
            .filter(|w| matches!(&w.kind, WarningKind::UnsupportedAtRule(n) if n == "media"))
            .count(),
        1
    );
}

#[test]
fn keyframes_block_is_skipped_whole() {
    let src = "@keyframes spin { from { color: red } to { color: blue } } p { color: red }";
    assert_eq!(selectors(src), vec!["p"]);
}

#[test]
fn stray_close_brace_is_ignored() {
    assert_eq!(
        selectors("} a { color: red } } b { color: blue }"),
        vec!["a", "b"]
    );
}

#[test]
fn eof_inside_a_block_keeps_the_rule() {
    let r = parse("a { color: red; b { color: blue");
    // §5.4.7: EOF closes the open block. The rule for `a` is kept (with
    // whatever declarations parsed); nothing panics or aborts silently.
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert_eq!(r.stylesheet.rules()[0].source_text, "a");
}

#[test]
fn brace_inside_attribute_selector_string_does_not_end_the_prelude() {
    assert_eq!(
        selectors(r#"a[title="{"] { color: red } b { color: blue }"#),
        vec![r#"a[title="{"]"#, "b"]
    );
}

#[test]
fn at_rule_warning_carries_the_at_rule_position() {
    let r = parse("a { color: red }\n@import 'x';");
    let w = r
        .warnings
        .iter()
        .find(|w| matches!(&w.kind, WarningKind::UnsupportedAtRule(_)))
        .unwrap();
    assert_eq!((w.line, w.column), (2, 1));
}
