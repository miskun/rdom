//! C12G-FRAME-COST — what a running animation costs per frame (architect
//! N3, N4; API N9): a frame lays out only when a value layout reads
//! changed, a stepped animation wakes only at its steps, an animation of
//! nothing rdom renders asks for no frames, the transition hook visits
//! only what the cascade restyled, and the layouts of one frame are
//! bounded.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{ListenerOptions, NodeId};

use super::keyframes_tests::animated;
use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

fn needs_frames(app: &App<TestBackend>) -> bool {
    app.animations.needs_frames(app.scheduler.borrow().now())
}

/// `ms` of 16 ms frames.
fn run(app: &mut App<TestBackend>, ms: u64) {
    for _ in 0..ms / 16 {
        app.advance(16).unwrap();
    }
}

const BLINK: &str = "@keyframes blink { 50% { visibility: hidden } } ";

/// CSS Easing 1 §2.3: `step-end` holds each keyframe's value until the
/// next, so `blink 1s step-end` changes twice a second — at 500 ms and at
/// the iteration's end. The app wakes for those, not 60 times a second
/// (as the caret blink wakes at its flips), and each wake composites,
/// lays out and paints once.
#[test]
fn a_stepped_blink_wakes_only_at_its_flips() {
    let (mut app, _) = animated(&format!(
        "{BLINK} #a {{ animation: blink 1s step-end infinite }}"
    ));
    app.take_frame_stats();
    run(&mut app, 1008);
    let s = app.take_frame_stats();
    assert_eq!((s.paints, s.composites), (2, 2), "{s:?}");
    assert!(s.layouts <= 2, "{s:?}");
}

/// A frame lays out when a value layout reads moved, not because such a
/// longhand is animating: a continuous color pulse beside a stepped
/// `visibility` blink paints every frame and lays out only at the flips.
#[test]
fn layout_follows_a_changed_value_not_an_animated_longhand() {
    let (mut app, _) = animated(&format!(
        "{BLINK} @keyframes pulse {{ from {{ color: rgb(0,0,0) }} to {{ color: rgb(200,0,0) }} }} \
         #a {{ animation: pulse 1s linear infinite, blink 1s step-end infinite }}"
    ));
    app.take_frame_stats();
    run(&mut app, 1008);
    let s = app.take_frame_stats();
    assert!(s.paints >= 60, "{s:?}");
    assert_eq!(s.layouts, 2, "{s:?}");
}

/// An animation of a property rdom does not render — `transform` until
/// Phase 15, an unknown name — has an empty effect: it asks for no
/// frames, and its events still fire on schedule (CSS Animations 2 §4.2).
#[test]
fn an_animation_of_nothing_rendered_needs_no_frames_but_fires_its_events() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "a").unwrap();
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(
        "@keyframes spin { to { transform: rotate(1turn) } } \
         #a { animation: spin 100ms linear 3 }",
    );
    let terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    let log: Rc<RefCell<Vec<(String, u64)>>> = Rc::default();
    for name in ["animationstart", "animationiteration", "animationend"] {
        let log = log.clone();
        let clock = app.scheduler.clone();
        let t0 = clock.borrow().now();
        app.dom_mut()
            .add_event_listener(div, name, ListenerOptions::default(), move |ctx| {
                let at = clock
                    .borrow()
                    .now()
                    .saturating_duration_since(t0)
                    .as_millis();
                log.borrow_mut()
                    .push((ctx.event.event_type.clone(), at as u64));
            })
            .unwrap();
    }
    app.advance(0).unwrap();
    assert!(!needs_frames(&app), "an empty effect asks for no frames");
    app.take_frame_stats();
    for _ in 0..40 {
        app.advance(10).unwrap();
    }
    let s = app.take_frame_stats();
    assert_eq!((s.paints, s.composites, s.layouts), (0, 0, 0), "{s:?}");
    let got = log.borrow().clone();
    assert_eq!(
        got,
        [
            ("animationstart".to_string(), 0),
            ("animationiteration".to_string(), 100),
            ("animationiteration".to_string(), 200),
            ("animationend".to_string(), 300),
        ]
    );
}

