//! ACID-FIX-15 — HTML §8.1.7.3 "update the rendering": a frame first
//! updates its animations and dispatches their events (Web Animations 1
//! §4.4 "update animations and send events"), then styles, lays out and
//! paints — so what a `transitionstart` or `animationstart` listener
//! changes is drawn in the frame whose time the event happened at, not
//! one frame later.

use rdom_core::{ListenerOptions, NodeId};

use super::keyframes_tests::animated;
use crate::render::{TestBackend, VirtualScreen};
use crate::runtime::app::App;

/// The glyph the terminal shows at `(x, y)` of the app's 40 × 12 screen.
fn glyph(app: &App<TestBackend>, x: u16, y: u16) -> String {
    let mut screen = VirtualScreen::new(40, 12);
    screen.apply(app.terminal().backend().bytes());
    screen.cell(x, y).expect("on screen").symbol().to_string()
}

/// A listener for `event` on `#a` that writes `s` into its `<span>`.
fn mark_on(app: &mut App<TestBackend>, div: NodeId, event: &str) {
    let span = app.dom().node(div).first_child().expect("the span").id();
    app.dom_mut()
        .add_event_listener(div, event, ListenerOptions::default(), move |ctx| {
            ctx.dom.set_text_content(span, "s").unwrap();
        })
        .unwrap();
}

/// CSS Transitions 1 §6: `transitionstart` when the delay ends; its
/// listener's change is in the frame at that time.
#[test]
fn a_transitionstart_listeners_change_is_drawn_in_its_frame() {
    let (mut app, div) = animated(
        "#a { color: rgb(0, 0, 0); transition: color 100ms linear 50ms } \
         #a.on { color: rgb(200, 0, 0) }",
    );
    mark_on(&mut app, div, "transitionstart");
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(25).unwrap();
    assert_eq!(glyph(&app, 0, 0), "x", "in the delay");
    app.advance(25).unwrap();
    assert_eq!(glyph(&app, 0, 0), "s", "the frame the delay ends");
}

/// CSS Animations 2 §4.2: `animationstart` when the active phase
/// begins; its listener's change is in the frame at that time.
#[test]
fn an_animationstart_listeners_change_is_drawn_in_its_frame() {
    let (mut app, div) =
        animated("@keyframes k { to { width: 3 } } #a.on { animation: k 100ms linear 50ms }");
    mark_on(&mut app, div, "animationstart");
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(25).unwrap();
    assert_eq!(glyph(&app, 0, 0), "x", "in the delay");
    app.advance(25).unwrap();
    assert_eq!(glyph(&app, 0, 0), "s", "the frame the delay ends");
}
