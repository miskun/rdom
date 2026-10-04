//! `revert` through the cascade (CSS Cascade 4 §7.3): an author or
//! inline declaration of `revert` rolls the property back to the value
//! the user-agent origin gives it; with no UA declaration for the
//! property, that is the `unset` value (inherited or initial).

use super::*;
use crate::style::{Color, Stylesheet, TuiStyle, Value};
use crate::{TuiDom, TuiNodeMutExt};
use rdom_core::NodeId;

/// `<div><button/><span/></div>` under the root.
fn tree() -> (TuiDom, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    let button = dom.create_element("button");
    let span = dom.create_element("span");
    dom.append_child(div, button).unwrap();
    dom.append_child(div, span).unwrap();
    dom.append_child(root, div).unwrap();
    (dom, div, button, span)
}

fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let mut sheet = Stylesheet::new();
    for rule in parsed.stylesheet.rules() {
        sheet
            .add_rule(&rule.source_text, rule.style.clone())
            .unwrap();
    }
    sheet
}

/// Cascade 4 §7.3: `revert` in the author origin rolls back to the UA
/// origin's cascaded value — `button`'s UA color, bold and display.
#[test]
fn author_revert_rolls_back_to_the_ua_value() {
    let (mut ua_dom, _, ua_button, _) = tree();
    ua_dom.cascade(&Stylesheet::new());
    let ua = computed_of(&ua_dom, ua_button);

    let (mut dom, _, button, _) = tree();
    dom.cascade(&sheet(
        "button { color: red; font-weight: normal; display: block } \
         button { color: revert; font-weight: revert; display: revert }",
    ));
    let got = computed_of(&dom, button);
    assert_eq!(got.fg, ua.fg);
    assert_ne!(got.fg, Color::Rgb(255, 0, 0));
    assert_eq!(got.modifiers, ua.modifiers);
    assert_eq!(got.display, ua.display);
}

/// With no UA declaration for the property, rolling back to the UA
/// origin leaves the property unset: `color` inherits, `background`
/// takes its initial value.
#[test]
fn author_revert_without_a_ua_rule_acts_like_unset() {
    let (mut dom, _, _, span) = tree();
    dom.cascade(&sheet(
        "div { color: red; background: blue } \
         span { color: green; background: green } \
         span { color: revert; background: revert }",
    ));
    let got = computed_of(&dom, span);
    assert_eq!(got.fg, Color::Rgb(255, 0, 0), "inherited");
    assert_eq!(got.bg, crate::style::ComputedStyle::initial().bg, "initial");
}

/// `revert` discards every author declaration of the property,
/// `!important` ones and inline style included, not just the earlier
/// rules.
#[test]
fn revert_skips_important_and_inline_author_declarations() {
    let (mut dom, _, button, span) = tree();
    dom.node_mut(span)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    let mut important_revert = TuiStyle::new();
    important_revert.fg = Some(Value::Revert);
    important_revert.important |= crate::style::ImportantMask::FG;
    let sheet = sheet("div { color: red } button { color: green !important }")
        .rule_unchecked("span, button", important_revert);
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, span).fg, Color::Rgb(255, 0, 0));
    let (mut ua_dom, _, ua_button, _) = tree();
    ua_dom.cascade(&Stylesheet::new());
    assert_eq!(
        computed_of(&dom, button).fg,
        computed_of(&ua_dom, ua_button).fg
    );
}

/// An inline `revert` rolls back past the author rules too.
#[test]
fn inline_revert_rolls_back_to_the_ua_value() {
    let (mut dom, _, _, span) = tree();
    let mut inline = TuiStyle::new();
    inline.bg = Some(Value::Revert);
    dom.node_mut(span).set_inline_style(inline);
    dom.cascade(&sheet("span { background: green }"));
    assert_eq!(
        computed_of(&dom, span).bg,
        crate::style::ComputedStyle::initial().bg
    );
}

/// CSS Variables 1 §2 + Cascade 4 §7.3: `revert` on a custom property
/// rolls it back to the UA origin, which declares none — the inherited
/// value.
#[test]
fn custom_property_revert_takes_the_inherited_value() {
    let (mut dom, div, _, span) = tree();
    dom.cascade(&sheet(
        "div { --x: outer } span { --x: inner } span { --x: revert }",
    ));
    assert_eq!(
        computed_of(&dom, div).vars.get("x").map(String::as_str),
        Some("outer")
    );
    assert_eq!(
        computed_of(&dom, span).vars.get("x").map(String::as_str),
        Some("outer")
    );
}
