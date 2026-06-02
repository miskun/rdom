//! `FOCUS-VOCAB-1`: a focused scroll container scrolls with the keyboard.
//!
//! The `scrollable_list` demo's `.list` (overflow-y:auto, 50 plain-div rows,
//! no focusable children) is implicitly keyboard-focusable. Once focused,
//! ArrowDown / PageDown / End must scroll it — focusability without keyboard
//! scrolling would be pointless. Companion to the wheel + focus-thumb tests.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::runtime::focus::tabindex::is_tab_focusable;
use rdom_tui::{NodeId, TuiDom};

fn key_press(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

fn find_by_class(dom: &TuiDom, id: NodeId, class: &str) -> Option<NodeId> {
    let n = dom.node(id);
    if n.get_attribute("class")
        .map(|s| s.split_whitespace().any(|c| c == class))
        .unwrap_or(false)
    {
        return Some(id);
    }
    for c in n.child_nodes() {
        if let Some(f) = find_by_class(dom, c.id(), class) {
            return Some(f);
        }
    }
    None
}

fn scroll_y(app: &App<TestBackend>, list: NodeId) -> usize {
    app.dom().node(list).tui_ext().unwrap().scroll_y
}

#[test]
fn focused_scrollable_list_scrolls_with_arrow_keys() {
    let idx = DEMOS
        .iter()
        .position(|d| d.slug() == "layout/scrollable-list")
        .expect("scrollable-list demo registered");
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, idx);

    let backend = TestBackend::new(123, 26);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();

    let list = find_by_class(app.dom(), app.dom().root(), "list").expect("`.list` exists");

    // It's reachable by Tab (scrollable + no focusable descendant).
    assert!(
        is_tab_focusable(app.dom(), list),
        "the scrollable `.list` must be Tab-focusable"
    );

    // Focus it, then ArrowDown must advance scroll_y.
    app.dom_mut().set_focused(Some(list));
    assert_eq!(scroll_y(&app, list), 0, "starts unscrolled");

    app.handle_event(key_press(KeyCode::Down));
    app.draw_if_dirty().unwrap();
    let after_down = scroll_y(&app, list);
    assert!(
        after_down > 0,
        "ArrowDown on the focused scroll container must scroll it; got {after_down}"
    );

    // End jumps to the bottom (clamped to max scroll).
    app.handle_event(key_press(KeyCode::End));
    app.draw_if_dirty().unwrap();
    assert!(
        scroll_y(&app, list) > after_down,
        "End must scroll further toward the bottom"
    );

    // ArrowUp scrolls back up.
    let at_bottom = scroll_y(&app, list);
    app.handle_event(key_press(KeyCode::Up));
    app.draw_if_dirty().unwrap();
    assert!(
        scroll_y(&app, list) < at_bottom,
        "ArrowUp must scroll back up"
    );
}
