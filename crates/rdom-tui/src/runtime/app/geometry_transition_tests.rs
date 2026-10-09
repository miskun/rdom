//! C12-ANIMATABLE: a running transition's value is the element's computed
//! value, so layout reads it frame by frame. CSS Transitions 1 §3 / Web
//! Animations 1 §5.4.5 (the effect stack composites onto the computed
//! value); found by C10G-DETAILS-CONTENT-BOX — geometry transitions fired
//! their events while layout took the end value at once.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

/// An app over `<div id=a><span>x</span></div>` styled by `css`, one frame
/// drawn. The clock is the app's (`App::advance`).
fn app(css: &str) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "a").unwrap();
    let span = dom.create_element("span");
    let t = dom.create_text_node("x");
    dom.append_child(span, t).unwrap();
    dom.append_child(div, span).unwrap();
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(css);
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    (app, div, span)
}

fn rect(app: &App<TestBackend>, id: NodeId) -> crate::layout::LayoutRect {
    app.dom().node(id).ext().unwrap().layout
}

/// CSS Transitions 1 §3: `height: 2 → 10` over 100ms linear is 6 rows
/// half-way — layout reads the running value, not the end value.
#[test]
fn a_height_transition_moves_the_box_frame_by_frame() {
    let (mut app, div, _) = app("#a { height: 2; transition: height 100ms linear } \
         #a.on { height: 10 }");
    assert_eq!(rect(&app, div).height, 2);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(rect(&app, div).height, 2, "the start value");
    app.advance(50).unwrap();
    assert_eq!(rect(&app, div).height, 6, "half-way");
    app.advance(50).unwrap();
    assert_eq!(rect(&app, div).height, 10, "the end value");
    assert!(app.animations.is_empty());
}

/// Every geometry longhand reaches layout: `width`, `padding`, `margin`,
/// `inset` (a relative shift) and `gap` (a column flex container's rows).
#[test]
fn width_padding_margin_inset_and_gap_transitions_reach_layout() {
    let (mut app, div, span) = app("#a { display: flex; flex-direction: column; width: 10; \
         padding: 0; margin-top: 0; position: relative; top: 0; row-gap: 0; \
         transition: all 100ms linear } \
         #a.on { width: 20; padding-left: 4; margin-top: 4; top: 4 } \
         span { display: block }");
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    let r = rect(&app, div);
    // width 15 + padding-left 2 (content-box).
    assert_eq!(r.width, 17, "width and padding half-way");
    // margin-top 2 + top 2.
    assert_eq!(r.y, 4, "margin and inset half-way");
    assert_eq!(rect(&app, span).x - r.x, 2, "padding-left half-way");
}

