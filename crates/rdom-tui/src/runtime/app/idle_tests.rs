//! `P7G-IDLE-WALKS-1`: an idle loop iteration — no event, no timer, no
//! injected closure, no `dom_mut()` access — walks no tree: the
//! per-frame checks that do (scroll offsets moved since paint, smooth
//! scrolls in flight, `:valid` / `:invalid` marks) run only after code
//! that may have changed what they look at. Validity restyles still
//! follow a typed value, a `set_custom_validity` from a timer and a
//! radio checked from an injected closure.

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
use rdom_core::{ListenerOptions, NodeId, Position, Selection};

use super::redraw::FrameStats;
use crate::TuiDom;
use crate::accessors::TuiAccessorsMut;
use crate::layout::{Display, Overflow, Size};
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::{App, AppContext};
use crate::runtime::timers::TuiTimers;
use crate::style::cascade::computed_of;
use crate::style::{Stylesheet, TuiStyle};

const RED: Color = Color::Rgb(255, 0, 0);

fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

/// A scroll pane, and a form with a required text field `name`, a
/// required radio group `a` / `b` and a plain field `other`, under
/// `:invalid` rules; one frame drawn.
struct Page {
    app: App<TestBackend>,
    name: NodeId,
    a: NodeId,
    b: NodeId,
    other: NodeId,
}

fn page() -> Page {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pane = el(&mut dom, root, "div", &[("id", "pane")]);
    for i in 0..20 {
        let p = el(&mut dom, pane, "p", &[]);
        let t = dom.create_text_node(&format!("line {i}"));
        dom.append_child(p, t).unwrap();
    }
    let form = el(&mut dom, root, "form", &[]);
    let name = el(&mut dom, form, "input", &[("required", "")]);
    let a = el(
        &mut dom,
        form,
        "input",
        &[("type", "radio"), ("name", "g"), ("required", "")],
    );
    let b = el(&mut dom, form, "input", &[("type", "radio"), ("name", "g")]);
    let other = el(&mut dom, form, "input", &[("value", "ok")]);
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "#pane",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(3))
                .overflow_y(Overflow::Auto),
        )
        .rule_unchecked("input:invalid", TuiStyle::new().fg(RED));
    let terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    Page {
        app,
        name,
        a,
        b,
        other,
    }
}

#[test]
fn an_idle_tick_walks_no_tree_and_draws_nothing() {
    let mut page = page();
    page.app.advance(0).unwrap();
    page.app.take_frame_stats();
    for _ in 0..3 {
        page.app.advance(50).unwrap();
    }
    assert_eq!(page.app.take_frame_stats(), FrameStats::default());
}

#[test]
fn a_typed_value_still_restyles_invalid() {
    let Page { mut app, name, .. } = page();
    assert_eq!(computed_of(app.dom(), name).fg, RED);
    app.dom_mut().set_focused(Some(name));
    let t = app.dom().node(name).child_nodes().next().unwrap().id();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));
    app.advance(0).unwrap();
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Char('x'),
        KeyModifiers::empty(),
    )));
    app.advance(0).unwrap();
    assert_ne!(computed_of(app.dom(), name).fg, RED);
}

#[test]
fn a_custom_validity_set_from_a_timer_still_restyles_invalid() {
    let Page { mut app, other, .. } = page();
    assert_ne!(computed_of(app.dom(), other).fg, RED);
    let root = app.dom().root();
    app.dom_mut()
        .add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
            ctx.set_timeout(
                move |tctx| {
                    tctx.dom
                        .node_mut(other)
                        .set_custom_validity("taken")
                        .unwrap();
                },
                20,
            );
        })
        .unwrap();
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Char('x'),
        KeyModifiers::empty(),
    )));
    app.advance(0).unwrap();
    assert_ne!(computed_of(app.dom(), other).fg, RED, "not yet");
    app.advance(20).unwrap();
    assert_eq!(computed_of(app.dom(), other).fg, RED);
}

#[test]
fn checking_a_radio_still_restyles_its_whole_group() {
    let Page { mut app, a, b, .. } = page();
    assert_eq!(computed_of(app.dom(), a).fg, RED);
    assert_eq!(computed_of(app.dom(), b).fg, RED);
    app.handle().inject(move |ctx: &mut AppContext<'_>| {
        ctx.dom.node_mut(b).click();
    });
    app.advance(0).unwrap();
    assert_ne!(computed_of(app.dom(), a).fg, RED, "its group is satisfied");
    assert_ne!(computed_of(app.dom(), b).fg, RED);
}

/// `P7G-TICK-TOUCHED-1`: a tick handler that runs and changes nothing —
/// the documented channel-draining pattern on an empty channel — walks
/// no tree and draws nothing.
#[test]
fn an_on_tick_that_changes_nothing_walks_no_tree() {
    let Page { app, .. } = page();
    let mut app = app.with_tick_handler(|_: &mut AppContext<'_>| super::ControlFlow::Continue);
    app.advance(0).unwrap();
    app.take_frame_stats();
    for _ in 0..3 {
        app.tick();
        app.advance(50).unwrap();
    }
    assert_eq!(app.take_frame_stats(), FrameStats::default());
}

/// `P7G-TICK-TOUCHED-1`: an interval that fires and changes nothing
/// leaves the App clean too.
#[test]
fn an_interval_that_changes_nothing_walks_no_tree() {
    let Page { mut app, .. } = page();
    app.scheduler.borrow_mut().set_interval(|_| true, 50);
    app.advance(0).unwrap();
    app.take_frame_stats();
    for _ in 0..3 {
        app.advance(50).unwrap();
    }
    assert_eq!(app.take_frame_stats(), FrameStats::default());
}

/// `P7G-TICK-TOUCHED-1`: a tick handler that mutates still repaints, and
/// one that sets a custom validity (no DOM mutation) still restyles.
#[test]
fn an_on_tick_that_changes_something_still_draws() {
    let Page {
        app, name, other, ..
    } = page();
    let mut step = 0;
    let mut app = app.with_tick_handler(move |ctx: &mut AppContext<'_>| {
        step += 1;
        match step {
            1 => {
                ctx.dom.set_attribute(name, "data-seen", "1").unwrap();
            }
            2 => {
                ctx.dom
                    .node_mut(other)
                    .set_custom_validity("taken")
                    .unwrap();
            }
            _ => {}
        }
        super::ControlFlow::Continue
    });
    app.advance(0).unwrap();
    app.take_frame_stats();
    app.tick();
    app.advance(0).unwrap();
    assert_eq!(app.take_frame_stats().paints, 1, "a mutation repaints");
    assert_ne!(computed_of(app.dom(), other).fg, RED);
    app.tick();
    app.advance(0).unwrap();
    assert_eq!(
        computed_of(app.dom(), other).fg,
        RED,
        "a custom validity from a tick handler restyles :invalid"
    );
}
