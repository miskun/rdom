//! `cssText` serialization: `!important` markers, round-trips, and
//! the shorthand-suppresses-longhands rule (D-M4-2).

use super::dom_with;
use crate::{TuiAccessors, TuiAccessorsMut};

// ── css_text + round-trip ────────────────────────────────────

#[test]
fn css_text_round_trips_through_set_css_text() {
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        let mut sd = nm.style_mut().unwrap();
        sd.set_property("color", "red").unwrap();
        sd.set_property("gap", "2").unwrap();
    }
    let serialized = dom.node(div).style().unwrap().css_text();

    // Build a second element, apply the serialized text,
    // assert the same property values.
    let root = dom.root();
    let other = dom.create_element("div");
    dom.append_child(root, other).unwrap();
    dom.node_mut(other)
        .style_mut()
        .unwrap()
        .set_css_text(&serialized)
        .unwrap();
    let s = dom.node(other).style().unwrap();
    assert_eq!(s.get_property_value("color"), "red");
    assert_eq!(s.get_property_value("gap"), "2");
}

#[test]
fn css_text_includes_important_marker() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property_important("color", "red")
        .unwrap();
    let text = dom.node(div).style().unwrap().css_text();
    assert!(text.contains("color: red !important"));
}

// ── cssText: shorthand suppresses longhands (D-M4-2) ─────────

#[test]
fn css_text_padding_shorthand_skips_longhands() {
    // Pre-D-M4-2 bug: cssText emitted padding AND each of the
    // four padding-* longhands because they all read from
    // style.padding. Round-tripping cssText through set_css_text
    // back to cssText would not be lossless. Browser-faithful:
    // emit shorthand only when its name comes up; skip longhands.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("padding", "1 2 3 4")
        .unwrap();
    let css = dom.node(div).style().unwrap().css_text();
    assert!(
        css.contains("padding: 1 2 3 4;"),
        "shorthand should emit, got {css:?}"
    );
    assert!(
        !css.contains("padding-top"),
        "padding-top longhand must not be emitted when shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("padding-right"),
        "padding-right longhand must not be emitted when shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("padding-bottom"),
        "padding-bottom longhand must not be emitted when shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("padding-left"),
        "padding-left longhand must not be emitted when shorthand fires, got {css:?}"
    );
}

#[test]
fn css_text_inset_shorthand_skips_longhands_when_all_four_set() {
    // inset fires only when all four sides are set. Then the
    // four longhand (top/right/bottom/left) declarations must
    // be skipped — same rule as padding.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("inset", "1 2 3 4")
        .unwrap();
    let css = dom.node(div).style().unwrap().css_text();
    assert!(css.contains("inset:"), "inset shorthand should emit");
    assert!(
        !css.contains("top: 1"),
        "top longhand must not emit when inset shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("right: 2"),
        "right longhand must not emit when inset shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("bottom: 3"),
        "bottom longhand must not emit when inset shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("left: 4"),
        "left longhand must not emit when inset shorthand fires, got {css:?}"
    );
}

#[test]
fn css_text_inset_longhand_only_emits_set_sides() {
    // Counter to the previous test: when only one side is set,
    // the inset shorthand serializes None, so the lone longhand
    // emits on its own. This is the *correct* behavior pre-fix
    // too — confirming the fix doesn't regress it.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("top", "5")
        .unwrap();
    let css = dom.node(div).style().unwrap().css_text();
    assert!(css.contains("top: 5"), "lone top should emit, got {css:?}");
    assert!(
        !css.contains("inset:"),
        "inset must not emit when only top is set, got {css:?}"
    );
}

#[test]
fn css_text_overflow_shorthand_skips_longhands() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("overflow", "scroll")
        .unwrap();
    let css = dom.node(div).style().unwrap().css_text();
    assert!(css.contains("overflow: scroll"));
    assert!(
        !css.contains("overflow-x"),
        "overflow-x longhand must not emit when overflow shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("overflow-y"),
        "overflow-y longhand must not emit when overflow shorthand fires, got {css:?}"
    );
}

#[test]
fn css_text_transition_shorthand_skips_longhands() {
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("transition", "color 200ms ease 0ms")
        .unwrap();
    let css = dom.node(div).style().unwrap().css_text();
    assert!(
        css.contains("transition:"),
        "transition shorthand should emit, got {css:?}"
    );
    assert!(
        !css.contains("transition-property"),
        "transition-property longhand must not emit when shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("transition-duration"),
        "transition-duration longhand must not emit when shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("transition-timing-function"),
        "transition-timing-function longhand must not emit when shorthand fires, got {css:?}"
    );
    assert!(
        !css.contains("transition-delay"),
        "transition-delay longhand must not emit when shorthand fires, got {css:?}"
    );
}

#[test]
fn css_text_round_trip_padding_is_lossless() {
    // The headline correctness test for D-M4-2. cssText output
    // must parse back into an equivalent inline style.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("padding", "1 2 3 4")
        .unwrap();
    let css_first = dom.node(div).style().unwrap().css_text();
    // Round-trip: feed cssText back through set_css_text and
    // confirm the result re-serializes the same string.
    let (mut dom2, div2) = dom_with("div");
    dom2.node_mut(div2)
        .style_mut()
        .unwrap()
        .set_css_text(&css_first)
        .unwrap();
    let css_second = dom2.node(div2).style().unwrap().css_text();
    assert_eq!(
        css_first, css_second,
        "cssText round-trip must be lossless (was broken pre-D-M4-2)"
    );
}
