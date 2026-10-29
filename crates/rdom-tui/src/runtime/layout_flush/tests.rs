//! The layout flush: geometry a handler asks for is computed against the
//! tree as the handler left it, as a browser's forced layout is — and the
//! frame after it still runs everything a layout starts.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use super::flush_layout;
use crate::prelude::*;

fn row(dom: &mut TuiDom, text: &str) -> NodeId {
    let d = dom.create_element("div");
    let t = dom.create_text_node(text);
    dom.append_child(d, t).unwrap();
    d
}

/// A 20 × 8 `App` over a 3-row `overflow-y: auto` `.s` of `n` one-row
/// rows `r0`…, and a `Go` button below it (row 3), one frame drawn.
fn scroller_app(n: usize) -> (App<TestBackend>, NodeId, Vec<NodeId>, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let rows: Vec<NodeId> = (0..n)
        .map(|k| {
            let r = row(&mut dom, &format!("r{k}"));
            dom.append_child(s, r).unwrap();
            r
        })
        .collect();
    let go = dom.create_element("button");
    let t = dom.create_text_node("Go");
    dom.append_child(go, t).unwrap();
    dom.append_child(root, go).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".s { overflow-y: auto; height: 3; display: block } button { display: block }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    (app, s, rows, go)
}

fn click_at(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
}

// ---- scroll_into_view ------------------------------------------------------

/// The report: a handler creates an element and scrolls it into view. A
/// browser lays out first (CSSOM View §5.2 runs against the current
/// layout), so the element — inserted at row 3 of the scroller — is
/// scrolled to the top, and the handler reads the offset it made.
#[test]
fn scroll_into_view_of_an_element_created_in_the_handler() {
    let (mut app, s, rows, go) = scroller_app(6);
    let seen = Rc::new(Cell::new(None));
    let seen_in = seen.clone();
    let before = rows[3];
    app.dom_mut()
        .add_event_listener(go, "click", ListenerOptions::default(), move |ctx| {
            let fresh = row(ctx.dom, "new");
            ctx.dom.insert_before(s, fresh, Some(before)).unwrap();
            ctx.dom.node_mut(fresh).scroll_into_view().unwrap();
            seen_in.set(ctx.dom.node(s).scroll_top());
        })
        .unwrap();
    click_at(&mut app, 1, 3);
    assert_eq!(seen.get(), Some(3), "the handler reads the scroll it made");
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(3));
}

/// An element moved before the call is scrolled to where it is now, not
/// to its last laid-out row: four rows inserted above `r1` take it from
/// row 1 to row 5.
#[test]
fn scroll_into_view_of_a_moved_element() {
    let (mut app, s, rows, _) = scroller_app(8);
    let dom = app.dom_mut();
    for k in 0..4 {
        let r = row(dom, &format!("x{k}"));
        dom.insert_before(s, r, Some(rows[1])).unwrap();
    }
    dom.node_mut(rows[1]).scroll_into_view().unwrap();
    assert_eq!(dom.node(s).scroll_top(), Some(5));
}

/// Before the `App`'s first frame there is no layout to bring up to date:
/// the scroll waits for the first one rather than being dropped.
#[test]
fn scroll_into_view_before_the_first_frame_scrolls_at_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let rows: Vec<NodeId> = (0..8)
        .map(|k| {
            let r = row(&mut dom, &format!("r{k}"));
            dom.append_child(s, r).unwrap();
            r
        })
        .collect();
    let sheet =
        rdom_css::from_css_strict(".s { overflow-y: auto; height: 3; display: block }").unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.dom_mut().node_mut(rows[4]).scroll_into_view().unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(4));
}

// ---- focus -----------------------------------------------------------------

/// HTML's focusing steps scroll at once, against the current layout: a
/// handler that moves a button below the scroller's view and focuses it
/// reads the scroll in the same handler.
#[test]
fn focus_in_a_handler_scrolls_before_the_handler_returns() {
    let (mut app, s, rows, go) = scroller_app(2);
    let target = app.dom_mut().create_element("button");
    let tt = app.dom_mut().create_text_node("T");
    app.dom_mut().append_child(target, tt).unwrap();
    app.dom_mut()
        .insert_before(s, target, Some(rows[1]))
        .unwrap();
    app.draw_if_dirty().unwrap();
    let seen = Rc::new(Cell::new(None));
    let seen_in = seen.clone();
    app.dom_mut()
        .add_event_listener(go, "click", ListenerOptions::default(), move |ctx| {
            for k in 0..5 {
                let r = row(ctx.dom, &format!("x{k}"));
                ctx.dom.insert_before(s, r, Some(target)).unwrap();
            }
            ctx.dom.node_mut(target).focus();
            seen_in.set(ctx.dom.node(s).scroll_top());
        })
        .unwrap();
    click_at(&mut app, 1, 3);
    assert_eq!(app.dom().focused(), Some(target));
    assert_eq!(seen.get(), Some(4), "`nearest` brings row 6 to the bottom");
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(4));
}

/// Focus before the first frame scrolls at it (no layout to flush yet).
#[test]
fn focus_before_the_first_frame_scrolls_at_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let buttons: Vec<NodeId> = (0..8)
        .map(|k| {
            let b = dom.create_element("button");
            let t = dom.create_text_node(&format!("b{k}"));
            dom.append_child(b, t).unwrap();
            dom.append_child(s, b).unwrap();
            b
        })
        .collect();
    let sheet = rdom_css::from_css_strict(
        ".s { overflow-y: auto; height: 3; display: block } button { display: block }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.dom_mut().node_mut(buttons[6]).focus();
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(4));
}

// ---- flush_layout ----------------------------------------------------------

