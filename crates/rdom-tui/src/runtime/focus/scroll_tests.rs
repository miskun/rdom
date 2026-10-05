//! C8G-FOCUS-SCROLL — HTML's focusing steps scroll the focused element
//! into view for keyboard and script focus only: not for focus a pointer
//! moved (the tree's row click), not where HTML says the viewport is not
//! scrolled (a closing dialog returning focus, HTML §4.11.4 "close the
//! dialog"), not under `FocusOptions { preventScroll }` (HTML
//! `focus(options)`); and against the layout the focus is shown in — in an
//! `App`, at its next layout, not against rects a handler made stale.

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::prelude::*;
use crate::runtime::focus::FocusOptions;

/// A 3-row `overflow-y: auto` `.s` holding `n` buttons `b0`…, cascaded
/// (with `css`) and laid out 20 × 8 on a bare document.
fn scroller(n: usize, css: &str) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let buttons = (0..n)
        .map(|k| {
            let b = dom.create_element("button");
            let t = dom.create_text_node(&format!("b{k}"));
            dom.append_child(b, t).unwrap();
            dom.append_child(s, b).unwrap();
            b
        })
        .collect();
    let sheet = rdom_css::from_css_strict(&format!(
        ".s {{ overflow-y: auto; height: 3; display: block }} button {{ display: block }} {css}"
    ))
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 8));
    (dom, s, buttons)
}

/// HTML `focus({ preventScroll: true })` focuses without scrolling;
/// `focus()` scrolls the element into view.
#[test]
fn prevent_scroll_focuses_without_scrolling() {
    let (mut dom, s, b) = scroller(10, "");
    dom.node_mut(b[8])
        .focus_with(FocusOptions::new().prevent_scroll(true));
    assert_eq!(dom.focused(), Some(b[8]));
    assert_eq!(dom.node(s).scroll_top(), Some(0));
    dom.node_mut(b[9]).focus();
    assert_eq!(dom.node(s).scroll_top(), Some(7));
}

/// HTML §4.11.4 "close the dialog": focus returns to the element focused
/// before it opened, "the viewport should not be scrolled by doing this
/// step" — the user scrolled away meanwhile, and stays there.
#[test]
fn a_closing_dialog_returns_focus_without_scrolling() {
    let (mut dom, s, b) = scroller(10, "dialog { display: block }");
    let root = dom.root();
    let dialog = dom.create_element("dialog");
    let ok = dom.create_element("button");
    dom.append_child(dialog, ok).unwrap();
    dom.append_child(root, dialog).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".s { overflow-y: auto; height: 3; display: block } button { display: block }",
    )
    .unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 8));
    dom.node_mut(b[8]).focus();
    assert_eq!(dom.node(s).scroll_top(), Some(6));
    crate::runtime::builtins::dialog::show_modal(&mut dom, dialog);
    dom.node_mut(s).set_scroll(0, 0);
    dom.layout_dom(Rect::new(0, 0, 20, 8));
    crate::runtime::builtins::dialog::close(&mut dom, dialog, "");
    assert_eq!(dom.focused(), Some(b[8]));
    assert_eq!(dom.node(s).scroll_top(), Some(0));
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

/// A click on a tree row focuses the tree — focus a pointer moved, which
/// does not scroll: the 6-row tree, its last two rows showing at the top
/// of its 3-row scroller, stays where it is (`nearest` moved it).
#[test]
fn a_tree_row_click_does_not_scroll() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let tree = dom.create_element("ul");
    dom.set_attribute(tree, "role", "tree").unwrap();
    dom.append_child(s, tree).unwrap();
    for k in 0..6 {
        let li = dom.create_element("li");
        dom.set_attribute(li, "role", "treeitem").unwrap();
        // Focusable rows: the press focuses the row, the click the tree.
        dom.set_attribute(li, "tabindex", "-1").unwrap();
        let t = dom.create_text_node(&format!("row{k}"));
        dom.append_child(li, t).unwrap();
        dom.append_child(tree, li).unwrap();
    }
    for k in 0..3 {
        let p = dom.create_element("p");
        let t = dom.create_text_node(&format!("after{k}"));
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
    }
    let sheet = rdom_css::from_css_strict(".s { overflow-y: auto; height: 3 }").unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut().node_mut(s).set_scroll(0, 4);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 4, 0);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(tree));
    assert_eq!(app.dom().node(s).scroll_top(), Some(4));
}

/// In an `App`, a handler that moves an element and focuses it gets it
/// scrolled into view where it is laid out next — five rows inserted
/// above `t` take it from row 1 to row 6, below the 3-row scroller, which
/// scrolls to 4 (its stale row 1 was in view: no scroll).
#[test]
fn focus_in_a_handler_scrolls_against_the_next_layout() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "class", "s").unwrap();
    dom.append_child(root, s).unwrap();
    let first = dom.create_element("div");
    let t0 = dom.create_text_node("a");
    dom.append_child(first, t0).unwrap();
    dom.append_child(s, first).unwrap();
    let target = dom.create_element("button");
    let tt = dom.create_text_node("T");
    dom.append_child(target, tt).unwrap();
    dom.append_child(s, target).unwrap();
    let go = dom.create_element("button");
    let tg = dom.create_text_node("Go");
    dom.append_child(go, tg).unwrap();
    dom.append_child(root, go).unwrap();
    dom.add_event_listener(go, "click", ListenerOptions::default(), move |ctx| {
        for _ in 0..5 {
            let d = ctx.dom.create_element("div");
            let t = ctx.dom.create_text_node("x");
            ctx.dom.append_child(d, t).unwrap();
            ctx.dom.insert_before(s, d, Some(target)).unwrap();
        }
        ctx.dom.node_mut(target).focus();
    })
    .unwrap();
    let sheet = rdom_css::from_css_strict(
        ".s { overflow-y: auto; height: 3; display: block } button { display: block }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 3);
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(target));
    assert_eq!(app.dom().node(s).scroll_top(), Some(4));
}
