//! Write side: `set_property` / `set_property_important` /
//! `remove_property` / `set_css_text`, and the `try_*` parse channel.

use super::dom_with;
use crate::{TuiAccessors, TuiAccessorsMut};

// ── Write: set_property ──────────────────────────────────────

#[test]
fn set_property_writes_inline_style_field() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();
    // Re-read via the read side.
    assert_eq!(
        dom.node(div).style().unwrap().get_property_value("color"),
        "red"
    );
}

#[test]
fn set_property_writes_style_attribute_atomically() {
    // §8.5 acceptance criterion: programmatic set_property
    // updates BOTH the inline-style field AND the style="…"
    // attribute. This is the "attribute coherence" lock.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();
    // (a) inline_style.fg was written.
    assert_eq!(
        dom.node(div).style().unwrap().get_property_value("color"),
        "red"
    );
    // (b) style="…" attribute was re-serialized.
    let attr = dom.node(div).get_attribute("style").unwrap_or("");
    assert!(
        attr.contains("color: red"),
        "style attribute should contain `color: red`, got {attr:?}"
    );
}

#[test]
fn set_property_invalid_value_is_silent_no_op() {
    let (mut dom, div) = dom_with("div");
    let r = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "not-a-color");
    assert!(r.is_ok());
    assert_eq!(
        dom.node(div).style().unwrap().get_property_value("color"),
        "",
        "inline_style.fg should be unset after invalid value"
    );
    // Attribute should not have been touched.
    assert_eq!(dom.node(div).get_attribute("style"), None);
}

#[test]
fn set_property_unknown_name_is_silent_no_op() {
    let (mut dom, div) = dom_with("div");
    let r = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("not-a-property", "x");
    assert!(r.is_ok());
    assert_eq!(dom.node(div).get_attribute("style"), None);
}

#[test]
fn set_property_clears_prior_important_bit() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property_important("color", "red")
        .unwrap();
    assert_eq!(
        dom.node(div)
            .style()
            .unwrap()
            .get_property_priority("color"),
        "important"
    );
    // Browser: setProperty without "important" clears the bit.
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "blue")
        .unwrap();
    assert_eq!(
        dom.node(div)
            .style()
            .unwrap()
            .get_property_priority("color"),
        ""
    );
}

// ── Write: set_property_important ────────────────────────────

#[test]
fn set_property_important_raises_priority() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property_important("color", "red")
        .unwrap();
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.get_property_value("color"), "red");
    assert_eq!(style.get_property_priority("color"), "important");
    // !important is emitted in css_text.
    assert!(style.css_text().contains("!important"));
}

#[test]
fn set_property_important_writes_attribute_with_marker() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property_important("color", "red")
        .unwrap();
    let attr = dom.node(div).get_attribute("style").unwrap_or("");
    assert!(
        attr.contains("!important"),
        "style attribute should include !important, got {attr:?}"
    );
}

// ── Write: remove_property ───────────────────────────────────

#[test]
fn remove_property_returns_previous_value() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();
    let prev = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .remove_property("color")
        .unwrap();
    assert_eq!(prev, "red");
    assert_eq!(
        dom.node(div)
            .style()
            .unwrap()
            .get_property_priority("color"),
        ""
    );
}

#[test]
fn remove_property_clears_field_and_important_bit() {
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        let mut sd = nm.style_mut().unwrap();
        sd.set_property_important("color", "red").unwrap();
        sd.remove_property("color").unwrap();
    }
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.get_property_value("color"), "");
    assert_eq!(style.get_property_priority("color"), "");
}

#[test]
fn remove_property_unset_returns_empty_string() {
    let (mut dom, div) = dom_with("div");
    let prev = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .remove_property("color")
        .unwrap();
    assert_eq!(prev, "");
}

#[test]
fn remove_property_unknown_returns_empty_string() {
    let (mut dom, div) = dom_with("div");
    let prev = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .remove_property("bogus")
        .unwrap();
    assert_eq!(prev, "");
}

// ── Write: set_css_text ──────────────────────────────────────

#[test]
fn set_css_text_replaces_entire_declaration() {
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        let mut sd = nm.style_mut().unwrap();
        sd.set_property("color", "red").unwrap();
        sd.set_css_text("display: block; gap: 2").unwrap();
    }
    // color was cleared; display + gap are set.
    let style = dom.node(div).style().unwrap();
    assert_eq!(style.get_property_value("color"), "");
    assert_eq!(style.get_property_value("display"), "block");
    assert_eq!(style.get_property_value("gap"), "2");
}

#[test]
fn set_css_text_serializes_back_to_attribute() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_css_text("color: red; gap: 3")
        .unwrap();
    let attr = dom.node(div).get_attribute("style").unwrap_or("");
    assert!(attr.contains("color: red"));
    assert!(attr.contains("gap: 3"));
}

// ── try_set_property surfaces parse failures (A-M4-2) ────────

#[test]
fn try_set_property_returns_unknown_property() {
    use crate::cssom::{DispatchError, SetPropertyError};
    let (mut dom, div) = dom_with("div");
    let mut nm = dom.node_mut(div);
    let r = nm.style_mut().unwrap().try_set_property("colour", "red");
    assert!(matches!(
        r,
        Err(SetPropertyError::Parse(DispatchError::UnknownProperty))
    ));
}

#[test]
fn try_set_property_returns_invalid_value() {
    use crate::cssom::{DispatchError, SetPropertyError};
    let (mut dom, div) = dom_with("div");
    let mut nm = dom.node_mut(div);
    let r = nm
        .style_mut()
        .unwrap()
        .try_set_property("color", "not-a-color");
    assert!(matches!(
        r,
        Err(SetPropertyError::Parse(DispatchError::InvalidValue))
    ));
}

#[test]
fn try_set_property_succeeds_on_valid_input() {
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        nm.style_mut()
            .unwrap()
            .try_set_property("color", "red")
            .unwrap();
    }
    assert_eq!(
        dom.node(div).style().unwrap().get_property_value("color"),
        "red"
    );
}

#[test]
fn try_set_property_important_raises_priority_and_surfaces_errors() {
    use crate::cssom::{DispatchError, SetPropertyError};
    let (mut dom, div) = dom_with("div");

    // Success path raises !important.
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .try_set_property_important("color", "red")
        .unwrap();
    assert_eq!(
        dom.node(div)
            .style()
            .unwrap()
            .get_property_priority("color"),
        "important"
    );

    // Failure path surfaces the parse error.
    let r = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .try_set_property_important("colour", "red");
    assert!(matches!(
        r,
        Err(SetPropertyError::Parse(DispatchError::UnknownProperty))
    ));
}

#[test]
fn set_property_remains_silent_on_parse_failure() {
    // A-M4-2 acceptance: try_set_property surfaces errors,
    // but plain set_property must keep its browser-faithful
    // silence so existing call sites don't change behavior.
    let (mut dom, div) = dom_with("div");
    let r = dom
        .node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("colour", "red");
    assert!(r.is_ok(), "set_property must silently swallow parse errors");
    assert_eq!(
        dom.node(div).style().unwrap().get_property_value("color"),
        ""
    );
}