/// The transition hook's cost is the cascade's: a class change on one of
/// 300 rows, on a page without a transition, visits that row — not two
/// 300-entry maps.
#[test]
fn the_transition_hook_visits_only_what_the_cascade_restyled() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let rows: Vec<NodeId> = (0..300)
        .map(|_| {
            let r = dom.create_element("div");
            dom.append_child(root, r).unwrap();
            r
        })
        .collect();
    let sheet = rdom_css::parse("div.on { color: red }");
    let terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    crate::runtime::animation::DIFF_VISITS.with(|c| c.set(0));
    app.dom_mut()
        .set_attribute(rows[150], "class", "on")
        .unwrap();
    app.advance(0).unwrap();
    let visits = crate::runtime::animation::DIFF_VISITS.with(std::cell::Cell::get);
    assert!(visits <= 2, "{visits} elements visited");
}

/// CSS Values 5 §10: once no box is `calc-size()`d — the `interpolate-size`
/// transition that made one ended — layout stops paying the second pass
/// and its collecting walk (the document flag is cleared).
#[test]
fn the_calc_size_flag_clears_when_none_remains() {
    let (mut app, div) = animated(
        "#a { interpolate-size: allow-keywords; height: 0; overflow: hidden; \
         transition: height 50ms linear } #a.open { height: auto }",
    );
    app.dom_mut().set_attribute(div, "class", "open").unwrap();
    app.advance(0).unwrap();
    app.advance(20).unwrap();
    assert!(crate::style::doc_flags::has_calc_sizes(app.dom()));
    app.advance(60).unwrap();
    app.advance(0).unwrap();
    assert!(
        !crate::style::doc_flags::has_calc_sizes(app.dom()),
        "no calc-size() is left"
    );
}

