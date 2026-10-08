//! C11G-MODAL-INERT — inertness in the runtime (HTML §6.3): a modal
//! dialog or an `inert` subtree takes its nodes out of hit-testing, focus,
//! sequential navigation and keyboard activation, through the one
//! `Dom::is_inert` answer.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::layout::Display;
use crate::node::TuiNodeExt;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::{dialog, popover};
use crate::runtime::focus::{focus_node, tabindex};
use crate::runtime::hit_test::HitTestExt;
use crate::style::{Stylesheet, TuiStyle};
use crate::{TuiAccessorsMut, TuiDom, TuiNodeMutExt};

fn el(dom: &mut TuiDom, parent: NodeId, tag: &str) -> NodeId {
    let e = dom.create_element(tag);
    dom.append_child(parent, e).unwrap();
    e
}

fn button(dom: &mut TuiDom, parent: NodeId, label: &str) -> NodeId {
    let b = el(dom, parent, "button");
    let t = dom.create_text_node(label);
    dom.append_child(b, t).unwrap();
    b
}

fn app(dom: TuiDom) -> App<TestBackend> {
    let terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app
}

fn count_clicks(dom: &mut TuiDom, id: NodeId) -> Rc<Cell<u32>> {
    let n = Rc::new(Cell::new(0));
    let c = n.clone();
    dom.add_event_listener(id, "click", ListenerOptions::default(), move |_| {
        c.set(c.get() + 1);
    })
    .unwrap();
    n
}

fn centre(app: &App<TestBackend>, id: NodeId) -> (u16, u16) {
    let r = app.dom().node(id).layout_rect().expect("laid out");
    (
        (r.x + i32::from(r.width) / 2) as u16,
        (r.y + i32::from(r.height) / 2) as u16,
    )
}

fn click(app: &mut App<TestBackend>, (x, y): (u16, u16)) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
    app.draw_if_dirty().unwrap();
}

fn press(app: &mut App<TestBackend>, code: KeyCode) {
    app.handle_event(CtEvent::Key(KeyEvent::new(code, KeyModifiers::empty())));
    app.draw_if_dirty().unwrap();
}

// ── A modal dialog ─────────────────────────────────────────────────

/// HTML §4.11.4 "dialog focusing steps": with no `autofocus` and no
/// focus delegate, the control is the dialog itself — so the focus
/// leaves the page, and Enter no longer activates the button that had
/// it (architect B1's scenario).
#[test]
fn a_modal_dialog_with_no_focusable_content_takes_the_focus() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let del = button(&mut dom, root, "Delete");
    let dlg = el(&mut dom, root, "dialog");
    let p = el(&mut dom, dlg, "p");
    let t = dom.create_text_node("Delete?");
    dom.append_child(p, t).unwrap();
    let clicks = count_clicks(&mut dom, del);
    let mut app = app(dom);
    focus_node(app.dom_mut(), Some(del));
    dialog::show_modal(app.dom_mut(), dlg);
    assert_eq!(app.dom().focused(), Some(dlg), "the dialog is focused");
    app.draw_if_dirty().unwrap();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(clicks.get(), 0, "nothing outside the modal is activated");
}

/// HTML §6.3: inert nodes cannot be focused — `focus_node` and
/// `focus()` leave the focus where it is.
#[test]
fn focus_refuses_an_element_a_modal_dialog_made_inert() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outside = button(&mut dom, root, "out");
    let dlg = el(&mut dom, root, "dialog");
    let inside = button(&mut dom, dlg, "in");
    let mut app = app(dom);
    dialog::show_modal(app.dom_mut(), dlg);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(inside));
    focus_node(app.dom_mut(), Some(outside));
    assert_eq!(app.dom().focused(), Some(inside), "focus_node refused");
    app.dom_mut().node_mut(outside).focus();
    assert_eq!(app.dom().focused(), Some(inside), "focus() refused");
    assert!(!tabindex::is_focusable(app.dom(), outside));
}

