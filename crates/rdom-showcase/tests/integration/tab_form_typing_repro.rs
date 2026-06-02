//! Regression: typing into a tab-form input after **switching to** the demo.
//!
//! `input::seed_all` runs once at `App::build`, so inputs in a demo mounted
//! *later* (the user switches to Tab Form from another demo) had no text-node
//! child — focusing them seeded no caret and the first keystroke was dropped
//! ("the input is focused but I can't type"). The focus path now seeds the
//! editable lazily (`input::ensure_seeded`), so a switched-in input is
//! typeable. Without the demo switch the bug doesn't reproduce — `seed_all`
//! covered the initial demo.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, TuiDom};

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

fn find_by_tag(dom: &TuiDom, id: NodeId, tag: &str) -> Option<NodeId> {
    if dom.node(id).tag_name() == Some(tag) {
        return Some(id);
    }
    for c in dom.node(id).child_nodes() {
        if let Some(f) = find_by_tag(dom, c.id(), tag) {
            return Some(f);
        }
    }
    None
}

#[test]
fn switched_in_tab_form_input_accepts_typed_text() {
    let idx = DEMOS
        .iter()
        .position(|d| d.slug() == "forms/tab-form")
        .expect("tab-form demo registered");
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    // Launch on the first demo…
    mount_demo(&mut state, &mut dom, 0);

    let backend = TestBackend::new(123, 26);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();

    // …then SWITCH to Tab Form (inputs mounted after `App::build`'s seed pass).
    mount_demo(&mut state, app.dom_mut(), idx);
    app.draw_if_dirty().unwrap();

    let input = find_by_tag(app.dom(), app.dom().root(), "input").expect("tab-form input");

    // Tab from the autofocused sidebar tree to the first input.
    for _ in 0..12 {
        if app.dom().focused() == Some(input) {
            break;
        }
        app.handle_event(key(KeyCode::Tab));
    }
    assert_eq!(
        app.dom().focused(),
        Some(input),
        "Tab should reach the form input"
    );

    for c in "hi".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    assert_eq!(
        app.dom().node(input).get_attribute("value"),
        Some("hi"),
        "a switched-in input must accept typed text (lazily seeded on focus)"
    );
}
