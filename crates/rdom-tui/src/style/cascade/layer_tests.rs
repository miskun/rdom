//! Cascade layers through the cascade (CSS Cascade 5 §6.4): layer
//! order by first declaration, unlayered over layered for normal
//! declarations, the order reversed for `!important`, sublayers below
//! their parent's own rules, `revert-layer`, and one layer order across
//! every sheet of a cascade.

use super::*;
use crate::TuiDom;
use crate::style::{Color, Stylesheet};
use rdom_core::NodeId;

const RED: Color = Color::Rgb(255, 0, 0);
const GREEN: Color = Color::Rgb(0, 128, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// `<p id="x">` under the root.
fn para() -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "id", "x").unwrap();
    dom.append_child(root, p).unwrap();
    (dom, p)
}

/// A UA-less sheet parsed from `css`, which must parse cleanly.
fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    parsed.stylesheet
}

fn fg(css: &str) -> Color {
    let (mut dom, p) = para();
    dom.cascade(&sheet(css));
    computed_of(&dom, p).fg
}

/// §6.4: unlayered normal declarations beat layered ones, whatever
/// the specificity.
#[test]
fn unlayered_beats_layered_for_normal_declarations() {
    assert_eq!(
        fg("@layer base { #x { color: red } } p { color: blue }"),
        BLUE
    );
}

/// §6.4.2: later layers beat earlier ones regardless of specificity;
/// the order is that of first declaration, so a statement fixes it
/// before any block.
#[test]
fn layer_order_is_first_declaration_order() {
    assert_eq!(
        fg("@layer a { #x { color: red } } @layer b { p { color: blue } }"),
        BLUE
    );
    assert_eq!(
        fg("@layer b, a; @layer a { p { color: red } } @layer b { #x { color: blue } }"),
        RED
    );
}

/// §6.4: `!important` reverses the layer order — earlier layers beat
/// later ones, and every layer beats unlayered important declarations.
#[test]
fn important_reverses_the_layer_order() {
    assert_eq!(
        fg("@layer a { p { color: red !important } } \
            @layer b { p { color: blue !important } } \
            #x { color: green !important }"),
        RED
    );
    // Important over normal still holds across layers.
    assert_eq!(
        fg("@layer a { p { color: red !important } } p { color: blue }"),
        RED
    );
}

/// §6.4.3: a layer's own rules beat its sublayers; `a.b` names the
/// sublayer `b` of `a`, the same as a nested block.
#[test]
fn sublayers_rank_below_their_parent() {
    assert_eq!(
        fg("@layer a { @layer b { #x { color: red } } p { color: blue } }"),
        BLUE
    );
    assert_eq!(
        fg("@layer a.b { #x { color: red } } @layer a { p { color: blue } }"),
        BLUE
    );
    // `a.b` comes after an earlier sibling of `a`'s other sublayer.
    assert_eq!(
        fg("@layer a.c { p { color: red } } @layer a.b { p { color: green } }"),
        GREEN
    );
}

/// §6.4.1: each anonymous layer is its own layer, in order.
#[test]
fn anonymous_layers_are_distinct() {
    assert_eq!(
        fg("@layer { #x { color: red } } @layer { p { color: blue } }"),
        BLUE
    );
}

/// Cascade 5 §7.4: `revert-layer` rolls back to the previous layer —
/// the cascade without this layer's declarations; from the first
/// layer it reaches the user-agent origin (here: the inherited
/// `unset` value), and from unlayered rules it reaches the last layer.
#[test]
fn revert_layer_rolls_back_to_the_previous_layer() {
    assert_eq!(
        fg("@layer a { p { color: red } } \
            @layer b { p { color: blue } #x { color: revert-layer } }"),
        RED
    );
    assert_eq!(
        fg("@layer a { p { color: red } } p { color: blue } #x { color: revert-layer }"),
        RED
    );
    assert_eq!(
        fg("@layer a { p { color: red } #x { color: revert-layer } }"),
        crate::style::ComputedStyle::initial().fg
    );
    assert_eq!(
        fg("@layer a { p { color: red !important } } \
            @layer b { p { color: blue !important } } \
            #x { color: green } \
            @layer a { #x { color: revert-layer !important } }"),
        BLUE
    );
}

/// The sheets of one cascade share one layer order, merged by name in
/// sheet order (CSSOM §6.2 orders a document's sheets together; an
/// `App` passes its `<style>` sheets, then its own): `b` is declared
/// first, by the first sheet, so the second sheet's `a` follows it and
/// wins; unlayered rules of either sheet beat both.
#[test]
fn sheets_share_one_layer_order() {
    let first = sheet("@layer b { #x { color: blue } }");
    let second = sheet("@layer a { p { color: red } } @layer b { #x { color: green } }");
    let (mut dom, p) = para();
    dom.cascade_all(&[&first, &second]);
    assert_eq!(computed_of(&dom, p).fg, RED);

    let third = sheet("p { color: blue }");
    dom.cascade_all(&[&third, &second]);
    assert_eq!(computed_of(&dom, p).fg, BLUE);
}

/// `Stylesheet::append` (used by `rdom_css::from_css` and
/// `extend_from_style_tags`) keeps each rule's layer.
#[test]
fn from_css_keeps_layers() {
    let sheet = rdom_css::from_css("@layer a { #x { color: red } } p { color: blue }");
    let (mut dom, p) = para();
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, p).fg, BLUE);
}

// ── Element-attached styles (Cascade 4 §6.1, Cascade 5 §6.1) ─────────

/// `<p id="x" style="…">` cascaded against `css`; its `color`.
fn fg_with_inline(css: &str, inline: &str) -> Color {
    use crate::TuiNodeMutExt;
    let (mut dom, p) = para();
    let parsed = rdom_css::parse_inline(inline);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    dom.node_mut(p).set_inline_style(parsed.style);
    dom.cascade(&sheet(css));
    computed_of(&dom, p).fg
}

/// Cascade 4 §6.1 "Element-Attached Styles": within one origin and
/// importance, the `style` attribute's declarations win over rule
/// declarations — for `!important` too, so an inline `!important`
/// beats an author `!important`.
#[test]
fn inline_important_beats_author_important() {
    assert_eq!(
        fg_with_inline("p { color: red !important }", "color: blue !important"),
        BLUE
    );
    assert_eq!(
        fg_with_inline("#x { color: red !important }", "color: blue !important"),
        BLUE
    );
    // Importance still decides first: author `!important` beats a
    // normal inline declaration.
    assert_eq!(
        fg_with_inline("p { color: red !important }", "color: blue"),
        RED
    );
}

/// Cascade 5 §6.1: the element-attached criterion sorts above cascade
/// layers, so the `style` attribute beats every layer at both
/// importances — even the first layer, which wins among `!important`
/// rules.
#[test]
fn inline_sorts_above_every_layer() {
    assert_eq!(
        fg_with_inline(
            "@layer a { p { color: red !important } } p { color: green !important }",
            "color: blue !important"
        ),
        BLUE
    );
    assert_eq!(
        fg_with_inline(
            "@layer a { #x { color: red } } p { color: green }",
            "color: blue"
        ),
        BLUE
    );
}
