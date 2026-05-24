//! `rdom-showcase` binary entry point.
//!
//! Builds the shell, mounts the demo at `DEMOS[0]` into the main
//! view, pushes the demo's stylesheet onto the App's sheet stack,
//! runs the event loop. M2 ships this static layout; M3 makes the
//! sidebar interactive.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_showcase::{
    DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet, wire_scroll_indicator,
    wire_sidebar_click, wire_sidebar_keys, wire_view_tab_click,
};
use rdom_tui::{App, TuiDom};

fn main() -> std::io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let state = Rc::new(RefCell::new(ShowcaseState::from_handles(&handles)));

    // Initial mount: demo 0 in Demo view.
    mount_demo(&mut state.borrow_mut(), &mut dom, 0);

    // Sidebar click handler — walks up from the click target to
    // find the `<li>` with `data-demo-slug`, then swaps demos.
    wire_sidebar_click(&mut dom, handles.sidebar, Rc::clone(&state));
    // Sidebar keyboard handler — Arrow keys to navigate between
    // demo `<li>`s, Enter / Space to activate the focused one.
    // Tab / Shift+Tab is handled by the runtime's built-in
    // focus traversal (the `<li>`s carry `tabindex="0"`).
    wire_sidebar_keys(&mut dom, handles.sidebar, Rc::clone(&state));
    // View-tabs click handler — Demo / Source toggle in the main
    // view header.
    wire_view_tab_click(&mut dom, handles.view_tabs, Rc::clone(&state));
    // Scroll listener — populates the indicator at the bottom of
    // `<main>` with "Row N/M — P%" text whenever any scrollable
    // descendant fires a scroll event.
    wire_scroll_indicator(&mut dom, handles.scroll_indicator);

    // Construct the App with the shell's base stylesheet.
    let mut app = App::new(dom, base_stylesheet())?;

    // Pre-push every demo's stylesheet onto the App's sheet stack.
    // Each demo's CSS uses unique class-scoped selectors (e.g.
    // `.hello`, `.flex-row-demo`, `.hover-demo`), so the cascade
    // naturally applies only the mounted demo's rules — switching
    // demos is just a subtree swap, no per-demo sheet push/remove
    // required.
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }

    app.run()
}
