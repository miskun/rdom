//! C12-FOCUS-FLUSH — HTML §6.6.3 "focusing steps" and the `focus()`
//! method (§6.6.6): whether the new focus target is a focusable area is
//! decided against up-to-date style, so the user agent updates style
//! first, as the engines do. Under an `App`, `focus()` cascades the
//! dirty subtrees that hold the element — and nothing else.

use crate::TuiDom;
use crate::accessors::TuiAccessorsMut;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;
use crate::style::cascade::match_probe;
use rdom_core::NodeId;

struct Page {
    app: App<TestBackend>,
    panel: NodeId,
    input: NodeId,
    other: NodeId,
}

/// A panel holding an input, and an unrelated section of ten paragraphs.
fn page(css: &str) -> Page {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = |dom: &mut TuiDom, parent: NodeId, tag: &str, class: &str| {
        let id = dom.create_element(tag);
        if !class.is_empty() {
            dom.set_attribute(id, "class", class).unwrap();
        }
        dom.append_child(parent, id).unwrap();
        id
    };
    let panel = el(&mut dom, root, "div", "panel");
    let input = el(&mut dom, panel, "input", "");
    let other = el(&mut dom, root, "section", "");
    for _ in 0..10 {
        el(&mut dom, other, "p", "");
    }
    let sheet = rdom_css::parse(css);
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 16)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    Page {
        app,
        panel,
        input,
        other,
    }
}

/// §6.6.6: `focus()` on an input whose panel the same code just showed
/// focuses it — the style the panel's change calls for is computed first.
#[test]
fn focus_flushes_the_style_of_a_just_shown_element() {
    let mut p = page(".panel { display: none } .panel.open { display: block }");
    p.app
        .dom_mut()
        .set_attribute(p.panel, "class", "panel open")
        .unwrap();
    p.app.dom_mut().node_mut(p.input).focus();
    assert_eq!(p.app.dom().focused(), Some(p.input));
    // The reverse: a panel hidden by the same code refuses the focus.
    p.app.dom_mut().node_mut(p.input).blur();
    p.app
        .dom_mut()
        .set_attribute(p.panel, "class", "panel")
        .unwrap();
    p.app.dom_mut().node_mut(p.input).focus();
    assert_eq!(p.app.dom().focused(), None, "a just-hidden input");
}

/// The flush cascades only the dirty subtrees holding the element — the
/// panel and its input, not the unrelated section that is dirty too — and
/// leaves the rest for the frame; with nothing dirty above the element it
/// matches nothing. (`match_probe` counts rule-matching passes: one per
/// element and pseudo-element it styles.)
#[test]
fn focus_flushes_only_the_dirty_subtrees_above_the_element() {
    let mut p = page(".panel.open { color: blue } .x { color: red }");
    // What cascading the panel's subtree costs: a frame with only it
    // dirty.
    p.app
        .dom_mut()
        .set_attribute(p.panel, "class", "panel open")
        .unwrap();
    match_probe::take();
    p.app.draw_if_dirty().unwrap();
    let panel_cost = match_probe::take();
    p.app
        .dom_mut()
        .set_attribute(p.panel, "class", "panel")
        .unwrap();
    p.app.draw_if_dirty().unwrap();
    // Both dirty; the flush cascades the panel's subtree only.
    p.app
        .dom_mut()
        .set_attribute(p.other, "class", "x")
        .unwrap();
    p.app
        .dom_mut()
        .set_attribute(p.panel, "class", "panel open")
        .unwrap();
    match_probe::take();
    p.app.dom_mut().node_mut(p.input).focus();
    assert_eq!(p.app.dom().focused(), Some(p.input));
    assert_eq!(match_probe::take(), panel_cost, "the panel's subtree only");
    // The section is left for the frame (the focus change itself dirtied
    // the panel's `:focus-within` chain again).
    assert!(p.app.dirty_roots_snapshot().contains(&p.other));
    p.app.draw_if_dirty().unwrap();
    match_probe::take();
    // Nothing dirty above the element: no cascade at all.
    p.app.dom_mut().node_mut(p.input).focus();
    assert_eq!(match_probe::take(), 0, "a clean element flushes nothing");
}

/// The frame after a flush still lays out and paints what the flush
/// restyled, and still starts the transitions its change calls for (CSS
/// Transitions 1 §3: the before-change style is the last frame's) — with
/// no dirty root left for it to cascade.
#[test]
fn the_frame_after_a_flush_lays_out_and_starts_its_transitions() {
    let mut p = page(
        ".panel { opacity: 0.5; transition: opacity 100ms linear } \
         .panel.open { opacity: 1 }",
    );
    p.app
        .dom_mut()
        .set_attribute(p.panel, "class", "panel open")
        .unwrap();
    assert!(super::flush_style(p.app.dom_mut(), p.input));
    assert!(
        p.app.dirty_roots_snapshot().is_empty(),
        "the flush took them"
    );
    p.app.take_frame_stats();
    p.app.advance(0).unwrap();
    let stats = p.app.take_frame_stats();
    assert_eq!(stats.layouts, 1, "the flushed change is laid out");
    assert_eq!(stats.paints, 1);
    assert_eq!(
        p.app.get_animations(p.panel).len(),
        1,
        "the opacity transition started"
    );
}

/// A document no `App` runs has nothing published: the caller cascades
/// it, and a flush does nothing.
#[test]
fn a_bare_document_flushes_nothing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    assert!(!super::flush_style(&mut dom, div));
}
