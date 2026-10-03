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

// ── Escapes (CSS Syntax 3 §4.3.7) ────────────────────────────────

/// CSS Syntax 3 §4.3.7 / §4.3.11: escapes in an identifier decode
/// everywhere an identifier appears — a property name (`col\6f r` is
/// `color`), a keyword value (`fl\65x` is `flex`).
#[test]
fn escapes_decode_in_property_names_and_keywords() {
    use rdom_tui::layout::Flow;
    use rdom_tui::style::Value;
    use rdom_tui::{Color, TuiColor};
    let r = parse(r"a { col\6f r: r\65 d; display: fl\65x }");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    let s = &r.stylesheet.rules()[0].style;
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(s.flow, Some(Value::Specified(Flow::Flex)));
}

/// §4.3.7 in selectors: `.\31 0` is class `10`, `#a\:b` is id `a:b`,
/// and an escaped `{` / `,` does not end or split the prelude.
#[test]
fn escapes_decode_in_selectors() {
    let mut dom: rdom_core::Dom = rdom_core::Dom::new();
    let el = dom.create_element("p");
    dom.add_class(el, "10").unwrap();
    dom.set_attribute(el, "id", "a:b").unwrap();
    dom.add_class(el, "x{,y").unwrap();
    let root = dom.root();
    dom.append_child(root, el).unwrap();
    let r = parse(r".\31 0 {} #a\:b {} .x\{\,y {}");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    let rules = r.stylesheet.rules();
    assert_eq!(rules.len(), 3, "{rules:?}");
    for rule in rules {
        assert!(dom.matches_list(el, &rule.selector), "{}", rule.source_text);
    }
}
