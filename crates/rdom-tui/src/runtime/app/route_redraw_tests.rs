//! `P7G-ROUTE-REDRAW-1`: a mouse route's frame reruns only what the
//! router's own work needs. A hover change re-cascades the elements
//! whose `:hover` state flipped (the dirty tracker's subtree roots), a
//! wheel scroll or a scrollbar press lays out and repaints, and a
//! listener's `request_redraw` still cascades the whole tree (it may
//! follow a direct `TuiExt` style write no mutation reports).

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::layout::{Display, Overflow, Size};
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

const RED: Color = Color::Rgb(255, 0, 0);

fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
}

fn fg(app: &App<TestBackend>, id: NodeId) -> Color {
    app.dom()
        .node(id)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .fg
}

/// Two 3-row blocks, `a` on rows 0–2 and `b` on rows 3–5, red while
/// hovered; one frame drawn.
fn hover_app() -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("div");
    let b = dom.create_element("div");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked("div:hover", TuiStyle::new().fg(RED));
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, a, b)
}

#[test]
fn a_hover_move_cascades_only_the_elements_whose_hover_flipped() {
    let (mut app, a, b) = hover_app();
    app.take_frame_stats();
    mouse(&mut app, MouseEventKind::Moved, 1, 1);
    app.advance(0).unwrap();
    assert_eq!(fg(&app, a), RED, "the hovered block restyles");
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 0, "{stats:?}");
    assert_eq!(stats.subtree_cascades, 1, "{stats:?}");
    assert_eq!(stats.paints, 1, "{stats:?}");

    mouse(&mut app, MouseEventKind::Moved, 1, 4);
    app.advance(0).unwrap();
    assert_ne!(fg(&app, a), RED, "the old hover target restyles");
    assert_eq!(fg(&app, b), RED, "the new hover target restyles");
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 0, "{stats:?}");
}

/// A 5-row, 20-column scroll pane of 30 lines; one frame drawn.
fn pane_app() -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pane = dom.create_element("div");
    dom.set_attribute(pane, "id", "pane").unwrap();
    dom.append_child(root, pane).unwrap();
    for i in 0..30 {
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

fn scroll_y(app: &App<TestBackend>, id: NodeId) -> i32 {
    app.dom().node(id).ext().unwrap().scroll_y
}

#[test]
fn a_wheel_scroll_performs_no_cascade() {
    let (mut app, pane) = pane_app();
    // Hover the pane first so the wheel frame has no hover change.
    mouse(&mut app, MouseEventKind::Moved, 2, 2);
    app.advance(0).unwrap();
    app.take_frame_stats();
    mouse(&mut app, MouseEventKind::ScrollDown, 2, 2);
    app.advance(0).unwrap();
    assert!(scroll_y(&app, pane) > 0, "the wheel scrolled the pane");
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades + stats.subtree_cascades, 0, "{stats:?}");
    assert_eq!(stats.layouts, 1, "{stats:?}");
    assert_eq!(stats.paints, 1, "{stats:?}");
}

#[test]
fn a_scrollbar_press_performs_no_full_cascade() {
    let (mut app, pane) = pane_app();
    mouse(&mut app, MouseEventKind::Moved, 19, 4);
    app.advance(0).unwrap();
    app.take_frame_stats();
    // The vertical scrollbar is the pane's last column; row 4 is track
    // below the thumb: a page down.
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 19, 4);
    app.advance(0).unwrap();
    assert!(
        scroll_y(&app, pane) > 0,
        "the track press scrolled the pane"
    );
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 0, "{stats:?}");
    assert_eq!(stats.layouts, 1, "{stats:?}");
}

#[test]
fn a_listener_request_redraw_still_forces_a_full_cascade() {
    let (mut app, a, _) = hover_app();
    app.dom_mut()
        .add_event_listener(a, "mousemove", ListenerOptions::default(), |ctx| {
            ctx.request_redraw();
        })
        .unwrap();
    mouse(&mut app, MouseEventKind::Moved, 1, 1);
    app.advance(0).unwrap();
    app.take_frame_stats();
    // No hover change: only the listener asks for the frame.
    mouse(&mut app, MouseEventKind::Moved, 2, 1);
    app.advance(0).unwrap();
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 1, "{stats:?}");
}

/// `a:hover + b` reads the hover state of `b`'s previous sibling: the
/// tracker's subtree roots must reach `b` too.
#[test]
fn a_hover_restyles_a_sibling_selected_through_hover() {
    let (mut app, _, b) = hover_app();
    app.push_stylesheet(
        Stylesheet::bare()
            .rule_unchecked("div:hover + div", TuiStyle::new().fg(Color::Rgb(0, 0, 255))),
    );
    app.advance(0).unwrap();
    mouse(&mut app, MouseEventKind::Moved, 1, 1);
    app.advance(0).unwrap();
    assert_eq!(
        fg(&app, b),
        Color::Rgb(0, 0, 255),
        "b follows a hovered sibling"
    );
    mouse(&mut app, MouseEventKind::Moved, 15, 7);
    app.advance(0).unwrap();
    assert_ne!(
        fg(&app, b),
        Color::Rgb(0, 0, 255),
        "and drops it with the hover"
    );
}
