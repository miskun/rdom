//! `C1G-TRANSITION-PREV`: a registered custom property's transition
//! re-cascades its `var()` consumers every frame. CSS Transitions 1 §3
//! compares a style change against the *before-change style* — the
//! previous computed values with running animations brought up to the
//! current time — so an unrelated restyle mid-transition must see the
//! animated values the last frame produced, not the values of the last
//! cascade, or it starts transitions nothing asked for.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rdom_core::ListenerOptions;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

/// Transitions 1 §3: `--c` (registered `<color>`) transitions red →
/// blue and `color: var(--c)` follows it frame by frame. Adding an
/// unrelated class mid-transition changes nothing the transitions read,
/// so `color` is neither retargeted nor cancelled.
#[test]
fn an_unrelated_restyle_mid_transition_starts_no_transition() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(
        "@property --c { syntax: '<color>'; inherits: true; initial-value: red } \
         div { color: var(--c); transition: all 1s linear } \
         div.on { --c: blue }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let mut app = App::with_backend(
        dom,
        Stylesheet::new(),
        Terminal::new(TestBackend::new(10, 3)).unwrap(),
    )
    .unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    let cancelled: Rc<RefCell<Vec<String>>> = Rc::default();
    {
        let cancelled = cancelled.clone();
        app.dom_mut()
            .add_event_listener(
                div,
                "transitioncancel",
                ListenerOptions::default(),
                move |ctx| {
                    let t = ctx.event.detail.as_transition().expect("typed detail");
                    cancelled.borrow_mut().push(t.property_name.clone());
                },
            )
            .unwrap();
    }
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    // Real time moves the transitions (they run on the wall clock).
    std::thread::sleep(Duration::from_millis(40));
    app.advance(0).unwrap();
    assert!(cancelled.borrow().is_empty(), "{:?}", cancelled.borrow());

    app.dom_mut().set_attribute(div, "class", "on x").unwrap();
    app.advance(0).unwrap();
    assert!(
        cancelled.borrow().is_empty(),
        "the unrelated class change retargeted {:?}",
        cancelled.borrow()
    );
}
