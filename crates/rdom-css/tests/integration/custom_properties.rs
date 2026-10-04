//! §11.6 — Custom properties at :root.
//!
//! `:root { --name: value; }` adds `name` to the stylesheet's
//! VarMap (the same one `Stylesheet::define_var(name, value)`
//! populates). Other rules can then reference it via
//! `var(--name)` and the cascade resolves through the same chain.
//!
//! Only `:root` registers vars; under any other selector the
//! declaration rides on its rule and the cascade scopes it per element.
//! Per-element (cascade-scoped) custom properties are a documented
//! divergence, see `DIVERGENCES.md`.

use rdom_css::parse;

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
    // The reference is kept as tokens (CSS Variables 1 §3); the
    // cascade substitutes it per element.
    let r = parse(":root { --accent: #3d90ce; } button { color: var(--accent); }");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    assert_eq!(r.stylesheet.var("accent"), Some("#3d90ce"));
    let button_rule = r
        .stylesheet
        .rules()
        .iter()
        .find(|x| x.source_text == "button")
        .expect("button rule present");
    assert!(button_rule.style.fg.is_none());
    assert_eq!(button_rule.style.pending[0].name, "color");
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
fn var_with_fallback_color_is_pending() {
    let r = parse("a { color: var(--missing, #ff0000); }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let style = &r.stylesheet.rules()[0].style;
    assert!(style.fg.is_none());
    assert!(style.pending[0].has_var);
}

/// `CSS-VARS-SCOPE-1`: a custom property under any selector stays on
/// the rule (the cascade scopes it per element); nothing warns, and
/// only `:root` publishes through the sheet-level map.
#[test]
fn custom_property_under_any_selector_stays_on_the_rule() {
    let r = parse(".dark { --accent: red; color: blue }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rule = &r.stylesheet.rules()[0];
    assert_eq!(
        rule.style.custom_properties,
        vec![rdom_tui::style::CustomDeclaration {
            name: "accent".into(),
            value: "red".into(),
            important: false,
        }]
    );
    assert!(
        rule.style.fg.is_some(),
        "the rest of the block still applies"
    );
    assert_eq!(
        r.stylesheet.var("accent"),
        None,
        "sheet-level map is :root only"
    );
}

/// `:root` inside a selector list is one rule for every selector; the
/// declaration rides on the rule and the sheet-level map is untouched.
#[test]
fn root_in_a_selector_list_keeps_the_declaration_on_the_rule() {
    let r = parse(":root, body { --accent: red }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(
        r.stylesheet
            .rules()
            .iter()
            .all(|rule| rule.style.custom_property_value("accent") == Some("red"))
    );
}

/// A `style="--x: …"` attribute declares the property on that element.
#[test]
fn inline_custom_property_is_kept() {
    let r = rdom_css::parse_inline("--accent: red !important; color: blue");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(r.style.fg.is_some());
    let d = &r.style.custom_properties[0];
    assert_eq!(
        (d.name.as_str(), d.value.as_str(), d.important),
        ("accent", "red", true)
    );
}
