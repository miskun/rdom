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
