//! Regression (`SCROLL-OVERFLOW-NESTED-ANON-1`): the Smooth scroll demo
//! fits the showcase's view pane, so the pane must not scroll.
//!
//! The demo's `.log` is a scroll container whose entries are separated
//! by whitespace. The anonymous boxes of those runs sat at the entries'
//! positions in the log's hidden content and leaked into the pane's
//! scroll size (25 rows for a 15-row demo), so the pane showed a
//! scrollbar and Top / Bottom — whose `scrollIntoView()` scrolls every
//! scroll container on the ancestor chain — moved the pane as well.

use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, TuiAccessors, TuiAccessorsMut, TuiDom};

fn by_class(dom: &TuiDom, class: &str) -> NodeId {
    dom.node(dom.root())
        .query_selector(&format!(".{class}"))
        .unwrap_or_else(|| panic!(".{class} is mounted"))
        .id()
}

/// The full showcase shell, switched to the Smooth scroll demo.
fn shell_on_scroll_demo() -> App<TestBackend> {
    let idx = DEMOS
        .iter()
        .position(|d| d.slug() == "animations/scroll-live-style")
        .expect("scroll-live-style demo registered");
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0);
    let terminal = Terminal::new(TestBackend::new(123, 26)).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    mount_demo(&mut state, app.dom_mut(), idx);
    app.advance(0).unwrap();
    app
}

#[test]
fn the_smooth_scroll_demo_fits_its_pane() {
    let app = shell_on_scroll_demo();
    let pane = by_class(app.dom(), "view-content");
    let ext = app.dom().node(pane).ext().unwrap();
    assert!(
        ext.scroll_content_height <= ext.content_layout.height as usize,
        "pane content {} rows in a {}-row pane",
        ext.scroll_content_height,
        ext.content_layout.height
    );
}

#[test]
fn top_and_bottom_scroll_the_log_and_leave_the_pane_still() {
    let mut app = shell_on_scroll_demo();
    let pane = by_class(app.dom(), "view-content");
    let log = by_class(app.dom(), "log");
    let bottom = by_class(app.dom(), "bottom");
    let top = by_class(app.dom(), "top");

    app.dom_mut().node_mut(bottom).click();
    app.advance(0).unwrap();
    app.advance(400).unwrap();
    assert!(
        app.dom().node(log).scroll_top().unwrap() > 0,
        "Bottom scrolls the log"
    );
    assert_eq!(app.dom().node(pane).scroll_top(), Some(0), "after Bottom");

    app.dom_mut().node_mut(top).click();
    app.advance(0).unwrap();
    app.advance(400).unwrap();
    assert_eq!(
        app.dom().node(log).scroll_top(),
        Some(0),
        "Top scrolls the log back"
    );
    assert_eq!(app.dom().node(pane).scroll_top(), Some(0), "after Top");
}
