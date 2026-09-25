//! Reveal — scroll something into view.
//!
//! Two entry points: the caret inside a scrolling text leaf (after a
//! caret move or an edit, with a deferred re-run once the next layout
//! knows the true extent), and an arbitrary node's region for
//! keyboard-navigated widgets (`scroll_into_view`). Both use
//! `block: "nearest"` alignment that never hides the region's top
//! edge, and both write through `scroll.rs`.

use rdom_core::NodeId;

use super::ScrollAxis;
use super::geometry::{is_vertical_scroll_container, scrolls_vertically_by_style};
use super::scroll::{ClampTo, set_scroll_with};
use crate::TuiDom;
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;

/// Keep the caret visible inside a text leaf that scrolls (a
/// `<textarea>` taller than its box): after a caret move or an edit,
/// scroll the caret's own inline-flow container so the caret row is
/// inside its scrollport. No-op when the caret's container does not
/// scroll vertically or there is no collapsed selection.
pub(crate) fn reveal_caret(dom: &mut TuiDom) {
    reveal_caret_with(dom, ClampTo::NextLayout, true);
}

/// Re-run a pending caret reveal against the layout that just ran.
/// Returns `true` when a scroll offset changed, so the caller lays out
/// once more before painting. Clears the flag either way.
pub(crate) fn service_caret_reveal(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection() else {
        return false;
    };
    let focus = sel.focus;
    let Some(crate::render::inline::InlineFlow::Ifc { block }) =
        crate::render::inline::inline_flow_for_text(dom, focus.node)
    else {
        return false;
    };
    let pending = dom
        .node(block)
        .tui_ext()
        .is_some_and(|e| e.caret_reveal_pending);
    if !pending {
        return false;
    }
    if let Some(ext) = dom.node_mut(block).ext_mut() {
        ext.caret_reveal_pending = false;
    }
    let before = dom.node(block).tui_ext().map(|e| e.scroll_y);
    reveal_caret_with(dom, ClampTo::CurrentExtent, false);
    dom.node(block).tui_ext().map(|e| e.scroll_y) != before
}

fn reveal_caret_with(dom: &mut TuiDom, clamp: ClampTo, mark_pending: bool) {
    let Some(sel) = dom.selection() else { return };
    if !sel.is_collapsed() {
        return;
    }
    let focus = sel.focus;
    let Some(flow) = crate::render::inline::inline_flow_for_text(dom, focus.node) else {
        return;
    };
    let crate::render::inline::InlineFlow::Ifc { block } = flow else {
        return;
    };
    // The edit that moved the caret may have added a line the last
    // layout's extent does not know about yet — possibly the first line
    // that overflows at all, so the check is on `overflow-y`, not on the
    // stale extent: reveal now against that extent (no clamp), and again
    // after the next layout.
    if mark_pending
        && scrolls_vertically_by_style(dom, block)
        && let Some(ext) = dom.node_mut(block).ext_mut()
    {
        ext.caret_reveal_pending = true;
    }
    if !is_vertical_scroll_container(dom, block) {
        return;
    }
    let Some((x, y)) = crate::render::inline::cell_of_position(dom, focus) else {
        return;
    };
    ensure_visible_vertical_with(
        dom,
        block,
        LayoutRect {
            x: x as i32,
            y: y as i32,
            width: 1,
            height: 1,
        },
        clamp,
    );
}

/// Scroll the nearest vertically-scrollable ancestor of `node` so the
/// viewport-coord region `reveal` becomes visible. Uses
/// `block: "nearest"` alignment that never hides `reveal`'s TOP edge,
/// so a navigation cursor row stays anchored — the ARIA listbox/tree
/// pattern, and the browser's focus scroll-into-view. Vertical axis
/// only for now; nested scroll containers resolve to the nearest one
/// (sufficient for current widgets — revisit if a nested-scroller case
/// appears).
///
/// Reuses the shared scroll writer, so the clamp to `[0, max]` and the
/// `scroll` event dispatch are shared with wheel / scrollbar interaction.
pub(crate) fn scroll_into_view(dom: &mut TuiDom, node: NodeId, reveal: LayoutRect) {
    let mut cur = dom.node(node).parent_node().map(|p| p.id());
    while let Some(id) = cur {
        if is_vertical_scroll_container(dom, id) {
            ensure_visible_vertical(dom, id, reveal);
            return;
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
}

/// Adjust `container`'s vertical scroll so `reveal` is in the
/// scrollport, anchoring `reveal`'s top edge (never scroll so far down
/// that the top leaves the view).
fn ensure_visible_vertical(dom: &mut TuiDom, container: NodeId, reveal: LayoutRect) {
    ensure_visible_vertical_with(dom, container, reveal, ClampTo::CurrentExtent)
}

fn ensure_visible_vertical_with(
    dom: &mut TuiDom,
    container: NodeId,
    reveal: LayoutRect,
    clamp: ClampTo,
) {
    let (port_top, port_bottom, cur_scroll) = {
        let Some(ext) = dom.node(container).tui_ext() else {
            return;
        };
        let border = dom
            .node(container)
            .computed()
            .map(|c| c.border)
            .unwrap_or_default();
        let pb = crate::layout::compute_padding_box(ext.layout, border);
        (pb.y, pb.y + pb.height as i32, ext.scroll_y as i32)
    };
    let r_top = reveal.y;
    let r_bottom = reveal.y + reveal.height as i32;
    let delta = if r_top < port_top {
        // Region above the scrollport — scroll up to reveal its top.
        r_top - port_top
    } else if r_bottom > port_bottom {
        // Region below — scroll down just enough, but never past the
        // top edge (so a region taller than the port aligns its top).
        (r_bottom - port_bottom).min(r_top - port_top).max(0)
    } else {
        0
    };
    if delta != 0 {
        set_scroll_with(
            dom,
            container,
            ScrollAxis::Vertical,
            cur_scroll + delta,
            clamp,
        );
    }
}
