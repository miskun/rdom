//! ARIA tree behavior — keyboard navigation, expand/collapse,
//! and the active-descendant cursor for `<ul role=tree>`.
//!
//! ## Model (see DIVERGENCES.md "ARIA tree")
//!
//! - The `[role=tree]` container holds focus (it's implicitly
//!   focusable — see `focus::tabindex`). The cursor is an internal
//!   `data-rdom-active` marker moved among visible `[role=treeitem]`s;
//!   the UA highlights it only while the container is focused.
//! - A branch is any treeitem with an `aria-expanded` attribute
//!   (presence, not child count). `aria-expanded="true|false"`
//!   opens/closes; expand/collapse fires a non-bubbling `toggle`
//!   event (same shape as `<details>`), which is also the lazy-load
//!   hook for an unloaded branch.
//! - Activation (Enter / Space / pointer) dispatches a `click` on
//!   the active item so apps get one hook for selection / navigation.
//!
//! Keys (all `preventDefault`-overridable): Up/Down move the cursor,
//! Home/End jump, Right expands-or-descends, Left collapses-or-
//! ascends, Enter/Space toggle a branch + activate. No vi keys —
//! apps add `j`/`k`/`g`/`G` via their own listener.

#[cfg(test)]
mod tests;

use rdom_core::{EventDetail, ListenerOptions, NodeId, ToggleDetail, ToggleState};

use crate::runtime::focus::focus_node;
use crate::tui_event::TuiDispatchExt;
use crate::{TuiDom, TuiEvent};

const ACTIVE_ATTR: &str = "data-rdom-active";

/// Install the tree keydown + click default actions. Called once
/// from `App::build`.
pub fn install(dom: &mut TuiDom) {
    let root = dom.root();

    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        if role(ctx.dom, focused) != Some("tree") {
            return;
        }
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        // v1 ignores modified keys — they belong to app shortcuts.
        if key.modifiers.ctrl || key.modifiers.alt || key.modifiers.meta {
            return;
        }
        handle_key(ctx.dom, focused, key.key.as_str());
    })
    .expect("tree keydown listener install");

    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(item) = closest_treeitem(ctx.dom, target) else {
            return;
        };
        let Some(tree) = enclosing_tree(ctx.dom, item) else {
            return;
        };
        // Active-descendant model: clicking a row focuses the
        // container and moves the cursor to the row.
        focus_node(ctx.dom, Some(tree));
        set_active(ctx.dom, tree, item);
        // Clicking anywhere on a branch row toggles it (lens-faithful
        // — the arrow isn't the only hit target). Gated on a real
        // pointer click: keyboard-synthesized clicks carry no mouse
        // detail, so Enter/Space don't double-toggle.
        if is_branch(ctx.dom, item) && ctx.event.detail.as_mouse().is_some() {
            set_expanded(ctx.dom, item, !is_expanded(ctx.dom, item));
        }
    })
    .expect("tree click listener install");
}

// ── Keyboard ────────────────────────────────────────────────────

fn handle_key(dom: &mut TuiDom, tree: NodeId, key: &str) {
    let items = visible_items(dom, tree);
    if items.is_empty() {
        return;
    }
    let cur = active(dom, tree);
    let cur_idx = cur.and_then(|c| items.iter().position(|&i| i == c));

    match key {
        "ArrowDown" => {
            let i = cur_idx.map(|i| (i + 1).min(items.len() - 1)).unwrap_or(0);
            set_active(dom, tree, items[i]);
        }
        "ArrowUp" => {
            let i = cur_idx.map(|i| i.saturating_sub(1)).unwrap_or(0);
            set_active(dom, tree, items[i]);
        }
        "Home" => set_active(dom, tree, items[0]),
        "End" => set_active(dom, tree, items[items.len() - 1]),
        "ArrowRight" => {
            let item = cur.unwrap_or(items[0]);
            if is_branch(dom, item) {
                if is_expanded(dom, item) {
                    // Descend to the first child.
                    if let Some(first) = first_child_item(dom, item) {
                        set_active(dom, tree, first);
                    }
                } else {
                    // Expand (the lazy-load hook fires here via `toggle`).
                    set_expanded(dom, item, true);
                }
            }
        }
        "ArrowLeft" => {
            let item = cur.unwrap_or(items[0]);
            if is_branch(dom, item) && is_expanded(dom, item) {
                set_expanded(dom, item, false);
            } else if let Some(parent) = parent_item(dom, item) {
                set_active(dom, tree, parent);
            }
        }
        "Enter" | " " => {
            let item = cur.unwrap_or(items[0]);
            if is_branch(dom, item) {
                set_expanded(dom, item, !is_expanded(dom, item));
            }
            activate(dom, item);
        }
        _ => {}
    }
}

