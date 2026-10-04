//! `C1G-TRANSITION-PREV`: a registered custom property's transition
//! re-cascades its `var()` consumers every frame. CSS Transitions 1 §3
//! compares a style change against the *before-change style* — the
//! previous computed values with running animations brought up to the
//! current time — so an unrelated restyle mid-transition must see the
//! animated values the last frame produced, not the values of the last
//! cascade, or it starts transitions nothing asked for.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rdom_core::ListenerOptions;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

/// Transitions 1 §3: `--c` (registered `<color>`) transitions red →
/// blue and `color: var(--c)` follows it frame by frame. Adding an
/// unrelated class mid-transition changes nothing the transitions read,
/// so `color` is neither retargeted nor cancelled.
#[test]
fn an_unrelated_restyle_mid_transition_starts_no_transition() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(
        "@property --c { syntax: '<color>'; inherits: true; initial-value: red } \
         div { color: var(--c); transition: all 1s linear } \
         div.on { --c: blue }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let mut app = App::with_backend(
        dom,
        Stylesheet::new(),
        Terminal::new(TestBackend::new(10, 3)).unwrap(),
    )
    .unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    let cancelled: Rc<RefCell<Vec<String>>> = Rc::default();
    {
        let cancelled = cancelled.clone();
        app.dom_mut()
            .add_event_listener(
                div,
                "transitioncancel",
                ListenerOptions::default(),
                move |ctx| {
                    let t = ctx.event.detail.as_transition().expect("typed detail");
                    cancelled.borrow_mut().push(t.property_name.clone());
                },
            )
            .unwrap();
    }
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    // Real time moves the transitions (they run on the wall clock).
    std::thread::sleep(Duration::from_millis(40));
    app.advance(0).unwrap();
    assert!(cancelled.borrow().is_empty(), "{:?}", cancelled.borrow());

    app.dom_mut().set_attribute(div, "class", "on x").unwrap();
    app.advance(0).unwrap();
    assert!(
        cancelled.borrow().is_empty(),
        "the unrelated class change retargeted {:?}",
        cancelled.borrow()
    );
}

/// `C1G-REGISTERED-CLONES`: the registrations ("later wins" over every
/// sheet, Properties and Values 1 §3) are built once per stylesheet
/// change and shared by the cascade and the transition engine — not
/// rebuilt by every cascade, restyle or transition frame.
#[test]
fn registrations_are_built_once_per_stylesheet_change() {
    use crate::style::cascade::registry_probe::take_builds;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(
        "@property --c { syntax: '<color>'; inherits: true; initial-value: red } \
         div { color: var(--c); transition: all 1s linear } \
         div.on { --c: blue }",
    );
    let mut app = App::with_backend(
        dom,
        Stylesheet::new(),
        Terminal::new(TestBackend::new(10, 3)).unwrap(),
    )
    .unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    take_builds();
    // Restyles and transition frames: no sheet changed.
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    app.advance(0).unwrap();
    app.dom_mut().set_attribute(div, "class", "on x").unwrap();
    app.advance(0).unwrap();
    assert_eq!(take_builds(), 0, "no sheet changed");
    // A sheet change rebuilds them, once.
    app.push_stylesheet(Stylesheet::bare());
    app.advance(0).unwrap();
    app.advance(0).unwrap();
    assert_eq!(take_builds(), 1, "one sheet change");
}

/// `C1G-PROPERTY-RESTYLE` — an inheriting `--theme` transition on the
/// root element moves every `var()` consumer below it each frame (CSS
/// Properties and Values 1 §6.2), but no frame of it matches a
/// selector: the restyle reuses each element's matched rules and only
/// re-resolves the styles. The consumers still follow the value.
#[test]
fn a_theme_transition_frame_matches_no_selectors() {
    use crate::style::cascade::match_probe;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let main = dom.create_element("main");
    dom.append_child(root, main).unwrap();
    let mut leaves = Vec::new();
    for _ in 0..3 {
        let section = dom.create_element("section");
        let p = dom.create_element("p");
        dom.append_child(section, p).unwrap();
        dom.append_child(main, section).unwrap();
        leaves.push(p);
    }
    let sheet = rdom_css::parse(
        "@property --theme { syntax: '<color>'; inherits: true; initial-value: red } \
         main { transition: --theme 1s linear } \
         main.dark { --theme: blue } \
         section > p { color: var(--theme) } \
         p::before { content: 'x'; color: var(--theme) }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let mut app = App::with_backend(
        dom,
        Stylesheet::new(),
        Terminal::new(TestBackend::new(20, 5)).unwrap(),
    )
    .unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    let fg = |app: &App<TestBackend>, id| {
        app.dom()
            .node(id)
            .ext()
            .unwrap()
            .computed
            .as_ref()
            .unwrap()
            .fg
    };
    assert_eq!(fg(&app, leaves[2]), crate::style::Color::Rgb(255, 0, 0));
    app.dom_mut().set_attribute(main, "class", "dark").unwrap();
    app.advance(0).unwrap();
    match_probe::take();
    std::thread::sleep(Duration::from_millis(60));
    app.advance(0).unwrap();
    assert_eq!(
        match_probe::take(),
        0,
        "a transition frame matched selectors"
    );
    let crate::style::Color::Rgb(r, _, b) = fg(&app, leaves[2]) else {
        panic!("{:?}", fg(&app, leaves[2]))
    };
    assert!(r < 255 && b > 0, "the consumer follows the value: {r} {b}");
    let before = app
        .dom()
        .node(leaves[2])
        .ext()
        .unwrap()
        .computed_before
        .clone()
        .unwrap();
    assert_eq!(before.fg, fg(&app, leaves[2]), "so does the pseudo-element");
}
