//! C14G-ROOT-ELEMENT in an `App`: the root fragment is the root element —
//! restyled when what its selectors read changes, its background the
//! canvas's.

use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::{TuiDom, TuiNodeExt};

fn app(markup: &str, css: &str) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut app = App::with_backend(dom, crate::style::Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(parsed.stylesheet);
    app.advance(0).unwrap();
    app
}

/// Selectors 4 §4.5 (`:has()`): `:root:has(.on)` follows a class set on a
/// descendant — the root is restyled, and what inherits from it with it.
#[test]
fn root_has_follows_its_descendants() {
    let mut app = app(
        r#"<p id="p">x</p>"#,
        ":root { color: rgb(0, 0, 255) } :root:has(.on) { color: rgb(255, 0, 0) }",
    );
    let p = app.dom().get_element_by_id("p").unwrap();
    let fg = |app: &App<TestBackend>| app.dom().node(p).computed().unwrap().fg;
    assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
    app.dom_mut().set_attribute(p, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(fg(&app), Color::Rgb(255, 0, 0));
}

/// Selectors 4 §9.2: `:root:hover` holds while anything is hovered —
/// the root follows a hover arriving from nothing.
#[test]
fn root_hover_follows_the_pointer() {
    use crossterm::event::{Event as CtEvent, MouseEvent, MouseEventKind};
    let mut app = app(
        r#"<p id="p">x</p>"#,
        ":root { color: rgb(0, 0, 255) } :root:hover { color: rgb(255, 0, 0) }",
    );
    let p = app.dom().get_element_by_id("p").unwrap();
    let fg = |app: &App<TestBackend>| app.dom().node(p).computed().unwrap().fg;
    assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: crossterm::event::KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
    assert_eq!(app.dom().hovered(), Some(p));
    assert_eq!(fg(&app), Color::Rgb(255, 0, 0));
}

/// The analysis behind it: only a selector that can match the root by a
/// user-action pseudo-class puts the root in the chain.
#[test]
fn root_state_is_read_only_by_nameless_compounds() {
    let uses =
        |css: &str| crate::style::dirty_tracker::uses_root_state(&rdom_css::parse(css).stylesheet);
    assert!(uses(":root:hover { color: red }"));
    assert!(uses(":focus-within { color: red }"));
    assert!(uses(":root:not(:hover) p { color: red }"));
    assert!(!uses(
        "button:hover, .x:focus-within, a:hover span { color: red }"
    ));
    assert!(!uses(":root { color: red }"));
    assert!(
        !crate::style::dirty_tracker::uses_root_state(&crate::style::Stylesheet::new()),
        "the UA sheet has none"
    );
}

fn click(app: &mut App<TestBackend>, column: u16, row: u16) {
    use crossterm::event::{
        Event as CtEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        }));
        app.advance(0).unwrap();
    }
}

/// CSSOM View §5 / UI Events: a click on the canvas outside every box
/// targets the root element — the root fragment (C14G-ROOT-ELEMENT,
/// C14-HIT-HTML) — so a listener there hears it.
#[test]
fn a_canvas_click_reaches_the_root() {
    let mut app = app(r#"<p id="p">x</p>"#, "p { margin: 0 }");
    let root = app.dom().root();
    let heard = std::rc::Rc::new(std::cell::Cell::new(None));
    let h = heard.clone();
    app.dom_mut()
        .add_event_listener(
            root,
            "click",
            rdom_core::ListenerOptions::default(),
            move |ctx| h.set(Some(ctx.event.target)),
        )
        .unwrap();
    click(&mut app, 8, 2);
    assert_eq!(heard.get(), Some(Some(root)));
}

/// HTML §6.12 light dismiss: a click on the canvas closes an auto popover
/// even when the popover is the first top-level element (hitting "the
/// document element" there would have hit the popover itself).
#[test]
fn a_canvas_click_light_dismisses_a_first_popover() {
    let mut app = app(
        r#"<div id="pop" popover>menu</div><p>x</p>"#,
        "p { margin: 0 } #pop { position: absolute; top: 0; left: 0; margin: 0; padding: 0; border: none }",
    );
    let pop = app.dom().get_element_by_id("pop").unwrap();
    crate::runtime::builtins::popover::show_popover(app.dom_mut(), pop).unwrap();
    app.advance(0).unwrap();
    assert!(crate::runtime::builtins::popover::is_showing(
        app.dom(),
        pop
    ));
    click(&mut app, 8, 2);
    assert!(!crate::runtime::builtins::popover::is_showing(
        app.dom(),
        pop
    ));
}
