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

use crate::{DEMOS, Demo};

/// Which view of the current demo is mounted in `<main>`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ViewMode {
    /// Live demo subtree from `Demo::build`.
    Demo,
    /// `<pre>` block containing the demo's `MARKUP` + `CSS`
    /// strings (`Demo::source()`). Authors browse this to learn
    /// what code produces the live demo on the left.
    Source,
}

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
    /// Current view mode. Switching demos resets to `ViewMode::Demo`
    /// so authors always see the live demo first.
    pub view: ViewMode,
    /// Where demos mount — `<main>` from [`crate::shell::ShellHandles`].
    pub main_id: NodeId,
    /// The `<nav class="view-tabs">` container — needed at mount
    /// time so the active-tab class flips when view mode changes.
    pub view_tabs_id: NodeId,
}

impl ShowcaseState {
    /// Construct from `ShellHandles`. Initial state: no demo mounted
    /// (`current_idx = usize::MAX` so the first `mount_demo` call
    /// always proceeds), Demo view, active-tab class will land on
    /// the first mount.
    pub fn from_handles(handles: &crate::shell::ShellHandles) -> Self {
        Self {
            current_idx: usize::MAX,
            view: ViewMode::Demo,
            main_id: handles.main,
            view_tabs_id: handles.view_tabs,
        }
    }
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
    // Switching demos resets the view to Demo — authors always
    // start with the live view of a freshly-clicked demo.
    state.view = ViewMode::Demo;
    state.current_idx = demo_idx;
    remount_current_view(state, dom);
    update_tab_active_class(dom, state.view_tabs_id, state.view);
}

/// Switch the currently-mounted view between Demo and Source for
/// the active demo. No-op if the requested view is already mounted.
pub fn set_view(state: &mut ShowcaseState, dom: &mut TuiDom, view: ViewMode) {
    if state.view == view {
        return;
    }
    state.view = view;
    remount_current_view(state, dom);
    update_tab_active_class(dom, state.view_tabs_id, state.view);
}

/// Internal: clear `<main>`'s view-content and mount whatever the
/// current view mode dictates. Used by both `mount_demo` (demo
/// changed) and `set_view` (view mode changed, demo stayed).
fn remount_current_view(state: &mut ShowcaseState, dom: &mut TuiDom) {
    debug_assert!(state.current_idx < DEMOS.len());
    dom.clear_children(state.main_id)
        .expect("main_id from build_shell stays valid for the App's lifetime");
    match state.view {
        ViewMode::Demo => {
            let demo_root = DEMOS[state.current_idx].build(dom);
            dom.append_child(state.main_id, demo_root)
                .expect("demo_root was just created and has no parent");
        }
        ViewMode::Source => {
            let source_root = build_source_view(dom, DEMOS[state.current_idx]);
            dom.append_child(state.main_id, source_root)
                .expect("source_root was just created and has no parent");
        }
    }
}

/// Build a `<pre>` block containing the demo's `MARKUP` + `CSS`
/// strings, separated by a header line. Returns the root.
fn build_source_view(dom: &mut TuiDom, demo: &dyn Demo) -> NodeId {
    let source = demo.source();
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "source-view").unwrap();

    append_labeled_pre(dom, root, "Markup", source.markup);
    append_labeled_pre(dom, root, "CSS", source.css);

    root
}

fn append_labeled_pre(dom: &mut TuiDom, parent: NodeId, label: &str, body: &str) {
    let h = dom.create_element("h2");
    let h_text = dom.create_text_node(label);
    dom.append_child(h, h_text).unwrap();
    dom.append_child(parent, h).unwrap();

    let pre = dom.create_element("pre");
    let body_text = dom.create_text_node(body);
    dom.append_child(pre, body_text).unwrap();
    dom.append_child(parent, pre).unwrap();
}

/// Walk the tab buttons under `tabs_root`; on each, ensure the
/// `active` class is present iff its `data-view` matches the
/// currently-active view. Uses class-list operations so we don't
/// disturb other classes the buttons carry.
fn update_tab_active_class(dom: &mut TuiDom, tabs_root: NodeId, view: ViewMode) {
    let target = match view {
        ViewMode::Demo => "demo",
        ViewMode::Source => "source",
    };
    let mut buttons = Vec::new();
    collect_view_tabs(dom, tabs_root, &mut buttons);
    for btn in buttons {
        let is_target = dom
            .node(btn)
            .get_attribute("data-view")
            .map(|v| v == target)
            .unwrap_or(false);
        if is_target {
            let _ = dom.add_class(btn, "active");
        } else {
            let _ = dom.remove_class(btn, "active");
        }
    }
}

fn collect_view_tabs(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some("button")
        && dom.node(id).get_attribute("data-view").is_some()
    {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        collect_view_tabs(dom, child.id(), out);
    }
}

/// Install the sidebar's click handler. Walks up from the click
/// target until it finds a `<li>` carrying `data-demo-slug`, then
/// looks up the demo by slug and calls [`mount_demo`].
///
/// Single listener on the sidebar — the click event bubbles up
/// from whichever `<li>` or descendant was clicked.
/// Install the view-tabs click handler. Single listener on the
/// `<nav class="view-tabs">` container; walks the target's
/// ancestors looking for a `data-view` attribute, then calls
/// [`set_view`].
pub fn wire_view_tab_click(dom: &mut TuiDom, view_tabs: NodeId, state: Rc<RefCell<ShowcaseState>>) {
    dom.add_event_listener(view_tabs, "click", ListenerOptions::default(), move |ctx| {
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(view) = find_view_attr_ancestor(ctx.dom, target) else {
            return;
        };
        set_view(&mut state.borrow_mut(), ctx.dom, view);
    })
    .expect("view_tabs is a valid node");
}

fn find_view_attr_ancestor(dom: &TuiDom, start: NodeId) -> Option<ViewMode> {
    let mut cur = Some(start);
    while let Some(id) = cur {
        if dom.node(id).tag_name() == Some("button")
            && let Some(v) = dom.node(id).get_attribute("data-view")
        {
            return match v {
                "demo" => Some(ViewMode::Demo),
                "source" => Some(ViewMode::Source),
                _ => None,
            };
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

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
}
