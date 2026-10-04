//! Color values at computed-value time (CSS Color 4 / 5): the forms
//! that depend on the element — `currentcolor` — one section per item.

use super::*;
use crate::TuiDom;
use crate::style::{Color, ComputedStyle, Stylesheet};
use rdom_core::NodeId;

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// `<section><div class="x"></div></section>`.
fn tree() -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let div = dom.create_element("div");
    dom.set_attribute(div, "class", "x").unwrap();
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

// ── currentcolor: CSS Color 4 §6.4 ──────────────────────────────

/// §6.4: `currentcolor` is the element's `color`, case-insensitively.
#[test]
fn currentcolor_is_the_elements_color() {
    let c = div("div { color: red; background-color: currentColor; border-color: CURRENTCOLOR }");
    assert_eq!(c.bg, RED);
    assert_eq!(c.border_fg, RED);
}

/// §6.4: it resolves against the element's final `color`, whichever
/// rule declares that — not the `color` cascaded so far.
#[test]
fn currentcolor_uses_the_final_color() {
    let c = div("div { background-color: currentcolor; color: red } .x { color: blue }");
    assert_eq!(c.fg, BLUE);
    assert_eq!(c.bg, BLUE);
    let c = div("div { border-color: initial } .x { color: blue }");
    assert_eq!(c.border_fg, BLUE);
}

/// §6.4: in `color` itself, `currentcolor` is the inherited value.
#[test]
fn currentcolor_in_color_is_inherit() {
    let c = div("section { color: red } div { color: currentcolor }");
    assert_eq!(c.fg, RED);
}

/// Backgrounds 3 §3.1: `border-color` still defaults to the element's
/// color.
#[test]
fn border_color_defaults_to_the_color() {
    let c = div("div { border-style: solid } .x { color: blue }");
    assert_eq!(c.border_fg, BLUE);
}

/// A custom property holding `currentcolor` resolves where it is used
/// (CSS Variables 1 §3: substitution precedes the color's own
/// computation).
#[test]
fn currentcolor_through_var() {
    let c = div(
        ":root, section { --c: currentcolor } div { background-color: var(--c) } .x { color: red }",
    );
    assert_eq!(c.bg, RED);
}

/// The same for a `::before` box, which has its own `color`.
#[test]
fn currentcolor_on_a_pseudo_element() {
    let (mut dom, _, div) = tree();
    dom.cascade(&sheet(
        r#"div::before { content: "x"; background-color: currentcolor } div::before { color: red }"#,
    ));
    let before = dom
        .node(div)
        .ext()
        .unwrap()
        .computed_before
        .clone()
        .unwrap();
    assert_eq!(before.bg, RED);
}

/// §6.4: an inherited property keeps `currentcolor` as specified, so a
/// child resolves it against its own color (`caret-color` resolves at
/// paint, `caret.rs`).
#[test]
fn inherited_caret_color_keeps_currentcolor() {
    use crate::TuiColor;
    use crate::layout::CaretColor;
    let c = div("section { caret-color: currentcolor; color: red } div { color: blue }");
    assert_eq!(c.caret_color, CaretColor::Color(TuiColor::CurrentColor));
}

// ── color-mix(): CSS Color 5 §2 ─────────────────────────────────

/// A mix with `currentcolor` takes the element's final color; one with
/// `var()` is substituted first; in `color`, `currentcolor` is the
/// parent's.
#[test]
fn color_mix_resolves_at_computed_value_time() {
    let c =
        div("div { background-color: color-mix(in srgb, currentcolor, blue) } .x { color: red }");
    assert_eq!(c.bg, Color::Rgb(128, 0, 128));
    let c =
        div("section { --c: red } div { background-color: color-mix(in srgb, var(--c), blue) }");
    assert_eq!(c.bg, Color::Rgb(128, 0, 128));
    let c = div("section { color: red } div { color: color-mix(in srgb, currentcolor, blue) }");
    assert_eq!(c.fg, Color::Rgb(128, 0, 128));
}

// ── Relative colors: CSS Color 5 §4 ─────────────────────────────

/// A relative color from `currentcolor` uses the element's color.
#[test]
fn relative_color_from_currentcolor() {
    let c = div("div { background-color: rgb(from currentcolor r g b / 50%) } .x { color: red }");
    assert_eq!(c.bg, Color::Rgba(255, 0, 0, 128));
}
