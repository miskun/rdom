//! Where a layout flush cannot bring the layout up to date — before the
//! `App`'s first frame, after a resize, a sheet-set change or a change of
//! the document's environment, while a `<style>` sheet waits to be
//! re-parsed — it says so (`LayoutFlush::Stale`), and the scrolls asked
//! for meanwhile wait for the next layout (`pending`): done after it, in
//! the order asked, a focus's only while the element keeps the focus.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::Event as CtEvent;
use rdom_core::{ListenerOptions, NodeId};

use super::{LayoutFlush, flush_layout};
use crate::prelude::*;
use crate::style::CascadeExt;

const CSS: &str = ".s { overflow-y: auto; height: 3; display: block } .r { display: block }";

/// A 20 × 8 `App` over `<style>` (`style`), a 3-row scroller `.s` of
/// eight focusable one-row `.r` rows, and a `Go` button, styled by `css`;
/// no frame drawn.
fn app(css: &str, style: &str) -> (App<TestBackend>, NodeId, Vec<NodeId>, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let mut markup = format!("<style>{style}</style><div class=\"s\">");
    for k in 0..8 {
        markup.push_str(&format!(
            "<div class=\"r\" tabindex=\"0\" id=\"r{k}\">r{k}</div>"
        ));
    }
    markup.push_str("</div><button id=\"go\">Go</button>");
    rdom_parser::parse_into(&mut dom, &markup, root).unwrap();
    let s = dom.query_selector(".s").unwrap().id();
    let rows = (0..8)
        .map(|k| dom.get_element_by_id(&format!("r{k}")).unwrap())
        .collect();
    let go = dom.get_element_by_id("go").unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let app = App::with_backend(dom, parsed.stylesheet, terminal).unwrap();
    (app, s, rows, go)
}

fn top(app: &App<TestBackend>, s: NodeId) -> Option<i32> {
    app.dom().node(s).scroll_top()
}

// ---- what flush_layout reports ---------------------------------------------

#[test]
fn flush_layout_before_the_first_frame_is_stale() {
    let (mut app, _, _, _) = app(CSS, "");
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::Stale);
}

#[test]
fn flush_layout_after_a_frame_is_current() {
    let (mut app, _, _, _) = app(CSS, "");
    app.draw_if_dirty().unwrap();
    let flushed = flush_layout(app.dom_mut());
    assert_eq!(flushed, LayoutFlush::Current);
    assert!(flushed.is_current());
}

#[test]
fn flush_layout_on_a_bare_document_is_unmanaged() {
    let mut dom = TuiDom::new();
    let flushed = flush_layout(&mut dom);
    assert_eq!(flushed, LayoutFlush::Unmanaged);
    assert!(!flushed.is_current());
}

// ---- the cases that wait ---------------------------------------------------

/// A `resize` listener runs before the frame lays the new size out: its
/// focus scrolls after that layout.
#[test]
fn a_resize_listener_focus_scrolls_after_the_next_layout() {
    let (mut app, s, rows, _) = app(CSS, "");
    app.draw_if_dirty().unwrap();
    let seen = Rc::new(Cell::new(None));
    let seen_in = seen.clone();
    let target = rows[6];
    let root = app.dom().root();
    app.dom_mut()
        .add_event_listener(root, "resize", ListenerOptions::default(), move |ctx| {
            seen_in.set(Some(flush_layout(ctx.dom)));
            ctx.dom.node_mut(target).focus();
        })
        .unwrap();
    app.terminal_mut().backend_mut().resize(22, 8);
    app.handle_event(CtEvent::Resize(22, 8));
    assert_eq!(seen.get(), Some(LayoutFlush::Stale));
    assert_eq!(top(&app, s), Some(0), "waits for the layout");
    app.draw_if_dirty().unwrap();
    assert_eq!(top(&app, s), Some(4), "`nearest`: row 6 at the bottom");
}