/// HTML §6.3.2: an unrendered modal dialog still blocks the document —
/// the pointer reaches nothing beneath it, and the focus it could not
/// take leaves the page at the next frame (the focus fixup: an inert
/// element is no focusable area).
#[test]
fn an_unrendered_modal_dialog_still_makes_the_page_inert() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outside = button(&mut dom, root, "out");
    let hidden = el(&mut dom, root, "div");
    dom.node_mut(hidden)
        .set_inline_style(TuiStyle::new().display(Display::None));
    let dlg = el(&mut dom, hidden, "dialog");
    button(&mut dom, dlg, "in");
    let clicks = count_clicks(&mut dom, outside);
    let mut app = app(dom);
    focus_node(app.dom_mut(), Some(outside));
    let at = centre(&app, outside);
    dialog::show_modal(app.dom_mut(), dlg);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().hit_test(at.0, at.1), None, "nothing is hit");
    click(&mut app, at);
    assert_eq!(clicks.get(), 0);
    assert_eq!(app.dom().focused(), None, "the inert focus was fixed up");
}

/// HTML §6.3.2 excepts only the subject dialog and its descendants: a
/// popover shown above a modal dialog from outside it is inert, while a
/// popover inside the dialog is interactive.
#[test]
fn a_popover_above_a_modal_dialog_is_inert_unless_inside_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let dlg = el(&mut dom, root, "dialog");
    let inner = el(&mut dom, dlg, "div");
    dom.set_attribute(inner, "popover", "manual").unwrap();
    let inner_btn = button(&mut dom, inner, "inner");
    let outer = el(&mut dom, root, "div");
    dom.set_attribute(outer, "popover", "manual").unwrap();
    let outer_btn = button(&mut dom, outer, "outer");
    let outer_clicks = count_clicks(&mut dom, outer_btn);
    let mut app = app(dom);
    dialog::show_modal(app.dom_mut(), dlg);
    popover::show_popover(app.dom_mut(), outer).unwrap();
    app.draw_if_dirty().unwrap();
    let at = centre(&app, outer_btn);
    assert_ne!(app.dom().hit_test(at.0, at.1), Some(outer_btn));
    click(&mut app, at);
    assert_eq!(outer_clicks.get(), 0, "the outside popover is inert");
    popover::hide_popover(app.dom_mut(), outer).unwrap();
    popover::show_popover(app.dom_mut(), inner).unwrap();
    app.draw_if_dirty().unwrap();
    let at = centre(&app, inner_btn);
    assert_eq!(app.dom().hit_test(at.0, at.1), Some(inner_btn));
}

// ── The `inert` attribute ──────────────────────────────────────────

/// HTML §6.3.1: an `inert` subtree is out of sequential navigation, cannot
/// be focused, and the pointer passes through it ("as if
/// `pointer-events: none`") to what is beneath — here, the parent.
#[test]
fn an_inert_subtree_is_skipped_by_tab_focus_and_the_pointer() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div");
    let a = button(&mut dom, wrap, "a");
    let b = button(&mut dom, root, "b");
    dom.set_attribute(wrap, "inert", "").unwrap();
    let a_clicks = count_clicks(&mut dom, a);
    let mut app = app(dom);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.dom().focused(), Some(b));
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.dom().focused(), Some(b), "the inert button is no stop");
    app.dom_mut().node_mut(a).focus();
    assert_eq!(app.dom().focused(), Some(b));
    let at = centre(&app, a);
    let hit = app.dom().hit_test(at.0, at.1);
    assert!(hit != Some(a) && hit != Some(wrap), "hit {hit:?}");
    click(&mut app, at);
    assert_eq!(a_clicks.get(), 0);
}

/// The focus fixup (HTML "update the rendering"): an element that became
/// inert while focused is no focusable area, so the focus leaves it.
#[test]
fn a_focused_element_made_inert_loses_the_focus() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div");
    let a = button(&mut dom, wrap, "a");
    let mut app = app(dom);
    focus_node(app.dom_mut(), Some(a));
    app.dom_mut().set_attribute(wrap, "inert", "").unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), None);
}

/// HTML §6.3: an inert node's "text selection functionality must act as
/// if `user-select` was `none`" — a point on its text gives no position
/// inside it.
#[test]
fn inert_text_is_not_selectable() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let q = el(&mut dom, root, "p");
    let u = dom.create_text_node("world");
    dom.append_child(q, u).unwrap();
    dom.set_attribute(p, "inert", "").unwrap();
    let app = app(dom);
    let r = app.dom().node(p).layout_rect().expect("laid out");
    let pos = app.dom().position_at(r.x as u16 + 1, r.y as u16);
    assert!(pos.is_none_or(|pos| pos.node != t), "{pos:?}");
}
