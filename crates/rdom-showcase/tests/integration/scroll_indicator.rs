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
    wire_scroll_indicator(&mut dom, handles.main, handles.scroll_indicator);

    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in rdom_showcase::DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    (app, handles)
}

/// Find the text content of the indicator (concatenated text-node
/// children).
fn indicator_text(dom: &TuiDom, indicator: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(indicator).child_nodes() {
        if let Some(t) = child.node_value() {
            out.push_str(t);
        }
    }
    out
}

#[test]
fn empty_indicator_before_any_scroll() {
    let (app, handles) = make_app();
    assert_eq!(indicator_text(app.dom(), handles.scroll_indicator), "");
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

    let text = indicator_text(app.dom(), handles.scroll_indicator);
    // Format: "P% — cell Y/H". Y is the raw scroll_y; H is
    // content_height. For Y=40, H=100, viewport=20 → max_scroll
    // = 80, percent = 40*100/80 = 50%.
    assert!(
        text.contains("cell 40/100") && text.contains("50%"),
        "indicator shows the current cell + percent (got {text:?})"
    );
}

#[test]
fn mount_demo_clears_stale_scroll_indicator() {
    // After scrolling demo A, switching to demo B should clear
    // the indicator (A's scrollable element is gone; stale text
    // would lie about B's state).
    let (mut app, handles) = make_app();
    let target = handles.main;

    if let Some(ext) = app.dom_mut().node_mut(target).ext_mut() {
        ext.scroll_content_height = 100;
        ext.content_layout = rdom_tui::layout::LayoutRect::new(0, 0, 50, 20);
    }
    app.dom_mut().node_mut(target).set_scroll_top(40).unwrap();
    assert!(!indicator_text(app.dom(), handles.scroll_indicator).is_empty());

    // Switch to a different demo. Indicator must clear.
    let mut state = ShowcaseState::from_handles(&handles);
    state.current_idx = 0; // setup() already mounted demo 0
    mount_demo(&mut state, app.dom_mut(), 1);

    assert_eq!(
        indicator_text(app.dom(), handles.scroll_indicator),
        "",
        "indicator clears on demo switch — stale scroll info no longer applies"
    );
}

#[test]
fn indicator_stays_empty_when_target_has_no_overflow() {
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

    assert_eq!(indicator_text(app.dom(), handles.scroll_indicator), "");
}
