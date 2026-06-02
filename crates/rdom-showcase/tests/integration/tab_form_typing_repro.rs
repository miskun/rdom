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
use rdom_tui::node::TuiNodeExt;
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

fn find_by_class(dom: &TuiDom, id: NodeId, class: &str) -> Option<NodeId> {
    if dom
        .node(id)
        .get_attribute("class")
        .map(|s| s.split_whitespace().any(|c| c == class))
        .unwrap_or(false)
    {
        return Some(id);
    }
    for c in dom.node(id).child_nodes() {
        if let Some(f) = find_by_class(dom, c.id(), class) {
            return Some(f);
        }
    }
    None
}

/// Build the showcase, switch to Tab Form, in a `w×h` viewport. Returns the
/// app plus the first `<input>` and the `.view-content` scroll pane.
fn tab_form_app(w: u16, h: u16) -> (App<TestBackend>, NodeId, NodeId) {
    let idx = DEMOS
        .iter()
        .position(|d| d.slug() == "forms/tab-form")
        .expect("tab-form demo registered");
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0);

    let terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    mount_demo(&mut state, app.dom_mut(), idx);
    app.draw_if_dirty().unwrap();

    let input = find_by_tag(app.dom(), app.dom().root(), "input").expect("input");
    let pane = find_by_class(app.dom(), app.dom().root(), "view-content").expect("view pane");
    (app, input, pane)
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

#[test]
fn pagedown_in_a_focused_input_scrolls_the_containing_pane() {
    // A small viewport so the Tab Form overflows its `.view-content` scroll
    // pane. Focus an input, then PageDown must scroll the PANE (the nearest
    // scrollable ancestor of the focused input), not no-op.
    let (mut app, input, pane) = tab_form_app(60, 9);

    for _ in 0..12 {
        if app.dom().focused() == Some(input) {
            break;
        }
        app.handle_event(key(KeyCode::Tab));
    }
    assert_eq!(
        app.dom().focused(),
        Some(input),
        "Tab should reach the input"
    );

    let scroll_y = |app: &App<TestBackend>| app.dom().node(pane).tui_ext().unwrap().scroll_y;
    // Precondition: the pane actually overflows (else the test proves nothing).
    let content = app
        .dom()
        .node(pane)
        .tui_ext()
        .unwrap()
        .scroll_content_height;
    let view = app.dom().node(pane).tui_ext().unwrap().layout.height as usize;
    assert!(
        content > view,
        "test viewport too tall; pane doesn't overflow (content={content}, view={view})"
    );

    let before = scroll_y(&app);
    app.handle_event(key(KeyCode::PageDown));
    app.draw_if_dirty().unwrap();
    assert!(
        scroll_y(&app) > before,
        "PageDown while focused on an input must scroll the containing pane \
         (before={before}, after={})",
        scroll_y(&app)
    );
}
