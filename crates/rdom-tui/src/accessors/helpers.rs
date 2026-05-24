//! Private helpers used across the `TuiAccessors` / `TuiAccessorsMut`
//! impl blocks. Visibility is `pub(super)` so siblings (`read_ref`,
//! `read_mut`, `write`) can call in; nothing in this file is part of
//! the public API.

use rdom_core::NodeId;

use crate::{Result, TuiDom, TuiExt};

/// Parse a numeric f64 attribute. Used by `<progress>` /
/// `<meter>` accessors — mirrors the parsing in
/// `runtime::builtins::gauge::parse_attr` but is kept local to
/// avoid bumping that helper to `pub`.
pub(super) fn parse_numeric_attribute(
    node: &rdom_core::NodeRef<'_, TuiExt>,
    name: &str,
) -> Option<f64> {
    node.get_attribute(name).and_then(|s| s.parse().ok())
}

/// Walk up from `start` (inclusive) to the nearest `<form>`
/// ancestor. Returns `None` when no ancestor matches.
pub(super) fn nearest_form_ancestor(dom: &TuiDom, start: NodeId) -> Option<NodeId> {
    let mut cur = Some(start);
    while let Some(id) = cur {
        let node = dom.node(id);
        if node.tag_name() == Some("form") {
            return Some(id);
        }
        cur = node.parent_node().map(|p| p.id());
    }
    None
}

pub(super) fn write_boolean_attribute(
    node: &mut rdom_core::NodeMut<'_, TuiExt>,
    name: &str,
    value: bool,
) -> Result<()> {
    if value {
        node.set_attribute(name, "")
    } else {
        node.remove_attribute(name).map(|_| ())
    }
}

pub(super) fn is_text_family_input(dom: &TuiDom, id: NodeId) -> bool {
    matches!(
        dom.node(id).get_attribute("type"),
        None | Some("text")
            | Some("password")
            | Some("email")
            | Some("url")
            | Some("tel")
            | Some("search")
            | Some("number")
    )
}

/// Mark the first descendant `<option>` whose effective value matches
/// `target` as `selected`; clear `selected` from every other option.
/// No match → every option ends up unselected. Matches
/// `HTMLSelectElement.value` setter.
pub(super) fn set_select_value(dom: &mut TuiDom, select: NodeId, target: &str) -> Result<()> {
    let options: Vec<NodeId> = collect_options(dom, select);
    // Find the first match in document order.
    let first_match: Option<NodeId> = options
        .iter()
        .copied()
        .find(|&id| crate::runtime::builtins::select::option_value(dom, id) == target);
    for opt in options {
        let should_select = Some(opt) == first_match;
        let is_selected = dom.has_attribute(opt, "selected");
        if should_select && !is_selected {
            dom.set_attribute(opt, "selected", "")?;
        } else if !should_select && is_selected {
            dom.remove_attribute(opt, "selected")?;
        }
    }
    Ok(())
}

fn collect_options(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_options(dom, root, &mut out);
    out
}

fn walk_options(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some("option") {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_options(dom, child.id(), out);
    }
}

pub(super) fn read_scroll_x(dom: &TuiDom, id: NodeId) -> i32 {
    use crate::node::TuiNodeExt;
    dom.node(id)
        .tui_ext()
        .map(|e| e.scroll_x as i32)
        .unwrap_or(0)
}

pub(super) fn read_scroll_y(dom: &TuiDom, id: NodeId) -> i32 {
    use crate::node::TuiNodeExt;
    dom.node(id)
        .tui_ext()
        .map(|e| e.scroll_y as i32)
        .unwrap_or(0)
}

