//! Navigation — mount/swap logic + sidebar click wiring.
//!
//! The showcase mounts exactly one demo at a time into the shell's
//! `<main>` element. Switching to another demo is a subtree swap:
//! clear `<main>`'s children, build the new demo's subtree, append.
//! Per-demo CSS is preloaded as separate stylesheets on the App at
//! startup; since each demo's CSS uses unique class-scoped selectors
//! (convention enforced by review), the cascade naturally applies
//! only the active demo's rules.
//!
//! The actual subtree replacement exercises M1 D2's substrate
//! contract — focus / selection / hover / pointer-capture state
//! that pointed into the old subtree gets cleaned up automatically
//! by `detach_from_parent`'s purge step.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use crate::DEMOS;

/// Shared mutable state the navigation owns: which demo is
/// currently mounted, and where to mount the next one.
///
/// Wrapped in `Rc<RefCell<...>>` so the sidebar's click listener
/// (a `'static` closure) and the initial-mount path can both
/// touch it.
pub struct ShowcaseState {
    /// Index into [`DEMOS`] of the currently-mounted demo. Set to
    /// `usize::MAX` before any demo has been mounted so the first
    /// call to [`mount_demo`] always proceeds.
    pub current_idx: usize,
    /// Where demos mount — `<main>` from [`crate::shell::ShellHandles`].
    pub main_id: NodeId,
}

/// Swap the mounted demo. No-op if `demo_idx` is already mounted.
///
/// Mechanics: clear `<main>`'s children (which detaches the previous
/// demo's subtree, triggering M1 D2's interaction-state cleanup),
/// build the new demo's subtree under `dom`, append it to `<main>`.
/// The cascade picks up the new subtree on the next paint —
/// `MutationObserver` records flow through, `DirtyTracker` marks
/// the new root.
///
/// **Infallibility.** Both DOM mutations here are infallible by
/// construction: `main_id` came from [`crate::build_shell`] and is
/// kept alive for the App's lifetime, and `demo_root` was created
/// one line earlier and is therefore not currently parented.
/// `expect()` is the correct error discipline — if either call
/// errored, the showcase's invariants are broken at the substrate
/// level and continuing would produce an inconsistent DOM (empty
/// `<main>` with `current_idx` pointing at a demo that's not
/// there). Panic is the safer outcome.
pub fn mount_demo(state: &mut ShowcaseState, dom: &mut TuiDom, demo_idx: usize) {
    if state.current_idx == demo_idx {
        return;
    }
    assert!(
        demo_idx < DEMOS.len(),
        "mount_demo: idx {demo_idx} out of range (have {} demos)",
        DEMOS.len()
    );
    // Atomic-as-far-as-this-function-is-concerned subtree swap.
    // The DOM's `clear_children` fires a `ChildListChanged` record
    // with every detached child + runs the purge step from
    // `rdom-core::tree::detach_from_parent`.
    dom.clear_children(state.main_id)
        .expect("main_id from build_shell stays valid for the App's lifetime");
    let demo_root = DEMOS[demo_idx].build(dom);
    dom.append_child(state.main_id, demo_root)
        .expect("demo_root was just created and has no parent");
    state.current_idx = demo_idx;
}

/// Install the sidebar's click handler. Walks up from the click
/// target until it finds a `<li>` carrying `data-demo-slug`, then
/// looks up the demo by slug and calls [`mount_demo`].
///
/// Single listener on the sidebar — the click event bubbles up
/// from whichever `<li>` or descendant was clicked.
pub fn wire_sidebar_click(dom: &mut TuiDom, sidebar: NodeId, state: Rc<RefCell<ShowcaseState>>) {
    dom.add_event_listener(sidebar, "click", ListenerOptions::default(), move |ctx| {
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(idx) = find_demo_idx_from_target(ctx.dom, target) else {
            return;
        };
        mount_demo(&mut state.borrow_mut(), ctx.dom, idx);
    })
    .expect("sidebar is a valid node");
}

/// Walk up from `start` looking for an `<li>` element carrying
/// `data-demo-slug`. Returns the matching demo's index in
/// [`DEMOS`], or `None` if no such ancestor exists.
///
/// **Why pinned to `<li>`:** the contract is "demo activation is
/// triggered only by interacting with a demo's `<li>` row in the
/// sidebar." If a `data-demo-slug` somehow ended up on a different
/// element (e.g. a future `<details>` with a slug attribute would
/// fire both a category toggle AND a demo swap on every header
/// click — silent footgun), the ancestor walk would still hit it
/// and trigger a swap. Restricting the match to `<li>` makes the
/// contract a tag-and-attribute pair rather than just an attribute.
fn find_demo_idx_from_target(dom: &TuiDom, start: NodeId) -> Option<usize> {
    let mut cur = Some(start);
    while let Some(id) = cur {
        let node = dom.node(id);
        if node.tag_name() == Some("li")
            && let Some(slug) = node.get_attribute("data-demo-slug")
            && let Some(idx) = DEMOS.iter().position(|d| d.slug() == slug)
        {
            return Some(idx);
        }
        cur = node.parent_node().map(|p| p.id());
    }
    None
}