// ── State mutation + events ─────────────────────────────────────

/// Set `aria-expanded` and fire a non-bubbling `toggle` event (same
/// shape as `<details>`). No-op when already in the target state.
fn set_expanded(dom: &mut TuiDom, item: NodeId, open: bool) {
    if is_expanded(dom, item) == open {
        return;
    }
    let _ = dom.set_attribute(item, "aria-expanded", if open { "true" } else { "false" });
    let (old_state, new_state) = if open {
        (ToggleState::Closed, ToggleState::Open)
    } else {
        (ToggleState::Open, ToggleState::Closed)
    };
    let mut ev = TuiEvent::new("toggle");
    ev.event = ev.event.clone().with_bubbles(false);
    ev.event.detail = EventDetail::Toggle(Box::new(ToggleDetail {
        old_state,
        new_state,
    }));
    let _ = dom.dispatch_tui_event(item, &mut ev);
}

/// Dispatch a bubbling `click` on `item` so apps get one activation
/// hook for both keyboard and pointer. The tree's own click default
/// action treats a detail-less (keyboard) click as cursor-only.
fn activate(dom: &mut TuiDom, item: NodeId) {
    let mut ev = TuiEvent::new("click");
    let _ = dom.dispatch_tui_event(item, &mut ev);
}

/// Move the `data-rdom-active` cursor to `item`, clearing it from
/// the previously-active row in the same tree.
fn set_active(dom: &mut TuiDom, tree: NodeId, item: NodeId) {
    if let Some(prev) = active(dom, tree)
        && prev != item
    {
        let _ = dom.remove_attribute(prev, ACTIVE_ATTR);
    }
    let _ = dom.set_attribute(item, ACTIVE_ATTR, "");
}

// ── Structure queries ───────────────────────────────────────────

/// Visible treeitems of `tree` in document order — descends into a
/// branch only when it's expanded, so collapsed subtrees are skipped.
fn visible_items(dom: &TuiDom, tree: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    collect_visible(dom, tree, &mut out);
    out
}

fn collect_visible(dom: &TuiDom, container: NodeId, out: &mut Vec<NodeId>) {
    for item in treeitem_children(dom, container) {
        out.push(item);
        if is_expanded(dom, item)
            && let Some(group) = child_group(dom, item)
        {
            collect_visible(dom, group, out);
        }
    }
}

fn active(dom: &TuiDom, tree: NodeId) -> Option<NodeId> {
    fn walk(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
        for child in dom.node(id).child_nodes() {
            if child.has_attribute(ACTIVE_ATTR) {
                return Some(child.id());
            }
            if let Some(found) = walk(dom, child.id()) {
                return Some(found);
            }
        }
        None
    }
    walk(dom, tree)
}

fn is_branch(dom: &TuiDom, item: NodeId) -> bool {
    dom.node(item).has_attribute("aria-expanded")
}

fn is_expanded(dom: &TuiDom, item: NodeId) -> bool {
    dom.node(item).get_attribute("aria-expanded") == Some("true")
}

/// First `[role=treeitem]` inside `item`'s child group, if any.
fn first_child_item(dom: &TuiDom, item: NodeId) -> Option<NodeId> {
    let group = child_group(dom, item)?;
    treeitem_children(dom, group).into_iter().next()
}

/// Nearest ancestor `[role=treeitem]` of `item` (its parent group's
/// owning item), if any.
fn parent_item(dom: &TuiDom, item: NodeId) -> Option<NodeId> {
    let mut cur = dom.node(item).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if role(dom, id) == Some("treeitem") {
            return Some(id);
        }
        if role(dom, id) == Some("tree") {
            return None;
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// Direct `[role=treeitem]` children of `container`.
fn treeitem_children(dom: &TuiDom, container: NodeId) -> Vec<NodeId> {
    dom.node(container)
        .child_nodes()
        .filter(|n| n.get_attribute("role") == Some("treeitem"))
        .map(|n| n.id())
        .collect()
}

/// First direct `[role=group]` child of `item`.
fn child_group(dom: &TuiDom, item: NodeId) -> Option<NodeId> {
    dom.node(item)
        .child_nodes()
        .find(|n| n.get_attribute("role") == Some("group"))
        .map(|n| n.id())
}

fn closest_treeitem(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if role(dom, n) == Some("treeitem") {
            return Some(n);
        }
        if role(dom, n) == Some("tree") {
            return None;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

fn enclosing_tree(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(n) = cur {
        if role(dom, n) == Some("tree") {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

fn role(dom: &TuiDom, id: NodeId) -> Option<&str> {
    dom.node(id).get_attribute("role")
}
