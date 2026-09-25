//! Read side: getters, `length` / `item`, and the build-script
//! camelCase aliases.

use super::dom_with;
use crate::{TuiAccessors, TuiAccessorsMut, TuiDom};

// ── Read side ────────────────────────────────────────────────

#[test]
fn get_property_value_returns_empty_when_unset() {
    let (dom, div) = dom_with("div");
    let style = dom.node(div).style().expect("element has style");
    assert_eq!(style.get_property_value("color"), "");
}

#[test]
fn style_returns_none_for_non_element() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let text = dom.create_text_node("hi");
    dom.append_child(root, text).unwrap();
    // Text nodes have no ext → no style accessor.
    assert!(dom.node(text).style().is_none());
}

#[test]
fn get_property_priority_empty_by_default() {
    let (dom, div) = dom_with("div");
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.get_property_priority("color"), "");
}

#[test]
fn length_zero_on_fresh_element() {
    let (dom, div) = dom_with("div");
    assert_eq!(dom.node(div).style().unwrap().length(), 0);
}

#[test]
fn css_text_empty_on_fresh_element() {
    let (dom, div) = dom_with("div");
    assert_eq!(dom.node(div).style().unwrap().css_text(), "");
}

#[test]
fn item_out_of_range_is_none() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).style().unwrap().item(0).is_none());
}

// ── length / item ────────────────────────────────────────────

#[test]
fn length_counts_set_properties() {
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        let mut sd = nm.style_mut().unwrap();
        sd.set_property("color", "red").unwrap();
        sd.set_property("gap", "2").unwrap();
    }
    assert_eq!(dom.node(div).style().unwrap().length(), 2);
}

#[test]
fn item_returns_property_names_in_canonical_order() {
    // property_names() lists "color" before "gap"; item(0)
    // should hit the first set name in that order.
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        let mut sd = nm.style_mut().unwrap();
        sd.set_property("gap", "2").unwrap();
        sd.set_property("color", "red").unwrap();
    }
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.item(0), Some("color"));
    assert_eq!(style.item(1), Some("gap"));
    assert_eq!(style.item(2), None);
}

// ── Build-script camelCase aliases (step 27) ─────────────────

#[test]
fn alias_color_delegates_to_get_property_value() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.color(), "red");
    assert_eq!(style.color(), style.get_property_value("color"));
}

#[test]
fn alias_background_color_handles_kebab_to_snake_conversion() {
    // Single-hyphen property name. The generated alias is
    // `background_color` (snake), reading the kebab key
    // `"background-color"` internally.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("background-color", "blue")
        .unwrap();
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.background_color(), "blue");
    assert_eq!(
        style.background_color(),
        style.get_property_value("background-color")
    );
}

#[test]
fn alias_transition_timing_function_handles_multi_hyphen() {
    // Multi-hyphen property name → multi-underscore method.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("transition-timing-function", "ease-in-out")
        .unwrap();
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.transition_timing_function(), "ease-in-out");
}

#[test]
fn alias_z_index_handles_short_compound() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("z-index", "3")
        .unwrap();
    assert_eq!(dom.node(div).style().unwrap().z_index(), "3");
}

#[test]
fn alias_returns_empty_when_property_unset() {
    // Aliases delegate to get_property_value, which returns
    // "" for unset properties (CSSOM convention). Generated
    // aliases inherit that behavior.
    let (dom, div) = dom_with("div");
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.color(), "");
    assert_eq!(style.padding(), "");
    assert_eq!(style.font_weight(), "");
}
