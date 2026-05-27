//! Regression: hover on scrollable_list demo rows doesn't fire until
//! the user clicks an item — reported as "after the terminal regains
//! focus, hover stops working until I click."
//!
//! The user clicks a sidebar `<li>` to mount the demo; somewhere
//! along the way `router.pointer_capture` (or another sticky state)
//! gets engaged but never released, so `handle_move` falls into the
//! captured branch (mouse/mod.rs:251) which short-circuits hover
//! updates.
//!
//! Each test pins one invariant of the click→hover flow so we can
//! see which step actually goes wrong.

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_showcase::{
    DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet, wire_focus_hints,
    wire_scroll_indicator, wire_sidebar_click, wire_sidebar_keys,
};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, TuiDom};
use std::cell::RefCell;
use std::rc::Rc;

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

fn mouse_event(kind: MouseEventKind, x: u16, y: u16) -> CtEvent {
    CtEvent::Mouse(CtMouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}

fn build_app() -> (App<TestBackend>, rdom_showcase::ShellHandles) {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let state = Rc::new(RefCell::new(ShowcaseState::from_handles(&handles)));
    mount_demo(&mut state.borrow_mut(), &mut dom, 0); // HelloWorld initially

    wire_sidebar_click(&mut dom, handles.sidebar, Rc::clone(&state));
    wire_sidebar_keys(&mut dom, handles.sidebar, Rc::clone(&state));
    wire_scroll_indicator(&mut dom, handles.main, handles.status_bar);
    wire_focus_hints(&mut dom, handles.status_bar);

    let backend = TestBackend::new(123, 26);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    (app, handles)
}

fn find_sidebar_li_for(app: &App<TestBackend>, slug: &str) -> NodeId {
    fn walk(dom: &TuiDom, id: NodeId, slug: &str) -> Option<NodeId> {
        let n = dom.node(id);
        if n.get_attribute("data-demo-slug") == Some(slug) {
            return Some(id);
        }
        for c in n.child_nodes() {
            if let Some(f) = walk(dom, c.id(), slug) {
                return Some(f);
            }
        }
        None
    }
    walk(app.dom(), app.dom().root(), slug).expect("sidebar li for slug")
}

#[test]
fn hover_works_after_sidebar_click_mounts_demo() {
    let (mut app, _h) = build_app();

    // Click the "Scrollable list" sidebar item to mount the demo
    // exactly like a user would.
    let li = find_sidebar_li_for(&app, "layout/scrollable-list");
    let li_rect = app.dom().node(li).tui_ext().unwrap().layout;
    let lx = (li_rect.x + (li_rect.width as i32) / 2) as u16;
    let ly = (li_rect.y + (li_rect.height as i32) / 2) as u16;
    app.handle_event(mouse_event(MouseEventKind::Down(MouseButton::Left), lx, ly));
    app.handle_event(mouse_event(MouseEventKind::Up(MouseButton::Left), lx, ly));
    app.draw_if_dirty().unwrap();

    // After the click, pointer_capture MUST be released so subsequent
    // motion can update hover. The router's auto-release on mouseup
    // is browser-faithful (mouse/mod.rs:222-224); this asserts the
    // contract.
    assert_eq!(
        app.dom().pointer_capture(),
        None,
        "mouseup must auto-release pointer_capture; got {:?}",
        app.dom().pointer_capture()
    );

    // Now find a `.row` inside the just-mounted scrollable_list demo
    // and send a Moved event over it. Hover MUST update.
    let row = find_by_class(app.dom(), app.dom().root(), "row").expect("`.row` exists");
    let row_rect = app.dom().node(row).tui_ext().unwrap().layout;
    let rx = (row_rect.x + (row_rect.width as i32) / 2) as u16;
    let ry = (row_rect.y + (row_rect.height as i32) / 2) as u16;

    app.handle_event(mouse_event(MouseEventKind::Moved, rx, ry));
    app.draw_if_dirty().unwrap();

    let hovered = app.dom().hovered();
    eprintln!(
        "DBG row={row:?} row_rect={row_rect:?} cursor=({rx},{ry}) hovered={hovered:?} \
         capture={:?}",
        app.dom().pointer_capture()
    );
    assert_eq!(
        hovered,
        Some(row),
        "moving the mouse over `.row` after a sidebar click must set hovered to that row"
    );
}

#[test]
fn drag_off_screen_then_back_in_does_not_strand_pointer_capture() {
    // Repro the user's described focus-loss scenario: user clicks the
    // sidebar item, then drags off the terminal (mousedown +
    // off-screen motion + no mouseup, simulating focus loss / mouseup
    // outside terminal). The next hover must still work.
    //
    // If pointer_capture stays engaged from a drag that never got its
    // mouseup, `handle_move` (mouse/mod.rs:251) routes follow-ups to
    // the captured holder and skips hover updates entirely — exactly
    // the user's symptom (hover dead until next click). This test
    // pins the failure mode so the fix has something to prove.
    let (mut app, _h) = build_app();

    // Click sidebar to mount the demo.
    let li = find_sidebar_li_for(&app, "layout/scrollable-list");
    let li_rect = app.dom().node(li).tui_ext().unwrap().layout;
    let lx = (li_rect.x + (li_rect.width as i32) / 2) as u16;
    let ly = (li_rect.y + (li_rect.height as i32) / 2) as u16;
    app.handle_event(mouse_event(MouseEventKind::Down(MouseButton::Left), lx, ly));
    app.handle_event(mouse_event(MouseEventKind::Up(MouseButton::Left), lx, ly));
    app.draw_if_dirty().unwrap();

    // Now simulate "user starts a drag, drags off-window, no mouseup
    // is delivered." Mousedown on a row engages selection drag-
    // capture (selection/drag.rs:93). We DON'T send the mouseup.
    let row = find_by_class(app.dom(), app.dom().root(), "row").expect("`.row` exists");
    let row_rect = app.dom().node(row).tui_ext().unwrap().layout;
    let rx = (row_rect.x + (row_rect.width as i32) / 2) as u16;
    let ry = (row_rect.y + (row_rect.height as i32) / 2) as u16;
    app.handle_event(mouse_event(MouseEventKind::Down(MouseButton::Left), rx, ry));

    eprintln!(
        "DBG after-down: capture={:?} hovered={:?}",
        app.dom().pointer_capture(),
        app.dom().hovered()
    );

    // Simulate the user's "unfocus / refocus" cycle. From rdom's POV,
    // events simply stop arriving for a while, then resume. The
    // mouseup that would normally release capture is lost.

    // First motion event after refocus, over a DIFFERENT row. The
    // user EXPECTS hover to update.
    let row_target = {
        // Find a row a few entries down so it has a distinct rect.
        let list = find_by_class(app.dom(), app.dom().root(), "list").unwrap();
        let mut rows: Vec<NodeId> = Vec::new();
        for c in app.dom().node(list).child_nodes() {
            if c.get_attribute("class") == Some("row") {
                rows.push(c.id());
            }
        }
        rows[5] // pick row 6
    };
    let target_rect = app.dom().node(row_target).tui_ext().unwrap().layout;
    let tx = (target_rect.x + (target_rect.width as i32) / 2) as u16;
    let ty = (target_rect.y + (target_rect.height as i32) / 2) as u16;
    app.handle_event(mouse_event(MouseEventKind::Moved, tx, ty));
    app.draw_if_dirty().unwrap();

    eprintln!(
        "DBG after-move: capture={:?} hovered={:?} target_row={:?}",
        app.dom().pointer_capture(),
        app.dom().hovered(),
        row_target
    );
    assert_eq!(
        app.dom().hovered(),
        Some(row_target),
        "after a drag that never got its mouseup (e.g., button released outside the terminal \
         or terminal lost focus mid-drag), subsequent hover must STILL work. If the router \
         leaves pointer_capture set, all motion routes to the captured holder and bypasses \
         the hover state machine — the user has to click an item to force the mouseup auto-\
         release. Fix shape: clear sticky router state when focus changes (or detect stale \
         mousedown-without-mouseup another way)."
    );
}
