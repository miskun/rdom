//! CAPTURE (`specs/CAPTURE.md` §Deterministic environment): an App on a
//! virtual clock reads no wall time — not even in a frame drawn before
//! its first `advance` — and `advance` runs what the live loop runs while
//! no input arrives: a tick per elapsed tick period, and the `AppHandle`'s
//! redraw and quit requests.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use super::{App, AppContext, ControlFlow};
use crate::render::{Terminal, TestBackend};
use crate::{Stylesheet, TuiDom};

fn app() -> App<TestBackend> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.append_child(root, p).unwrap();
    let terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
    App::with_backend(dom, Stylesheet::bare(), terminal).unwrap()
}

#[test]
fn a_virtual_clock_does_not_move_when_a_frame_draws() {
    let mut app = app().with_virtual_clock();
    let start = app.scheduler.borrow().now();
    // Long enough that a wall clock would have moved.
    std::thread::sleep(Duration::from_millis(2));
    app.draw_if_dirty().unwrap();
    assert_eq!(
        app.scheduler.borrow().now(),
        start,
        "the frame read no wall time"
    );
    app.advance(30).unwrap();
    assert_eq!(
        app.scheduler.borrow().now(),
        start + Duration::from_millis(30)
    );
}

#[test]
fn advance_ticks_once_per_elapsed_tick_period() {
    let ticks = Rc::new(Cell::new(0));
    let t = Rc::clone(&ticks);
    let mut app = app()
        .with_virtual_clock()
        .with_tick_rate(Duration::from_millis(50))
        .with_tick_handler(move |_: &mut AppContext<'_>| {
            t.set(t.get() + 1);
            ControlFlow::Continue
        });
    app.advance(0).unwrap();
    assert_eq!(ticks.get(), 0, "no tick period has passed");
    app.advance(120).unwrap();
    assert_eq!(ticks.get(), 2, "ticks at 50 and 100");
    app.advance(30).unwrap();
    assert_eq!(ticks.get(), 3, "and at 150");
    app.advance(49).unwrap();
    assert_eq!(ticks.get(), 3, "199 is before the next");
}

#[test]
fn a_tick_that_quits_is_reported() {
    let mut app = app()
        .with_virtual_clock()
        .with_tick_rate(Duration::from_millis(10))
        .with_tick_handler(|_: &mut AppContext<'_>| ControlFlow::Quit);
    app.advance(5).unwrap();
    assert!(!app.quit_requested());
    app.advance(5).unwrap();
    assert!(app.quit_requested());
}

#[test]
fn advance_takes_the_handles_redraw_and_quit_requests() {
    let mut app = app().with_virtual_clock();
    app.advance(0).unwrap();
    let handle = app.handle();
    handle.request_redraw();
    app.advance(0).unwrap();
    assert!(!app.needs_redraw(), "the requested frame was drawn");
    assert!(!app.quit_requested());
    handle.quit();
    app.advance(0).unwrap();
    assert!(app.quit_requested());
}
