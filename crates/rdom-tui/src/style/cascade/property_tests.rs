//! Registered custom properties through the cascade (CSS Properties
//! and Values API 1): the initial value, `inherits`, syntax validation
//! at computed-value time, and `Stylesheet::register_property` (the
//! `CSS.registerProperty` of a Rust-built sheet).

use super::*;
use crate::TuiDom;
use crate::style::{Color, ComputedStyle, Stylesheet};
use rdom_core::NodeId;
use rdom_style::PropertyRegistration;

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// `<section><div></div></section>`.
fn tree() -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let div = dom.create_element("div");
    dom.append_child(section, div).unwrap();
    dom.append_child(root, section).unwrap();
    (dom, section, div)
}

fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    parsed.stylesheet
}

fn div_with(sheet: &Stylesheet) -> ComputedStyle {
    let (mut dom, _, div) = tree();
    dom.cascade(sheet);
    computed_of(&dom, div)
}

fn div(css: &str) -> ComputedStyle {
    div_with(&sheet(css))
}

const COLOR_NO_INHERIT: &str =
    "@property --c { syntax: '<color>'; inherits: false; initial-value: red } ";
const COLOR_INHERIT: &str =
    "@property --c { syntax: '<color>'; inherits: true; initial-value: red } ";

/// §2.1: an undeclared registered property has its initial value.
#[test]
fn registered_property_starts_at_its_initial_value() {
    let got = div(&format!("{COLOR_NO_INHERIT} div {{ color: var(--c) }}"));
    assert_eq!(got.fg, RED);
    assert_eq!(got.vars.get("c").map(String::as_str), Some("red"));
}

/// §2.1: `inherits: false` — a child does not inherit the parent's
/// value but starts at the initial value; `inherits: true` inherits.
#[test]
fn inherits_flag() {
    let got = div(&format!(
        "{COLOR_NO_INHERIT} section {{ --c: blue }} div {{ color: var(--c) }}"
    ));
    assert_eq!(got.fg, RED);
    let got = div(&format!(
        "{COLOR_INHERIT} section {{ --c: blue }} div {{ color: var(--c) }}"
    ));
    assert_eq!(got.fg, BLUE);
}

/// §2.4: a value that does not match the syntax (after `var()`
/// substitution) is invalid at computed-value time — the property is
/// `unset`: initial when it does not inherit, the parent's when it
/// does.
#[test]
fn values_are_validated_against_the_syntax() {
    let got = div(&format!(
        "{COLOR_NO_INHERIT} div {{ --c: 12; color: var(--c) }}"
    ));
    assert_eq!(got.fg, RED);
    let got = div(&format!(
        "{COLOR_INHERIT} section {{ --c: blue }} div {{ --x: 3; --c: var(--x); color: var(--c) }}"
    ));
    assert_eq!(got.fg, BLUE);
    let got = div(&format!(
        "{COLOR_NO_INHERIT} div {{ --c: blue; color: var(--c) }}"
    ));
    assert_eq!(got.fg, BLUE);
    // `<length>`: cells validate, colors do not.
    let lit = div("div { width: 4 }");
    let got = div(
        "@property --w { syntax: '<length>'; inherits: false; initial-value: 4 } \
         div { --w: red; width: var(--w) }",
    );
    assert_eq!(got.width, lit.width);
}

/// An unregistered custom property stays untyped.
#[test]
fn unregistered_properties_stay_untyped() {
    let got = div("div { --c: 12; color: var(--c, blue) }");
    assert_eq!(got.vars.get("c").map(String::as_str), Some("12"));
}

/// `initial` on a registered property is its initial value (§2.1).
#[test]
fn initial_keyword_is_the_registered_initial_value() {
    let got = div(&format!(
        "{COLOR_INHERIT} section {{ --c: blue }} div {{ --c: initial; color: var(--c) }}"
    ));
    assert_eq!(got.fg, RED);
}

/// `CSS.registerProperty` for a Rust-built sheet:
/// `Stylesheet::register_property`.
#[test]
fn register_property_in_rust() {
    let mut s = sheet("div { color: var(--c) }");
    s.register_property(PropertyRegistration::new("--c", "<color>", false, Some("blue")).unwrap());
    assert_eq!(div_with(&s).fg, BLUE);
}
