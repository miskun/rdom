//! C12-KEYFRAMES — animation events (CSS Animations 1 §5, CSS Animations
//! 2 §4.2) and the frame cost of a running animation.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{ListenerOptions, NodeId};

use super::keyframes_tests::animated;
use crate::render::TestBackend;
use crate::runtime::app::App;

type Log = Rc<RefCell<Vec<(String, String, f64, Option<String>)>>>;

/// Record every animation event reaching `node` (its own and, bubbling,
/// its descendants'): type, name, `elapsedTime`, `pseudoElement`.
fn record(app: &mut App<TestBackend>, node: NodeId) -> Log {
    let log: Log = Rc::default();
    for name in [
        "animationstart",
        "animationiteration",
        "animationend",
        "animationcancel",
    ] {
        let log = log.clone();
        app.dom_mut()
            .add_event_listener(node, name, ListenerOptions::default(), move |ctx| {
                let a = ctx.event.detail.as_animation().expect("typed detail");
                log.borrow_mut().push((
                    ctx.event.event_type.clone(),
                    a.animation_name.clone(),
                    (a.elapsed * 1000.0).round() / 1000.0,
                    a.pseudo_element.clone(),
                ));
            })
            .unwrap();
    }
    log
}

fn kinds(log: &Log) -> Vec<(String, f64)> {
    log.borrow().iter().map(|e| (e.0.clone(), e.2)).collect()
}

const GROW: &str = "@keyframes grow { from { width: 2 } to { width: 10 } } ";

/// CSS Animations 2 §4.2: `animationstart` when the active phase begins
/// (after the delay), `animationiteration` at each new iteration, with
/// the iterations' time as `elapsedTime`, and `animationend` at the end.
#[test]
fn start_iteration_and_end_fire_in_order() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a.on {{ animation: grow 100ms linear 50ms 2 }}"
    ));
    let log = record(&mut app, div);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert!(kinds(&log).is_empty(), "in the delay");
    app.advance(60).unwrap();
    app.advance(100).unwrap();
    app.advance(100).unwrap();
    assert_eq!(
        kinds(&log),
        [
            ("animationstart".to_string(), 0.0),
            ("animationiteration".to_string(), 0.1),
            ("animationend".to_string(), 0.2)
        ]
    );
    assert!(log.borrow().iter().all(|e| e.1 == "grow" && e.3.is_none()));
}

/// §4.2: a negative delay starts part-way — `animationstart`'s
/// `elapsedTime` is the time skipped; a jump past the whole run in one
/// frame fires both `animationstart` and `animationend`.
#[test]
fn a_negative_delay_and_a_skipped_run() {
    let (mut app, div) = animated(&format!(
        "{GROW} @keyframes once {{ to {{ width: 3 }} }} \
         #a.on {{ animation: grow 100ms linear -40ms }} #a.zero {{ animation: once 0s }}"
    ));
    let log = record(&mut app, div);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(kinds(&log), [("animationstart".to_string(), 0.04)]);
    log.borrow_mut().clear();
    app.dom_mut().set_attribute(div, "class", "zero").unwrap();
    app.advance(0).unwrap();
    assert_eq!(
        kinds(&log),
        [
            ("animationcancel".to_string(), 0.04),
            ("animationstart".to_string(), 0.0),
            ("animationend".to_string(), 0.0)
        ]
    );
}

/// §4.2: an animation removed while running — its name dropped, its
/// element hidden — fires `animationcancel` with the active time it
/// reached.
#[test]
fn removal_mid_run_cancels() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ animation: grow 100ms linear }} #a.off {{ display: none }}"
    ));
    let log = record(&mut app, div);
    app.advance(30).unwrap();
    app.dom_mut().set_attribute(div, "class", "off").unwrap();
    app.advance(0).unwrap();
    let got = kinds(&log);
    assert_eq!(got.last(), Some(&("animationcancel".to_string(), 0.03)));
}

/// §5.1: an animation of a `::before` names it in `pseudoElement`; the
/// event is dispatched at the element.
#[test]
fn a_pseudo_elements_events_name_it() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a::before {{ content: 'x'; animation: grow 50ms linear }}"
    ));
    let log = record(&mut app, div);
    app.advance(60).unwrap();
    let pseudo: Vec<_> = log.borrow().iter().map(|e| e.3.clone()).collect();
    assert!(!pseudo.is_empty());
    assert!(pseudo.iter().all(|p| p.as_deref() == Some("::before")));
}

