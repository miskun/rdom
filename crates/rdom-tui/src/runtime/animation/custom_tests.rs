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
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&red]),
    ));
    dom.cascade(&red);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&blue);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1);

    frame(&mut dom, &mut reg, &blue, start + Duration::from_millis(50));
    let mid = computed(&dom, div);
    // Red → blue at the midpoint, in Oklab (CSS Color 4 §12.1).
    assert_eq!(mid.fg, Color::Rgb(140, 83, 162));
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
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&narrow]),
    ));
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
        reg.set_registered_properties(std::rc::Rc::new(
            crate::style::cascade::PropertyRegistry::new(&[&one]),
        ));
        dom.cascade(&one);
        diff_and_register(&mut dom, &mut reg, start);
        dom.cascade(&two);
        diff_and_register(&mut dom, &mut reg, start);
        assert!(reg.is_empty(), "{syntax}");
    }
}

/// `C1G-PROPERTY-RESTYLE` — CSS Transitions 1 §3: during
/// `transition-delay` the animated value is the start value. The first
/// frame applies it (the cascade had already moved to the end value);
/// later frames inside the delay change nothing, so they restyle
/// nothing.
#[test]
fn no_restyle_while_the_value_holds_during_the_delay() {
    let css = |value: &str| {
        let parsed = rdom_css::parse(&format!(
            "@property --c {{ syntax: '<color>'; inherits: true; initial-value: red }} \
             div {{ --c: {value}; transition: --c 100ms linear 1s; color: var(--c) }}"
        ));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        parsed.stylesheet
    };
    let (mut dom, _) = div();
    let (red, blue) = (css("red"), css("blue"));
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&red]),
    ));
    dom.cascade(&red);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&blue);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1);

    reg.advance(&mut dom, start);
    assert_eq!(reg.take_restyle().len(), 1, "the start value is applied");
    for ms in [100, 400, 900] {
        reg.advance(&mut dom, start + Duration::from_millis(ms));
        assert!(reg.take_restyle().is_empty(), "{ms}ms: inside the delay");
    }
    reg.advance(&mut dom, start + Duration::from_millis(1050));
    assert_eq!(reg.take_restyle().len(), 1, "running: the value moves");
}

/// `C2-ANGLE` — CSS Properties and Values 1 §6.2: a registered `<angle>`
/// interpolates (as degrees); a trigonometric consumer follows it.
#[test]
fn registered_angle_property_transitions() {
    let css = |angle: &str| {
        let parsed = rdom_css::parse(&format!(
            "@property --a {{ syntax: '<angle>'; inherits: false; initial-value: 0deg }} \
             div {{ --a: {angle}; transition: --a 100ms linear; width: calc(sin(var(--a)) * 20) }}"
        ));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        parsed.stylesheet
    };
    let (mut dom, div) = div();
    let (from, to) = (css("0deg"), css("0.25turn"));
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&from]),
    ));
    dom.cascade(&from);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&to);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1, "the angle animates");
    frame(&mut dom, &mut reg, &to, start + Duration::from_millis(50));
    assert_eq!(
        computed(&dom, div).width,
        crate::layout::Size::Fixed(14),
        "sin(45deg) × 20 = 14.1"
    );
}

/// `C2G-CALC-SEMANTICS` — CSS Values 4 §10.9: an infinite angle clamps
/// to the largest finite one, so a registered `<angle>` transitioning
/// between `calc(-infinity * 1deg)` and `calc(infinity * 1deg)` animates
/// through finite, valid `<angle>`s (it used to interpolate ∞ − ∞ into
/// `NaNdeg`, which a consumer could not use).
#[test]
fn registered_angle_transition_between_infinities_stays_finite() {
    let css = |angle: &str| {
        let parsed = rdom_css::parse(&format!(
            "@property --a {{ syntax: '<angle>'; inherits: false; initial-value: 0deg }} \
             div {{ --a: {angle}; transition: --a 100ms linear; width: calc(sign(var(--a)) * 5 + 10) }}"
        ));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        parsed.stylesheet
    };
    let (mut dom, div) = div();
    let (from, to) = (css("calc(-infinity * 1deg)"), css("calc(infinity * 1deg)"));
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&from]),
    ));
    dom.cascade(&from);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&to);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1, "the angle animates");
    frame(&mut dom, &mut reg, &to, start + Duration::from_millis(25));
    assert_eq!(
        computed(&dom, div).width,
        crate::layout::Size::Fixed(5),
        "a quarter of the way from the most negative angle: sign is -1"
    );
    frame(&mut dom, &mut reg, &to, start + Duration::from_millis(100));
    assert_eq!(computed(&dom, div).width, crate::layout::Size::Fixed(15));
}

