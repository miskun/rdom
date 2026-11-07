//! CAPTURE phase 4: an inspector's change — a forced state, a sheet edit —
//! restyles the app without app code seeing it: no mutation record, the
//! real state in `matches()`, and no event from an animation it started
//! (withheld and listed), while the app's own animations' events go out.

use std::cell::Cell;
use std::rc::Rc;

use rdom_core::{ForcedState, ListenerOptions, Mutation, MutationObserver, NodeId};

use super::App;
use crate::render::{Terminal, TestBackend};
use crate::{Color, TuiDom};

const CSS: &str = ".b { color: #0000ff; transition: color 100ms linear; }\n.b:hover { color: #ff0000; }\n.b.on { color: #00ff00; }";

struct Count(Rc<Cell<usize>>);
impl MutationObserver<crate::TuiExt> for Count {
    fn observe(&mut self, _: &mut TuiDom, _: &Mutation) {
        self.0.set(self.0.get() + 1);
    }
}

/// The app, its `.b` element, and its listener's and observer's counts.
type Fixture = (App<TestBackend>, NodeId, Rc<Cell<usize>>, Rc<Cell<usize>>);

fn app() -> Fixture {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = dom.create_element("p");
    dom.set_attribute(b, "class", "b").unwrap();
    let t = dom.create_text_node("x");
    dom.append_child(b, t).unwrap();
    dom.append_child(root, b).unwrap();
    let events = Rc::new(Cell::new(0));
    let e = events.clone();
    for name in ["transitionrun", "transitionstart", "transitionend"] {
        let e = e.clone();
        let _ = dom.add_event_listener(b, name, ListenerOptions::default(), move |_| {
            e.set(e.get() + 1)
        });
    }
    let terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
    let mut app = App::with_backend(dom, rdom_css::from_css(CSS), terminal)
        .unwrap()
        .with_virtual_clock();
    app.advance(0).unwrap();
    let records = Rc::new(Cell::new(0));
    app.dom_mut()
        .add_mutation_observer(Box::new(Count(records.clone())));
    (app, b, events, records)
}

fn fg(app: &App<TestBackend>, id: NodeId) -> Color {
    use crate::node::TuiNodeExt;
    app.dom().node(id).computed().unwrap().fg
}

#[test]
fn a_forced_state_restyles_unseen_and_its_transition_fires_no_event() {
    let (mut app, b, events, records) = app();
    app.inspect(|a| a.force_state(b, ForcedState::Hover, true))
        .unwrap();
    app.advance(200).unwrap();
    assert_eq!(fg(&app, b), Color::Rgb(255, 0, 0), "styled as hovered");
    assert!(
        !app.dom().matches(b, ":hover").unwrap(),
        "app code sees the real state"
    );
    assert_eq!(records.get(), 0, "no mutation record");
    assert_eq!(
        events.get(),
        0,
        "the inspector's transition fires no app event"
    );
    let suppressed = app.take_suppressed_events();
    let kinds: Vec<&str> = suppressed.iter().map(|e| e.event.as_str()).collect();
    assert_eq!(
        kinds,
        ["transitionrun", "transitionstart", "transitionend"],
        "{suppressed:?}"
    );
    assert!(suppressed.iter().all(|e| e.node == b && e.of == "color"));
    app.inspect(|a| a.clear_forced_states()).unwrap();
    app.advance(200).unwrap();
    assert_eq!(fg(&app, b), Color::Rgb(0, 0, 255), "back");
}

#[test]
fn the_apps_own_transition_still_fires_its_events() {
    let (mut app, b, events, _) = app();
    app.inspect(|a| a.force_state(b, ForcedState::Hover, true))
        .unwrap();
    app.advance(200).unwrap();
    app.inspect(|a| a.clear_forced_states()).unwrap();
    app.advance(200).unwrap();
    app.take_suppressed_events();
    // The app's own change of the same property: its events are the app's.
    app.dom_mut().set_attribute(b, "class", "b on").unwrap();
    app.advance(0).unwrap();
    app.advance(200).unwrap();
    assert_eq!(events.get(), 3, "run, start, end");
    assert!(app.take_suppressed_events().is_empty());
}

#[test]
fn a_sheet_edit_restyles_and_maps_from_a_trace() {
    let (mut app, b, _, records) = app();
    let trace = app.explain_style(b).unwrap();
    let rule = trace.winner("color").unwrap().rule.clone().unwrap();
    let id = app.stylesheet_at(rule.sheet).expect("an App sheet");
    app.inspect(|a| {
        a.edit_stylesheet(id, |s| {
            s.set_rule_declaration(rule.index, "color", "#00ffff", false)
        })
    })
    .unwrap()
    .unwrap()
    .unwrap();
    app.advance(200).unwrap();
    assert_eq!(fg(&app, b), Color::Rgb(0, 255, 255));
    assert_eq!(records.get(), 0, "an edit changes a sheet, not the DOM");
    assert_eq!(
        app.stylesheet_at(rule.sheet + 1),
        None,
        "past the App's sheets"
    );
}
