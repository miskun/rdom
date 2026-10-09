//! C14G-CONDITIONAL-SPEC (2/2) — CSS Conditional 3 §2: a conditional
//! group rule's contents apply only while its condition holds, for every
//! at-rule inside it: an `@property` registers, and a layer joins the
//! layer order (CSS Cascade 5 §6.4.3), only while the condition is true —
//! but a layer inside an element-sensitive `@container` always counts.

use super::*;

const PROPERTY: &str = "@media (width < 20) { @property --g { syntax: '<length>'; inherits: false; initial-value: 2 } } \
                        #a { margin-left: var(--g) }";

fn margin_left(dom: &TuiDom) -> rdom_tui::layout::MarginValue {
    dom.node(by_id(dom, "a"))
        .computed()
        .unwrap()
        .margin
        .left
        .clone()
}

/// CSS Properties and Values 1 §3 with Conditional 3 §2: an `@property`
/// inside `@media` registers only while the query matches — `--g` is
/// unregistered (no initial value, `var(--g)` invalid, margin 0) on a wide
/// terminal and registered (2) on a narrow one; a flip re-registers.
#[test]
fn a_property_rule_registers_only_while_its_condition_holds() {
    let mut dom = doc(r#"<div id="a">a</div>"#);
    styled(&mut dom, PROPERTY, 30, 3);
    assert_eq!(
        margin_left(&dom),
        rdom_tui::layout::MarginValue::Cells(0),
        "wide"
    );
    styled(&mut dom, PROPERTY, 10, 3);
    assert_eq!(
        margin_left(&dom),
        rdom_tui::layout::MarginValue::Cells(2),
        "narrow"
    );
    styled(&mut dom, PROPERTY, 30, 3);
    assert_eq!(
        margin_left(&dom),
        rdom_tui::layout::MarginValue::Cells(0),
        "wide again"
    );
}

const LAYERS: &str =
    "@layer base { #a { color: rgb(255, 0, 0) } } @layer theme { #a { color: rgb(0, 0, 255) } }";

/// CSS Cascade 5 §6.4.3: "Layers that are defined inside of a conditional
/// group rule do not contribute to the layer order unless the condition is
/// true" — `theme`, first named inside an unmatched `@media`, comes after
/// `base` and wins (it came first and lost); matched, it comes first.
#[test]
fn a_layer_first_named_in_an_unmatched_condition_does_not_take_its_order() {
    let css = format!("@media (width > 50) {{ @layer theme {{}} }} @layer base, theme; {LAYERS}");
    let mut dom = doc(r#"<div id="a">a</div>"#);
    styled(&mut dom, &css, 30, 3);
    assert_eq!(fg(&dom, "a"), BLUE, "unmatched: base, theme");
    styled(&mut dom, &css, 60, 3);
    assert_eq!(fg(&dom, "a"), RED, "matched: theme, base");
}

/// §6.4.3: "unless the conditional group rule can evaluate differently for
/// different elements" — a layer inside `@container` always counts.
#[test]
fn a_layer_in_a_container_rule_always_counts() {
    let css =
        format!("@container (width > 999) {{ @layer theme {{}} }} @layer base, theme; {LAYERS}");
    let mut dom = doc(r#"<div id="a">a</div>"#);
    styled(&mut dom, &css, 30, 3);
    assert_eq!(fg(&dom, "a"), RED, "theme, base");
}

/// §6.4.3 across sheets: the layer order is fixed by the counting
/// declarations of every sheet, in order, before any layer named only
/// under a false condition. Sheet 0 (`<style media="print">`) names `a`;
/// sheet 1 says `@layer b, a`, so `b` < `a` and `a`'s green wins — the
/// inert `a` of sheet 0 took its place ahead of sheet 1's `b`
/// (C15G-LAYER-ORDER).
#[test]
fn a_layer_named_only_in_an_inert_earlier_sheet_takes_no_order() {
    let mut print = sheet("@layer a { #a { color: rgb(255, 0, 0) } }");
    print.set_media(Some(rdom_tui::MediaList::parse("print")));
    let screen = sheet(
        "@layer b, a; @layer a { #a { color: rgb(0, 128, 0) } } @layer b { #a { color: rgb(0, 0, 255) } }",
    );
    let mut dom = doc(r#"<div id="a">a</div>"#);
    dom.set_viewport(Viewport::new(30, 3));
    dom.cascade_all(&[&print, &screen]);
    assert_eq!(fg(&dom, "a"), GREEN, "b < a");
    // The same with the inert layer under `@media (false)`-like condition.
    let earlier = sheet("@media (width > 999) { @layer a { #a { color: rgb(255, 0, 0) } } }");
    dom.cascade_all(&[&earlier, &screen]);
    assert_eq!(fg(&dom, "a"), GREEN, "b < a under an unmatched @media");
}