/// `C2G-RESTYLE-WALK` — CSS Lists 3 §3.1: a counter's value at an element
/// is the sum of the increments before it in tree order, so a transitioning
/// `--step` feeding `counter-increment: c var(--step)` moves `counter(c)`
/// for every element after the animated one. Mid-transition (`--step` 1 →
/// 11, half way: 6) the later sibling's `::before` reads 6, through the
/// App's restyle (`restyle_vars`), not the cascaded end value 11.
#[test]
fn animated_counter_increment_reaches_later_siblings() {
    let css = |step: u32| {
        let parsed = rdom_css::parse(&format!(
            "@property --step {{ syntax: '<integer>'; inherits: false; initial-value: 0 }} \
             main {{ counter-reset: c }} \
             .a {{ --step: {step}; transition: --step 100ms linear; counter-increment: c var(--step) }} \
             .b::before {{ content: counter(c) }}"
        ));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        parsed.stylesheet
    };
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let main = dom.create_element("main");
    dom.append_child(root, main).unwrap();
    let a = dom.create_element("div");
    dom.set_attribute(a, "class", "a").unwrap();
    dom.append_child(main, a).unwrap();
    let b = dom.create_element("div");
    dom.set_attribute(b, "class", "b").unwrap();
    dom.append_child(main, b).unwrap();
    let (from, to) = (css(1), css(11));
    let registry = std::rc::Rc::new(crate::style::cascade::PropertyRegistry::new(&[&from]));
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(registry.clone());
    dom.cascade(&from);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&to);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1, "--step animates");

    reg.advance(&mut dom, start + Duration::from_millis(50));
    let restyle = reg.take_restyle();
    assert_eq!(restyle, vec![a]);
    crate::style::cascade::restyle_vars(&mut dom, &[&to], registry, &restyle);
    let before = dom.node(b).ext().unwrap().computed_before.clone().unwrap();
    assert_eq!(before.content.as_deref(), Some("6"));
}

/// `C2G-REGISTERED-ABSOLUTE` — CSS Properties and Values 1 §2.4, §6.2: a
/// registered `<length>` computes to absolute cells, so `10vw` → `50vw`
/// (8 → 40 cells at 80 columns) interpolates: half way it is 24.
#[test]
fn registered_viewport_length_transitions() {
    let (mut dom, div) = div();
    dom.set_viewport(rdom_style::calc::Viewport::new(80, 20));
    let narrow = sheet("<length>", "0", "10vw", "width");
    let wide = sheet("<length>", "0", "50vw", "width");
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&narrow]),
    ));
    dom.cascade(&narrow);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&wide);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1, "the length animates");
    frame(&mut dom, &mut reg, &wide, start + Duration::from_millis(50));
    assert_eq!(computed(&dom, div).width, crate::layout::Size::Fixed(24));
}

/// `C2G-REGISTERED-ABSOLUTE` — CSS Properties and Values 1 §6.2, CSS
/// Values 4 §3.4.3: a `<length-percentage>` interpolates its length and
/// its percentage apart — `10` → `calc(20 + 50%)` is `calc(15 + 25%)`
/// half way.
#[test]
fn registered_length_percentage_transitions() {
    let (mut dom, div) = div();
    let from = sheet("<length-percentage>", "0", "10", "width");
    let to = sheet("<length-percentage>", "0", "calc(20 + 50%)", "width");
    let start = Instant::now();
    let mut reg = AnimationRegistry::new();
    reg.set_registered_properties(std::rc::Rc::new(
        crate::style::cascade::PropertyRegistry::new(&[&from]),
    ));
    dom.cascade(&from);
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&to);
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1, "the length-percentage animates");
    frame(&mut dom, &mut reg, &to, start + Duration::from_millis(50));
    assert_eq!(
        computed(&dom, div).width,
        div_style("width: calc(15 + 25%)").width
    );
}
