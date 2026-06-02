//! Regression: the sticky-header demo must not show TWO nested scrollbars on
//! a short terminal. It fills the view pane (`height: 100%`) so it scrolls
//! itself and never overflows the pane — the pane stays un-scrolled.

use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, TuiDom};

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

/// `(content_height, viewport_height)` of a node's scroll metrics.
fn overflow(app: &App<TestBackend>, id: NodeId) -> (usize, usize) {
    let ext = app.dom().node(id).tui_ext().unwrap();
    (ext.scroll_content_height, ext.layout.height as usize)
}

#[test]
fn sticky_demo_shows_one_scrollbar_on_a_short_terminal() {
    let idx = DEMOS
        .iter()
        .position(|d| d.slug() == "positioning/sticky")
        .expect("sticky demo registered");
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, idx);

    // Short terminal: fewer rows than the demo's 21 items.
    let terminal = Terminal::new(TestBackend::new(70, 12)).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();

    let demo = find_by_class(app.dom(), app.dom().root(), "sticky-demo").expect(".sticky-demo");
    let pane = find_by_class(app.dom(), app.dom().root(), "view-content").expect(".view-content");

    // The demo itself scrolls (its 21 items overflow the pane-height box).
    let (demo_content, demo_view) = overflow(&app, demo);
    assert!(
        demo_content > demo_view,
        "the sticky demo should scroll itself (content={demo_content}, view={demo_view})"
    );

    // …but it fills the pane, so the PANE does not overflow → no 2nd scrollbar.
    let (pane_content, pane_view) = overflow(&app, pane);
    assert!(
        pane_content <= pane_view,
        "the view pane must not also scroll (content={pane_content}, view={pane_view}) — \
         that's the second scrollbar"
    );
}
