//! M5 D6 — implicit event dispatch on detach.
//!
//! Tests `EVT-DETACH-1` closure: when the focused or hovered
//! element is detached from the tree, the runtime dispatches
//! the implicit events browsers fire in the same situation —
//! `blur` + `focusout` on the previously-focused element,
//! `mouseout` + `mouseleave` on the previously-hovered element.
//!
//! All dispatches happen BEFORE the structural detach, while the
//! tree's parent chain is still intact, so bubbling works
//! through the still-attached ancestors.

use std::cell::Cell;
use std::rc::Rc;

use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::{App, ListenerOptions, Stylesheet, TuiDom};

fn make_app() -> (
    App<TestBackend>,
    rdom_tui::NodeId,
    rdom_tui::NodeId,
    rdom_tui::NodeId,
) {
    // <root>
    //   <outer>
    //     <inner>focusable / hoverable</inner>
    //   </outer>
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let inner = dom.create_element("div");
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let backend = TestBackend::new(20, 5);
    let terminal = Terminal::new(backend).unwrap();
    let app = App::with_backend(dom, Stylesheet::bare(), terminal).unwrap();
    (app, root, outer, inner)
}

#[test]
fn detach_focused_dispatches_blur_then_focusout() {
    let (mut app, _root, outer, inner) = make_app();
    app.dom_mut().set_focused(Some(inner));

    let log: Rc<std::cell::RefCell<Vec<String>>> = Rc::new(std::cell::RefCell::new(Vec::new()));
    let l = log.clone();
    app.dom_mut()
        .add_event_listener(inner, "blur", ListenerOptions::default(), move |_| {
            l.borrow_mut().push("blur".into());
        })
        .unwrap();
    let l = log.clone();
    app.dom_mut()
        .add_event_listener(inner, "focusout", ListenerOptions::default(), move |_| {
            l.borrow_mut().push("focusout".into());
        })
        .unwrap();

    // Detach `outer` — `inner` (focused) goes with it.
    app.dom_mut().clear_children(_root).unwrap();

    let events = log.borrow().clone();
    assert_eq!(
        events,
        vec!["blur".to_string(), "focusout".to_string()],
        "blur fires before focusout on implicit focus loss"
    );
    assert_eq!(app.dom().focused(), None, "focus cleared after detach");
    // outer remains here so the compiler doesn't drop our reference;
    // it's not asserted on.
    let _ = outer;
}

