//! C14-CONTAINER in an `App`: a resize that moves a query container's
//! width re-evaluates its queries in the frame's layout (the App's own
//! sheets, published on the document), and the restyled elements'
//! transitions start at the next frame, as a style flush's do.

use crossterm::event::Event as CtEvent;

use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::{TuiDom, TuiNodeExt};

fn app(css: &str, w: u16) -> (App<TestBackend>, rdom_core::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, r#"<div id="card"><p id="t">x</p></div>"#, root).unwrap();
    let t = dom.get_element_by_id("t").unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(w, 3)).unwrap();
    let mut app = App::with_backend(dom, parsed.stylesheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    (app, t)
}

fn resize(app: &mut App<TestBackend>, w: u16) {
    app.terminal_mut().backend_mut().resize(w, 3);
    app.handle_event(CtEvent::Resize(w, 3));
    app.draw_if_dirty().unwrap();
}

const CSS: &str = "#card { container-type: inline-size; width: 50% }
                   #t { color: rgb(0, 0, 255) }
                   @container (width >= 30) { #t { color: rgb(255, 0, 0) } }";

/// CSS Conditional 5 §6.4: the query follows the container through a
/// resize, with no viewport unit and no media query in the sheet.
#[test]
fn a_resize_requeries_the_container() {
    let (mut app, t) = app(CSS, 40);
    let fg = |app: &App<TestBackend>| app.dom().node(t).computed().unwrap().fg;
    assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
    resize(&mut app, 60);
    assert_eq!(fg(&app), Color::Rgb(255, 0, 0));
    resize(&mut app, 40);
    assert_eq!(fg(&app), Color::Rgb(0, 0, 255));
}

/// CSS Transitions 1 §3: a style change a container query makes is a
/// style change — its transition starts (at the next frame).
#[test]
fn a_container_flip_starts_a_transition() {
    let (mut app, t) = app(&format!("{CSS} #t {{ transition: color 1s }}"), 40);
    resize(&mut app, 60);
    app.advance(16).unwrap();
    assert_eq!(app.get_animations(t).len(), 1);
}