/// The `<details>` use case: a `::details-content` box transitions its
/// height between two lengths as an element does (CSS Pseudo-Elements 4,
/// HTML §15.5.20).
#[test]
fn a_details_content_height_transition_reaches_layout() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("details");
    let s = dom.create_element("summary");
    let st = dom.create_text_node("S");
    dom.append_child(s, st).unwrap();
    dom.append_child(d, s).unwrap();
    for _ in 0..6 {
        let p = dom.create_element("p");
        let t = dom.create_text_node("line");
        dom.append_child(p, t).unwrap();
        dom.append_child(d, p).unwrap();
    }
    dom.append_child(root, d).unwrap();
    let sheet = rdom_css::parse(
        "details::details-content { display: block; height: 0; overflow: hidden; \
         transition: height 100ms linear } \
         details[open]::details-content { height: 4 }",
    );
    let terminal = Terminal::new(TestBackend::new(20, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.dom_mut().set_attribute(d, "open", "").unwrap();
    app.advance(0).unwrap();
    let content = crate::render::box_tree::slot::content_box(app.dom(), d).expect("a slot box");
    app.advance(50).unwrap();
    assert_eq!(rect(&app, content).height, 2, "half-way");
    app.advance(60).unwrap();
    assert_eq!(rect(&app, content).height, 4, "open");
}

/// An inherited property's running value is what the descendants inherit
/// (CSS Cascade 4 §7.2: the computed value is inherited, and the animated
/// value is the computed value).
#[test]
fn descendants_inherit_the_running_value() {
    let (mut app, div, span) = app(
        "#a { color: rgb(0, 0, 0); transition: color 100ms linear } \
         #a.on { color: rgb(200, 200, 200) }",
    );
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
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
    assert_ne!(
        fg(&app, div),
        crate::style::Color::Rgb(200, 200, 200),
        "running"
    );
    assert_eq!(
        fg(&app, span),
        fg(&app, div),
        "the span inherits the running color"
    );
    app.advance(60).unwrap();
    assert_eq!(fg(&app, span), crate::style::Color::Rgb(200, 200, 200));
}

/// Cost: a page with no running transition lays nothing out on a tick; a
/// running one lays out once per frame (layout is a whole-tree pass,
/// DIVERGENCES §3), and stops when it ends.
#[test]
fn only_a_running_transition_costs_frames() {
    let (mut app, div, _) = app("#a { height: 2; transition: height 100ms linear } \
         #a.on { height: 10 }");
    // The first frame's follow-up settles.
    app.advance(16).unwrap();
    app.take_frame_stats();
    for _ in 0..3 {
        app.advance(16).unwrap();
    }
    let idle = app.take_frame_stats();
    assert_eq!((idle.layouts, idle.paints), (0, 0), "{idle:?}");
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.take_frame_stats();
    for _ in 0..3 {
        app.advance(16).unwrap();
    }
    let running = app.take_frame_stats();
    assert_eq!(running.layouts, 3, "{running:?}");
    assert_eq!(
        running.full_cascades + running.subtree_cascades,
        0,
        "{running:?}"
    );
    app.advance(100).unwrap();
    app.take_frame_stats();
    app.advance(16).unwrap();
    let after = app.take_frame_stats();
    assert_eq!((after.layouts, after.paints), (0, 0), "{after:?}");
}

/// A pseudo-element's geometry transitions too: a positioned `::before`
/// is laid out from its computed style, which holds the running value.
#[test]
fn a_pseudo_elements_width_transition_reaches_layout() {
    let (mut app, div, _) = app("#a { position: relative } \
         #a::before { content: 'x'; position: absolute; width: 2; \
         transition: width 100ms linear } \
         #a.on::before { width: 10 }");
    let width = |app: &App<TestBackend>| {
        let ext = app.dom().node(div).ext().unwrap();
        ext.positioned_pseudos().next().map(|p| p.border_box.width)
    };
    assert_eq!(width(&app), Some(2));
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(width(&app), Some(6), "half-way");
    app.advance(60).unwrap();
    assert_eq!(width(&app), Some(10));
}

/// CSS Transitions 1 §3: a style change mid-transition is the new
/// after-change style — a property no transition covers takes its new
/// value at once while the running one keeps running.
#[test]
fn a_change_mid_transition_applies_to_the_other_properties() {
    let (mut app, div, _) = app(
        "#a { width: 4; height: 2; transition: height 100ms linear } \
         #a.on { height: 10 } #a.on.wide { width: 12 }",
    );
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    app.dom_mut()
        .set_attribute(div, "class", "on wide")
        .unwrap();
    app.advance(0).unwrap();
    let r = rect(&app, div);
    assert_eq!(
        (r.width, r.height),
        (12, 6),
        "the width at once, the height running"
    );
    app.advance(60).unwrap();
    let r = rect(&app, div);
    assert_eq!((r.width, r.height), (12, 10));
}

/// CSS Values 5 §11 (`interpolate-size: allow-keywords`): a size keyword
/// interpolates with a length through `calc-size()` — `height: 0` → `auto`
/// grows the box toward its content's height — the `<details>`
/// opening animation, on `::details-content`.
#[test]
fn interpolate_size_animates_details_content_to_auto() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("details");
    let s = dom.create_element("summary");
    let st = dom.create_text_node("S");
    dom.append_child(s, st).unwrap();
    dom.append_child(d, s).unwrap();
    for _ in 0..6 {
        let p = dom.create_element("p");
        let t = dom.create_text_node("line");
        dom.append_child(p, t).unwrap();
        dom.append_child(d, p).unwrap();
    }
    dom.append_child(root, d).unwrap();
    let sheet = rdom_css::parse(
        "details { interpolate-size: allow-keywords } \
         details::details-content { display: block; height: 0; overflow: hidden; \
         transition: height 100ms linear } \
         details[open]::details-content { height: auto }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.dom_mut().set_attribute(d, "open", "").unwrap();
    app.advance(0).unwrap();
    let content = crate::render::box_tree::slot::content_box(app.dom(), d).expect("a slot box");
    assert_eq!(rect(&app, content).height, 0, "the start value");
    app.advance(50).unwrap();
    assert_eq!(
        rect(&app, content).height,
        3,
        "half-way to its content's 6 rows"
    );
    app.advance(60).unwrap();
    assert_eq!(rect(&app, content).height, 6, "auto");
}

/// C14-CONTAIN — HTML §15.5.20 with CSS Containment 2 §4 and CSS
/// Transitions 2 §3.1: a closed `<details>`'s slot is `content-visibility:
/// hidden`, and a `content-visibility` transition under `allow-discrete`
/// keeps it `visible` until the end — so closing animates the height back
/// to 0 with the content shown, as in a browser.
#[test]
fn closing_details_animates_with_content_visibility() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("details");
    dom.set_attribute(d, "open", "").unwrap();
    let s = dom.create_element("summary");
    let st = dom.create_text_node("S");
    dom.append_child(s, st).unwrap();
    dom.append_child(d, s).unwrap();
    let mut first = None;
    for _ in 0..6 {
        let p = dom.create_element("p");
        let t = dom.create_text_node("line");
        dom.append_child(p, t).unwrap();
        dom.append_child(d, p).unwrap();
        first.get_or_insert(p);
    }
    dom.append_child(root, d).unwrap();
    let sheet = rdom_css::parse(
        "details { interpolate-size: allow-keywords } \
         details::details-content { display: block; height: 0; overflow: hidden; \
         transition: height 100ms linear, content-visibility 100ms allow-discrete } \
         details[open]::details-content { height: auto }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    let content = crate::render::box_tree::slot::content_box(app.dom(), d).expect("a slot box");
    assert_eq!(rect(&app, content).height, 6, "open");
    app.dom_mut().remove_attribute(d, "open").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(rect(&app, content).height, 3, "half-way back to 0");
    let p = first.unwrap();
    assert!(
        crate::node::is_rendered(app.dom(), p),
        "the content still shows while the transition runs"
    );
    app.advance(60).unwrap();
    assert_eq!(rect(&app, content).height, 0, "closed");
    assert!(
        !crate::node::is_rendered(app.dom(), p),
        "and skipped once it ends"
    );
}

/// `interpolate-size: numeric-only` (the initial value): a keyword and a
/// length do not interpolate (CSS Values 5 §11), so `width: auto` → `10`
/// takes the end value at once and starts no transition.
#[test]
fn numeric_only_keeps_auto_discrete() {
    let (mut app, div, _) = app("#a { width: auto; transition: width 100ms linear } \
         #a.on { width: 10 }");
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(rect(&app, div).width, 10);
    assert!(app.animations.is_empty());
}

/// `allow-keywords` on the root reaches every element (it inherits): a
/// block's `width: auto` (its containing block's 40 columns) → `10` is 25
/// half-way.
#[test]
fn allow_keywords_animates_an_auto_width() {
    let (mut app, div, _) = app(
        "#a { interpolate-size: allow-keywords; width: auto; transition: width 100ms linear } \
         #a.on { width: 10 }",
    );
    assert_eq!(rect(&app, div).width, 40);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(rect(&app, div).width, 25);
    app.advance(60).unwrap();
    assert_eq!(rect(&app, div).width, 10);
}

/// `calc-size()` as authored (CSS Values 5 §10): `calc-size(auto, size + 2)`
/// is two rows taller than the content.
#[test]
fn calc_size_sizes_from_its_basis() {
    let (app, div, _) = app("#a { height: calc-size(auto, size + 2) }");
    assert_eq!(rect(&app, div).height, 3, "one row of content + 2");
}

/// C12-BEHAVIOR (CSS Transitions 2 §3.1): under `allow-discrete` a box
/// going `display: none` keeps its box — laid out and painted — until the
/// transition ends, and then has none.
#[test]
fn a_display_none_transition_keeps_the_box_until_it_ends() {
    let (mut app, div, _) = app(
        "#a { height: 2; transition: display 100ms allow-discrete } \
         #a.gone { display: none }",
    );
    app.dom_mut().set_attribute(div, "class", "gone").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(rect(&app, div).height, 2, "still laid out half-way");
    app.advance(60).unwrap();
    let shown = app
        .dom()
        .node(div)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .display;
    assert_eq!(shown, crate::layout::Display::None, "gone at the end");
}

/// C12-BEHAVIOR, CSS Position 4 §3.3–§3.4: a hidden popover whose
/// `overlay` transitions (`allow-discrete`) waits in the top layer —
/// painted there, no longer `:popover-open` — until the transition ends;
/// one that does not leaves at once.
#[test]
fn an_overlay_transition_keeps_a_hidden_popover_in_the_top_layer() {
    use crate::runtime::builtins::popover;
    for (transition, waits) in [
        (
            "transition: overlay 100ms allow-discrete, display 100ms allow-discrete",
            true,
        ),
        ("", false),
    ] {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let pop = dom.create_element("div");
        dom.set_attribute(pop, "popover", "").unwrap();
        let t = dom.create_text_node("pop");
        dom.append_child(pop, t).unwrap();
        dom.append_child(root, pop).unwrap();
        let sheet = rdom_css::parse(&format!("[popover] {{ {transition} }}"));
        assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
        let terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
        let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
        app.push_stylesheet(sheet.stylesheet);
        app.advance(0).unwrap();
        popover::show_popover(app.dom_mut(), pop).unwrap();
        // The frame that cascades it starts its entry transitions; let
        // them end.
        app.advance(0).unwrap();
        app.advance(200).unwrap();
        popover::hide_popover(app.dom_mut(), pop).unwrap();
        app.advance(0).unwrap();
        app.advance(50).unwrap();
        assert_eq!(
            app.dom().is_in_top_layer(pop),
            waits,
            "{transition:?} half-way"
        );
        assert!(!app.dom().matches(pop, ":popover-open").unwrap());
        app.advance(60).unwrap();
        app.advance(0).unwrap();
        assert!(!app.dom().is_in_top_layer(pop), "{transition:?} at the end");
    }
}

// ── C12G-COMPUTED-DOCS: the base computed style on node handles ──────

/// Web Animations 1 §5.4.5: a node's computed style holds the running
/// transitions' and animations' values; its *base* value — the style
/// before the effect stack — is `base_computed()`. Mid-flight the two
/// differ; with nothing running they are one style.
#[test]
fn base_computed_is_the_style_under_the_running_values() {
    use crate::TuiNodeExt;
    let (mut app, div, _) = app("@keyframes w { from { width: 2 } to { width: 10 } } \
         #a { height: 2; transition: height 100ms linear } #a.tall { height: 10 } \
         #a.spin { animation: w 100ms linear }");
    app.dom_mut()
        .set_attribute(div, "class", "tall spin")
        .unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    let node = app.dom().node(div);
    let (now, base) = (node.computed().unwrap(), node.base_computed().unwrap());
    use crate::layout::Size;
    assert_eq!(
        (now.height.clone(), now.width.clone()),
        (Size::Fixed(6), Size::Fixed(6)),
        "running values"
    );
    assert_eq!(
        base.height,
        Size::Fixed(10),
        "the transition's end, the cascade's value"
    );
    assert_eq!(base.width, Size::Auto, "no animation in the base");
    app.advance(100).unwrap();
    let node = app.dom().node(div);
    assert!(std::ptr::eq(
        node.computed().unwrap(),
        node.base_computed().unwrap()
    ));
}

/// A restyle that leaves an element's cascaded style as it was, while a
/// transition runs on it, keeps the base style it had — the same
/// allocation — so the transition hook does not diff it again
/// (C12G-CARRYOVER: architect N3's `keep_cascaded`).
#[test]
fn an_unchanged_restyle_keeps_the_base_style() {
    let (mut app, div, _) =
        app("#a { height: 2; transition: height 100ms linear } #a.tall { height: 10 }");
    app.dom_mut().set_attribute(div, "class", "tall").unwrap();
    app.advance(0).unwrap();
    app.advance(30).unwrap();
    let base = |app: &App<TestBackend>| {
        app.dom()
            .node(div)
            .ext()
            .unwrap()
            .base_computed_for(crate::ext::StyleSlot::Host)
            .cloned()
            .unwrap()
    };
    let before = base(&app);
    app.dom_mut().set_attribute(div, "data-x", "1").unwrap();
    app.advance(16).unwrap();
    assert!(
        std::rc::Rc::ptr_eq(&before, &base(&app)),
        "a new base was kept"
    );
    assert_eq!(
        rect(&app, div).height,
        6,
        "the transition runs on (46 ms: 5.68)"
    );
}
