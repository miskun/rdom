//! Transitions on registered custom properties (CSS Properties and
//! Values API 1 §6.2; CSS Transitions 1 §3): a registered property
//! animates as its syntax type when that type interpolates, and the
//! `var()` consumers follow the animated value.

use std::time::{Duration, Instant};

use super::*;
use crate::style::Stylesheet;
use crate::{CascadeExt, TuiDom};

fn sheet(syntax: &str, initial: &str, value: &str, property: &str) -> Stylesheet {
    let css = format!(
        "@property --c {{ syntax: '{syntax}'; inherits: false; initial-value: {initial} }} \
         div {{ --c: {value}; transition: --c 100ms linear; {property}: var(--c) }}"
    );
    let parsed = rdom_css::parse(&css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    parsed.stylesheet
}

fn div() -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    (dom, div)
}

/// One frame of the App's pipeline after the cascade: advance the
/// transitions, then re-cascade the subtrees whose animated custom
/// properties moved.
fn frame(dom: &mut TuiDom, reg: &mut AnimationRegistry, sheet: &Stylesheet, now: Instant) {
    reg.advance(dom, now);
    let restyle = reg.take_restyle();
    if !restyle.is_empty() {
        dom.cascade_subtrees(sheet, &restyle);
    }
}

fn computed(dom: &TuiDom, id: NodeId) -> std::rc::Rc<crate::style::ComputedStyle> {
    dom.node(id).ext().unwrap().computed.clone().unwrap()
}

/// A registered `<color>` interpolates; `color: var(--c)` follows it,
/// while the cascaded value of `--c` is already the end value.
#[test]
fn registered_color_property_transitions() {
    let (mut dom, div) = div();
    let red = sheet("<color>", "red", "rgb(255, 0, 0)", "color");
    let blue = sheet("<color>", "red", "rgb(0, 0, 255)", "color");
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(&[&red]);
    dom.cascade(&red);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&blue);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1);

    frame(&mut dom, &mut reg, &blue, start + Duration::from_millis(50));
    let mid = computed(&dom, div);
    let Color::Rgb(r, g, b) = mid.fg else {
        panic!("{:?}", mid.fg)
    };
    assert!(
        (r as i16 - 128).abs() <= 2 && g == 0 && (b as i16 - 128).abs() <= 2,
        "{r} {g} {b}"
    );
    let cascaded = mid.vars.get("c").and_then(|v| crate::style::parse_color(v));
    assert_eq!(
        cascaded,
        Some(Color::Rgb(0, 0, 255)),
        "`--c` is cascaded at its end value"
    );

    frame(
        &mut dom,
        &mut reg,
        &blue,
        start + Duration::from_millis(150),
    );
    assert!(reg.is_empty());
    assert_eq!(computed(&dom, div).fg, Color::Rgb(0, 0, 255));
}

/// A registered `<length>` interpolates in whole cells.
#[test]
fn registered_length_property_transitions() {
    let (mut dom, div) = div();
    let narrow = sheet("<length>", "0", "2", "width");
    let wide = sheet("<length>", "0", "12", "width");
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(&[&narrow]);
    dom.cascade(&narrow);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&wide);
    diff_and_register(&mut dom, &mut reg, start);
    frame(&mut dom, &mut reg, &wide, start + Duration::from_millis(50));
    let mut lit = div_style("width: 7");
    lit.vars = computed(&dom, div).vars.clone();
    assert_eq!(computed(&dom, div).width, lit.width);
}

fn div_style(decl: &str) -> crate::style::ComputedStyle {
    let (mut dom, div) = div();
    dom.cascade(&rdom_css::parse(&format!("div {{ {decl} }}")).stylesheet);
    (*computed(&dom, div)).clone()
}

/// An unregistered custom property, or one whose syntax does not
/// interpolate, changes at once (Transitions 1 §3: not animatable).
#[test]
fn unregistered_and_discrete_custom_properties_do_not_transition() {
    for (syntax, a, b) in [("*", "x", "y"), ("<custom-ident>", "x", "y")] {
        let (mut dom, _) = div();
        let one = sheet(syntax, "x", a, "--d");
        let two = sheet(syntax, "x", b, "--d");
        let start = Instant::now();
        let mut reg = AnimationRegistry::new();
        reg.set_registered_properties(&[&one]);
        dom.cascade(&one);
        diff_and_register(&mut dom, &mut reg, start);
        dom.cascade(&two);
        diff_and_register(&mut dom, &mut reg, start);
        assert!(reg.is_empty(), "{syntax}");
    }
}
