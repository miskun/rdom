//! The layout flush: geometry a handler asks for is computed against the
//! tree as the handler left it, as a browser's forced layout is — and the
//! frame after it still runs everything a layout starts.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use super::{LayoutFlush, flush_layout};
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
    assert_eq!(
        flush_layout(dom),
        LayoutFlush::LaidOut,
        "a change was pending"
    );
    assert_eq!(dom.node(s).layout_rect().map(|r| r.width), Some(9));
    assert_eq!(
        flush_layout(dom),
        LayoutFlush::Current,
        "nothing changed since"
    );
}

// ---- the frame after a flush -----------------------------------------------

/// A handler moves an element whose width is transitioning — a removal is
/// a disconnection, so the frame cancels the transition and renders it
/// afresh at its base width (CSS Transitions 1 §3) — then flushes. The
/// flush laid it out at its transitioning width; the frame's cancel moves
/// it with no record, so the frame lays out again rather than reuse.
#[test]
fn the_frame_lays_out_an_element_whose_transition_a_move_cancelled() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<div id="l"><div id="a">a</div><div id="e" class="e">e</div></div>"#,
        root,
    )
    .unwrap();
    let list = dom.get_element_by_id("l").unwrap();
    let a = dom.get_element_by_id("a").unwrap();
    let e = dom.get_element_by_id("e").unwrap();
    let parsed =
        rdom_css::parse(".e { width: 4; transition: width 10s linear } .e.wide { width: 24 }");
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
    let mut app = App::with_backend(dom, parsed.stylesheet, terminal).unwrap();
    app.advance(0).unwrap();
    app.dom_mut().set_attribute(e, "class", "e wide").unwrap();
    app.advance(0).unwrap();
    app.advance(1000).unwrap();
    let mid = app.dom().node(e).layout_rect().unwrap().width;
    assert!(mid < 24, "transitioning: {mid}");
    // The handler: move `e` above `a`, then flush.
    let dom = app.dom_mut();
    dom.insert_before(list, e, Some(a)).unwrap();
    assert_eq!(flush_layout(dom), LayoutFlush::LaidOut);
    app.draw_if_dirty().unwrap();
    assert!(app.get_animations(e).is_empty(), "the move cancelled it");
    assert_eq!(
        app.dom().node(e).layout_rect().map(|r| r.width),
        Some(24),
        "laid out at its base width"
    );
}

/// HTML's focusing steps scroll the element focused. The flush a focus
/// makes can run a listener that moves the focus on (a popover's
/// `beforetoggle`); its own focus scrolls, and the outer one, whose
/// element lost the focus, does not undo it.
#[test]
fn a_focus_a_flush_listener_moved_on_is_not_scrolled() {
    let (mut app, s, rows, _) = scroller_app(8);
    for r in &rows {
        app.dom_mut().set_attribute(*r, "tabindex", "0").unwrap();
    }
    let p = app.dom_mut().create_element("div");
    app.dom_mut().set_attribute(p, "popover", "").unwrap();
    let root = app.dom().root();
    app.dom_mut().append_child(root, p).unwrap();
    app.draw_if_dirty().unwrap();
    crate::runtime::builtins::popover::show_popover(app.dom_mut(), p).unwrap();
    app.draw_if_dirty().unwrap();
    let first = rows[0];
    app.dom_mut()
        .add_event_listener(p, "beforetoggle", ListenerOptions::default(), move |ctx| {
            ctx.dom.node_mut(first).focus();
        })
        .unwrap();
    app.dom_mut().set_attribute(p, "popover", "manual").unwrap();
    app.dom_mut().node_mut(rows[6]).focus();
    assert_eq!(app.dom().focused(), Some(rows[0]), "the listener's focus");
    assert_eq!(app.dom().node(s).scroll_top(), Some(0), "r0 stays in view");
}

/// The frame after a flush runs what follows a layout — the focus fixup,
/// re-snaps, layout transitions — but does not lay the tree out again
/// when nothing changed since the flush: one layout, as a browser makes.
#[test]
fn the_frame_after_a_flush_reuses_its_layout() {
    use crate::render::layout_pass::ROUNDS;
    let (mut app, s, _, _) = scroller_app(2);
    app.take_frame_stats();
    app.dom_mut().node_mut(s).set_width(9u16);
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::LaidOut);
    ROUNDS.with(|r| r.set(0));
    app.draw_if_dirty().unwrap();
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 0, "no second layout");
    let stats = app.take_frame_stats();
    assert_eq!(
        (stats.layouts, stats.reuses),
        (0, 1),
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
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::LaidOut);
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

