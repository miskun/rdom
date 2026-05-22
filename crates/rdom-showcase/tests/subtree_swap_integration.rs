//! M3 D6 — end-to-end subtree-swap correctness, exercising the
//! showcase's real `mount_demo` against the M1 D2 substrate
//! contract.
//!
//! The substrate-level tests in
//! `crates/rdom-tui/tests/subtree_replacement_contract.rs`
//! already cover each individual purge case (focus / hover /
//! pointer-capture / selection) when `detach_from_parent`
//! removes a node. These tests assert the same guarantees hold
//! when the showcase's `mount_demo` runs — i.e., that swapping
//! demos through the showcase's actual entry point does not
//! leak interaction state pointing into the old subtree.
//!
//! Each test follows the same shape:
//!   1. Build the shell + mount demo A.
//!   2. Manually set focus / hover / pointer-capture / selection
//!      to a node inside demo A's subtree.
//!   3. Mount demo B.
//!   4. Assert the interaction state no longer references demo A.
//!
//! If any of these fail, the showcase's mount/swap path has
//! drifted away from the substrate contract — a substrate fix,
//! not a showcase fix, is the answer.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_showcase::{
    DEMOS, ShowcaseState, build_shell, mount_demo, wire_sidebar_click, wire_sidebar_keys,
};
use rdom_tui::{
    Event, EventDetail, KeyboardDetail, KeyboardModifiers, NodeId, Position, Selection, TuiDom,
};

/// Build shell + mount demo 0, returning the populated state +
/// `<main>` handle. Common setup across the tests below.
fn setup() -> (TuiDom, ShowcaseState) {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState {
        current_idx: usize::MAX,
        main_id: handles.main,
    };
    mount_demo(&mut state, &mut dom, 0);
    (dom, state)
}

#[test]
fn focus_on_old_demo_clears_when_swapping_to_new_demo() {
    let (mut dom, mut state) = setup();
    let main_id = state.main_id;

    // Grab a node from inside demo 0's mounted subtree.
    let demo_root = dom
        .node(main_id)
        .child_nodes()
        .next()
        .expect("demo 0 is mounted")
        .id();
    let target = demo_root; // the demo's own root counts as "inside"
    dom.set_focused(Some(target));
    assert_eq!(dom.focused(), Some(target));

    // Swap to demo 1. The substrate's purge step in
    // `detach_from_parent` should drop the focused id since it
    // now points into a detached subtree.
    mount_demo(&mut state, &mut dom, 1);

    assert_eq!(
        dom.focused(),
        None,
        "focus on a node in the old subtree must clear on demo swap (substrate contract)"
    );
}

#[test]
fn hover_on_old_demo_clears_when_swapping_to_new_demo() {
    let (mut dom, mut state) = setup();
    let main_id = state.main_id;

    let demo_root = dom
        .node(main_id)
        .child_nodes()
        .next()
        .expect("demo 0 is mounted")
        .id();
    dom.set_hovered(Some(demo_root));
    assert_eq!(dom.hovered(), Some(demo_root));

    mount_demo(&mut state, &mut dom, 1);

    assert_eq!(
        dom.hovered(),
        None,
        "hover on a node in the old subtree must clear on demo swap"
    );
}

#[test]
fn pointer_capture_on_old_demo_clears_when_swapping_to_new_demo() {
    let (mut dom, mut state) = setup();
    let main_id = state.main_id;

    let demo_root = dom
        .node(main_id)
        .child_nodes()
        .next()
        .expect("demo 0 is mounted")
        .id();
    dom.set_pointer_capture(demo_root).unwrap();
    assert_eq!(dom.pointer_capture(), Some(demo_root));

    mount_demo(&mut state, &mut dom, 1);

    assert_eq!(
        dom.pointer_capture(),
        None,
        "pointer capture pointing into the old subtree must release on demo swap"
    );
}

#[test]
fn selection_inside_old_demo_clears_when_swapping_to_new_demo() {
    let (mut dom, mut state) = setup();
    let main_id = state.main_id;

    // Anchor a selection inside the mounted demo's subtree. We use
    // the demo's root as the position node — selection model
    // accepts any NodeId.
    let demo_root = dom
        .node(main_id)
        .child_nodes()
        .next()
        .expect("demo 0 is mounted")
        .id();
    let pos = Position::new(demo_root, 0);
    dom.set_selection(Some(Selection::caret(pos)));
    assert!(dom.selection().is_some());

    mount_demo(&mut state, &mut dom, 1);

    assert_eq!(
        dom.selection(),
        None,
        "selection with both endpoints in the old subtree must clear on demo swap"
    );
}

#[test]
fn swap_leaves_main_with_exactly_one_demo_subtree() {
    // Sanity guard: after N swaps, `<main>` still has exactly one
    // child — the currently-mounted demo's root. Catches an
    // append-without-clear regression.
    let (mut dom, mut state) = setup();
    let main_id = state.main_id;

    for idx in [1usize, 2, 0, 2, 1] {
        mount_demo(&mut state, &mut dom, idx);
        let children: Vec<_> = dom.node(main_id).child_nodes().collect();
        assert_eq!(
            children.len(),
            1,
            "after mounting demo {idx}, <main> has exactly one child"
        );
    }
}

