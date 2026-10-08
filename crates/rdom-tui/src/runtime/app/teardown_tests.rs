//! C12G — what happens to running transitions and animations when their
//! box goes away: an element leaving the document (C12G-DETACHED), a
//! `::before` / `::after` that stops generating (C12G-PSEUDO-GONE).
//!
//! Web Animations 1 §5.6 / CSS Animations 1 §4.1 / CSS Transitions 1 §3:
//! an element that is no longer rendered — disconnected, or a
//! pseudo-element with no box — has its CSS animations and transitions
//! cancelled (`animationcancel` / `transitioncancel`), and a re-rendered
//! one starts afresh.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{ListenerOptions, NodeId};

use super::keyframes_tests::{animated, width};
use crate::render::TestBackend;
use crate::runtime::app::App;

type Log = Rc<RefCell<Vec<String>>>;

/// Record the transition and animation events reaching `node`.
fn record(app: &mut App<TestBackend>, node: NodeId) -> Log {
    let log: Log = Rc::default();
    for name in [
        "transitionrun",
        "transitionstart",
        "transitionend",
        "transitioncancel",
        "animationstart",
        "animationiteration",
        "animationend",
        "animationcancel",
    ] {
        let log = log.clone();
        app.dom_mut()
            .add_event_listener(node, name, ListenerOptions::default(), move |ctx| {
                log.borrow_mut().push(ctx.event.event_type.clone());
            })
            .unwrap();
    }
    log
}

fn needs_frames(app: &App<TestBackend>) -> bool {
    app.animations.needs_frames(app.scheduler.borrow().now())
}

fn has(log: &Log, kind: &str) -> bool {
    log.borrow().iter().any(|e| e == kind)
}

// ── C12G-DETACHED ────────────────────────────────────────────────

const SPIN: &str = "@keyframes spin { from { width: 2 } to { width: 10 } } \
                    #a { animation: spin 1s steps(4) infinite }";

/// CSS Animations 1 §4.1 ("an element ... removed from the document"
/// stops running its animations, Web Animations 1 §5.6 cancels them): an
/// infinite spinner row taken out with `remove_child` — detached, not
/// dropped — fires `animationcancel`, and the idle app asks for no
/// frames.
#[test]
fn a_detached_spinner_is_cancelled_and_needs_no_frames() {
    let (mut app, div) = animated(SPIN);
    let log = record(&mut app, div);
    app.advance(100).unwrap();
    let wake = |app: &App<TestBackend>| app.animations.next_wake(app.scheduler.borrow().now());
    assert!(
        wake(&app).is_some(),
        "the spinner wakes the app at its steps"
    );
    let root = app.dom().root();
    app.dom_mut().remove_child(root, div).unwrap();
    app.advance(16).unwrap();
    assert!(has(&log, "animationcancel"), "{:?}", log.borrow());
    assert!(!needs_frames(&app), "a detached spinner asks for no frames");
    assert!(wake(&app).is_none(), "nor wakes the app");
    app.take_frame_stats();
    app.advance(16).unwrap();
    let idle = app.take_frame_stats();
    assert_eq!((idle.paints, idle.composites), (0, 0), "{idle:?}");
    assert!(app.animations.is_empty());
}

/// CSS Transitions 1 §3: a transition of an element that is no longer
/// rendered is cancelled — `transitioncancel`, no more frames.
#[test]
fn a_transition_removed_mid_run_is_cancelled() {
    let (mut app, div) =
        animated("#a { width: 2; transition: width 100ms linear } #a.wide { width: 10 }");
    let log = record(&mut app, div);
    app.dom_mut().set_attribute(div, "class", "wide").unwrap();
    app.advance(0).unwrap();
    app.advance(30).unwrap();
    assert!(needs_frames(&app));
    let root = app.dom().root();
    app.dom_mut().remove_child(root, div).unwrap();
    app.advance(16).unwrap();
    assert!(has(&log, "transitioncancel"), "{:?}", log.borrow());
    assert!(!has(&log, "transitionend"), "{:?}", log.borrow());
    assert!(!needs_frames(&app));
    assert!(app.animations.is_empty());
}

/// §3: an element inserted again has no before-change style — its new
/// values apply at once, rather than transitioning from the style it had
/// before it left (architect N2b).
#[test]
fn a_reinserted_element_starts_fresh() {
    let (mut app, div) =
        animated("#a { width: 2; transition: width 100ms linear } #a.wide { width: 10 }");
    app.dom_mut().set_attribute(div, "class", "wide").unwrap();
    app.advance(0).unwrap();
    app.advance(200).unwrap();
    assert_eq!(width(&app, div), 10);
    let root = app.dom().root();
    app.dom_mut().remove_child(root, div).unwrap();
    app.advance(16).unwrap();
    app.dom_mut().set_attribute(div, "class", "").unwrap();
    app.dom_mut().append_child(root, div).unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 2, "no transition from the old style");
    assert!(!needs_frames(&app));
}

