//! `P7G-OFF-EVENT-PAINT-1`: whatever the dirty tracker records reaches
//! the next frame, whoever made the change — an interval, an injected
//! closure, `dom_mut()` — and not only a change made by an input event's
//! listener. A text edit lays out and repaints; a selection change
//! repaints; every other mutation kind draws a frame too.

use rdom_core::{NodeId, Position, Selection};

use super::redraw::FrameStats;
use crate::TuiDom;
use crate::layout::Display;
use crate::render::{Terminal, TestBackend, VirtualScreen};
use crate::runtime::app::{App, AppContext};
use crate::style::{Stylesheet, TuiStyle};

const W: u16 = 20;
const H: u16 = 4;

/// `<p id=clock>12:00</p><p id=other>x</p>`, caret blink off, drawn once.
fn clock_app() -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "id", "clock").unwrap();
    let t = dom.create_text_node("12:00");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    let other = dom.create_element("p");
    dom.set_attribute(other, "id", "other").unwrap();
    let x = dom.create_text_node("x");
    dom.append_child(other, x).unwrap();
    dom.append_child(root, other).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked("p", TuiStyle::new().display(Display::Block));
    let terminal = Terminal::new(TestBackend::new(W, H)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal)
        .unwrap()
        .with_caret_blink(None);
    app.advance(0).unwrap();
    (app, p, t)
}

fn top_row(app: &App<TestBackend>) -> String {
    let mut screen = VirtualScreen::new(W, H);
    screen.apply(app.terminal().backend().bytes());
    screen.row(0).trim_end().to_string()
}

#[test]
fn an_interval_text_edit_repaints_on_the_next_advance() {
    let (mut app, _, t) = clock_app();
    assert_eq!(top_row(&app), "12:00");
    app.scheduler.borrow_mut().set_interval(
        move |tctx| {
            tctx.dom.node_mut(t).set_node_value("12:01").unwrap();
            true
        },
        1000,
    );
    app.advance(1000).unwrap();
    assert_eq!(
        top_row(&app),
        "12:01",
        "no input event came, the text changed"
    );
}

#[test]
fn an_injected_text_edit_repaints_on_the_next_advance() {
    let (mut app, _, t) = clock_app();
    app.handle().inject(move |ctx: &mut AppContext<'_>| {
        ctx.dom.node_mut(t).set_node_value("12:02").unwrap();
    });
    app.advance(0).unwrap();
    assert_eq!(top_row(&app), "12:02");
}

#[test]
fn a_scripted_selection_change_repaints_without_layout() {
    let (mut app, _, t) = clock_app();
    app.take_frame_stats();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));
    app.advance(0).unwrap();
    let stats = app.take_frame_stats();
    assert_eq!(stats.paints, 1, "the selection is painted: {stats:?}");
    assert_eq!(
        stats.layouts, 0,
        "a selection change is paint-only: {stats:?}"
    );
    assert_eq!(stats.full_cascades + stats.subtree_cascades, 0, "{stats:?}");
}

/// Each mutation kind the tracker observes, made through `dom_mut()`
/// with no event, draws the next frame.
#[test]
fn every_mutation_kind_reaches_a_frame() {
    type Change = fn(&mut TuiDom, NodeId, NodeId);
    let changes: [(&str, Change); 8] = [
        ("attribute", |dom, p, _| {
            dom.set_attribute(p, "data-x", "1").unwrap();
        }),
        ("class", |dom, p, _| dom.add_class(p, "on").unwrap()),
        ("child list: element", |dom, p, _| {
            let s = dom.create_element("span");
            dom.append_child(p, s).unwrap();
        }),
        ("child list: removal", |dom, _, t| {
            dom.remove_child(dom.node(t).parent_node().unwrap().id(), t)
                .unwrap();
        }),
        ("character data", |dom, _, t| {
            dom.node_mut(t).set_node_value("12:59").unwrap();
        }),
        ("hover", |dom, p, _| dom.set_hovered(Some(p))),
        ("focus", |dom, p, _| {
            dom.set_attribute(p, "tabindex", "0").unwrap();
            dom.set_focused(Some(p));
        }),
        ("selection", |dom, _, t| {
            dom.set_selection(Some(Selection::caret(Position::new(t, 1))));
        }),
    ];
    for (name, change) in changes {
        let (mut app, p, t) = clock_app();
        app.take_frame_stats();
        change(app.dom_mut(), p, t);
        app.advance(0).unwrap();
        let stats = app.take_frame_stats();
        assert_eq!(stats.paints, 1, "{name}: {stats:?}");
        assert_ne!(stats, FrameStats::default(), "{name}");
    }
}