#[test]
fn swap_to_same_demo_does_not_disturb_focus() {
    // No-op swap (same idx) must not run the purge step — there's
    // no detachment happening. Confirms the early-return in
    // `mount_demo` is load-bearing.
    let (mut dom, mut state) = setup();
    let main_id = state.main_id;

    let demo_root = dom
        .node(main_id)
        .child_nodes()
        .next()
        .expect("demo 0 is mounted")
        .id();
    dom.set_focused(Some(demo_root));

    mount_demo(&mut state, &mut dom, 0); // same idx — no-op

    assert_eq!(
        dom.focused(),
        Some(demo_root),
        "re-mounting the same demo doesn't rebuild the subtree or disturb focus"
    );
}

#[test]
fn swap_renders_clean_at_full_viewport() {
    // After a swap, the renderer must successfully paint the new
    // subtree under the full chrome — no panics from stale
    // cascade / dirty-tracker state pointing at detached nodes.
    // Paints between mounts to exercise dirty-tracker through
    // interleaved swap-then-paint, not just one terminal paint
    // after multiple swaps.
    use rdom_tui::App;
    use rdom_tui::render::{Terminal, TestBackend};

    let (dom, initial_state) = setup();
    let main_id = initial_state.main_id;

    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    let mut app =
        App::with_backend(dom, rdom_showcase::shell::base_stylesheet(), terminal).unwrap();
    for demo in rdom_showcase::DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }

    let mut state = ShowcaseState {
        current_idx: initial_state.current_idx,
        main_id,
    };
    for idx in [1usize, 2, 0, 2, 1] {
        mount_demo(&mut state, app.dom_mut(), idx);
        app.draw_if_dirty().unwrap();
    }
}

// ─── End-to-end listener tests ──────────────────────────────────────
//
// These tests fire synthetic `click` / `keydown` events through
// `Dom::dispatch_event` against a fully-wired showcase (build_shell
// + wire_sidebar_click + wire_sidebar_keys) and assert that the
// listener wiring actually swaps the mounted demo. Without these,
// `mount_demo` and `next_demo_li` are unit-tested but the listeners
// themselves — half of M3's deliverable — could be swapped /
// reversed / disconnected without anything failing.

/// Find the first `<li data-demo-slug="…">` under `sidebar` whose
/// slug matches the demo at `demo_idx`. Used to target synthetic
/// events at a specific demo's row.
fn find_li_for_demo(dom: &TuiDom, sidebar: NodeId, demo_idx: usize) -> NodeId {
    let target_slug = DEMOS[demo_idx].slug();
    let mut stack = vec![sidebar];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        if node.tag_name() == Some("li")
            && node.get_attribute("data-demo-slug") == Some(target_slug)
        {
            return id;
        }
        for child in node.child_nodes() {
            stack.push(child.id());
        }
    }
    panic!("no <li> for demo idx {demo_idx} ({target_slug}) in sidebar");
}

/// Build a fully wired showcase: shell + listeners + initial mount
/// of demo 0. Returns (dom, state-handle, sidebar-id).
fn wired_setup() -> (TuiDom, Rc<RefCell<ShowcaseState>>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let state = Rc::new(RefCell::new(ShowcaseState {
        current_idx: usize::MAX,
        main_id: handles.main,
    }));
    mount_demo(&mut state.borrow_mut(), &mut dom, 0);
    wire_sidebar_click(&mut dom, handles.sidebar, Rc::clone(&state));
    wire_sidebar_keys(&mut dom, handles.sidebar, Rc::clone(&state));
    (dom, state, handles.sidebar)
}

#[test]
fn click_on_li_mounts_that_demo() {
    let (mut dom, state, sidebar) = wired_setup();
    assert_eq!(state.borrow().current_idx, 0);

    let target_li = find_li_for_demo(&dom, sidebar, 2);
    let mut click = Event::new("click");
    dom.dispatch_event(target_li, &mut click).unwrap();

    assert_eq!(
        state.borrow().current_idx,
        2,
        "clicking demo 2's <li> mounts demo 2"
    );
}

#[test]
fn click_on_text_inside_li_bubbles_up_and_mounts() {
    // The click target is usually the text node inside the <li>,
    // not the <li> itself — the ancestor walk has to find the
    // <li> with data-demo-slug.
    let (mut dom, state, sidebar) = wired_setup();
    let target_li = find_li_for_demo(&dom, sidebar, 1);
    let text_node = dom
        .node(target_li)
        .child_nodes()
        .next()
        .expect("<li> has a text child")
        .id();

    let mut click = Event::new("click");
    dom.dispatch_event(text_node, &mut click).unwrap();

    assert_eq!(state.borrow().current_idx, 1);
}

