//! §11.1 — Tokenizer tests. Comments, whitespace, identifiers,
//! and the basic rule-recognition shape that depends on them.
//!
//! These tests exercise the public `parse` API but the assertions
//! are tokenizer-shaped (count of rules / warnings; *not* property
//! semantics — those live in `properties.rs`).

use rdom_css::{WarningKind, parse};

#[test]
fn empty_input_produces_no_rules() {
    let r = parse("");
    assert_eq!(r.stylesheet.rules().len(), 0);
    assert!(r.warnings.is_empty());
}

#[test]
fn whitespace_only_produces_no_rules() {
    let r = parse("   \n\t  \r\n");
    assert_eq!(r.stylesheet.rules().len(), 0);
    assert!(r.warnings.is_empty());
}

#[test]
fn comment_only_is_skipped() {
    let r = parse("/* hi */");
    assert_eq!(r.stylesheet.rules().len(), 0);
    assert!(r.warnings.is_empty());
}

#[test]
fn empty_rule_produces_one_rule() {
    let r = parse("button {}");
    assert_eq!(
        r.stylesheet.rules().len(),
        1,
        "rules: {:?}",
        r.stylesheet.rules()
    );
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
}

#[test]
fn two_empty_rules_produce_two() {
    let r = parse("a {} b {}");
    assert_eq!(r.stylesheet.rules().len(), 2);
    assert!(r.warnings.is_empty());
}

#[test]
fn comment_between_rules_is_skipped() {
    let r = parse("a {} /* hi */ b {}");
    assert_eq!(r.stylesheet.rules().len(), 2);
    assert!(r.warnings.is_empty());
}

#[test]
fn unterminated_comment_emits_warning_and_drops_rule() {
    let r = parse("a /* hello");
    assert_eq!(r.stylesheet.rules().len(), 0);
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(r.warnings[0].kind, WarningKind::UnterminatedComment);
}

#[test]
fn whitespace_inside_rule_is_tolerated() {
    let r = parse("  \n\tbutton  \r\n  {  \n  }  ");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert!(r.warnings.is_empty());
}

#[test]
fn comment_inside_selector_is_skipped() {
    let r = parse("button /* nope */ {}");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert!(r.warnings.is_empty());
}
