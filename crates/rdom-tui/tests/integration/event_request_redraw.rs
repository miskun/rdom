//! `EVENT-REDRAW-1`: a listener that mutates no DOM but calls
//! `ctx.request_redraw()` must still cause the runtime to repaint.
//!
//! Models the interactive-`<canvas>` case: paint reads external app
//! state (behind an `Rc<RefCell>`), a key/mouse handler mutates that
//! state, and without a DOM mutation the dirty tracker would never mark
//! the frame dirty. `request_redraw` bridges that gap.

use std::cell::Cell;
use std::rc::Rc;

use rdom_core::EventCtx;
use rdom_tui::prelude::*;

/// The flag the event carries is the substrate contract; assert it
/// directly (the App loop ORs `event.redraw_requested()` into its
/// repaint decision — see runtime/app/mod.rs and the router).
#[test]
fn request_redraw_sets_event_flag_without_dom_mutation() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();

    // External (non-DOM) state the "paint" would read.
    let external = Rc::new(Cell::new(0u32));
    let ext = external.clone();
    dom.add_event_listener(
        btn,
        "keydown",
        ListenerOptions::default(),
        move |ctx: &mut EventCtx<'_, _>| {
            ext.set(ext.get() + 1); // mutate state OUTSIDE the DOM
            ctx.request_redraw(); // ← the new affordance
        },
    )
    .unwrap();

    let mut e = Event::new("keydown");
    dom.dispatch_event(btn, &mut e).unwrap();

    assert_eq!(external.get(), 1, "listener ran");
    assert!(
        e.redraw_requested(),
        "request_redraw() must set the event's redraw flag (the host harvests this)"
    );
}

/// A listener that does NOT request a redraw leaves the flag clear.
#[test]
fn no_request_leaves_flag_clear() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();
    dom.add_event_listener(btn, "click", ListenerOptions::default(), |_ctx| {})
        .unwrap();

    let mut e = Event::new("click");
    dom.dispatch_event(btn, &mut e).unwrap();
    assert!(!e.redraw_requested());
}