/// §4.2: events of one frame go out by time, then composite order — the
/// element's animations in `animation-name` order.
#[test]
fn one_frames_events_follow_composite_order() {
    let (mut app, div) = animated(
        "@keyframes a { to { width: 3 } } @keyframes b { to { width: 4 } } \
         #a.on { animation: b 10ms, a 10ms }",
    );
    let log = record(&mut app, div);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    let names: Vec<_> = log
        .borrow()
        .iter()
        .map(|e| (e.0.clone(), e.1.clone()))
        .collect();
    assert_eq!(
        names,
        [
            ("animationstart".to_string(), "b".to_string()),
            ("animationstart".to_string(), "a".to_string())
        ]
    );
}

/// Cost: an infinite animation costs one composite per frame, and a
/// layout only when it moves a property layout reads — a background
/// color is painted without one; nothing runs once the page is idle.
#[test]
fn an_infinite_animation_costs_a_composite_per_frame_and_layout_only_for_geometry() {
    let (mut app, _) = animated(
        "@keyframes pulse { from { background-color: rgb(0,0,0) } to { background-color: rgb(200,0,0) } } \
         #a { animation: pulse 100ms linear infinite }",
    );
    app.advance(16).unwrap();
    app.take_frame_stats();
    for _ in 0..4 {
        app.advance(16).unwrap();
    }
    let paint_only = app.take_frame_stats();
    assert_eq!(
        (paint_only.paints, paint_only.layouts, paint_only.composites),
        (4, 0, 4),
        "{paint_only:?}"
    );
    assert_eq!(paint_only.full_cascades + paint_only.subtree_cascades, 0);

    let (mut app, _) = animated(
        "@keyframes h { from { height: 1 } to { height: 5 } } \
         #a { animation: h 100ms linear infinite }",
    );
    app.advance(16).unwrap();
    app.take_frame_stats();
    for _ in 0..4 {
        app.advance(16).unwrap();
    }
    let geometry = app.take_frame_stats();
    assert_eq!(
        (geometry.paints, geometry.layouts, geometry.composites),
        (4, 4, 4),
        "{geometry:?}"
    );

    let (mut app, _) = animated(&format!("{GROW} #a {{ animation: grow 20ms linear }}"));
    app.advance(30).unwrap();
    app.advance(16).unwrap();
    app.take_frame_stats();
    app.advance(16).unwrap();
    let idle = app.take_frame_stats();
    assert_eq!(
        (idle.paints, idle.layouts, idle.composites),
        (0, 0, 0),
        "{idle:?}"
    );
}

/// A `::details-content` box animates as the element it is, and its
/// events go to the `<details>`, naming the pseudo-element (CSS
/// Animations 1 §5.1), as its transitions' do.
#[test]
fn a_details_content_animation_reports_to_its_details() {
    use crate::render::Terminal;
    use crate::style::Stylesheet;
    let mut dom: crate::TuiDom = crate::TuiDom::new();
    let root = dom.root();
    let details = dom.create_element("details");
    dom.set_attribute(details, "open", "").unwrap();
    let summary = dom.create_element("summary");
    let s = dom.create_text_node("s");
    dom.append_child(summary, s).unwrap();
    let p = dom.create_element("p");
    let x = dom.create_text_node("x");
    dom.append_child(p, x).unwrap();
    dom.append_child(details, summary).unwrap();
    dom.append_child(details, p).unwrap();
    dom.append_child(root, details).unwrap();
    let sheet = rdom_css::parse(
        "@keyframes fade { from { opacity: 0 } to { opacity: 1 } } \
         details::details-content { animation: fade 50ms }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    let log = record(&mut app, details);
    app.advance(0).unwrap();
    app.advance(60).unwrap();
    let got: Vec<_> = log
        .borrow()
        .iter()
        .map(|e| (e.0.clone(), e.3.clone()))
        .collect();
    let content = Some("::details-content".to_string());
    assert_eq!(
        got,
        [
            ("animationstart".to_string(), content.clone()),
            ("animationend".to_string(), content)
        ]
    );
}
