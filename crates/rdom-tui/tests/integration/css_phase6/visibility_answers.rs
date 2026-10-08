//! C6G-VISIBILITY-ONE-ANSWER — one "rendered and visible" answer for
//! focus (HTML §6.6.2: a focusable area is being rendered; as the
//! engines read it, an element whose used `visibility` is not `visible`
//! is not one): Tab, the public predicates, programmatic `focus()` and
//! the focus fixup that blurs a focused element once it is hidden.

use std::cell::Cell;
use std::rc::Rc;

use super::el;
use rdom_core::ListenerOptions;
use rdom_tui::runtime::focus::tabindex::{is_focusable, is_tab_focusable};
use rdom_tui::{App, Rect, Terminal, TestBackend, TuiAccessorsMut, TuiDom};

fn cascade(dom: &mut TuiDom, css: &str) {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses");
    rdom_tui::CascadeExt::cascade(dom, &sheet);
    rdom_tui::LayoutExt::layout_dom(dom, Rect::new(0, 0, 20, 5));
}

/// HTML §6.6.2: a `visibility: hidden` button — and one inside a
/// `display: none` box — is no focusable area, so the public predicates
/// say no, as Tab does.
#[test]
fn a_hidden_button_is_not_tab_focusable() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "button", "h");
    let n = el(&mut dom, root, "div", "n");
    let inner = el(&mut dom, n, "button", "");
    cascade(&mut dom, ".h { visibility: hidden } .n { display: none }");
    assert!(!is_tab_focusable(&dom, b));
    assert!(!is_focusable(&dom, b));
    assert!(!is_tab_focusable(&dom, inner));
    assert!(!is_focusable(&dom, inner));
}

/// HTML `focus()`: "if the element is not a focusable area, return" —
/// a hidden button, and a button in a `display: none` box, take no focus.
#[test]
fn programmatic_focus_refuses_a_hidden_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "button", "h");
    let n = el(&mut dom, root, "div", "n");
    let inner = el(&mut dom, n, "button", "");
    cascade(&mut dom, ".h { visibility: hidden } .n { display: none }");
    dom.node_mut(b).focus();
    assert_eq!(dom.focused(), None);
    dom.node_mut(inner).focus();
    assert_eq!(dom.focused(), None);
}

/// HTML "update the rendering": when the focused area stops being a
/// focusable area, the focus fixup runs the focusing steps for the
/// viewport — `blur` fires and nothing is focused.
#[test]
fn a_focused_element_that_becomes_hidden_is_blurred() {
    for class in ["h", "n"] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "button", "");
        let blurs = Rc::new(Cell::new(0));
        let seen = blurs.clone();
        dom.add_event_listener(b, "blur", ListenerOptions::default(), move |_| {
            seen.set(seen.get() + 1)
        })
        .unwrap();
        let sheet = rdom_css::from_css_strict(".h { visibility: hidden } .n { display: none }")
            .expect("sheet parses");
        let terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
        let mut app = App::with_backend(dom, sheet, terminal).unwrap();
        app.draw_if_dirty().unwrap();
        app.dom_mut().node_mut(b).focus();
        assert_eq!(app.dom().focused(), Some(b), "{class}");
        app.dom_mut().set_attribute(b, "class", class).unwrap();
        app.draw_if_dirty().unwrap();
        assert_eq!(app.dom().focused(), None, "{class}");
        assert_eq!(blurs.get(), 1, "{class}");
    }
}

/// C7G-FOCUS-FIXUP — HTML "update the rendering" runs the focus fixup
/// every rendering update, not only after a style change: a `visibility`
/// transition to `hidden` presents `visible` while it runs (CSS Display 3
/// §4), so the button keeps the focus, and the frame in which it ends —
/// with no cascade of its own — blurs it.
#[test]
fn a_visibility_transition_that_ends_hidden_blurs() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "button", "");
    let blurs = Rc::new(Cell::new(0));
    let seen = blurs.clone();
    dom.add_event_listener(b, "blur", ListenerOptions::default(), move |_| {
        seen.set(seen.get() + 1)
    })
    .unwrap();
    let sheet = rdom_css::from_css_strict(
        "button { transition: visibility 40ms linear } .h { visibility: hidden }",
    )
    .expect("sheet parses");
    let terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut().node_mut(b).focus();
    app.dom_mut().set_attribute(b, "class", "h").unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), Some(b), "visible while it runs");
    std::thread::sleep(std::time::Duration::from_millis(60));
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().focused(), None, "hidden once it ends");
    assert_eq!(blurs.get(), 1);
}

/// C7G-UPGRADE-GUIDE, then C12-FOCUS-FLUSH — the upgrade guide's "open
/// panel, focus input" case. HTML `focus()` refuses an element that is
/// not a focusable area, and decides it against up-to-date style: the
/// `App`'s document flushes the input's dirty style first
/// (`runtime::style_flush`), so a handler that shows a `display: none`
/// panel and focuses its input in the same call focuses it — and so does
/// an animation frame callback it requests, or one that callback
/// requests (the nested-callback workaround this used to need).
#[test]
fn focusing_a_just_shown_input_flushes_its_style() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use rdom_tui::runtime::timers::TuiTimers;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum When {
        Handler,
        OneAnimationFrame,
        TwoAnimationFrames,
    }
    for when in [
        When::Handler,
        When::OneAnimationFrame,
        When::TwoAnimationFrames,
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let open = el(&mut dom, root, "button", "");
        let panel = el(&mut dom, root, "div", "closed");
        let input = el(&mut dom, panel, "input", "");
        dom.add_event_listener(open, "click", ListenerOptions::default(), move |ctx| {
            ctx.dom.set_attribute(panel, "class", "").unwrap();
            match when {
                When::Handler => {
                    ctx.dom.node_mut(input).focus();
                }
                When::OneAnimationFrame => {
                    ctx.request_animation_frame(move |t, _| t.dom.node_mut(input).focus());
                }
                When::TwoAnimationFrames => {
                    ctx.request_animation_frame(move |t, _| {
                        t.request_animation_frame(move |t, _| t.dom.node_mut(input).focus());
                    });
                }
            }
        })
        .unwrap();
        let sheet = rdom_css::from_css_strict(".closed { display: none }").expect("sheet parses");
        let terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
        let mut app = App::with_backend(dom, sheet, terminal).unwrap();
        app.draw_if_dirty().unwrap();
        app.dom_mut().node_mut(open).focus();
        // Enter activates the focused button (HTML §4.10.6): its `click`
        // listener runs inside `handle_event`, and each `advance` is one
        // turn of the event loop — timers and animation frame callbacks,
        // then the frame.
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        for _ in 0..4 {
            app.advance(20).unwrap();
        }
        assert_eq!(app.dom().focused(), Some(input), "{when:?}");
    }
}
