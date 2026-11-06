//! The showcase wired up: the shell, the mounted demo, the sidebar and
//! status-bar listeners — everything but the `App`. The binary
//! (`main.rs`) and the capture sessions ([`crate::capture`]) both start
//! here, so a capture shows exactly what the binary shows.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_tui::TuiDom;

use crate::{
    ShowcaseState, build_shell, mount_demo, seed_tree_cursor, wire_focus_hints,
    wire_mouse_position_indicator, wire_scroll_indicator, wire_sidebar_click,
};

/// Build the showcase into `dom` with demo `initial_idx` mounted. Give the
/// `App` the shell's base stylesheet, then `attach_sheet` the returned
/// state to it.
pub fn assemble(dom: &mut TuiDom, initial_idx: usize) -> Rc<RefCell<ShowcaseState>> {
    let handles = build_shell(dom);

    let state = Rc::new(RefCell::new(ShowcaseState::from_handles(&handles)));

    // Initial mount: deep-linked demo (default 0).
    mount_demo(&mut state.borrow_mut(), dom, initial_idx);
    // Seed the ARIA tree's active-descendant cursor onto the mounted
    // demo so the navigator boots already highlighting "where you
    // are" (and a CLI `--demo <slug>` deep-link expands + points at
    // its row). The tree built-in owns the cursor after the first
    // arrow / click.
    seed_tree_cursor(dom, handles.sidebar, initial_idx);

    // Sidebar click handler — walks up from the click target to
    // find the `<li>` with `data-demo-slug`, then swaps demos. The
    // ARIA tree built-in dispatches a bubbling `click` on the active
    // treeitem for BOTH pointer and keyboard (Enter / Space)
    // activation, so this single listener covers mouse and keyboard
    // — the arrows / Home / End / expand / collapse + the
    // active-descendant cursor all come from `runtime::builtins::tree`.
    wire_sidebar_click(dom, handles.sidebar, Rc::clone(&state));
    // Scroll listener — writes scroll info into the LEFT slot of
    // the status bar (the hints slot) whenever any scrollable
    // descendant of `<main>` fires a scroll event.
    wire_scroll_indicator(dom, handles.main, handles.status_bar_hints);
    // Live mouse-position gauge in the RIGHT slot of the status
    // bar — instantly tells the developer whether motion events
    // are flowing (numbers update) or stalled (numbers freeze).
    // Independent slot so it doesn't clobber the hints / scroll
    // content owned by other listeners.
    wire_mouse_position_indicator(dom, handles.status_bar_mouse_pos);
    // Focus listener — refreshes keyboard hints in the LEFT slot
    // whenever focus moves. The bar is pre-seeded in `build_shell`
    // with the global default; this listener handles changes.
    wire_focus_hints(dom, handles.status_bar_hints);
    state
}
