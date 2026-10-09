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

/// An App `w` × 5 over `markup` styled by `css`, one frame drawn.
fn app_over(markup: &str, css: &str, w: u16) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(w, 5)).unwrap();
    let mut app = App::with_backend(dom, parsed.stylesheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app
}

fn resize_to(app: &mut App<TestBackend>, w: u16) {
    app.terminal_mut().backend_mut().resize(w, 5);
    app.handle_event(CtEvent::Resize(w, 5));
    app.draw_if_dirty().unwrap();
}

/// C14G-CONTAINER-LOOP (architect B2a): a container whose last reader is
/// gone — the class its query styled removed — stops costing: the next
/// resize re-cascades it once (the restyle of the reader alone cannot tell
/// that nothing else reads it), finds no reader and forgets it; a resize
/// after that re-cascades nothing, and a frame with nothing dirty lays out
/// nothing (the App idles; it re-laid out and re-cascaded eight times
/// every frame).
#[test]
fn a_container_nobody_reads_stops_costing() {
    let mut app = app_over(
        r#"<div id="card"><p id="t" class="badge">x</p></div>"#,
        "#card { container-type: inline-size; width: 50% } \
         @container (width > 30) { .badge { color: rgb(255, 0, 0) } }",
        80,
    );
    let t = app.dom().get_element_by_id("t").unwrap();
    app.dom_mut().set_attribute(t, "class", "").unwrap();
    app.draw_if_dirty().unwrap();
    let passes = crate::render::layout_pass::container_pass::probe::take;
    passes();
    resize_to(&mut app, 70);
    assert!(passes() <= 1, "one look at the old readers");
    app.advance(16).unwrap();
    resize_to(&mut app, 60);
    assert_eq!(passes(), 0, "no reader: no re-cascade");
    app.advance(16).unwrap();
    app.take_frame_stats();
    app.advance(16).unwrap();
    let idle = app.take_frame_stats();
    assert_eq!(idle.layouts, 0, "{idle:?}");
}

/// C14G-CONTAINER-LOOP (architect B2b): a scroll container whose query
/// shows a scrollbar that takes the query back (CSS Overflow 3 §3.4's
/// gutter narrows the content box the query reads) settles: the layout
/// stops re-cascading inside the pass bound, and the next frame idles.
#[test]
fn an_oscillating_container_settles_and_idles() {
    let mut app = app_over(
        r#"<div id="c"><div id="list">x</div></div>"#,
        "#c { container-type: inline-size; overflow-y: auto; height: 3; width: 31 } \
         @container (width > 30) { #list { height: 10 } }",
        40,
    );
    // The frame after the settling one lays out once (its restyled
    // roots' transition hook), and finds nothing stale.
    app.advance(16).unwrap();
    app.take_frame_stats();
    crate::render::layout_pass::container_pass::probe::take();
    app.advance(16).unwrap();
    app.advance(16).unwrap();
    let idle = app.take_frame_stats();
    assert_eq!(idle.layouts, 0, "{idle:?}");
    assert_eq!(crate::render::layout_pass::container_pass::probe::take(), 0);
}

/// C14G-CONTAINER-FIDELITY (architect N2): a container flip during the
/// layout's restyle is a style change whose transition starts in the same
/// frame — the frame paints the before-change value (it painted the
/// after-change one, then faded back from the before-change value).
#[test]
fn a_container_flip_paints_the_before_change_value_first() {
    let (mut app, t) = app(&format!("{CSS} #t {{ transition: color 1s linear }}"), 40);
    resize(&mut app, 60);
    assert_eq!(
        app.get_animations(t).len(),
        1,
        "started in the flip's frame"
    );
    let fg = app.dom().node(t).computed().unwrap().fg;
    assert_eq!(fg, Color::Rgb(0, 0, 255), "the before-change value");
}

/// CSS Transitions 1 §3 (API N2): an element's first style is no style
/// change — a query holding at the first render (resolved in the layout's
/// restyle, the container unmeasured before it) starts no transition.
#[test]
fn a_first_render_under_a_container_query_does_not_transition() {
    let (mut app, t) = app(&format!("{CSS} #t {{ transition: color 1s linear }}"), 60);
    app.advance(16).unwrap();
    assert!(app.get_animations(t).is_empty(), "no transition");
    assert_eq!(
        app.dom().node(t).computed().unwrap().fg,
        Color::Rgb(255, 0, 0)
    );
}

/// CSS Conditional 5 §6.4.2 (architect N3): a restyle that replays an
/// element's recorded matches — a registered custom property's transition
/// step — tests its `style()` container queries again: while the
/// container's animated `--on` is between 0 and 1 `#t` is blue, and it
/// turns red when `--on` reaches 1 (the replay kept the result of the
/// cascade, which read the after-change `--on`).
#[test]
fn a_restyle_replay_retests_style_queries() {
    let css = "@property --on { syntax: '<number>'; inherits: true; initial-value: 0 } \
               #card { transition: --on 100ms linear } #card.on { --on: 1 } \
               #t { color: rgb(0, 0, 255) } \
               @container style(--on: 1) { #t { color: rgb(255, 0, 0) } }";
    let (mut app, t) = app(css, 40);
    let card = app.dom().get_element_by_id("card").unwrap();
    app.dom_mut().set_attribute(card, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    let fg = |app: &App<TestBackend>| app.dom().node(t).computed().unwrap().fg;
    assert_eq!(fg(&app), Color::Rgb(0, 0, 255), "--on is about 0.5");
    for _ in 0..10 {
        app.advance(16).unwrap();
    }
    assert_eq!(fg(&app), Color::Rgb(255, 0, 0), "--on is 1");
}
