//! `P7G-PAINT-ONLY-FRAME-1`: a frame reruns only the stages its cause
//! needs. A caret-blink flip only repaints; a scroll offset change (a
//! smooth-scroll step included) lays out and repaints; neither
//! re-cascades. Text edits re-cascade the elements whose selector
//! state their text feeds (`:placeholder-shown`, `::placeholder`,
//! `:empty`) through the dirty tracker, not through a full cascade;
//! `invalidate_cascade` (stylesheet changes) still cascades everything.

use std::time::Duration;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{NodeId, Position, Selection};

use super::redraw::FrameStats;
use crate::TuiDom;
use crate::accessors::TuiAccessorsMut;
use crate::layout::{Display, Overflow, ScrollBehavior, Size};
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

fn press(app: &mut App<TestBackend>, code: KeyCode) {
    app.handle_event(CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
}

/// A blinking caret in `<p contenteditable>hello</p>`, one frame drawn.
fn blinking_app() -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked("p", TuiStyle::new().display(Display::Block));
    let terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal)
        .unwrap()
        .with_caret_blink(Some(Duration::from_millis(500)));
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));
    app.advance(0).unwrap();
    app
}

#[test]
fn a_caret_blink_flip_neither_cascades_nor_lays_out() {
    let mut app = blinking_app();
    app.take_frame_stats();
    app.advance(500).unwrap();
    assert_eq!(
        app.take_frame_stats(),
        FrameStats {
            paints: 1,
            ..FrameStats::default()
        },
        "the flip repaints and does nothing else"
    );
}

/// A 5-row scroll pane of 30 lines with the given `scroll-behavior`,
/// one frame drawn.
fn pane_app(behavior: ScrollBehavior) -> (App<TestBackend>, NodeId) {
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
                .overflow_y(Overflow::Auto)
                .scroll_behavior(behavior),
        )
        .rule_unchecked("p", TuiStyle::new().display(Display::Block));
    let terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, pane)
}

#[test]
fn a_smooth_scroll_step_lays_out_without_cascading() {
    let (mut app, pane) = pane_app(ScrollBehavior::Smooth);
    app.dom_mut().node_mut(pane).scroll_to(0, 20).unwrap();
    app.advance(0).unwrap();
    app.take_frame_stats();
    app.advance(50).unwrap();
    let stats = app.take_frame_stats();
    assert_eq!(stats.paints, 1, "{stats:?}");
    assert_eq!(stats.layouts, 1, "{stats:?}");
    assert_eq!(stats.full_cascades + stats.subtree_cascades, 0, "{stats:?}");
}

#[test]
fn an_instant_scroll_lays_out_without_cascading() {
    let (mut app, pane) = pane_app(ScrollBehavior::Auto);
    app.take_frame_stats();
    // `scroll-behavior: auto`: an instant scroll.
    app.dom_mut().node_mut(pane).scroll_to(0, 7).unwrap();
    app.advance(0).unwrap();
    let stats = app.take_frame_stats();
    assert_eq!(stats.paints, 1, "{stats:?}");
    assert_eq!(stats.layouts, 1, "{stats:?}");
    assert_eq!(stats.full_cascades + stats.subtree_cascades, 0, "{stats:?}");
}

/// `textarea:placeholder-shown` colored red, the textarea focused with
/// the caret in its (empty, seeded) text node.
fn placeholder_app() -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ta = dom.create_element("textarea");
    dom.set_attribute(ta, "placeholder", "Notes").unwrap();
    dom.append_child(root, ta).unwrap();
    let sheet = Stylesheet::new().rule_unchecked(
        "textarea:placeholder-shown",
        TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
    );
    let terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.dom_mut().set_focused(Some(ta));
    let text = app
        .dom()
        .node(ta)
        .child_nodes()
        .next()
        .expect("the App seeds a textarea's text node")
        .id();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(text, 0))));
    app.advance(0).unwrap();
    (app, ta)
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

#[test]
fn typing_into_an_empty_textarea_drops_its_placeholder_shown_style_next_frame() {
    let (mut app, ta) = placeholder_app();
    assert_eq!(fg(&app, ta), Color::Rgb(255, 0, 0));
    press(&mut app, KeyCode::Char('a'));
    app.take_frame_stats();
    app.advance(0).unwrap();
    assert_ne!(fg(&app, ta), Color::Rgb(255, 0, 0), "typed text hides it");
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 0, "{stats:?}");
    // And back: deleting the text shows the placeholder again.
    press(&mut app, KeyCode::Backspace);
    app.advance(0).unwrap();
    assert_eq!(fg(&app, ta), Color::Rgb(255, 0, 0), "empty again");
}

/// Selectors 4 §14.2 `:empty`: appending a child (element or text) to
/// an empty element, or removing its last one, restyles it on the next
/// frame.
#[test]
fn empty_follows_child_insertion_and_removal() {
    for text in [false, true] {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let sheet = Stylesheet::new()
            .rule_unchecked("div:empty", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
        let terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
        let mut app = App::with_backend(dom, sheet, terminal).unwrap();
        app.advance(0).unwrap();
        assert_eq!(fg(&app, div), Color::Rgb(0, 0, 255));
        let child = if text {
            app.dom_mut().create_text_node("x")
        } else {
            app.dom_mut().create_element("span")
        };
        app.dom_mut().append_child(div, child).unwrap();
        app.advance(0).unwrap();
        assert_ne!(fg(&app, div), Color::Rgb(0, 0, 255), "text={text}");
        app.dom_mut().remove_child(div, child).unwrap();
        app.advance(0).unwrap();
        assert_eq!(fg(&app, div), Color::Rgb(0, 0, 255), "text={text}");
    }
}

#[test]
fn a_stylesheet_change_still_cascades_everything() {
    let (mut app, _) = pane_app(ScrollBehavior::Auto);
    app.take_frame_stats();
    app.push_stylesheet(Stylesheet::bare());
    app.advance(0).unwrap();
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 1, "{stats:?}");
    assert_eq!(stats.paints, 1, "{stats:?}");
}
