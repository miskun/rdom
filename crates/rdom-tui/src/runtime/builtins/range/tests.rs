//! `<input type="range">` tests.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::range;
use crate::style::Stylesheet;

fn test_app(dom: TuiDom) -> App<TestBackend> {
    // `Stylesheet::new()` already carries the `input[type=range]`
    // UA rule; `App::build` wires the keyboard handler + paint
    // hook automatically, so the test setup matches a real app.
    let sheet = Stylesheet::new();
    let backend = TestBackend::new(30, 3);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

fn range_app(min: f64, max: f64, value: Option<f64>) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "range").unwrap();
    dom.set_attribute(inp, "min", &min.to_string()).unwrap();
    dom.set_attribute(inp, "max", &max.to_string()).unwrap();
    if let Some(v) = value {
        dom.set_attribute(inp, "value", &v.to_string()).unwrap();
    }
    dom.append_child(root, inp).unwrap();
    let app = test_app(dom);
    (app, inp)
}

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

// ── Attribute defaults ────────────────────────────────────────────

#[test]
fn value_defaults_to_midpoint_when_unset() {
    let (app, inp) = range_app(0.0, 100.0, None);
    assert_eq!(range::value_of(app.dom(), inp), 50.0);
}

#[test]
fn value_defaults_to_midpoint_for_nonzero_min() {
    let (app, inp) = range_app(10.0, 20.0, None);
    assert_eq!(range::value_of(app.dom(), inp), 15.0);
}

#[test]
fn value_respects_explicit_attribute() {
    let (app, inp) = range_app(0.0, 100.0, Some(42.0));
    assert_eq!(range::value_of(app.dom(), inp), 42.0);
}

#[test]
fn min_max_defaults_are_0_and_100() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "type", "range").unwrap();
    dom.append_child(root, inp).unwrap();
    assert_eq!(range::range_of(&dom, inp), (0.0, 100.0));
}

#[test]
fn step_defaults_to_one() {
    let (app, inp) = range_app(0.0, 100.0, None);
    assert_eq!(range::step_of(app.dom(), inp), 1.0);
}

#[test]
fn step_any_treats_as_one() {
    let (mut app, inp) = range_app(0.0, 1.0, None);
    app.dom_mut().set_attribute(inp, "step", "any").unwrap();
    assert_eq!(range::step_of(app.dom(), inp), 1.0);
}

// ── Keyboard ─────────────────────────────────────────────────────

#[test]
fn right_arrow_increases_by_step() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Right));
    assert_eq!(range::value_of(app.dom(), inp), 51.0);
}

#[test]
fn up_arrow_increases_by_step() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Up));
    assert_eq!(range::value_of(app.dom(), inp), 51.0);
}

#[test]
fn left_arrow_decreases_by_step() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Left));
    assert_eq!(range::value_of(app.dom(), inp), 49.0);
}

#[test]
fn down_arrow_decreases_by_step() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Down));
    assert_eq!(range::value_of(app.dom(), inp), 49.0);
}

#[test]
fn home_sets_to_min() {
    let (mut app, inp) = range_app(10.0, 20.0, Some(15.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Home));
    assert_eq!(range::value_of(app.dom(), inp), 10.0);
}

#[test]
fn end_sets_to_max() {
    let (mut app, inp) = range_app(10.0, 20.0, Some(15.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::End));
    assert_eq!(range::value_of(app.dom(), inp), 20.0);
}

#[test]
fn pagedown_decreases_by_ten_steps() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::PageDown));
    assert_eq!(range::value_of(app.dom(), inp), 40.0);
}

#[test]
fn pageup_increases_by_ten_steps() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::PageUp));
    assert_eq!(range::value_of(app.dom(), inp), 60.0);
}

#[test]
fn value_clamps_to_max() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(99.0));
    app.dom_mut().set_focused(Some(inp));
    for _ in 0..5 {
        app.handle_event(key(KeyCode::Right));
    }
    assert_eq!(range::value_of(app.dom(), inp), 100.0);
}

#[test]
fn value_clamps_to_min() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(1.0));
    app.dom_mut().set_focused(Some(inp));
    for _ in 0..5 {
        app.handle_event(key(KeyCode::Left));
    }
    assert_eq!(range::value_of(app.dom(), inp), 0.0);
}

#[test]
fn disabled_range_ignores_keyboard() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_attribute(inp, "disabled", "").unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Right));
    assert_eq!(range::value_of(app.dom(), inp), 50.0);
}

#[test]
fn ctrl_modifier_skips_range_keyboard_handler() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Right,
        KeyModifiers::CONTROL,
    )));
    assert_eq!(range::value_of(app.dom(), inp), 50.0);
}

#[test]
fn custom_step_controls_increment() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    app.dom_mut().set_attribute(inp, "step", "5").unwrap();
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Right));
    assert_eq!(range::value_of(app.dom(), inp), 55.0);
}

// ── Events ────────────────────────────────────────────────────────

#[test]
fn keyboard_change_fires_input_and_change() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    let counts: Rc<Cell<(u32, u32)>> = Rc::new(Cell::new((0, 0)));
    {
        let c = counts.clone();
        app.dom_mut()
            .add_event_listener(inp, "input", ListenerOptions::default(), move |_| {
                let (i, ch) = c.get();
                c.set((i + 1, ch));
            })
            .unwrap();
    }
    {
        let c = counts.clone();
        app.dom_mut()
            .add_event_listener(inp, "change", ListenerOptions::default(), move |_| {
                let (i, ch) = c.get();
                c.set((i, ch + 1));
            })
            .unwrap();
    }
    app.dom_mut().set_focused(Some(inp));
    app.handle_event(key(KeyCode::Right));
    assert_eq!(counts.get(), (1, 1));
}

#[test]
fn no_op_keyboard_does_not_fire_events() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(100.0));
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(inp, "input", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(inp));
    // Already at max — Right is a no-op (clamp preserves value).
    app.handle_event(key(KeyCode::Right));
    assert_eq!(fired.get(), 0);
}

// ── Programmatic API ──────────────────────────────────────────────

#[test]
fn set_value_clamps_and_fires_events() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(50.0));
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(inp, "change", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    range::set_value(app.dom_mut(), inp, 999.0);
    assert_eq!(range::value_of(app.dom(), inp), 100.0);
    assert_eq!(fired.get(), 1);
}

#[test]
fn set_value_noop_when_same_as_current() {
    let (mut app, inp) = range_app(0.0, 100.0, Some(42.0));
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(inp, "change", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    range::set_value(app.dom_mut(), inp, 42.0);
    assert_eq!(fired.get(), 0);
}

// ── Attach / non-range elements ────────────────────────────────────

#[test]
fn attach_is_noop_on_non_range_input() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let text_inp = dom.create_element("input");
    dom.set_attribute(text_inp, "type", "text").unwrap();
    dom.append_child(root, text_inp).unwrap();
    range::attach(&mut dom, text_inp);
    use crate::runtime::builtins::canvas;
    assert!(!canvas::has_paint(&dom, text_inp));
}

#[test]
fn install_attaches_canvas_paint_to_existing_ranges() {
    // Required test (per the architect blocker for this round):
    // proves App::build → range::attach_all → canvas::set_paint
    // wires the paint hook automatically. Future App::build
    // refactors that drop the install call surface here.
    use crate::runtime::builtins::canvas;
    let (app, inp) = range_app(0.0, 100.0, None);
    assert!(canvas::has_paint(app.dom(), inp));
}
