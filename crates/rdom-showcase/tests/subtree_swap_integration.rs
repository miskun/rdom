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

use rdom_showcase::{ShowcaseState, build_shell, mount_demo};
use rdom_tui::{Position, Selection, TuiDom};

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
    use rdom_tui::App;
    use rdom_tui::render::{Terminal, TestBackend};

    let (mut dom, mut state) = setup();
    mount_demo(&mut state, &mut dom, 1);
    mount_demo(&mut state, &mut dom, 2);
    mount_demo(&mut state, &mut dom, 0);

    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    let mut app =
        App::with_backend(dom, rdom_showcase::shell::base_stylesheet(), terminal).unwrap();
    for demo in rdom_showcase::DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
}
