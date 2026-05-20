//! §11.6 — Custom properties at :root.
//!
//! `:root { --name: value; }` adds `name` to the stylesheet's
//! VarMap (the same one `Stylesheet::define_var(name, value)`
//! populates). Other rules can then reference it via
//! `var(--name)` and the cascade resolves through the same chain.
//!
//! M1 simplification: any `--name: value` declaration anywhere
//! (not just `:root`) registers a global var. Full cascade-
//! scoped custom properties are deferred to M5+.

use rdom_css::parse;
use rdom_tui::style::Value;
use rdom_tui::{Color, TuiColor};

#[test]
fn root_defines_single_var() {
    let r = parse(":root { --accent: #3d90ce; }");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    assert_eq!(r.stylesheet.var("accent"), Some("#3d90ce"));
}

#[test]
fn root_defines_multiple_vars() {
    let r = parse(":root { --accent: #3d90ce; --dim: #707070; }");
    assert!(r.warnings.is_empty());
    assert_eq!(r.stylesheet.var("accent"), Some("#3d90ce"));
    assert_eq!(r.stylesheet.var("dim"), Some("#707070"));
}

#[test]
fn root_var_with_named_color() {
    let r = parse(":root { --bg: red; }");
    assert!(r.warnings.is_empty());
    assert_eq!(r.stylesheet.var("bg"), Some("red"));
}

#[test]
fn var_reference_after_root_definition() {
    // Declaration order: vars first, then a rule that references
    // them. The reference is parsed as TuiColor::Var; the cascade
    // resolves it at compute time using the var map populated by
    // the :root rule.
    let r = parse(":root { --accent: #3d90ce; } button { color: var(--accent); }");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    assert_eq!(r.stylesheet.var("accent"), Some("#3d90ce"));
    // The button rule references the var:
    let button_rule = r
        .stylesheet
        .rules()
        .iter()
        .find(|x| x.source_text == "button")
        .expect("button rule present");
    let v = button_rule.style.fg.clone().expect("fg set");
    match v {
        Value::Specified(TuiColor::Var { name, .. }) => assert_eq!(name, "accent"),
        other => panic!("expected Var, got {other:?}"),
    }
}

#[test]
fn later_root_rule_overrides_earlier_var() {
    // Last-wins (matches CSS cascade for declarations on the
    // same selector).
    let r = parse(":root { --accent: #aaa; } :root { --accent: #bbb; }");
    assert!(r.warnings.is_empty());
    assert_eq!(r.stylesheet.var("accent"), Some("#bbb"));
}

#[test]
fn root_ignores_unknown_property() {
    // `:root { background: red; }` — `background` (without -color)
    // is not in the M1 property table; emits UnknownProperty.
    // Custom property still registers.
    let r = parse(":root { background: red; --accent: blue; }");
    assert_eq!(r.stylesheet.var("accent"), Some("blue"));
}

#[test]
fn var_resolves_var_with_fallback_color() {
    // The fallback chain is built by the parser; resolution
    // happens at cascade time. Just confirm the parser builds
    // the right structure.
    let r = parse("a { color: var(--missing, #ff0000); }");
    let v = r.stylesheet.rules()[0].style.fg.clone().expect("fg");
    match v {
        Value::Specified(TuiColor::Var { name, fallback }) => {
            assert_eq!(name, "missing");
            assert_eq!(
                fallback.as_deref(),
                Some(&TuiColor::Literal(Color::Rgb(0xff, 0, 0)))
            );
        }
        other => panic!("expected Var, got {other:?}"),
    }
}