/// `flush_layout` lays out what changed, so a reader after it sees the
/// new geometry; with nothing changed since, it does nothing.
#[test]
fn flush_layout_brings_the_layout_up_to_date_once() {
    let (mut app, s, _, _) = scroller_app(2);
    let dom = app.dom_mut();
    dom.node_mut(s).set_width(9u16);
    assert!(flush_layout(dom), "a change was pending");
    assert_eq!(dom.node(s).layout_rect().map(|r| r.width), Some(9));
    assert!(!flush_layout(dom), "nothing changed since");
}

/// Up to date already: nothing to do.
#[test]
fn flush_layout_after_a_frame_does_nothing() {
    let (mut app, _, _, _) = scroller_app(2);
    assert!(!flush_layout(app.dom_mut()));
}

/// A document no `App` runs has no viewport or sheets to lay out with:
/// the caller cascades and lays it out.
#[test]
fn flush_layout_on_a_bare_document_does_nothing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = row(&mut dom, "x");
    dom.append_child(root, d).unwrap();
    assert!(!flush_layout(&mut dom));
}

// ---- the frame after a flush -----------------------------------------------

/// The frame after a flush runs what follows a layout — the focus fixup,
/// re-snaps, layout transitions — but does not lay the tree out again
/// when nothing changed since the flush: one layout, as a browser makes.
#[test]
fn the_frame_after_a_flush_reuses_its_layout() {
    use crate::render::layout_pass::ROUNDS;
    let (mut app, s, _, _) = scroller_app(2);
    app.take_frame_stats();
    app.dom_mut().node_mut(s).set_width(9u16);
    assert!(flush_layout(app.dom_mut()));
    ROUNDS.with(|r| r.set(0));
    app.draw_if_dirty().unwrap();
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 0, "no second layout");
    assert_eq!(
        app.take_frame_stats().layouts,
        1,
        "the post-layout work ran"
    );
}

/// A scroll made after a flush moves offsets past that layout: the frame
/// reusing it still moves the scrolled boxes to them (a scroll update), so
/// it paints the element where the scroll put it.
#[test]
fn the_frame_moves_the_boxes_a_scroll_after_a_flush_moved() {
    let (mut app, s, rows, _) = scroller_app(8);
    let dom = app.dom_mut();
    dom.node_mut(s).set_width(12u16);
    dom.node_mut(rows[4]).scroll_into_view().unwrap();
    assert_eq!(dom.node(s).scroll_top(), Some(4));
    app.draw_if_dirty().unwrap();
    assert_eq!(
        app.dom().node(rows[4]).layout_rect().map(|r| r.y),
        Some(0),
        "r4 is at the scroller's top"
    );
}

/// A change after the flush is laid out by the frame.
#[test]
fn the_frame_lays_out_a_change_made_after_a_flush() {
    let (mut app, s, _, _) = scroller_app(2);
    app.dom_mut().node_mut(s).set_width(9u16);
    assert!(flush_layout(app.dom_mut()));
    app.dom_mut().node_mut(s).set_width(12u16);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(s).layout_rect().map(|r| r.width), Some(12));
}

/// Tab moves the focus and its scroll lays the tree out once: the flush
/// the focus makes, reused by the frame.
#[test]
fn a_tab_press_lays_out_once() {
    use crate::render::layout_pass::ROUNDS;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
    let (mut app, _, _, _) = scroller_app(2);
    ROUNDS.with(|r| r.set(0));
    app.handle_event(CtEvent::Key(KeyEvent {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
    app.draw_if_dirty().unwrap();
    assert!(app.dom().focused().is_some());
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 1);
}

/// CSS Scroll Snap 1 §5.4 through a flush: a box inserted above the snap
/// target and laid out by the handler still re-snaps the container in
/// the frame (`scroll_snap::the_frame_resnaps_after_layout`, flushed).
#[test]
fn a_flushed_layout_still_resnaps_in_the_frame() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let items: Vec<NodeId> = (0..5)
        .map(|_| {
            let id = dom.create_element("div");
            dom.set_attribute(id, "class", "i").unwrap();
            dom.append_child(s, id).unwrap();
            id
        })
        .collect();
    let sheet = rdom_css::from_css_strict(
        ".s { width: 10; height: 4; overflow-y: scroll; scroll-snap-type: y mandatory } \
         .i { height: 3; scroll-snap-align: start }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(12, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app.dom_mut().node_mut(s).scroll_to(0, 6).unwrap();
    app.advance(16).unwrap();
    let dom = app.dom_mut();
    let extra = dom.create_element("div");
    dom.set_attribute(extra, "class", "i").unwrap();
    dom.insert_before(s, extra, Some(items[0])).unwrap();
    assert!(flush_layout(dom));
    app.advance(16).unwrap();
    assert_eq!(app.dom().node(s).scroll_top(), Some(9));
}

/// CSS Transitions 1 §3 through a flush: a container query the handler's
/// layout flipped starts its transition in the frame, as a frame's own
/// layout's flip does (`app::container_query_tests`).
#[test]
fn a_container_flip_in_a_flushed_layout_starts_its_transition() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, r#"<div id="card"><p id="t">x</p></div>"#, root).unwrap();
    let card = dom.get_element_by_id("card").unwrap();
    let t = dom.get_element_by_id("t").unwrap();
    let parsed = rdom_css::parse(
        "#card { container-type: inline-size; width: 20 }
         #t { color: rgb(0, 0, 255); transition: color 1s }
         @container (width >= 30) { #t { color: rgb(255, 0, 0) } }",
    );
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(40, 3)).unwrap();
    let mut app = App::with_backend(dom, parsed.stylesheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut().node_mut(card).set_width(32u16);
    assert!(flush_layout(app.dom_mut()));
    app.advance(16).unwrap();
    assert_eq!(app.get_animations(t).len(), 1);
}
