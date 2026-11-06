//! The session's contract at its edges: the virtual clock from the first
//! frame, ticks and quit requests, resizes in `meta`, and app code or
//! requests that fail without taking a working app down.

use std::error::Error;
use std::time::Duration;

use rdom_tui::render::TestBackend;
use rdom_tui::{App, AppContext, ControlFlow, TuiDom};

use crate::capture::{ColorValue, Layer};
use crate::env::{Environment, Viewport};
use crate::input::Input;
use crate::session::{CaptureRequest, MAX_ADVANCE_MS, Session, SessionError};
use crate::{CaptureApp, FixtureInfo};

/// A box whose background transitions from red to blue over 100 ms,
/// started while the app builds; fixture "quits" asks to stop on its
/// second tick (a tick every 50 ms).
struct Fading;

const CSS: &str = "div { background: #ff0000; transition: background 100ms linear; height: 1; }
                   div.on { background: #0000ff; }";

impl CaptureApp for Fading {
    fn build(
        &self,
        env: &Environment,
        fixture: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div)?;
        let mut app = env.app(dom, rdom_css::from_css(CSS))?;
        // The build draws, starts the transition, takes a while and draws
        // again: on the wall clock the second frame would show it
        // under way by however long the build took.
        app.draw_if_dirty()?;
        app.dom_mut().set_attribute(div, "class", "on")?;
        app.draw_if_dirty()?;
        std::thread::sleep(Duration::from_millis(5));
        app.draw_if_dirty()?;
        if fixture == Some("quits") {
            let mut ticks = 0;
            app = app
                .with_tick_rate(Duration::from_millis(50))
                .with_tick_handler(move |_: &mut AppContext<'_>| {
                    ticks += 1;
                    if ticks >= 2 {
                        ControlFlow::Quit
                    } else {
                        ControlFlow::Continue
                    }
                });
        }
        Ok(app)
    }
}

fn env() -> Environment {
    Environment::default().with_viewport(4, 1)
}

fn bg(s: &Session<impl CaptureApp>) -> ColorValue {
    let c = s
        .capture(&CaptureRequest::with_layers([Layer::Grid]))
        .unwrap();
    c.grid.unwrap().rows[0][0].bg
}

#[test]
fn a_frame_drawn_while_building_reads_no_wall_time() {
    let mut s = Session::new(Fading);
    s.open(env(), None).unwrap();
    assert_eq!(
        bg(&s),
        ColorValue::Rgb(255, 0, 0),
        "the transition has not moved"
    );
    s.advance(50).unwrap();
    assert_eq!(bg(&s), ColorValue::Rgb(128, 0, 128), "halfway at 50 ms");
    s.advance(50).unwrap();
    assert_eq!(bg(&s), ColorValue::Rgb(0, 0, 255));
}

#[test]
fn ticks_run_on_the_virtual_clock_and_a_quit_is_reported() {
    let mut s = Session::new(Fading);
    s.open(env(), Some("quits")).unwrap();
    assert!(!s.advance(60).unwrap().quit_requested, "one tick");
    let c = s.advance(40).unwrap();
    assert!(c.quit_requested, "the second tick asked to stop");
    assert!(s.is_open(), "the session keeps the app");
}

#[test]
fn a_resize_is_recorded_in_the_captures_environment() {
    let mut s = Session::new(Fading);
    s.open(env(), None).unwrap();
    s.input(&[Input::Resize {
        width: 9,
        height: 3,
    }])
    .unwrap();
    let c = s.capture(&CaptureRequest::default()).unwrap();
    assert_eq!(
        c.meta.env.viewport,
        Viewport {
            width: 9,
            height: 3
        }
    );
}

#[test]
fn an_advance_past_the_bound_is_refused_and_the_app_stays_open() {
    let mut s = Session::new(Fading);
    s.open(env(), None).unwrap();
    assert!(matches!(
        s.advance(MAX_ADVANCE_MS + 1),
        Err(SessionError::InvalidInput(_))
    ));
    assert!(s.is_open());
    s.advance(MAX_ADVANCE_MS).unwrap();
}

#[test]
fn a_failed_open_leaves_the_open_app_open() {
    let mut s = Session::new(Fading);
    s.open(env(), None).unwrap();
    s.advance(30).unwrap();
    let mut bad = env();
    bad.viewport = Viewport {
        width: 0,
        height: 0,
    };
    assert!(matches!(
        s.open(bad, None),
        Err(SessionError::InvalidEnvironment(_))
    ));
    assert!(s.is_open());
    assert_eq!(
        s.capture(&CaptureRequest::default()).unwrap().meta.time_ms,
        30
    );
}

/// An app whose fixture list panics.
struct BadList;

impl CaptureApp for BadList {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        Ok(env.app(TuiDom::new(), rdom_tui::Stylesheet::new())?)
    }
    fn fixtures(&self) -> Vec<FixtureInfo> {
        panic!("no list today")
    }
}

#[test]
fn a_panic_listing_fixtures_is_an_error_not_a_crash() {
    let mut s = Session::new(BadList);
    assert!(
        matches!(s.fixtures(), Err(SessionError::AppPanicked(m)) if m.contains("no list today"))
    );
    assert!(matches!(
        s.open(env(), None),
        Err(SessionError::AppPanicked(_))
    ));
}