/// CSS Animations 1 §4.1: removing and re-inserting in one task still
/// cancels the running animation, and the re-inserted element starts it
/// again from its first keyframe, as browsers do.
#[test]
fn a_spinner_moved_out_and_back_restarts() {
    let (mut app, div) = animated(
        "@keyframes grow { from { width: 2 } to { width: 10 } } \
         #a { animation: grow 100ms linear infinite }",
    );
    let log = record(&mut app, div);
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6);
    let root = app.dom().root();
    app.dom_mut().remove_child(root, div).unwrap();
    app.dom_mut().append_child(root, div).unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 2, "restarted at its first keyframe");
    let got = log.borrow().clone();
    let cancel = got.iter().position(|e| e == "animationcancel");
    let start = got.iter().rposition(|e| e == "animationstart");
    assert!(
        matches!((cancel, start), (Some(c), Some(s)) if c < s),
        "{got:?}"
    );
}

/// DOM §4.2.3: moving an attached element with one `append_child` is a
/// removal and an insertion (insert → adopt → remove), so its running
/// animation is cancelled and restarts from its first keyframe, as
/// browsers restart CSS animations on a DOM move (C12G-MOVE-RECORD).
#[test]
fn a_spinner_moved_with_one_append_restarts() {
    let (mut app, div) = animated(
        "@keyframes grow { from { width: 2 } to { width: 10 } } \
         #a { animation: grow 100ms linear infinite }",
    );
    let root = app.dom().root();
    let host = app.dom_mut().create_element("section");
    app.dom_mut().append_child(root, host).unwrap();
    app.advance(0).unwrap();
    let log = record(&mut app, div);
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6);
    app.dom_mut().append_child(host, div).unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 2, "restarted at its first keyframe");
    let got = log.borrow().clone();
    let cancel = got.iter().position(|e| e == "animationcancel");
    let start = got.iter().rposition(|e| e == "animationstart");
    assert!(
        matches!((cancel, start), (Some(c), Some(s)) if c < s),
        "{got:?}"
    );
}

// ── C12G-PSEUDO-GONE ─────────────────────────────────────────────

/// The first row the app painted, trailing blanks trimmed.
fn first_row(app: &App<TestBackend>) -> String {
    let mut screen = crate::render::VirtualScreen::new(40, 12);
    screen.apply(app.terminal().backend().bytes());
    (0..40)
        .map(|x| screen.cell(x, 0).unwrap().symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn after_style(app: &App<TestBackend>, id: NodeId) -> bool {
    let ext = app.dom().node(id).ext().unwrap();
    ext.computed_for(crate::ext::StyleSlot::After).is_some()
        || ext
            .base_computed_for(crate::ext::StyleSlot::After)
            .is_some()
}

/// CSS Animations 1 §4.1 / CSS Pseudo-Elements 4 §2: a `::after` that
/// stops generating a box (its host lost the class giving it `content`)
/// has no animations — `animationcancel`, no glyph, no frames — and the
/// cascade's style for it is gone, not kept under the animation.
#[test]
fn a_spinner_after_that_stops_generating_is_gone() {
    let (mut app, div) = animated(
        "@keyframes spin { from { color: rgb(255,0,0) } to { color: rgb(0,0,255) } } \
         #a.s::after { content: '*'; animation: spin 1s infinite }",
    );
    let log = record(&mut app, div);
    app.dom_mut().set_attribute(div, "class", "s").unwrap();
    app.advance(0).unwrap();
    app.advance(100).unwrap();
    assert_eq!(first_row(&app), "x*");
    assert!(needs_frames(&app));
    app.dom_mut().set_attribute(div, "class", "").unwrap();
    app.advance(16).unwrap();
    assert!(has(&log, "animationcancel"), "{:?}", log.borrow());
    assert!(!after_style(&app, div), "no ::after style left");
    assert_eq!(first_row(&app), "x", "no glyph");
    assert!(!needs_frames(&app), "no frames");
    app.advance(500).unwrap();
    assert_eq!(first_row(&app), "x");
}

/// CSS Transitions 1 §3: a tooltip `::after` whose content goes (the
/// pointer left) while its color transitions is no longer rendered — the
/// transition is cancelled and the tip does not come back from the style
/// it had.
#[test]
fn a_tooltip_after_leaving_mid_transition_is_gone() {
    let (mut app, div) = animated(
        "#a::after { color: rgb(255,0,0); transition: color 200ms linear } \
         #a.on::after { content: 'tip' } #a.on.hot::after { color: rgb(0,0,255) }",
    );
    let log = record(&mut app, div);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.dom_mut().set_attribute(div, "class", "on hot").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert!(has(&log, "transitionstart"), "{:?}", log.borrow());
    assert_eq!(first_row(&app), "xtip");
    app.dom_mut().set_attribute(div, "class", "").unwrap();
    app.advance(16).unwrap();
    assert!(has(&log, "transitioncancel"), "{:?}", log.borrow());
    assert!(!after_style(&app, div));
    assert_eq!(first_row(&app), "x");
    assert!(!needs_frames(&app));
    app.advance(300).unwrap();
    assert_eq!(first_row(&app), "x", "the stale style is not put back");
    assert!(!has(&log, "transitionend"), "{:?}", log.borrow());
}
