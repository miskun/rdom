//! Scroll-container geometry — read-only over the last layout.
//!
//! Owns the padding-box scroll metrics (CSS Overflow 3 §3: the
//! scrollport is the padding box) and the predicates that decide
//! whether a box is a scroll container on either axis, plus the
//! ancestor walk to the nearest one. The scroll *writers* live in
//! `scroll.rs`.

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;
use crate::layout::Overflow;
use crate::node::TuiNodeExt;

/// `(viewport_size_in_cells, current_scroll_offset)` for a given
/// element + axis. Viewport = padding-box per CSS Overflow 3 §3.
pub(super) fn scroll_metrics(dom: &TuiDom, element: NodeId, axis: ScrollAxis) -> (u16, usize) {
    let ext = match dom.node(element).tui_ext() {
        Some(e) => e,
        None => return (0, 0),
    };
    let border = dom
        .node(element)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = crate::layout::compute_padding_box(ext.layout, border);
    match axis {
        ScrollAxis::Vertical => (pb.height, ext.scroll_y),
        ScrollAxis::Horizontal => (pb.width, ext.scroll_x),
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
/// `overflow-y` other than `visible`: the box clips and may scroll,
/// whether or not the last layout found anything to scroll.
pub(super) fn scrolls_vertically_by_style(dom: &TuiDom, id: NodeId) -> bool {
    let overflow_y = dom
        .node(id)
        .computed()
        .map(|c| c.overflow_y)
        .unwrap_or(Overflow::Visible);
    !matches!(overflow_y, Overflow::Visible)
}

pub(super) fn is_vertical_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    let Some(ext) = dom.node(id).tui_ext() else {
        return false;
    };
    if !scrolls_vertically_by_style(dom, id) {
        return false;
    }
    let border = dom
        .node(id)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = crate::layout::compute_padding_box(ext.layout, border);
    ext.scroll_content_height > pb.height as usize
}

/// `true` when `id` clips on the X axis and its content is wider than its
/// scrollport. Horizontal analog of [`is_vertical_scroll_container`].
pub(super) fn is_horizontal_scroll_container(dom: &TuiDom, id: NodeId) -> bool {
    let Some(ext) = dom.node(id).tui_ext() else {
        return false;
    };
    let overflow_x = dom
        .node(id)
        .computed()
        .map(|c| c.overflow_x)
        .unwrap_or(Overflow::Visible);
    if matches!(overflow_x, Overflow::Visible) {
        return false;
    }
    let border = dom
        .node(id)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let pb = crate::layout::compute_padding_box(ext.layout, border);
    ext.scroll_content_width > pb.width as usize
}
