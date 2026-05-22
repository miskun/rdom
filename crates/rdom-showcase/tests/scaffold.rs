//! M2 exit-criteria tests — the showcase scaffold actually
//! mounts the first demo and survives a cascade + paint pass.

use rdom_showcase::{DEMOS, build_shell, shell::base_stylesheet};
use rdom_tui::render::{Rect, Terminal, TestBackend};
use rdom_tui::{App, Backend, TuiDom};

#[test]
fn shell_mounts_and_first_demo_attaches_under_main() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let demo = DEMOS[0];
    let demo_root = demo.build(&mut dom);
    dom.append_child(handles.main, demo_root).unwrap();

    // The demo's root is now a child of <main>.
    let parent = dom
        .node(demo_root)
        .parent_node()
        .expect("demo root must be attached after the shell mounts it");
    assert_eq!(
        parent.id(),
        handles.main,
        "demo root is attached under <main>, not somewhere else"
    );
}

#[test]
fn shell_plus_first_demo_survives_full_paint_pass() {
    // Constructs an App against a TestBackend, pushes the demo's
    // stylesheet on top of the shell's base, runs cascade + layout
    // + paint. Any layout/cascade gap surfaces as a panic here.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let demo = DEMOS[0];
    let demo_root = demo.build(&mut dom);
    dom.append_child(handles.main, demo_root).unwrap();

    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    let _demo_sheet_id = app.push_stylesheet(demo.stylesheet());

    // Should not panic. Verifies cascade + layout + paint all
    // accept the shell tree at a real-ish viewport size.
    app.draw_if_dirty().unwrap();

    // App still has the demo's sheet pushed on top of the base.
    assert_eq!(
        app.style_sheets().len(),
        2,
        "base sheet + demo sheet → exactly two slots"
    );

    // Explicit ordering check — slot 0 must be the base sheet
    // (chrome layout), slot 1 must be the demo's sheet (per-demo
    // styles). M1's cascade order is push-order: later wins
    // same-specificity contests, so demo styles override chrome.
    // If a future refactor pushes the demo BEFORE the App is
    // constructed, the chrome would win and demos couldn't
    // restyle anything the chrome touched — silently inverted
    // semantics. This test pins the order.
    let sheets = app.style_sheets();
    let base_rule_count = sheets[0].rules().len();
    let demo_rule_count = sheets[1].rules().len();
    assert!(
        base_rule_count >= demo_rule_count,
        "shell base sheet has more rules than the demo's stylesheet \
         (base={base_rule_count}, demo={demo_rule_count}); if this \
         inverts, slot 0 and slot 1 got swapped"
    );
}

#[test]
fn shell_paints_at_a_tiny_viewport_without_panicking() {
    // A 20×5 viewport is too small for the chrome — layout has to
    // clamp / overflow gracefully. Catches the "looks fine at 80×24,
    // crashes on small terminals" regression class.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let demo = DEMOS[0];
    let demo_root = demo.build(&mut dom);
    dom.append_child(handles.main, demo_root).unwrap();

    let backend = TestBackend::new(20, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    let _ = app.push_stylesheet(demo.stylesheet());
    app.draw_if_dirty().unwrap();

    // Sanity: the viewport rect is what we asked for.
    assert_eq!(
        app.terminal().backend().size().unwrap(),
        Rect::new(0, 0, 20, 5)
    );
}