#[test]
fn click_on_summary_does_not_mount_anything() {
    // Clicking a category <summary> toggles the <details> open
    // state; it must NOT trigger a demo swap. Pins Finding 7:
    // only <li> elements with data-demo-slug fire mount_demo.
    let (mut dom, state, sidebar) = wired_setup();
    let initial = state.borrow().current_idx;

    // Find the first <summary> under the sidebar.
    let mut stack = vec![sidebar];
    let summary = loop {
        let id = stack.pop().expect("sidebar has a summary somewhere");
        if dom.node(id).tag_name() == Some("summary") {
            break id;
        }
        for child in dom.node(id).child_nodes() {
            stack.push(child.id());
        }
    };

    let mut click = Event::new("click");
    dom.dispatch_event(summary, &mut click).unwrap();

    assert_eq!(
        state.borrow().current_idx,
        initial,
        "clicking <summary> must not change the mounted demo"
    );
}

/// Build a `keydown` event with `key` as the only meaningful
/// payload — modifiers default, repeat=false.
fn keydown(key: &str) -> Event {
    let mut e = Event::new("keydown");
    e.detail = EventDetail::Keyboard(Box::new(KeyboardDetail {
        key: key.to_string(),
        modifiers: KeyboardModifiers::default(),
        repeat: false,
    }));
    e
}

#[test]
fn arrow_down_moves_focus_to_next_demo_li() {
    let (mut dom, _state, sidebar) = wired_setup();
    let first_li = find_li_for_demo(&dom, sidebar, 0);
    let expected_next = find_li_for_demo(&dom, sidebar, 1);
    dom.set_focused(Some(first_li));

    let mut e = keydown("ArrowDown");
    dom.dispatch_event(first_li, &mut e).unwrap();

    assert_eq!(
        dom.focused(),
        Some(expected_next),
        "ArrowDown moves focus to demo 1's <li>"
    );
}

#[test]
fn arrow_up_from_first_li_wraps_to_last() {
    let (mut dom, _state, sidebar) = wired_setup();
    let first_li = find_li_for_demo(&dom, sidebar, 0);
    // The sidebar groups by category, so document order != registry
    // order. ArrowUp wraps to the LAST <li> in document order, not
    // the last entry in `DEMOS`.
    let last_li = find_last_demo_li(&dom, sidebar);
    dom.set_focused(Some(first_li));

    let mut e = keydown("ArrowUp");
    dom.dispatch_event(first_li, &mut e).unwrap();

    assert_eq!(dom.focused(), Some(last_li), "ArrowUp on first <li> wraps");
}

/// Walk the sidebar in document order, return the last
/// `<li data-demo-slug>`. Used by the wrap-to-last test, which
/// can't assume registry order matches document order (demos are
/// grouped by category in the sidebar).
fn find_last_demo_li(dom: &TuiDom, sidebar: NodeId) -> NodeId {
    let mut last = None;
    walk_lis(dom, sidebar, &mut |id| last = Some(id));
    last.expect("sidebar contains at least one demo <li>")
}

fn walk_lis(dom: &TuiDom, id: NodeId, visit: &mut impl FnMut(NodeId)) {
    let node = dom.node(id);
    if node.tag_name() == Some("li") && node.get_attribute("data-demo-slug").is_some() {
        visit(id);
    }
    for child in node.child_nodes() {
        walk_lis(dom, child.id(), visit);
    }
}

#[test]
fn enter_on_focused_li_mounts_that_demo() {
    let (mut dom, state, sidebar) = wired_setup();
    let target_li = find_li_for_demo(&dom, sidebar, 2);
    dom.set_focused(Some(target_li));

    let mut e = keydown("Enter");
    dom.dispatch_event(target_li, &mut e).unwrap();

    assert_eq!(
        state.borrow().current_idx,
        2,
        "Enter on demo 2's <li> mounts demo 2"
    );
}

#[test]
fn space_on_focused_li_mounts_that_demo() {
    // Space activates focused elements just like Enter, per ARIA
    // / standard form control conventions.
    let (mut dom, state, sidebar) = wired_setup();
    let target_li = find_li_for_demo(&dom, sidebar, 1);
    dom.set_focused(Some(target_li));

    let mut e = keydown(" ");
    dom.dispatch_event(target_li, &mut e).unwrap();

    assert_eq!(state.borrow().current_idx, 1);
}

#[test]
fn arrow_keys_without_focus_inside_sidebar_are_noop() {
    let (mut dom, state, sidebar) = wired_setup();
    let initial = state.borrow().current_idx;
    // Focus is None — keydown listener should early-return.
    assert!(dom.focused().is_none());

    let mut e = keydown("ArrowDown");
    dom.dispatch_event(sidebar, &mut e).unwrap();

    assert_eq!(state.borrow().current_idx, initial);
    assert!(dom.focused().is_none(), "no focus to move");
}
