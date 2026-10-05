//! Scroll-container geometry — read-only over the last layout.
//!
//! Owns the scroll metrics over the scrollport (`layout_pass::scrollport`)
//! and the predicates that decide
//! whether a box is a scroll container on either axis, plus the
//! ancestor walk to the nearest one. The scroll *writers* live in
//! `scroll.rs`.

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;
use crate::layout::Overflow;
use crate::node::TuiNodeExt;

/// `(viewport_size_in_cells, current_scroll_offset)` for a given
/// element + axis — the offset as `scrollTop` / `scrollLeft` (negative
/// for an `rtl` box scrolled left). The viewport is the scrollport
/// (`layout_pass::scrollport`).
pub(super) fn scroll_metrics(dom: &TuiDom, element: NodeId, axis: ScrollAxis) -> (u16, i32) {
    let Some(ext) = dom.node(element).tui_ext() else {
        return (0, 0);
    };
    let port = crate::render::layout_pass::scrollport(dom, element).unwrap_or_default();
    match axis {
        ScrollAxis::Vertical => (port.height, ext.scroll_y),
        ScrollAxis::Horizontal => (port.width, ext.scroll_x),
    }
}

/// How far the scrollport of `element` sits from the start of its
/// scrollable overflow area along `axis` — the scroll offset a
/// scrollbar thumb, a thumb drag and the autoscroll bands measure,
/// physical and never negative (`layout_pass::offset_from_area_start`).
pub(crate) fn offset_from_area_start(dom: &TuiDom, element: NodeId, axis: ScrollAxis) -> usize {
    let (x, y) = crate::render::layout_pass::offset_from_area_start(dom, element);
    match axis {
        ScrollAxis::Vertical => y,
        ScrollAxis::Horizontal => x,
    }
}

/// The nearest scroll container of `id` (itself included): an ancestor
/// whose content overflows a non-`visible` axis after the last layout.
/// The keyboard scroll keys and the drag-autoscroll engine act on it,
/// and its scrollbar thumb carries the focus cue.
pub(super) fn nearest_scroll_container(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(id) = cur {
        if is_vertical_scroll_container(dom, id) || is_horizontal_scroll_container(dom, id) {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// `true` when `id` clips on the Y axis and has more content than its
/// scrollport can show (i.e. there's somewhere to scroll to).
/// `overflow-y` `hidden`, `scroll` or `auto`: the box may scroll,
/// whether or not the last layout found anything to scroll (`clip`
/// never does).
pub(super) fn scrolls_vertically_by_style(dom: &TuiDom, id: NodeId) -> bool {
    let overflow_y = dom
        .node(id)
        .computed()
        .map(|c| c.overflow_y)
        .unwrap_or(Overflow::Visible);
    overflow_y.is_scrollable()
}

pub(super) fn is_vertical_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    scrolls_vertically_by_style(dom, id) && crate::render::layout_pass::overflows(dom, id).1
}

/// `true` when `id` clips on the X axis and its content is wider than its
/// scrollport. Horizontal analog of [`is_vertical_scroll_container`].
pub(super) fn is_horizontal_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    let overflow_x = dom
        .node(id)
        .computed()
        .map(|c| c.overflow_x)
        .unwrap_or(Overflow::Visible);
    overflow_x.is_scrollable() && crate::render::layout_pass::overflows(dom, id).0
}