/// Install the sidebar's keyboard handler. Listens on the sidebar
/// for `keydown` and handles:
///
/// - `ArrowDown` / `ArrowUp` — move focus between sidebar `<li>`s
///   in document order. Wraps at edges.
/// - `Enter` / `Space` — activate the focused `<li>` (mount that
///   demo). Equivalent to clicking it.
///
/// Doesn't fight the runtime's `Tab` / `Shift+Tab` traversal —
/// that already handles moving focus between focusable elements
/// (the `<li>`s carry `tabindex="0"` so they participate).
///
/// **Known gap (M7 polish):** ArrowDown / ArrowUp on a focused
/// `<summary>` (category header) is a no-op — focus stays where
/// it is. ARIA authoring practice would say ArrowDown from a
/// category `<summary>` should descend into that category's
/// first `<li>`, and ArrowUp from the first `<li>` should rise
/// to its parent `<summary>`. Not wired because (a) the `<li>`s
/// are reachable via Tab regardless, (b) the right shape needs
/// real `aria-expanded` / `aria-tree` semantics that haven't
/// landed yet. Defer to M7 (showcase polish).
pub fn wire_sidebar_keys(dom: &mut TuiDom, sidebar: NodeId, state: Rc<RefCell<ShowcaseState>>) {
    dom.add_event_listener(sidebar, "keydown", ListenerOptions::default(), move |ctx| {
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        // Only act when focus is on a demo `<li>`.
        let Some(focused_idx) = find_demo_idx_from_target(ctx.dom, focused) else {
            return;
        };
        // Read the key from the event detail (set by the
        // runtime's keyboard router).
        let key = ctx
            .event
            .detail
            .as_keyboard()
            .map(|k| k.key.as_str())
            .unwrap_or("");
        match key {
            "ArrowDown" => {
                if let Some(next) = next_demo_li(ctx.dom, focused, sidebar, Direction::Down) {
                    ctx.dom.set_focused(Some(next));
                    ctx.event.prevent_default();
                }
            }
            "ArrowUp" => {
                if let Some(prev) = next_demo_li(ctx.dom, focused, sidebar, Direction::Up) {
                    ctx.dom.set_focused(Some(prev));
                    ctx.event.prevent_default();
                }
            }
            "Enter" | " " => {
                mount_demo(&mut state.borrow_mut(), ctx.dom, focused_idx);
                ctx.event.prevent_default();
            }
            _ => {}
        }
    })
    .expect("sidebar is a valid node");
}

#[derive(Copy, Clone)]
enum Direction {
    Up,
    Down,
}

/// Collect every `<li data-demo-slug>` under `sidebar` in document
/// order, find `current`'s position, return the neighbor in
/// `direction`. Wraps.
///
/// Cost: O(sidebar subtree size) per keystroke — we re-walk the
/// sidebar on every arrow because the tree can mutate (collapsing
/// a `<details>` category, dynamically adding demos at runtime).
/// At the showcase's scale (~tens of demos, two-level tree) this
/// is unmeasurable; if we ever ship hundreds of demos, cache the
/// list and invalidate on a `MutationObserver` listening for
/// `ChildListChanged` under the sidebar.
fn next_demo_li(
    dom: &TuiDom,
    current: NodeId,
    sidebar: NodeId,
    direction: Direction,
) -> Option<NodeId> {
    let lis = collect_demo_lis(dom, sidebar);
    if lis.is_empty() {
        return None;
    }
    let cur_pos = lis.iter().position(|&id| id == current)?;
    let next_pos = match direction {
        Direction::Down => (cur_pos + 1) % lis.len(),
        Direction::Up => {
            if cur_pos == 0 {
                lis.len() - 1
            } else {
                cur_pos - 1
            }
        }
    };
    Some(lis[next_pos])
}

/// Document-order walk of `<li data-demo-slug>` under `sidebar`.
fn collect_demo_lis(dom: &TuiDom, sidebar: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, sidebar, &mut out);
    out
}

fn walk(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some("li")
        && dom.node(id).get_attribute("data-demo-slug").is_some()
    {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk(dom, child.id(), out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_shell;

    #[test]
    fn mount_demo_initial_attaches_first_demo() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let mut state = ShowcaseState {
            current_idx: usize::MAX,
            main_id: handles.main,
        };

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
    fn mount_demo_swap_replaces_subtree() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let mut state = ShowcaseState {
            current_idx: usize::MAX,
            main_id: handles.main,
        };

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
    fn next_demo_li_arrow_down_walks_forward_in_document_order() {
        // Sidebar groups demos by category; arrow-down should
        // traverse `<li data-demo-slug>` in document order
        // regardless of which `<details>` they sit under.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let lis = collect_demo_lis(&dom, handles.sidebar);
        assert_eq!(
            lis.len(),
            crate::DEMOS.len(),
            "one focusable <li> per registered demo"
        );

        let after_first =
            next_demo_li(&dom, lis[0], handles.sidebar, Direction::Down).expect("has next");
        assert_eq!(after_first, lis[1]);
    }

    #[test]
    fn next_demo_li_arrow_down_wraps_at_end() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let lis = collect_demo_lis(&dom, handles.sidebar);
        let last = *lis.last().expect("at least one demo");

        let after_last =
            next_demo_li(&dom, last, handles.sidebar, Direction::Down).expect("wraps to first");
        assert_eq!(after_last, lis[0], "ArrowDown on the last item wraps");
    }

    #[test]
    fn next_demo_li_arrow_up_wraps_at_start() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let lis = collect_demo_lis(&dom, handles.sidebar);
        let first = lis[0];

        let before_first =
            next_demo_li(&dom, first, handles.sidebar, Direction::Up).expect("wraps to last");
        assert_eq!(
            before_first,
            *lis.last().unwrap(),
            "ArrowUp on the first item wraps"
        );
    }

    #[test]
    fn mount_demo_same_index_is_noop() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let mut state = ShowcaseState {
            current_idx: usize::MAX,
            main_id: handles.main,
        };

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
}
