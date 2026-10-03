//! Regression: typing into a tab-form input after **switching to** the demo.
//!
//! `input::seed_all` runs once at `App::build`, so inputs in a demo mounted
//! *later* (the user switches to Tab Form from another demo) had no text-node
//! child — focusing them seeded no caret and the first keystroke was dropped
//! ("the input is focused but I can't type"). The `App` now seeds inserted
//! controls at its next boundary (`INPUT-SEED-ON-INSERT-1`), and the focus
//! path seeds one focused before then (`input::ensure_seeded`) — which is
//! what the demo's `autofocus` Name input relies on, since `mount_demo`
//! focuses it right after inserting it.

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

    // The switch autofocuses the Name input (SHOWCASE-TAB-FORM-AUTOFOCUS-1);
    // the loop only Tabs if focus is elsewhere.
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

/// `SHOWCASE-TAB-FORM-AUTOFOCUS-1`: the Name input carries `autofocus`,
/// so switching to Tab form moves focus into it (the showcase runs the
/// `[autofocus]` algorithm on every mounted demo, as a browser does on
/// navigation) and typing lands there with no click and no Tab.
#[test]
fn switching_to_tab_form_focuses_the_name_input_and_typing_goes_into_it() {
    let (mut app, input, _pane) = tab_form_app(123, 26);
    assert_eq!(app.dom().node(input).get_attribute("name"), Some("name"));
    assert_eq!(
        app.dom().focused(),
        Some(input),
        "the switch focuses the autofocus Name input"
    );
    for c in "Ada".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    app.draw_if_dirty().unwrap();
    assert_eq!(app.dom().node(input).get_attribute("value"), Some("Ada"));
}

/// The demo's `MARKUP` (shown in the source disclosure) and its
/// hand-built DOM agree: the Name input, and only it, is `autofocus`.
#[test]
fn tab_form_markup_and_build_both_autofocus_the_name_input() {
    use rdom_showcase::demos::tab_form;
    let autofocused = |dom: &TuiDom| -> Vec<String> {
        fn walk(dom: &TuiDom, id: NodeId, out: &mut Vec<String>) {
            let n = dom.node(id);
            if n.has_attribute("autofocus") {
                out.push(format!(
                    "{}[name={}]",
                    n.tag_name().unwrap_or("?"),
                    n.get_attribute("name").unwrap_or("")
                ));
            }
            for c in n.child_nodes() {
                walk(dom, c.id(), out);
            }
        }
        let mut out = Vec::new();
        walk(dom, dom.root(), &mut out);
        out
    };
    let mut built: TuiDom = TuiDom::new();
    let root = built.root();
    let demo = tab_form::build(&mut built);
    built.append_child(root, demo).unwrap();
    let (parsed, _): (TuiDom, _) = rdom_parser::parse(tab_form::MARKUP).expect("MARKUP parses");
    let want = vec!["input[name=name]".to_string()];
    assert_eq!(autofocused(&built), want, "build()");
    assert_eq!(autofocused(&parsed), want, "MARKUP");
}

/// The user report behind `EDIT-CLICK-IN-CONTROL-1`: after switching to
/// Tab form, clicking into the empty Name input put the caret in the
/// "  Name: " label beside it, and the typed `Z` was lost.
#[test]
fn clicking_into_the_switched_in_name_input_types_into_it() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (mut app, input, _pane) = tab_form_app(123, 26);
    let r = app.dom().node(input).layout_rect().unwrap();
    let (x, y) = (r.x as u16 + 2, r.y as u16);
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
        app.draw_if_dirty().unwrap();
    }
    assert_eq!(app.dom().focused(), Some(input));
    let sel = *app.dom().selection().expect("a caret");
    assert_eq!(
        app.dom().node(sel.focus.node).parent_node().map(|p| p.id()),
        Some(input),
        "the caret is in the input's text, not the label's"
    );
    app.handle_event(key(KeyCode::Char('Z')));
    assert_eq!(app.dom().node(input).get_attribute("value"), Some("Z"));
}