/// Tab into a scroller marks it (`data-rdom-scroll-focus`, the keyboard
/// scroll cue) before the flush lays out, as the frame's prelude would:
/// the frame then finds nothing to restyle, and the press lays out once.
#[test]
fn a_tab_into_a_scroller_lays_out_once() {
    use crate::render::layout_pass::ROUNDS;
    use crate::runtime::scrollbar::SCROLL_FOCUS_ATTR;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
    let (mut app, s, rows, _) = scroller_app(6);
    for r in &rows {
        app.dom_mut().set_attribute(*r, "tabindex", "0").unwrap();
    }
    app.draw_if_dirty().unwrap();
    ROUNDS.with(|r| r.set(0));
    app.handle_event(CtEvent::Key(KeyEvent {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(rows[0]));
    assert!(app.dom().node(s).has_attribute(SCROLL_FOCUS_ATTR));
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 1);
}

/// A handler that empties a `required` input makes it `:invalid` at
/// once, as a browser's style update does: the flush lays it out with
/// its `:invalid` height, and the frame does not lay out again.
#[test]
fn a_validity_change_is_laid_out_by_the_flush() {
    use crate::render::layout_pass::ROUNDS;
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, r#"<input id="i" required value="x">"#, root).unwrap();
    let input = dom.get_element_by_id("i").unwrap();
    let sheet =
        rdom_css::from_css_strict("input { display: block } input:invalid { height: 3 }").unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(
        app.dom().node(input).layout_rect().map(|r| r.height),
        Some(1)
    );
    app.dom_mut().node_mut(input).set_value("").unwrap();
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::LaidOut);
    assert_eq!(
        app.dom().node(input).layout_rect().map(|r| r.height),
        Some(3)
    );
    ROUNDS.with(|r| r.set(0));
    app.draw_if_dirty().unwrap();
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 0, "no second layout");
}

/// A text control a handler inserts gets its text node from the flush
/// (control seeding), as it would before the frame's cascade: the frame
/// finds nothing left to do and does not lay out again.
#[test]
fn an_inserted_control_is_seeded_by_the_flush() {
    use crate::render::layout_pass::ROUNDS;
    let (mut app, s, _, _) = scroller_app(2);
    let dom = app.dom_mut();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", "hello").unwrap();
    dom.append_child(s, input).unwrap();
    assert_eq!(flush_layout(dom), LayoutFlush::LaidOut);
    assert_eq!(dom.node(input).value().as_deref(), Some("hello"));
    ROUNDS.with(|r| r.set(0));
    app.draw_if_dirty().unwrap();
    assert_eq!(ROUNDS.with(std::cell::Cell::get), 0, "no second layout");
}

/// The derived-state stages run user code — a popover hidden because its
/// `popover` attribute changed while showing fires `beforetoggle` — and a
/// listener that focuses re-enters the flush from inside it. Nothing the
/// stages share is borrowed across that, so the nested flush runs.
#[test]
fn a_listener_the_flush_runs_may_flush_again() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<button id="b">B</button><div id="p" popover>P</div>"#,
        root,
    )
    .unwrap();
    let b = dom.get_element_by_id("b").unwrap();
    let p = dom.get_element_by_id("p").unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.draw_if_dirty().unwrap();
    crate::runtime::builtins::popover::show_popover(app.dom_mut(), p).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut()
        .add_event_listener(p, "beforetoggle", ListenerOptions::default(), move |ctx| {
            ctx.dom.node_mut(b).focus();
        })
        .unwrap();
    app.dom_mut().set_attribute(p, "popover", "manual").unwrap();
    flush_layout(app.dom_mut());
    assert_eq!(app.dom().focused(), Some(b), "the listener's focus ran");
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
    assert_eq!(flush_layout(dom), LayoutFlush::LaidOut);
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
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::LaidOut);
    app.advance(16).unwrap();
    assert_eq!(app.get_animations(t).len(), 1);
}
