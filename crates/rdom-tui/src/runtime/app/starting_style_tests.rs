//! C12-STARTING — CSS Transitions 2 §3: `@starting-style` rules give an
//! element with no before-change style — newly rendered: first styled,
//! inserted, or out of `display: none` — the style its transitions start
//! from; the normal cascade ignores them.

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::popover;
use crate::style::Stylesheet;
use rdom_core::NodeId;

fn app_with(css: &str, build: impl FnOnce(&mut TuiDom) -> NodeId) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let id = build(&mut dom);
    let sheet = rdom_css::parse(css);
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    (app, id)
}

fn opacity(app: &App<TestBackend>, id: NodeId) -> f32 {
    app.dom()
        .node(id)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .opacity
}

/// §3: the normal cascade does not apply a starting-style rule.
#[test]
fn starting_style_rules_do_not_apply_otherwise() {
    let (app, div) = app_with(
        "div { opacity: 1 } @starting-style { div { opacity: 0.25 } }",
        |dom| {
            let root = dom.root();
            let div = dom.create_element("div");
            dom.append_child(root, div).unwrap();
            div
        },
    );
    assert_eq!(opacity(&app, div), 1.0);
}

/// §3: an element inserted into the document has no before-change style;
/// its starting style is the start of its transitions — it fades in.
#[test]
fn an_inserted_element_transitions_from_its_starting_style() {
    let (mut app, _) = app_with(
        "p { opacity: 1; transition: opacity 100ms linear } \
         @starting-style { p { opacity: 0 } }",
        |dom| dom.root(),
    );
    let root = app.dom().root();
    let p = app.dom_mut().create_element("p");
    app.dom_mut().append_child(root, p).unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert!(
        (opacity(&app, p) - 0.5).abs() < 0.01,
        "{}",
        opacity(&app, p)
    );
    app.advance(60).unwrap();
    assert_eq!(opacity(&app, p), 1.0);
}

/// The popover entry: shown, it comes out of `display: none` — no
/// before-change style — so `@starting-style` fades it in; hidden with
/// `allow-discrete` on `display` and `overlay`, it fades out in the top
/// layer and then leaves it.
#[test]
fn a_popover_fades_in_from_its_starting_style_and_out_in_the_top_layer() {
    let (mut app, pop) = app_with(
        "[popover] { opacity: 0; transition: opacity 100ms linear, \
         display 100ms allow-discrete, overlay 100ms allow-discrete } \
         [popover]:popover-open { opacity: 1 } \
         @starting-style { [popover]:popover-open { opacity: 0 } }",
        |dom| {
            let root = dom.root();
            let pop = dom.create_element("div");
            dom.set_attribute(pop, "popover", "").unwrap();
            let t = dom.create_text_node("pop");
            dom.append_child(pop, t).unwrap();
            dom.append_child(root, pop).unwrap();
            pop
        },
    );
    popover::show_popover(app.dom_mut(), pop).unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert!(
        (opacity(&app, pop) - 0.5).abs() < 0.01,
        "in: {}",
        opacity(&app, pop)
    );
    app.advance(60).unwrap();
    assert_eq!(opacity(&app, pop), 1.0);

    popover::hide_popover(app.dom_mut(), pop).unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert!(
        (opacity(&app, pop) - 0.5).abs() < 0.01,
        "out: {}",
        opacity(&app, pop)
    );
    assert!(app.dom().is_in_top_layer(pop), "still in the top layer");
    let display = app
        .dom()
        .node(pop)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .display;
    assert_ne!(display, crate::layout::Display::None, "still shown");
    app.advance(60).unwrap();
    app.advance(0).unwrap();
    assert!(!app.dom().is_in_top_layer(pop));
}