/// A sheet pushed since the last frame: the scroll waits for the frame,
/// which cascades under it — `.r` rows two tall put `r3` at 6.
#[test]
fn scroll_into_view_after_a_sheet_change_waits_for_the_frame() {
    let (mut app, s, rows, _) = app(CSS, "");
    app.draw_if_dirty().unwrap();
    app.push_stylesheet(rdom_css::from_css_strict(".r { height: 2 }").unwrap());
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::Stale);
    app.dom_mut().node_mut(rows[3]).scroll_into_view().unwrap();
    assert_eq!(top(&app, s), Some(0));
    app.draw_if_dirty().unwrap();
    assert_eq!(top(&app, s), Some(6));
}

/// A `<style>` element's text edited since the last frame: its sheet is
/// re-parsed by the frame, so the scroll waits for it.
#[test]
fn scroll_into_view_while_a_style_element_waits_waits_for_the_frame() {
    let (mut app, s, rows, _) = app(CSS, ".x { color: red }");
    app.draw_if_dirty().unwrap();
    let dom = app.dom_mut();
    let style = dom.query_selector("style").unwrap().id();
    let text = dom.node(style).first_child().unwrap().id();
    dom.node_mut(text)
        .set_node_value(".r { height: 2 }")
        .unwrap();
    assert_eq!(flush_layout(dom), LayoutFlush::Stale);
    dom.node_mut(rows[3]).scroll_into_view().unwrap();
    assert_eq!(top(&app, s), Some(0));
    app.draw_if_dirty().unwrap();
    assert_eq!(top(&app, s), Some(6));
}

/// The document's environment changed — here its color scheme, which a
/// `prefers-color-scheme` query reads — with no mutation the tracker sees:
/// the frame restyles the whole tree, and the scroll waits for it.
#[test]
fn a_color_scheme_change_waits_for_the_frame() {
    let css = format!("{CSS} @media (prefers-color-scheme: light) {{ .r {{ height: 2 }} }}");
    let (mut app, s, rows, _) = app(&css, "");
    app.set_color_scheme(rdom_style::color::ColorScheme::Dark);
    app.draw_if_dirty().unwrap();
    app.set_color_scheme(rdom_style::color::ColorScheme::Light);
    assert_eq!(
        app.dom().color_scheme(),
        rdom_style::color::ColorScheme::Light
    );
    assert_eq!(flush_layout(app.dom_mut()), LayoutFlush::Stale);
    app.dom_mut().node_mut(rows[3]).scroll_into_view().unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(top(&app, s), Some(6));
}

// ---- how waiting scrolls run -----------------------------------------------

/// Waiting scrolls run in the order asked: the later one wins on the
/// container both move, as it would done at once.
#[test]
fn waiting_scrolls_run_in_order() {
    let (mut app, s, rows, _) = app(CSS, "");
    app.dom_mut().node_mut(rows[6]).scroll_into_view().unwrap();
    app.dom_mut().node_mut(rows[1]).scroll_into_view().unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(top(&app, s), Some(1));
}

/// A waiting focus scroll is dropped once the element has lost the focus:
/// HTML scrolls the element focused, not one focused a moment ago.
#[test]
fn a_waiting_focus_scroll_is_dropped_when_the_focus_moves_on() {
    let (mut app, s, rows, go) = app(CSS, "");
    app.dom_mut().node_mut(rows[6]).focus();
    app.dom_mut().node_mut(go).focus();
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(go));
    assert_eq!(top(&app, s), Some(0));
}

/// A waiting scroll whose element left the document is dropped.
#[test]
fn a_waiting_scroll_of_a_removed_element_is_dropped() {
    let (mut app, s, rows, _) = app(CSS, "");
    app.dom_mut().node_mut(rows[6]).scroll_into_view().unwrap();
    app.dom_mut().remove_child(s, rows[6]).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(top(&app, s), Some(0));
}
