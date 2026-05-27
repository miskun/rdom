//! Regression: wheel-scrolling the `scrollable_list` demo's `.list`
//! container must advance its `scroll_y`. Reported as: wheel ticks
//! land on the demo, but nothing scrolls visually. Possible failure
//! modes this test pins:
//!
//! - `scroll_content_height` ≤ padding-box height → `max_y = 0` →
//!   `apply_scroll` saturates immediately (substrate overflow accounting).
//! - Wheel routing doesn't find `.list` as the nearest scrollable
//!   ancestor (hit-test or ancestor-walk regression).
//! - `Overflow::Auto` cascading wrong for the demo's `.list { overflow-y: auto }`.

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, TuiDom};

fn scrollable_list_demo_idx() -> usize {
    DEMOS
        .iter()
        .position(|d| d.slug() == "layout/scrollable-list")
        .expect("scrollable-list demo registered")
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

#[test]
fn wheel_inside_scrollable_list_advances_scroll_y() {
    // Build full showcase app the same way `main.rs` does it, mount
    // the scrollable_list demo, then paint once so layout populates
    // rects + scroll_content_height.
    let idx = scrollable_list_demo_idx();
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

    // Locate the demo's `.list` element — the `overflow-y: auto`
    // container with the 50 rows.
    let list = find_by_class(app.dom(), app.dom().root(), "list").expect("`.list` exists in DOM");
    let layout = app.dom().node(list).tui_ext().unwrap().layout;
    let scroll_content_height = app
        .dom()
        .node(list)
        .tui_ext()
        .unwrap()
        .scroll_content_height;

    eprintln!("DBG `.list` layout = {layout:?} scroll_content_height = {scroll_content_height}");

    // Sanity: 50 rows of height: 1 — content extent must be ≥ 50
    // regardless of the container's allocated height.
    assert!(
        scroll_content_height >= 50,
        "demo declares 50 rows of height: 1; scroll_content_height should reflect that. \
         Got {scroll_content_height} — likely cause: the flex layout shrank rows past their \
         declared height (M5-MIN-CONTENT-1) or `record_scroll_content_size` is dropping children."
    );

    // The list must be SHORTER than its content for wheel to do
    // anything. The viewport (123×26) should leave the list well
    // shorter than 50 rows.
    assert!(
        (layout.height as usize) < scroll_content_height,
        "list height ({}) >= content extent ({scroll_content_height}); test viewport too tall.",
        layout.height
    );

    // Route a wheel-down at the middle of `.list`'s on-screen rect.
    // Hit-test wants screen coordinates — the layout rect is in
    // those already.
    let click_x = (layout.x + (layout.width as i32) / 2) as u16;
    let click_y = (layout.y + (layout.height as i32) / 2) as u16;

    let scroll_y_before = app.dom().node(list).tui_ext().unwrap().scroll_y;
    assert_eq!(scroll_y_before, 0, "initial scroll_y must be 0");

    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: click_x,
        row: click_y,
        modifiers: KeyModifiers::empty(),
    }));
    app.draw_if_dirty().unwrap();

    let scroll_y_after = app.dom().node(list).tui_ext().unwrap().scroll_y;
    assert!(
        scroll_y_after > scroll_y_before,
        "wheel-down inside `.list` (at {click_x}, {click_y}) must advance scroll_y; \
         got before={scroll_y_before} after={scroll_y_after}. \
         layout={layout:?} content_h={scroll_content_height}"
    );
}
