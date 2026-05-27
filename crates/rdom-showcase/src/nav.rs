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

/// Shared mutable state the navigation owns: which demo is
/// currently mounted, plus the handles needed to swap it.
///
/// Wrapped in `Rc<RefCell<...>>` so the sidebar's click listener
/// (a `'static` closure) and the initial-mount path can both
/// touch it.
pub struct ShowcaseState {
    /// Index into [`DEMOS`] of the currently-mounted demo. Set to
    /// `usize::MAX` before any demo has been mounted so the first
    /// call to [`mount_demo`] always proceeds.
    pub current_idx: usize,
    /// Where demos mount — `<main>`'s view-content from
    /// [`crate::shell::ShellHandles`].
    pub main_id: NodeId,
    /// The `<details class="source-disclosure">` element below
    /// the view-content mount. `mount_demo` rebuilds its body
    /// (preserving the `<summary>`) with the active demo's
    /// MARKUP + CSS on every demo switch. UA's native
    /// `<details>` toggle handles open/close — no custom state.
    pub source_disclosure_id: NodeId,
    /// The status bar's **hints slot** — the left `<div>` inside
    /// `<footer class="status-bar">`. Cleared on every demo switch
    /// (the previous demo's scrollable element is gone; stale
    /// scroll info would lie). The mouse-position slot (right side)
    /// is OWNED by a separate listener and lives in its own div
    /// sibling, so writes here don't disturb it.
    pub status_bar_hints_id: NodeId,
}

impl ShowcaseState {
    /// Construct from `ShellHandles`. Initial state: no demo
    /// mounted (`current_idx = usize::MAX` so the first
    /// `mount_demo` call always proceeds).
    pub fn from_handles(handles: &crate::shell::ShellHandles) -> Self {
        Self {
            current_idx: usize::MAX,
            main_id: handles.main,
            source_disclosure_id: handles.source_disclosure,
            status_bar_hints_id: handles.status_bar_hints,
        }
    }
}

/// Swap the mounted demo. No-op if `demo_idx` is already mounted.
///
/// Mechanics: clear `<main>`'s view-content, rebuild the source
/// disclosure body, clear the scroll indicator (the previous
/// demo's scrollable element is gone). `clear_children` fires a
/// `ChildListChanged` record with every detached child + runs
/// the purge step from `rdom-core::tree::detach_from_parent`.
///
/// **Infallibility.** Both DOM mutations here are infallible by
/// construction: `main_id` / `source_disclosure_id` /
/// `status_bar_id` came from [`crate::build_shell`] and
/// stay alive for the App's lifetime; nodes created here have no
/// parent before append. `expect()` is the correct error
/// discipline.
pub fn mount_demo(state: &mut ShowcaseState, dom: &mut TuiDom, demo_idx: usize) {
    if state.current_idx == demo_idx {
        return;
    }
    assert!(
        demo_idx < DEMOS.len(),
        "mount_demo: idx {demo_idx} out of range (have {} demos)",
        DEMOS.len()
    );
    state.current_idx = demo_idx;
    let demo = DEMOS[demo_idx];

    // 1. Mount the live demo subtree.
    dom.clear_children(state.main_id)
        .expect("main_id from build_shell stays valid for the App's lifetime");
    let demo_root = demo.build(dom);
    dom.append_child(state.main_id, demo_root)
        .expect("demo_root was just created and has no parent");

    // 2. Rebuild the source disclosure body. Keep the `<summary>`
    //    that the shell put there; replace everything else with
    //    the new demo's MARKUP + CSS.
    rebuild_source_disclosure(dom, state.source_disclosure_id, demo);

    // 3. Reset the status bar — the previous demo's scrollable
    //    element is gone; stale "Row 7/50" text would lie about the
    //    new demo's state. Re-seed with the global default hints so
    //    the bar isn't empty between scroll events.
    crate::status_bar::seed_default_hints(dom, state.status_bar_hints_id);
}

/// Replace the body of the `<details class="source-disclosure">`
/// element with the new demo's MARKUP + CSS, preserving the
/// `<summary>` that the shell installed. The summary stays put so
/// the UA's open-state, focus, and click handler keep working
/// across demo switches.
fn rebuild_source_disclosure(dom: &mut TuiDom, disclosure: NodeId, demo: &dyn Demo) {
    // Collect non-summary children to remove. We can't
    // `clear_children` because that would drop the summary too.
    let to_remove: Vec<NodeId> = dom
        .node(disclosure)
        .child_nodes()
        .filter(|c| c.tag_name() != Some("summary"))
        .map(|c| c.id())
        .collect();
    for child in to_remove {
        let _ = dom.remove_child(disclosure, child);
    }

    let source = demo.source();
    append_labeled_pre(dom, disclosure, "Markup", source.markup);
    append_labeled_pre(dom, disclosure, "CSS", source.css);
}

/// Append `<h3>label</h3><pre>body</pre>` to `parent`. Used by
/// the source disclosure to render the demo's MARKUP / CSS.
fn append_labeled_pre(dom: &mut TuiDom, parent: NodeId, label: &str, body: &str) {
    let h = dom.create_element("h3");
    let h_text = dom.create_text_node(label);
    dom.append_child(h, h_text).unwrap();
    dom.append_child(parent, h).unwrap();

    let pre = dom.create_element("pre");
    let body_text = dom.create_text_node(body);
    dom.append_child(pre, body_text).unwrap();
    dom.append_child(parent, pre).unwrap();
}