/// Clamp `(x, y)` to `[0, content - viewport]` and write to the
/// element's scroll offsets. Non-scrollable elements (no
/// scrollable content beyond their viewport) end up pinned to
/// `(0, 0)` — the browser-faithful silent no-op behavior.
pub(super) fn write_scroll_clamped(dom: &mut TuiDom, id: NodeId, x: i32, y: i32) {
    use crate::node::TuiNodeExt;
    let (viewport_w, viewport_h, content_w, content_h) = match dom.node(id).tui_ext() {
        Some(e) => (
            e.content_layout.width as i32,
            e.content_layout.height as i32,
            e.scroll_content_width as i32,
            e.scroll_content_height as i32,
        ),
        None => return,
    };
    let max_x = (content_w - viewport_w).max(0);
    let max_y = (content_h - viewport_h).max(0);
    let clamped_x = x.clamp(0, max_x) as usize;
    let clamped_y = y.clamp(0, max_y) as usize;
    let (changed, _old) = if let Some(ext) = dom.node_mut(id).ext_mut() {
        let old = (ext.scroll_x, ext.scroll_y);
        ext.scroll_x = clamped_x;
        ext.scroll_y = clamped_y;
        ((old.0, old.1) != (clamped_x, clamped_y), old)
    } else {
        return;
    };
    if changed {
        // M5 D5: fire `scroll` on the element whose offset moved.
        // Programmatic + scrollbar + wheel all converge here when
        // they touch ext.scroll_*; the wheel path also has its own
        // dispatch site for the case where it walks past the
        // initial hit to find a scrollable ancestor.
        // `scroll`: bubbles, NOT cancelable per HTML.
        let mut tui = crate::TuiEvent::new("scroll");
        tui.event.cancelable = false;
        let _ = crate::TuiDispatchExt::dispatch_tui_event(dom, id, &mut tui);
    }
}

/// Walk up from `start` to find the nearest ancestor whose
/// computed `overflow` is `Hidden`, `Scroll`, or `Auto` (the
/// scrollable values). Returns `None` when no ancestor is
/// scrollable — `scroll_into_view` becomes a no-op in that case,
/// matching browser behavior.
pub(super) fn nearest_scrollable_ancestor(dom: &TuiDom, start: NodeId) -> Option<NodeId> {
    use crate::layout::Overflow;
    use crate::node::TuiNodeExt;
    let mut cur = dom.node(start).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if let Some(ext) = dom.node(id).tui_ext()
            && matches!(
                ext.overflow,
                Overflow::Hidden | Overflow::Scroll | Overflow::Auto
            )
        {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// Cumulative `(x, y)` position of `descendant` inside `ancestor`'s
/// pre-scroll content area. Walks the parent chain from
/// `descendant` up to (but not including) `ancestor`, summing each
/// step's `ext.layout` position and undoing the scroll offset that
/// the layout pass already applied at each intermediate parent.
pub(super) fn pre_scroll_offset_within(
    dom: &TuiDom,
    descendant: NodeId,
    ancestor: NodeId,
) -> (i32, i32) {
    use crate::node::TuiNodeExt;
    let (mut accum_x, mut accum_y) = (0i32, 0i32);
    let mut cur = descendant;
    while cur != ancestor {
        let Some(ext) = dom.node(cur).tui_ext() else {
            break;
        };
        accum_x += ext.layout.x;
        accum_y += ext.layout.y;
        let Some(parent) = dom.node(cur).parent_node() else {
            break;
        };
        let parent_id = parent.id();
        // The parent's own scroll offset was already applied when
        // positioning `cur`. Undo it so the accumulator stays in
        // pre-scroll coords — except for the final ancestor, whose
        // scroll offset we're about to overwrite anyway.
        if parent_id != ancestor
            && let Some(parent_ext) = dom.node(parent_id).tui_ext()
        {
            accum_x += parent_ext.scroll_x as i32;
            accum_y += parent_ext.scroll_y as i32;
        }
        cur = parent_id;
    }
    (accum_x, accum_y)
}

/// Walk ancestors honoring `contenteditable="inherit"` semantics:
///
/// - `"true"`, `""` (HTML boolean shorthand), `"plaintext-only"`
///   → effective `true`.
/// - `"false"` → effective `false` (overrides any inherited true
///   from a higher ancestor).
/// - absent / unrecognized → continue walking.
///
/// Falls off the root as `false`. Matches HTMLElement.isContentEditable.
pub(super) fn effective_content_editable(dom: &TuiDom, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(node_id) = cur {
        let node = dom.node(node_id);
        match node.get_attribute("contenteditable") {
            Some("true") | Some("") | Some("plaintext-only") => return true,
            Some("false") => return false,
            _ => {}
        }
        cur = node.parent_node().map(|p| p.id());
    }
    false
}