#[test]
fn detach_focused_focusout_bubbles_through_ancestors() {
    // `focusout` bubbles — handler on `outer` (an ancestor of
    // the focused `inner`) should fire because the tree is still
    // intact when the event dispatches.
    let (mut app, root, outer, inner) = make_app();
    app.dom_mut().set_focused(Some(inner));

    let outer_fired = Rc::new(Cell::new(false));
    let f = outer_fired.clone();
    app.dom_mut()
        .add_event_listener(outer, "focusout", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();

    app.dom_mut().clear_children(root).unwrap();
    assert!(
        outer_fired.get(),
        "focusout bubbles through still-attached ancestors"
    );
}

#[test]
fn detach_focused_blur_does_not_bubble() {
    // `blur` is non-bubbling per DOM. Handler on `outer` must
    // NOT fire when `inner` is the blur target.
    let (mut app, root, outer, inner) = make_app();
    app.dom_mut().set_focused(Some(inner));

    let outer_fired = Rc::new(Cell::new(false));
    let f = outer_fired.clone();
    app.dom_mut()
        .add_event_listener(outer, "blur", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();

    app.dom_mut().clear_children(root).unwrap();
    assert!(
        !outer_fired.get(),
        "blur on a descendant must not fire on the ancestor"
    );
}

#[test]
fn detach_hovered_dispatches_mouseout_then_mouseleave() {
    let (mut app, _root, outer, inner) = make_app();
    app.dom_mut().set_hovered(Some(inner));

    let log: Rc<std::cell::RefCell<Vec<String>>> = Rc::new(std::cell::RefCell::new(Vec::new()));
    let l = log.clone();
    app.dom_mut()
        .add_event_listener(inner, "mouseout", ListenerOptions::default(), move |_| {
            l.borrow_mut().push("mouseout".into());
        })
        .unwrap();
    let l = log.clone();
    app.dom_mut()
        .add_event_listener(inner, "mouseleave", ListenerOptions::default(), move |_| {
            l.borrow_mut().push("mouseleave".into());
        })
        .unwrap();

    app.dom_mut().clear_children(_root).unwrap();

    assert_eq!(
        log.borrow().clone(),
        vec!["mouseout".to_string(), "mouseleave".to_string()],
        "mouseout fires before mouseleave on implicit hover loss"
    );
    assert_eq!(app.dom().hovered(), None);
    let _ = outer;
}

#[test]
fn detach_hovered_mouseleave_does_not_bubble() {
    let (mut app, root, outer, inner) = make_app();
    app.dom_mut().set_hovered(Some(inner));

    let outer_fired = Rc::new(Cell::new(false));
    let f = outer_fired.clone();
    app.dom_mut()
        .add_event_listener(outer, "mouseleave", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();

    app.dom_mut().clear_children(root).unwrap();
    assert!(
        !outer_fired.get(),
        "mouseleave is non-bubbling — ancestor listener must not fire"
    );
}

#[test]
fn detach_node_with_both_focused_and_hovered_dispatches_full_ceremony() {
    let (mut app, root, _outer, inner) = make_app();
    app.dom_mut().set_focused(Some(inner));
    app.dom_mut().set_hovered(Some(inner));

    let log: Rc<std::cell::RefCell<Vec<String>>> = Rc::new(std::cell::RefCell::new(Vec::new()));
    for evt in ["blur", "focusout", "mouseout", "mouseleave"] {
        let l = log.clone();
        let name = evt.to_string();
        app.dom_mut()
            .add_event_listener(inner, evt, ListenerOptions::default(), move |_| {
                l.borrow_mut().push(name.clone());
            })
            .unwrap();
    }

    app.dom_mut().clear_children(root).unwrap();

    // Full ceremony: focus first, then hover. Per implicit_events.rs.
    assert_eq!(
        log.borrow().clone(),
        vec![
            "blur".to_string(),
            "focusout".to_string(),
            "mouseout".to_string(),
            "mouseleave".to_string(),
        ],
        "full ceremony fires in canonical order"
    );
}

#[test]
fn detach_unrelated_subtree_does_not_dispatch_implicit_events() {
    // Sanity guard: if the focused/hovered node is NOT inside the
    // subtree being detached, no implicit events fire.
    let (mut app, root, _outer, inner) = make_app();
    let sibling = app.dom_mut().create_element("div");
    app.dom_mut().append_child(root, sibling).unwrap();

    app.dom_mut().set_focused(Some(inner));

    let blur_fired = Rc::new(Cell::new(false));
    let f = blur_fired.clone();
    app.dom_mut()
        .add_event_listener(inner, "blur", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();

    // Detach the unrelated sibling, not the subtree containing `inner`.
    app.dom_mut().remove_child(root, sibling).unwrap();

    assert!(
        !blur_fired.get(),
        "unrelated detach must not fire implicit events"
    );
    assert_eq!(
        app.dom().focused(),
        Some(inner),
        "focused stays put when its subtree isn't being detached"
    );
}

#[test]
fn dispatched_events_are_synthetic() {
    // The implicit-detach dispatches mark events as synthetic so
    // handlers can distinguish them from user-initiated focus
    // changes / mouse motion.
    let (mut app, root, _outer, inner) = make_app();
    app.dom_mut().set_focused(Some(inner));

    let saw_synthetic = Rc::new(Cell::new(false));
    let s = saw_synthetic.clone();
    app.dom_mut()
        .add_event_listener(inner, "blur", ListenerOptions::default(), move |ctx| {
            s.set(ctx.event.is_synthetic());
        })
        .unwrap();

    app.dom_mut().clear_children(root).unwrap();

    assert!(
        saw_synthetic.get(),
        "implicit-detach blur is flagged synthetic"
    );
}