/// Architect N4: the most layout passes one frame runs. A frame whose
/// layout makes a focused element scroll into view, whose scroll moves a
/// scroll-driven animation of `width`, on a page with a `calc-size()`d box:
/// the services (re-snap, focus scroll, caret reveal) and the timelines'
/// re-step share one relayout (Scroll-driven Animations 1 §5), so two
/// layouts of two passes each (calc-size) of one round — four phase runs,
/// where the re-step's own relayout made six; the bound is
/// 2 layouts × 2 passes × `MAX_ROUNDS`.
#[test]
fn a_frame_runs_at_most_two_layouts() {
    use crate::render::layout_pass::{MAX_ROUNDS, ROUNDS};
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("div");
    dom.set_attribute(c, "id", "c").unwrap();
    let ct = dom.create_text_node("calc");
    dom.append_child(c, ct).unwrap();
    dom.append_child(root, c).unwrap();
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    let bar = dom.create_element("div");
    dom.set_attribute(bar, "id", "bar").unwrap();
    dom.append_child(s, bar).unwrap();
    let mut last = bar;
    for _ in 0..12 {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
        last = p;
    }
    dom.set_attribute(last, "tabindex", "0").unwrap();
    dom.append_child(root, s).unwrap();
    let sheet = rdom_css::parse(
        "@keyframes fill { from { width: 0 } to { width: 18 } } \
         #c { width: calc-size(max-content, size + 1) } \
         #s { height: 4; overflow: auto } p { margin: 0; height: 1 } \
         #bar { height: 1; animation: fill linear; animation-timeline: scroll() }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.advance(0).unwrap();
    crate::runtime::focus::focus_node(app.dom_mut(), Some(last));
    ROUNDS.with(|r| r.set(0));
    app.advance(0).unwrap();
    let runs = ROUNDS.with(std::cell::Cell::get);
    use crate::TuiAccessors;
    assert!(
        app.dom().node(s).scroll_top().unwrap_or(0) > 0,
        "focus scrolled"
    );
    let w = app.dom().node(bar).ext().unwrap().layout.width;
    assert_eq!(w, 18, "the re-stepped timeline is laid out");
    assert_eq!(runs, 4, "two layouts of two calc-size passes");
    assert!(runs <= 2 * 2 * MAX_ROUNDS);
}

/// Web Animations 1 §4.9.1: a backwards iteration samples its keyframes
/// from the end, so its steps fall mirrored — `25% { hidden }` under
/// `step-end` flips at 250 ms going forwards, and at 750 ms into the
/// reversed second iteration of `alternate` (local 1750 ms), where the
/// app wakes and shows it.
#[test]
fn a_reversed_iteration_wakes_at_its_mirrored_steps() {
    use crate::layout::Visibility;
    let (mut app, div) = animated(
        "@keyframes k { 25% { visibility: hidden } } \
         #a { animation: k 1s step-end infinite alternate }",
    );
    let visibility = |app: &App<TestBackend>| {
        app.dom()
            .node(div)
            .ext()
            .unwrap()
            .computed
            .as_ref()
            .unwrap()
            .visibility
    };
    for _ in 0..(1008 / 16) {
        app.advance(16).unwrap();
    }
    app.take_frame_stats();
    for _ in 0..(744 / 16) {
        app.advance(16).unwrap();
    }
    assert_eq!(visibility(&app), Visibility::Hidden, "hidden from 1000 ms");
    app.advance(16).unwrap();
    assert_eq!(
        visibility(&app),
        Visibility::Visible,
        "visible from 1750 ms"
    );
    let s = app.take_frame_stats();
    assert_eq!(s.paints, 1, "{s:?}");
}

/// CSS Easing 1 §2.3, CSS Transitions 1 §3: a `steps(4)` transition holds
/// each step's value, so over its 400 ms it changes at 100, 200, 300 and
/// 400 ms — the app wakes and paints at those steps, as a stepped CSS
/// animation does, not every frame (C12G-CARRYOVER).
#[test]
fn a_stepped_transition_paints_only_at_its_steps() {
    let (mut app, div) =
        animated("#a { width: 2; transition: width 400ms steps(4) } #a.wide { width: 10 }");
    app.dom_mut().set_attribute(div, "class", "wide").unwrap();
    app.advance(0).unwrap();
    app.take_frame_stats();
    run(&mut app, 256);
    assert_eq!(
        super::keyframes_tests::width(&app, div),
        6,
        "the second step"
    );
    run(&mut app, 256);
    let s = app.take_frame_stats();
    assert_eq!(s.paints, 4, "{s:?}");
    assert_eq!(super::keyframes_tests::width(&app, div), 10);
    assert!(!needs_frames(&app));
}

/// C14G-COST-PINS (architect N1, N21): the worst case TECH_DEBT
/// `ANIM-RELAYOUT-1` states for the layouts of one frame, constructed. A
/// frame runs at most four `layout_dom`s — its layout, the in-frame
/// transition hook's relayout after it (C14G-CONTAINER-FIDELITY), the
/// services' shared relayout, and the hook's relayout after that — and
/// each runs at most `1 + MAX_PASSES` layouts while query containers'
/// sizes move (CSS Conditional 5 §6.4). Here: 40 nested containers, each
/// as wide as its container less one (`100cqw - 1`), so each pass settles
/// one level; a resize by one column moves the outermost, so every level
/// moves; each re-cascaded level's `padding-bottom` (the parity of its
/// container's width) flips under a `step-end` transition, so the hook's
/// composite moves a value layout reads; and a row focused in a scroller
/// needs a focus scroll. No positioned box and no `calc-size()`, so each
/// layout is one run of phases 1–2: 4 × 9 = 36.
#[test]
fn a_frame_runs_at_most_four_layouts_of_the_container_cap() {
    use crate::render::layout_pass::ROUNDS;
    use crate::render::layout_pass::container_pass::MAX_PASSES;
    let mut markup = String::from(r#"<div class="list">"#);
    for i in 0..6 {
        markup.push_str(&format!(r#"<div class="row" id="r{i}">x</div>"#));
    }
    markup.push_str("</div>");
    markup.push_str(&r#"<div class="c">"#.repeat(40));
    markup.push('x');
    markup.push_str(&"</div>".repeat(40));
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, &markup, root).unwrap();
    let last = dom.get_element_by_id("r5").unwrap();
    dom.set_attribute(last, "tabindex", "0").unwrap();
    let sheet = rdom_css::parse(
        ".list { overflow-y: auto; height: 2; width: 8 } .row { height: 1 } \
         .c { container-type: inline-size; width: calc(100cqw - 1); \
              padding-bottom: mod(100cqw, 2); transition: padding-bottom 1s step-end }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    // Settle the chain: 40 levels at 8 a layout.
    for _ in 0..20 {
        app.advance(2000).unwrap();
    }
    crate::runtime::focus::focus_node(app.dom_mut(), Some(last));
    app.terminal_mut().backend_mut().resize(79, 10);
    app.handle_event(crossterm::event::Event::Resize(79, 10));
    app.take_frame_stats();
    ROUNDS.with(|r| r.set(0));
    app.draw_if_dirty().unwrap();
    let layouts = ROUNDS.with(std::cell::Cell::get);
    use crate::TuiAccessors;
    let list = app.dom().node(root).first_child().unwrap().id();
    assert!(
        app.dom().node(list).scroll_top().unwrap_or(0) > 0,
        "focus scrolled"
    );
    assert_eq!(layouts, 4 * (1 + MAX_PASSES), "four layout_doms at the cap");
}
