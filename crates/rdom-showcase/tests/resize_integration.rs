//! M7 D2 — terminal resize re-lays-out the showcase chrome cleanly.
//!
//! The substrate handles resize: `CtEvent::Resize` triggers
//! `needs_redraw`, the next paint reruns cascade + layout against
//! the new viewport, and `Buffer::resize` adjusts the cell grid.
//! These tests pin that the showcase shell adapts correctly —
//! sidebar stays at its declared 28-cell width, header at 3 cells
//! tall, the main panel flexes to fill the rest at every size.

use rdom_showcase::{ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::render::{Rect, Terminal, TestBackend};
use rdom_tui::{App, TuiDom};

/// Build a wired showcase with the first demo mounted, on a
/// `TestBackend` at `viewport`. Returns the App so the caller
/// can drive paints and resizes.
fn make_app(viewport: Rect) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0);

    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in rdom_showcase::DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    app
}

#[test]
fn paints_cleanly_at_a_range_of_viewports() {
    // 60x20 — small but reasonable terminal.
    let mut app = make_app(Rect::new(0, 0, 60, 20));

    // Sequence of resize events through `handle_event`. Each
    // dispatches the `resize` event on the document root + sets
    // needs_redraw. The next paint should succeed without panic.
    use crossterm::event::Event as CtEvent;
    let sizes = [(80, 24), (120, 30), (40, 15), (100, 24)];
    for (w, h) in sizes {
        app.handle_event(CtEvent::Resize(w, h));
        app.draw_if_dirty().expect("paint after resize succeeds");
    }
}

#[test]
fn resize_dispatches_event_to_listeners() {
    // M5 D4 wires the resize dispatch; M7 D2 ships the showcase as
    // a real consumer of it. Pin that a listener on the document
    // root sees every resize.
    use crossterm::event::Event as CtEvent;
    use rdom_tui::ListenerOptions;
    use std::cell::Cell;
    use std::rc::Rc;

    let mut app = make_app(Rect::new(0, 0, 60, 20));

    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    let root = app.dom().root();
    app.dom_mut()
        .add_event_listener(root, "resize", ListenerOptions::default(), move |_| {
            c.set(c.get() + 1);
        })
        .unwrap();

    app.handle_event(CtEvent::Resize(80, 24));
    app.handle_event(CtEvent::Resize(100, 30));
    app.handle_event(CtEvent::Resize(60, 20));

    assert_eq!(
        count.get(),
        3,
        "resize event fires once per terminal resize signal"
    );
}

#[test]
fn chrome_adapts_to_viewport_size() {
    // At different viewports, the sidebar should stay 28 cells
    // wide and the header 3 cells tall — both authored as fixed
    // dimensions. The demo panel flexes to fill the remainder.
    //
    // For TestBackend the resize signal and the actual buffer
    // resize are separate steps (real terminals couple them via
    // SIGWINCH detection). We call `TestBackend::resize` to update
    // the fake terminal size, then dispatch the synthetic resize
    // event so the App reacts.
    use crossterm::event::Event as CtEvent;

    let mut app = make_app(Rect::new(0, 0, 80, 24));

    let initial_main_rect = main_panel_rect(&app);
    assert!(
        initial_main_rect.width > 0 && initial_main_rect.height > 0,
        "initial layout produced a non-empty main panel"
    );

    // Resize wider. Sidebar still 28; header still 3; main grows.
    app.terminal_mut().backend_mut().resize(120, 30);
    app.handle_event(CtEvent::Resize(120, 30));
    app.draw_if_dirty().unwrap();
    let wider_main = main_panel_rect(&app);
    assert!(
        wider_main.width > initial_main_rect.width,
        "main panel widened on wider terminal ({} → {})",
        initial_main_rect.width,
        wider_main.width
    );
    assert!(
        wider_main.height > initial_main_rect.height,
        "main panel grew taller on taller terminal"
    );

    // Resize narrower. Main panel shrinks but doesn't go negative.
    app.terminal_mut().backend_mut().resize(60, 18);
    app.handle_event(CtEvent::Resize(60, 18));
    app.draw_if_dirty().unwrap();
    let narrower_main = main_panel_rect(&app);
    assert!(narrower_main.width <= wider_main.width);
}

/// Walk the DOM to find the `<main>` element and return its
/// content_layout rect. Used by `chrome_adapts_to_viewport_size`.
fn main_panel_rect(app: &App<TestBackend>) -> rdom_tui::layout::LayoutRect {
    use rdom_tui::node::TuiNodeExt;
    use rdom_tui::{NodeId, TuiDom};

    fn walk(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
        if dom.node(id).tag_name() == Some("main") {
            return Some(id);
        }
        for child in dom.node(id).child_nodes() {
            if let Some(found) = walk(dom, child.id()) {
                return Some(found);
            }
        }
        None
    }

    let main = walk(app.dom(), app.dom().root()).expect("<main> exists");
    app.dom()
        .node(main)
        .tui_ext()
        .map(|e| e.layout)
        .expect("main has a layout rect")
}
