//! M7 D3 — scroll-position indicator follows scroll events.
//!
//! The showcase's scroll-indicator at the bottom of `<main>` is
//! empty when no scrollable element is in play. Once a descendant
//! fires a scroll event (programmatic, wheel, scrollbar drag —
//! all wired in M5 D5), the indicator's text updates to
//! "Row N/M — P%".

use rdom_showcase::{
    ShellHandles, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet,
    wire_scroll_indicator,
};
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::{App, NodeId, TuiAccessorsMut, TuiDom};

fn make_app() -> (App<TestBackend>, ShellHandles) {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0);
    wire_scroll_indicator(&mut dom, handles.main, handles.status_bar);

    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in rdom_showcase::DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    (app, handles)
}

/// Find the text content of the indicator — walks the full
/// descendant tree (Phase 1b nested the hints inside
/// `<span class="key">` / `<span class="label">` elements, so
/// the prior "direct children only" walk no longer suffices).
fn indicator_text(dom: &TuiDom, indicator: NodeId) -> String {
    let mut out = String::new();
    fn walk(dom: &TuiDom, id: NodeId, out: &mut String) {
        let n = dom.node(id);
        if let Some(t) = n.node_value() {
            out.push_str(t);
        }
        for c in n.child_nodes() {
            walk(dom, c.id(), out);
        }
    }
    walk(dom, indicator, &mut out);
    out
}

#[test]
fn status_bar_shows_default_hints_before_any_scroll() {
    // Phase 1b: the status bar is seeded with the default keyboard
    // hints at build time. Before any scroll event fires, the bar
    // shows hints — not the empty string it had pre-Phase-1b.
    let (app, handles) = make_app();
    let text = indicator_text(app.dom(), handles.status_bar);
    assert!(
        text.contains("navigate") && text.contains("select"),
        "status bar should show default hints before scroll; got {text:?}"
    );
    // And the scroll listener hasn't fired anything yet.
    assert!(
        !text.contains("cell ") && !text.contains('%'),
        "no scroll info should appear pre-scroll; got {text:?}"
    );
}

#[test]
fn scroll_event_populates_indicator_text() {
    let (mut app, handles) = make_app();

    // Programmatically scroll the view-content (a scrollable
    // div we set up below). Use `set_scroll_top` so the M5 D5
    // dispatch path fires `scroll`.
    let target = handles.main;

    // Make the view-content overflow so the scroll math has
    // meaningful values.
    if let Some(ext) = app.dom_mut().node_mut(target).ext_mut() {
        ext.scroll_content_height = 100;
        ext.content_layout = rdom_tui::layout::LayoutRect::new(0, 0, 50, 20);
    }

    app.dom_mut().node_mut(target).set_scroll_top(40).unwrap();

    let text = indicator_text(app.dom(), handles.status_bar);
    // Format: "P% — cell Y/H". Y is the raw scroll_y; H is
    // content_height. For Y=40, H=100, viewport=20 → max_scroll
    // = 80, percent = 40*100/80 = 50%.
    assert!(
        text.contains("cell 40/100") && text.contains("50%"),
        "indicator shows the current cell + percent (got {text:?})"
    );
}

#[test]
fn mount_demo_replaces_stale_scroll_info_with_default_hints() {
    // After scrolling demo A, switching to demo B should wipe the
    // scroll info (A's scrollable element is gone; stale text
    // would lie about B's state). Phase 1b: instead of leaving the
    // bar empty, the default hints come back.
    let (mut app, handles) = make_app();
    let target = handles.main;

    if let Some(ext) = app.dom_mut().node_mut(target).ext_mut() {
        ext.scroll_content_height = 100;
        ext.content_layout = rdom_tui::layout::LayoutRect::new(0, 0, 50, 20);
    }
    app.dom_mut().node_mut(target).set_scroll_top(40).unwrap();
    let after_scroll = indicator_text(app.dom(), handles.status_bar);
    assert!(
        after_scroll.contains("cell "),
        "scroll listener should have written scroll info; got {after_scroll:?}"
    );

    // Switch to a different demo. Scroll info clears; hints return.
    let mut state = ShowcaseState::from_handles(&handles);
    state.current_idx = 0; // setup() already mounted demo 0
    mount_demo(&mut state, app.dom_mut(), 1);

    let after_swap = indicator_text(app.dom(), handles.status_bar);
    assert!(
        !after_swap.contains("cell "),
        "demo swap must drop stale scroll info; got {after_swap:?}"
    );
    assert!(
        after_swap.contains("navigate"),
        "demo swap should restore default hints; got {after_swap:?}"
    );
}

#[test]
fn no_overflow_target_does_not_overwrite_hints() {
    let (mut app, handles) = make_app();
    let target = handles.main;

    // No content overflow — content_height ≤ viewport. Scroll
    // events on this element should NOT populate the indicator.
    if let Some(ext) = app.dom_mut().node_mut(target).ext_mut() {
        ext.scroll_content_height = 5;
        ext.content_layout = rdom_tui::layout::LayoutRect::new(0, 0, 50, 20);
    }

    // Fire a programmatic scroll. `set_scroll_top` clamps and only
    // dispatches when the offset actually changes; here scroll_y
    // stays 0 so no event fires — which is the right behavior.
    let _ = app.dom_mut().node_mut(target).set_scroll_top(40);

    // Phase 1b: with no scroll, the bar still shows the default
    // hints seeded by `build_shell`.
    let text = indicator_text(app.dom(), handles.status_bar);
    assert!(
        !text.contains("cell "),
        "no scroll → no scroll info in the bar; got {text:?}"
    );
    assert!(
        text.contains("navigate"),
        "no scroll → default hints still visible; got {text:?}"
    );
}
