//! §11.6 — Custom properties at :root.
//!
//! `:root { --name: value; }` adds `name` to the stylesheet's
//! VarMap (the same one `Stylesheet::define_var(name, value)`
//! populates). Other rules can then reference it via
//! `var(--name)` and the cascade resolves through the same chain.
//!
//! Only `:root` registers vars; under any other selector the
//! declaration is dropped with `UnsupportedCustomPropertyScope`.
//! Per-element (cascade-scoped) custom properties are a documented
//! divergence, see `DIVERGENCES.md`.

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

/// Custom properties declared under any selector other than `:root`
/// are not applied (rdom has no per-element custom-property scope yet,
/// see `DIVERGENCES.md`). They must not vanish silently.
#[test]
fn custom_property_outside_root_warns_and_keeps_the_rule() {
    let r = parse(".dark { --accent: red; color: blue }");
    assert_eq!(r.stylesheet.rules().len(), 1, "the rule itself is kept");
    assert_eq!(r.stylesheet.var("accent"), None, "not registered globally");
    assert!(
        r.warnings.iter().any(|w| matches!(
            &w.kind,
            rdom_css::WarningKind::UnsupportedCustomPropertyScope { selector, name }
                if selector == ".dark" && name == "accent"
        )),
        "{:?}",
        r.warnings
    );
}

/// `:root` inside a selector list still counts as the root scope only
/// for the `:root` part — the whole list is one rule, so it warns.
#[test]
fn root_in_a_selector_list_warns() {
    let r = parse(":root, body { --accent: red }");
    assert!(
        r.warnings.iter().any(|w| matches!(
            &w.kind,
            rdom_css::WarningKind::UnsupportedCustomPropertyScope { .. }
        )),
        "{:?}",
        r.warnings
    );
}

/// The `style="…"` attribute has no `:root`; a custom property there is
/// dropped like any other non-root declaration, and it warns the same way.
#[test]
fn inline_custom_property_warns() {
    let r = rdom_css::parse_inline("--accent: red; color: blue");
    assert!(
        r.style.fg.is_some(),
        "the rest of the declaration list applies"
    );
    assert!(
        r.warnings.iter().any(|w| matches!(
            &w.kind,
            rdom_css::WarningKind::UnsupportedCustomPropertyScope { selector, name }
                if selector == "style attribute" && name == "accent"
        )),
        "{:?}",
        r.warnings
    );
}
