//! M5 follow-up: pin AbortSignal-based listener removal for every
//! new event shipped in M5. Per `SHOWCASE.md` exit criteria:
//! "Each event needs ... integration tests covering cancellation /
//! propagation / `AbortSignal` removal."
//!
//! Existing AbortSignal machinery (from `rdom-core::dispatch`) is
//! event-type-agnostic — these tests are about pinning that
//! consumers can rely on `AbortController::abort()` to remove
//! listeners for the new event types just like the old ones.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_tui::core_api::AbortController;
use rdom_tui::layout::{Overflow, Size};
use rdom_tui::render::{LayoutExt, Rect, Terminal, TestBackend};
use rdom_tui::style::{CascadeExt, TuiStyle};
use rdom_tui::{App, ListenerOptions, Stylesheet, TuiDispatchExt, TuiDom, TuiEvent};

fn test_app(dom: TuiDom) -> App<TestBackend> {
    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, Stylesheet::bare(), terminal).unwrap()
}

fn mouse_at(kind: MouseEventKind, x: u16, y: u16) -> CtMouseEvent {
    CtMouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    }
}

#[test]
fn keyup_listener_removed_via_abort() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();
    dom.set_focused(Some(btn));

    let ctrl = AbortController::new();
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(
        btn,
        "keyup",
        ListenerOptions::default().with_signal(ctrl.signal()),
        move |_| c.set(c.get() + 1),
    )
    .unwrap();

    let mut app = test_app(dom);
    let release = CtEvent::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: KeyEventState::empty(),
    });
    app.handle_event(release.clone());
    assert_eq!(count.get(), 1);

    ctrl.abort();
    app.handle_event(release);
    assert_eq!(count.get(), 1, "abort removes the keyup listener");
}

#[test]
fn contextmenu_listener_removed_via_abort() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(5)),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));

    let ctrl = AbortController::new();
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(
        el,
        "contextmenu",
        ListenerOptions::default().with_signal(ctrl.signal()),
        move |_| c.set(c.get() + 1),
    )
    .unwrap();

    let mut app = test_app(dom);
    app.handle_event(CtEvent::Mouse(mouse_at(
        MouseEventKind::Down(MouseButton::Right),
        5,
        2,
    )));
    assert_eq!(count.get(), 1);

    ctrl.abort();
    app.handle_event(CtEvent::Mouse(mouse_at(
        MouseEventKind::Down(MouseButton::Right),
        5,
        2,
    )));
    assert_eq!(count.get(), 1);
}

#[test]
fn dblclick_listener_removed_via_abort() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(5)),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));

    let ctrl = AbortController::new();
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(
        el,
        "dblclick",
        ListenerOptions::default().with_signal(ctrl.signal()),
        move |_| c.set(c.get() + 1),
    )
    .unwrap();

    let mut app = test_app(dom);
    // Two clicks → dblclick.
    for _ in 0..2 {
        app.handle_event(CtEvent::Mouse(mouse_at(
            MouseEventKind::Down(MouseButton::Left),
            5,
            2,
        )));
        app.handle_event(CtEvent::Mouse(mouse_at(
            MouseEventKind::Up(MouseButton::Left),
            5,
            2,
        )));
    }
    assert_eq!(count.get(), 1);

    ctrl.abort();
    // Another double-click sequence — would be dblclick=2 if listener stayed.
    // We need a fresh pair so register_click starts over; sleep beyond
    // MULTI_CLICK_THRESHOLD by issuing 4+ clicks (count wraps).
    for _ in 0..4 {
        app.handle_event(CtEvent::Mouse(mouse_at(
            MouseEventKind::Down(MouseButton::Left),
            5,
            2,
        )));
        app.handle_event(CtEvent::Mouse(mouse_at(
            MouseEventKind::Up(MouseButton::Left),
            5,
            2,
        )));
    }
    assert_eq!(count.get(), 1, "abort removes the dblclick listener");
}

#[test]
fn resize_listener_removed_via_abort() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let ctrl = AbortController::new();
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(
        root,
        "resize",
        ListenerOptions::default().with_signal(ctrl.signal()),
        move |_| c.set(c.get() + 1),
    )
    .unwrap();

    let mut app = test_app(dom);
    app.handle_event(CtEvent::Resize(40, 10));
    assert_eq!(count.get(), 1);

    ctrl.abort();
    app.handle_event(CtEvent::Resize(60, 15));
    assert_eq!(count.get(), 1);
}

#[test]
fn scroll_listener_removed_via_abort() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let scroller = dom.create_element("scroller");
    dom.append_child(root, scroller).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "scroller",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(5))
            .overflow(Overflow::Auto),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));
    if let Some(ext) = dom.node_mut(scroller).ext_mut() {
        ext.scroll_content_height = 50;
    }

    let ctrl = AbortController::new();
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    dom.add_event_listener(
        scroller,
        "scroll",
        ListenerOptions::default().with_signal(ctrl.signal()),
        move |_| c.set(c.get() + 1),
    )
    .unwrap();

    // Programmatic-style: dispatch directly via the substrate's
    // dispatch_tui_event using a hand-rolled scroll event so the
    // test doesn't depend on the wheel routing layer. The point is
    // the listener-removal contract, not the dispatch path.
    let mut e1 = TuiEvent::new("scroll");
    let _ = dom.dispatch_tui_event(scroller, &mut e1);
    assert_eq!(count.get(), 1);

    ctrl.abort();
    let mut e2 = TuiEvent::new("scroll");
    let _ = dom.dispatch_tui_event(scroller, &mut e2);
    assert_eq!(count.get(), 1);
}
