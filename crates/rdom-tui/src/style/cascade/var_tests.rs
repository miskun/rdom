//! `var()` in every property (CSS Variables 1 §3): token-level
//! substitution at computed-value time from the element's custom
//! properties, fallbacks with arbitrary tokens and nested `var()`,
//! invalid at computed-value time (`unset`), cycles, shorthands and
//! `content`.

use super::*;
use crate::TuiDom;
use crate::style::{Color, ComputedStyle, Stylesheet};
use rdom_core::NodeId;

const RED: Color = Color::Rgb(255, 0, 0);

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

/// The `<div>`'s computed style under `css`.
fn div(css: &str) -> ComputedStyle {
    let (mut dom, _, div) = tree();
    dom.cascade(&sheet(css));
    computed_of(&dom, div)
}

// ── Substitution (§3) ────────────────────────────────────────────────

/// §3: `var()` substitutes in any property, then the property's own
/// grammar parses the result — here lengths, a shorthand and a
/// keyword.
#[test]
fn var_substitutes_in_any_property() {
    let lit = div("div { width: 10; padding: 1 2; display: flex; z-index: 3 }");
    let var = div("div { --w: 10; --p: 1 2; --d: flex; --z: 3; \
         width: var(--w); padding: var(--p); display: var(--d); z-index: var(--z) }");
    assert_eq!(var.width, lit.width);
    assert_eq!(var.padding, lit.padding);
    assert_eq!(var.display, lit.display);
    assert_eq!(var.z_index, lit.z_index);
}

/// §3: the substitution reads the element's own custom properties,
/// inherited ones included.
#[test]
fn var_reads_inherited_custom_properties() {
    let lit = div("div { width: 10 }");
    assert_eq!(
        div("section { --w: 10 } div { width: var(--w) }").width,
        lit.width
    );
}

/// §3: `var()` may be one token among others, inside a function.
#[test]
fn var_inside_a_function_and_among_tokens() {
    assert_eq!(div("div { --r: 255; color: rgb(var(--r), 0, 0) }").fg, RED);
    let lit = div("div { padding: 1 2 3 4 }");
    assert_eq!(
        div("div { --x: 2 3; padding: 1 var(--x) 4 }").padding,
        lit.padding
    );
}

// ── Fallbacks (§3) ───────────────────────────────────────────────────

/// §3: the fallback is any token sequence (commas included) and may
/// itself hold `var()`.
#[test]
fn fallback_with_arbitrary_tokens_and_nested_var() {
    let lit = div("div { width: 7; padding: 1 2 }");
    let got = div("div { width: var(--missing, 7); padding: var(--missing, 1 2) }");
    assert_eq!(got.width, lit.width);
    assert_eq!(got.padding, lit.padding);
    let lit = div("div { width: 5 }");
    assert_eq!(
        div("div { --b: 5; width: var(--a, var(--b, 4)) }").width,
        lit.width
    );
    assert_eq!(div("div { color: var(--none, rgb(255, 0, 0)) }").fg, RED);
}

// ── Invalid at computed-value time (§3.1) ────────────────────────────

/// §3.1: a substitution that fails (no value, no fallback) or that the
/// property's grammar rejects makes the declaration invalid at
/// computed-value time: the property is `unset` — not the earlier
/// declaration of the same block, which the `var()` one replaced.
#[test]
fn invalid_at_computed_value_time_is_unset() {
    let initial = ComputedStyle::initial();
    let got = div("div { width: 3; width: var(--missing) }");
    assert_eq!(got.width, initial.width);
    let got = div("div { --c: red; width: 3; width: var(--c) }");
    assert_eq!(got.width, initial.width);
    // `color` inherits: `unset` takes the parent's.
    assert_eq!(
        div("section { color: red } div { --w: 3; color: var(--w) }").fg,
        RED
    );
}

// ── Custom properties and cycles (§2.3, §3) ──────────────────────────

/// §3: a custom property's own `var()` resolves where it is declared;
/// descendants inherit the substituted value.
#[test]
fn custom_properties_substitute_where_declared() {
    let lit = div("div { width: 4 }");
    assert_eq!(
        div("section { --a: 4; --b: var(--a) } div { --a: 8; width: var(--b) }").width,
        lit.width
    );
}

/// §2.3: custom properties in a dependency cycle are invalid at
/// computed-value time — even with a fallback; one that only refers to
/// a cycle uses its fallback.
#[test]
fn cycles_make_their_custom_properties_invalid() {
    let lit = div("div { width: 9 }");
    assert_eq!(
        div("div { --a: var(--b, 1); --b: var(--a, 2); width: var(--a, 9) }").width,
        lit.width
    );
    let lit = div("div { width: 3 }");
    assert_eq!(
        div("div { --a: var(--a); --c: var(--a, 3); width: var(--c) }").width,
        lit.width
    );
}

// ── Shorthands, `content`, importance ────────────────────────────────

/// §3: a shorthand with `var()` is substituted as a whole; a later
/// longhand in the same block still overrides its part.
#[test]
fn var_in_shorthands_keeps_declaration_order() {
    let lit = div("div { padding: 1 2; padding-left: 5 }");
    assert_eq!(
        div("div { --p: 1 2; padding: var(--p); padding-left: 5 }").padding,
        lit.padding
    );
    let lit = div("div { padding: 1 2 }");
    assert_eq!(
        div("div { --p: 1 2; padding-left: 5; padding: var(--p) }").padding,
        lit.padding
    );
}

/// `content: var(--x)` from CSS text (Variables 1 §3, Generated
/// Content 3 §1).
#[test]
fn var_in_content() {
    let (mut dom, _, div) = tree();
    dom.cascade(&sheet(
        "div { --label: \"hi\" } div::before { content: var(--label) \"!\" }",
    ));
    let ext = dom.node(div).ext().unwrap();
    let before = ext.computed_before.as_ref().expect("::before");
    assert_eq!(before.content.as_deref(), Some("hi!"));
}

/// An `!important` `var()` declaration keeps its importance.
#[test]
fn important_var_declarations() {
    let lit = div("div { width: 6 }");
    assert_eq!(
        div("div { --w: 6; width: var(--w) !important } div { width: 2 }").width,
        lit.width
    );
}

/// An element whose declarations hold no `var()` is styled from the
/// rule's declarations as parsed; a `var()` declaration is kept as
/// tokens until the cascade (`TuiStyle::pending`).
#[test]
fn declarations_without_var_are_not_pending() {
    let s = sheet("div { width: 3 } p { width: var(--w) }");
    assert!(s.rules()[0].style.pending.is_empty());
    assert!(!s.rules()[1].style.pending.is_empty());
}
