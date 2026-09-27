//! `P7-SCROLL-REPAINT-1`: a scroll offset change repaints on the next
//! frame with no `request_redraw`, whoever wrote it — an injected
//! closure, a timer, or a direct `TuiExt` write — as any scroll does in
//! a browser; an unchanged offset draws nothing.

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::accessors::TuiAccessorsMut;
use crate::layout::{Display, Overflow, Size};
use crate::render::{Terminal, TestBackend, VirtualScreen};
use crate::runtime::app::{App, AppContext};
use crate::runtime::timers::TuiTimers;
use crate::style::{Stylesheet, TuiStyle};

/// A 5-row focusable scroll pane of 20 one-row lines, drawn once.
fn pane_app() -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pane = dom.create_element("div");
    dom.set_attribute(pane, "id", "pane").unwrap();
    dom.set_attribute(pane, "tabindex", "0").unwrap();
    dom.append_child(root, pane).unwrap();
    for i in 0..20 {
        let line = dom.create_element("p");
        let t = dom.create_text_node(&format!("line {i}"));
        dom.append_child(line, t).unwrap();
        dom.append_child(pane, line).unwrap();
    }
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "#pane",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(5))
                .overflow_y(Overflow::Auto),
        )
        .rule_unchecked("p", TuiStyle::new().display(Display::Block));
    let terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, pane)
}

/// The top row as painted so far.
fn top_row(app: &App<TestBackend>) -> String {
    let mut screen = VirtualScreen::new(30, 6);
    screen.apply(app.terminal().backend().bytes());
    screen.row(0).trim_end().to_string()
}

#[test]
fn a_scroll_to_from_an_injected_closure_repaints_without_request_redraw() {
    let (mut app, pane) = pane_app();
    assert!(top_row(&app).starts_with("line 0"));
    app.handle().inject(move |ctx: &mut AppContext<'_>| {
        ctx.dom.node_mut(pane).scroll_to(0, 7).unwrap();
    });
    app.advance(0).unwrap();
    assert!(
        top_row(&app).starts_with("line 7"),
        "painted: {:?}",
        top_row(&app)
    );
}

#[test]
fn a_scroll_to_from_a_timer_repaints_without_request_redraw() {
    let (mut app, pane) = pane_app();
    app.dom_mut()
        .add_event_listener(pane, "keydown", ListenerOptions::default(), move |ctx| {
            ctx.set_timeout(
                move |tctx| {
                    tctx.dom.node_mut(pane).set_scroll_top(4).unwrap();
                },
                10,
            );
        })
        .unwrap();
    app.dom_mut().set_focused(Some(pane));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Char('x'),
        KeyModifiers::empty(),
    )));
    app.advance(0).unwrap();
    assert!(top_row(&app).starts_with("line 0"));
    app.advance(10).unwrap();
    assert!(
        top_row(&app).starts_with("line 4"),
        "painted: {:?}",
        top_row(&app)
    );
}

#[test]
fn a_direct_scroll_offset_write_repaints_on_the_next_frame() {
    let (mut app, pane) = pane_app();
    app.dom_mut().node_mut(pane).ext_mut().unwrap().scroll_y = 3;
    app.draw_if_dirty().unwrap();
    assert!(
        top_row(&app).starts_with("line 3"),
        "painted: {:?}",
        top_row(&app)
    );
}

#[test]
fn an_unchanged_scroll_offset_draws_no_frame() {
    let (mut app, pane) = pane_app();
    app.handle().inject(move |ctx: &mut AppContext<'_>| {
        ctx.dom.node_mut(pane).scroll_to(0, 0).unwrap();
    });
    // A drawn frame would repaint every cell after this.
    app.terminal_mut().queue_full_redraw();
    app.terminal_mut().backend_mut().clear_bytes();
    app.advance(0).unwrap();
    assert!(
        app.terminal().backend().bytes().is_empty(),
        "an unmoved offset skipped the frame"
    );
}

#[test]
fn a_scrolled_frame_is_drawn_once() {
    let (mut app, pane) = pane_app();
    app.handle().inject(move |ctx: &mut AppContext<'_>| {
        ctx.dom.node_mut(pane).scroll_to(0, 7).unwrap();
    });
    app.advance(0).unwrap();
    // The offset is painted: the next frame has nothing to draw.
    app.terminal_mut().queue_full_redraw();
    app.terminal_mut().backend_mut().clear_bytes();
    app.advance(0).unwrap();
    assert!(app.terminal().backend().bytes().is_empty());
}