/// CSS Transitions 1 §3: an element coming out of `display: none` has no
/// before-change style — without a starting style its values change at
/// once (it transitioned from its hidden values).
#[test]
fn a_newly_rendered_element_without_a_starting_style_does_not_transition() {
    let (mut app, div) = app_with(
        "div { display: none; opacity: 0.2; transition: opacity 100ms linear } \
         div.on { display: block; opacity: 1 }",
        |dom| {
            let root = dom.root();
            let div = dom.create_element("div");
            dom.append_child(root, div).unwrap();
            div
        },
    );
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(10).unwrap();
    assert_eq!(opacity(&app, div), 1.0);
    assert!(app.animations.is_empty());
}

/// CSS Transitions 2 §3: a `::before` that starts to generate a box has no
/// before-change style; its `@starting-style` style is the start of its
/// transitions — it fades in — as an element's is.
#[test]
fn a_new_pseudo_element_transitions_from_its_starting_style() {
    let (mut app, div) = app_with(
        "div.on::before { content: 'x'; opacity: 1; transition: opacity 100ms linear } \
         @starting-style { div.on::before { opacity: 0 } }",
        |dom| {
            let root = dom.root();
            let div = dom.create_element("div");
            dom.append_child(root, div).unwrap();
            div
        },
    );
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    let before = |app: &App<TestBackend>| {
        app.dom()
            .node(div)
            .ext()
            .unwrap()
            .computed_before
            .as_ref()
            .map(|c| c.opacity)
    };
    let half = before(&app).unwrap();
    assert!((half - 0.5).abs() < 0.01, "{half}");
    app.advance(60).unwrap();
    assert_eq!(before(&app), Some(1.0));
}

/// C12G-BEFORE-CHANGE — CSS Transitions 1 §3: the before-change style is
/// the previous style "with any styles derived from declarative
/// animations ... updated to the current time", so whether the element was
/// rendered is read from what is on screen. A popover reopened halfway
/// through its fade-out (`display` held by `allow-discrete`) was rendered:
/// every running transition reverses from where it is — its color too,
/// which no starting style names — rather than finishing the exit and
/// snapping back.
#[test]
fn a_popover_reopened_during_its_fade_out_reverses_from_where_it_is() {
    let (mut app, pop) = app_with(
        "[popover] { opacity: 0; color: rgb(0,0,0); transition: opacity 100ms linear, \
         color 100ms linear, display 100ms allow-discrete, overlay 100ms allow-discrete } \
         [popover]:popover-open { opacity: 1; color: rgb(200,200,200) } \
         @starting-style { [popover]:popover-open { opacity: 0 } }",
        |dom| {
            let root = dom.root();
            let pop = dom.create_element("div");
            dom.set_attribute(pop, "popover", "").unwrap();
            let t = dom.create_text_node("pop");
            dom.append_child(pop, t).unwrap();
            dom.append_child(root, pop).unwrap();
            pop
        },
    );
    let red = |app: &App<TestBackend>| match app
        .dom()
        .node(pop)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .fg
    {
        crate::style::Color::Rgb(r, _, _) => r,
        other => panic!("{other:?}"),
    };
    popover::show_popover(app.dom_mut(), pop).unwrap();
    app.advance(0).unwrap();
    app.advance(150).unwrap();
    assert_eq!((opacity(&app, pop), red(&app)), (1.0, 200));

    popover::hide_popover(app.dom_mut(), pop).unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    let (half_opacity, half_red) = (opacity(&app, pop), red(&app));
    assert!(half_red > 0 && half_red < 200, "{half_red}");

    popover::show_popover(app.dom_mut(), pop).unwrap();
    app.advance(0).unwrap();
    app.advance(20).unwrap();
    assert!(
        red(&app) > half_red,
        "the color reverses toward the open one: {} after {half_red}",
        red(&app)
    );
    assert!(opacity(&app, pop) > half_opacity, "{}", opacity(&app, pop));
    app.advance(200).unwrap();
    assert_eq!((opacity(&app, pop), red(&app)), (1.0, 200));
    assert!(app.animations.is_empty());
}