/// Install the sidebar's click handler. Walks up from the click
/// target until it finds a `<li>` carrying `data-demo-slug`, then
/// looks up the demo by slug and calls [`mount_demo`].
///
/// Single listener on the sidebar — the click event bubbles up
/// from whichever `<li>` or descendant was clicked.
/// Install the scroll-indicator listener. Fires on every `scroll`
/// event bubbling up from any element in the document; only
/// updates the indicator when the event target is a descendant
/// of `view_root` (the view-content mount) AND has scrollable
/// content. Filters out scroll events from outside the demo
/// panel (e.g. a hypothetical scrollable sidebar `<details>`)
/// so the indicator stays meaningful.
///
/// rdom currently fires `scroll` events with `bubbles = true`
/// for all targets, which is a deliberate divergence from
/// CSSOM View Module §6 (browsers fire scroll on non-Document
/// elements as a non-bubbling event). The bubble lets us install
/// a single listener on `dom.root()` instead of re-installing
/// per-scrollable on every demo mount. See
/// [`specs/DIVERGENCES.md`](../../specs/DIVERGENCES.md) §Events.
pub fn wire_scroll_indicator(dom: &mut TuiDom, view_root: NodeId, indicator: NodeId) {
    let root = dom.root();
    dom.add_event_listener(root, "scroll", ListenerOptions::default(), move |ctx| {
        let Some(target) = ctx.event.target else {
            return;
        };
        if !is_descendant_of(ctx.dom, target, view_root) {
            return;
        }
        let Some(info) = read_scroll_info(ctx.dom, target) else {
            return;
        };
        let text = format_scroll_text(&info);
        write_indicator_text(ctx.dom, indicator, &text);
    })
    .expect("dom.root() is valid");
}

/// `true` if `id` is a descendant of `ancestor` (or equal to it).
/// Walks `parent_node()` up to the document root.
fn is_descendant_of(dom: &TuiDom, id: NodeId, ancestor: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
}

#[derive(Debug, Clone, Copy)]
struct ScrollInfo {
    scroll_y: usize,
    content_height: usize,
    viewport_height: usize,
}

fn read_scroll_info(dom: &TuiDom, target: NodeId) -> Option<ScrollInfo> {
    use rdom_tui::node::TuiNodeExt;
    let ext = dom.node(target).tui_ext()?;
    let viewport_height = ext.content_layout.height as usize;
    let content_height = ext.scroll_content_height;
    if content_height <= viewport_height {
        return None;
    }
    Some(ScrollInfo {
        scroll_y: ext.scroll_y,
        content_height,
        viewport_height,
    })
}

fn format_scroll_text(info: &ScrollInfo) -> String {
    let max_scroll = info.content_height.saturating_sub(info.viewport_height);
    // `checked_div` collapses the zero-divisor case (no scrollable
    // range → pinned at 100% by convention).
    let percent = info
        .scroll_y
        .saturating_mul(100)
        .checked_div(max_scroll)
        .map(|p| p.min(100))
        .unwrap_or(100);
    // Cell-aware label: the substrate scrolls by cells regardless
    // of row height. For fixed-row demos (scrollable_list)
    // "cell Y of H" reads as rows; for mixed-height demos it
    // doesn't lie about row counts.
    format!(
        "{percent}% — cell {y}/{h}",
        y = info.scroll_y,
        h = info.content_height
    )
}

fn write_indicator_text(dom: &mut TuiDom, indicator: NodeId, text: &str) {
    // Replace the indicator's children with a single text node.
    let _ = dom.clear_children(indicator);
    let t = dom.create_text_node(text);
    let _ = dom.append_child(indicator, t);
}

/// Install a `mousemove` listener on the document root that writes
/// the cursor's current `X:<col> Y:<row>` into `indicator` (the
/// status bar). Useful as a live debugging gauge: if motion-event
/// delivery breaks (terminal-side mouse-tracking quirk), the
/// numbers freeze immediately and the user can tell at a glance,
/// without tailing a trace log.
///
/// `mousemove` is dispatched at the hit target and bubbles up; the
/// document-root listener catches every motion event in the app.
pub fn wire_mouse_position_indicator(dom: &mut TuiDom, indicator: NodeId) {
    let root = dom.root();
    dom.add_event_listener(root, "mousemove", ListenerOptions::default(), move |ctx| {
        let mouse = match ctx.event.detail.as_mouse() {
            Some(m) => m,
            None => {
                rdom_tui::rdom_trace!("mousemove listener: event.detail is not Mouse — skipping");
                return;
            }
        };
        let text = format!("X: {} Y: {}", mouse.client_x, mouse.client_y);
        rdom_tui::rdom_trace!(
            "mousemove listener: writing '{text}' to indicator NodeId({indicator:?})"
        );
        write_indicator_text(ctx.dom, indicator, &text);
    })
    .expect("dom.root() is valid");
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
