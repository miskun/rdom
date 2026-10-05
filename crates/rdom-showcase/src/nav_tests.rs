//! `nav` tests: mounting, swapping and the sidebar wiring.

use super::*;
use crate::build_shell;

#[test]
fn mount_demo_initial_attaches_first_demo() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);

    mount_demo(&mut state, &mut dom, 0);

    assert_eq!(state.current_idx, 0);
    let main_children: Vec<_> = dom.node(handles.main).child_nodes().collect();
    assert_eq!(
        main_children.len(),
        1,
        "main has exactly one child (the mounted demo's root)"
    );
}

#[test]
fn mount_demo_runs_autofocus_on_demo_subtree() {
    // The showcase glues the substrate's autofocus primitive
    // to its mount lifecycle: when a demo carrying an
    // `[autofocus]` element is mounted (or swapped in), focus
    // should land on that element. The permission_dialog demo
    // marks its first radio with `autofocus`; verify by
    // mounting it and checking the focused node.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);

    let permission_idx = crate::DEMOS
        .iter()
        .position(|d| d.slug() == "forms/permission-dialog")
        .expect("permission-dialog demo registered");
    mount_demo(&mut state, &mut dom, permission_idx);

    let focused = dom.focused().expect("autofocus moved focus into demo");
    let node = dom.node(focused);
    assert_eq!(node.tag_name(), Some("input"));
    assert_eq!(node.get_attribute("type"), Some("radio"));
    assert_eq!(
        node.get_attribute("id"),
        Some("allow-once"),
        "first radio (autofocus target) is focused"
    );
}

#[test]
fn mount_demo_swap_replaces_subtree() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);

    mount_demo(&mut state, &mut dom, 0);
    let first_demo_root = dom
        .node(handles.main)
        .child_nodes()
        .next()
        .expect("first demo mounted")
        .id();

    mount_demo(&mut state, &mut dom, 1);

    assert_eq!(state.current_idx, 1);
    let main_children: Vec<_> = dom.node(handles.main).child_nodes().collect();
    assert_eq!(
        main_children.len(),
        1,
        "main has exactly one child after swap"
    );
    let new_root = main_children[0].id();
    assert_ne!(
        new_root, first_demo_root,
        "main's child is a different node than before the swap"
    );
}

#[test]
fn seed_tree_cursor_marks_mounted_demo_leaf() {
    // Boot-time seeding: the cursor lands on the leaf for the
    // given demo, so the navigator highlights "where you are"
    // before any arrow press.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let idx = 0;
    seed_tree_cursor(&mut dom, handles.sidebar, idx);

    let active = find_active(&dom, handles.sidebar).expect("a leaf is marked active");
    assert_eq!(
        dom.node(active).get_attribute("data-demo-slug"),
        Some(DEMOS[idx].slug()),
        "the active-descendant cursor is on the mounted demo's leaf"
    );
}

#[test]
fn seed_tree_cursor_expands_collapsed_ancestor_branches() {
    // A deep-linked demo whose category branch was collapsed
    // must become visible: seeding expands every ancestor
    // branch treeitem.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    // Pick a demo, find its leaf + parent branch, collapse it.
    let idx = crate::DEMOS.len() - 1; // TreeNav, in Built-ins
    let leaf = find_leaf_by_slug(&dom, handles.sidebar, DEMOS[idx].slug()).unwrap();
    // Walk to the nearest ancestor branch treeitem.
    let mut branch = dom.node(leaf).parent_node().map(|p| p.id());
    while let Some(id) = branch {
        let node = dom.node(id);
        if node.get_attribute("role") == Some("treeitem") && node.has_attribute("aria-expanded") {
            break;
        }
        branch = node.parent_node().map(|p| p.id());
    }
    let branch = branch.expect("leaf has an ancestor branch");
    dom.set_attribute(branch, "aria-expanded", "false").unwrap();

    seed_tree_cursor(&mut dom, handles.sidebar, idx);

    assert_eq!(
        dom.node(branch).get_attribute("aria-expanded"),
        Some("true"),
        "seeding re-expands the collapsed ancestor branch"
    );
}

#[test]
fn seed_tree_cursor_moves_a_stale_cursor() {
    // Re-seeding clears the previous active marker so exactly
    // one leaf is ever the cursor.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    seed_tree_cursor(&mut dom, handles.sidebar, 0);
    seed_tree_cursor(&mut dom, handles.sidebar, 1);

    // Count every node carrying the marker under the sidebar.
    fn count_active(dom: &TuiDom, id: NodeId) -> usize {
        let mut n = if dom.node(id).has_attribute(ACTIVE_ATTR) {
            1
        } else {
            0
        };
        for c in dom.node(id).child_nodes() {
            n += count_active(dom, c.id());
        }
        n
    }
    assert_eq!(
        count_active(&dom, handles.sidebar),
        1,
        "exactly one leaf carries the active-descendant cursor after re-seeding"
    );
    let active = find_active(&dom, handles.sidebar).unwrap();
    assert_eq!(
        dom.node(active).get_attribute("data-demo-slug"),
        Some(DEMOS[1].slug()),
        "the cursor moved to the most recently seeded demo"
    );
}

#[test]
fn mount_demo_same_index_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);

    mount_demo(&mut state, &mut dom, 0);
    let root_a = dom
        .node(handles.main)
        .child_nodes()
        .next()
        .expect("demo mounted")
        .id();

    mount_demo(&mut state, &mut dom, 0);
    let root_b = dom
        .node(handles.main)
        .child_nodes()
        .next()
        .expect("demo still mounted")
        .id();

    assert_eq!(
        root_a, root_b,
        "re-mounting the same demo doesn't rebuild the subtree"
    );
}
