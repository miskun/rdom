//! `Element.scrollIntoView` — CSSOM View §5.2 "scroll an element into
//! view" (`P7-SCROLL-INTO-VIEW-ALIGN-1`).
//!
//! Every scroll container on the element's ancestor chain scrolls,
//! innermost first, each by §5.1 "determine the scroll-into-view
//! position" for the `block` (vertical) and `inline` (horizontal)
//! [`ScrollLogicalPosition`], and each through "perform a scroll" with
//! the options' behavior. The scrollport is the container's padding box
//! (CSS Overflow 3 §3), as for every other scroll writer.
//!
//! The boxes come from the last layout, and a scroll written since
//! then (a `scrollTo` earlier in the same listener) has not moved them
//! yet: each container's drift from the offsets its last layout used
//! (`ScrollState::laid_out`) is folded in, and after a container is
//! scrolled the element's box is carried along to where that scroll
//! puts it, so the next container out aligns the element's new
//! position — the rect CSSOM re-reads per container.
//!
//! Not the reveal of `reveal.rs`: the caret of a text control and the
//! listbox / tree cursor keep their instant, vertical, nearest-container
//! `nearest` reveal, as browsers reveal the caret and a focused element.

use rdom_core::NodeId;

use super::scroll::scroll_bounds;
use crate::TuiDom;
use crate::layout::{LayoutRect, Overflow};
use crate::node::TuiNodeExt;
use crate::runtime::smooth_scroll::{ScrollIntoViewOptions, ScrollLogicalPosition, perform_scroll};

/// Scroll every scroll container on `element`'s ancestor chain so it is
/// aligned per `options`. No-op for an element that is not rendered
/// (it or an ancestor is `display: none`, or it was never cascaded).
pub(crate) fn scroll_element_into_view(
    dom: &mut TuiDom,
    element: NodeId,
    options: ScrollIntoViewOptions,
) {
    // CSSOM View §5.2 step 1: no box → return. An element never
    // cascaded has none either.
    let rendered = dom.node(element).computed().is_some() && crate::node::is_rendered(dom, element);
    let Some(mut rect) = dom.node(element).tui_ext().map(|e| e.layout) else {
        return;
    };
    if !rendered {
        return;
    }
    let mut cur = dom.node(element).parent_node().map(|p| p.id());
    while let Some(container) = cur {
        if establishes_scrolling_box(dom, container) {
            rect = scroll_container(dom, container, rect, options);
        }
        cur = dom.node(container).parent_node().map(|p| p.id());
    }
}

/// Overflow other than `visible` on either axis: a scroll container,
/// scrollable from code even with `overflow: hidden`.
fn establishes_scrolling_box(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id).computed().is_some_and(|c| {
        !matches!(c.overflow_x, Overflow::Visible) || !matches!(c.overflow_y, Overflow::Visible)
    })
}

/// Align `rect` (the element's box in last-layout coordinates) inside
/// `container` and scroll it there. Returns the box where the scroll
/// puts it, in the coordinates of the containers further out.
fn scroll_container(
    dom: &mut TuiDom,
    container: NodeId,
    rect: LayoutRect,
    options: ScrollIntoViewOptions,
) -> LayoutRect {
    let Some(ext) = dom.node(container).tui_ext() else {
        return rect;
    };
    let border = dom
        .node(container)
        .computed()
        .map(|c| c.border)
        .unwrap_or_default();
    let port = crate::layout::compute_padding_box(ext.layout, border);
    let (cur_x, cur_y) = (ext.scroll_x, ext.scroll_y);
    let laid = super::state::laid_out(ext);
    let (laid_x, laid_y) = (laid.0, laid.1);
    // The inline axis's start is the right edge of an `rtl` box (CSS
    // Writing Modes 4 §2.1): `start` / `end` align that edge.
    let rtl = ext
        .computed
        .as_ref()
        .is_some_and(|c| c.text_direction == crate::layout::TextDirection::Rtl);
    // The element's edges relative to the scrollport at the current
    // offsets.
    let rel_x = rect.x - port.x - (cur_x - laid_x);
    let rel_y = rect.y - port.y - (cur_y - laid_y);
    let inline = match options.inline {
        ScrollLogicalPosition::Start if rtl => ScrollLogicalPosition::End,
        ScrollLogicalPosition::End if rtl => ScrollLogicalPosition::Start,
        other => other,
    };
    let Some(bounds) = scroll_bounds(dom, container) else {
        return rect;
    };
    let (to_x, to_y) = bounds.clamp(
        cur_x + align(rel_x, rect.width, port.width, inline),
        cur_y + align(rel_y, rect.height, port.height, options.block),
    );
    // §5.2: perform the scroll unless the position is unchanged and no
    // smooth scroll is in flight — `perform_scroll` is a no-op then.
    perform_scroll(dom, container, to_x, to_y, options.behavior);
    LayoutRect {
        x: port.x + rel_x - (to_x - cur_x),
        y: port.y + rel_y - (to_y - cur_y),
        ..rect
    }
}

/// CSSOM View §5.1 on one axis: the scroll delta that aligns an element
/// at `start` (relative to the scrollport's start edge) of `size` cells
/// inside a scrollport of `port` cells.
///
/// `Nearest` follows the spec's four cases; an element exactly as large
/// as the scrollport counts as fitting (the spec's strict comparisons
/// leave that case with no alignment, where browsers align the nearer
/// edge).
fn align(start: i32, size: u16, port: u16, position: ScrollLogicalPosition) -> i32 {
    let (size, port) = (i32::from(size), i32::from(port));
    let end = start + size;
    match position {
        ScrollLogicalPosition::Start => start,
        ScrollLogicalPosition::End => end - port,
        ScrollLogicalPosition::Center => start + (size - port).div_euclid(2),
        ScrollLogicalPosition::Nearest => {
            let before = start < 0;
            let after = end > port;
            let fits = size <= port;
            if (before && after) || (!before && !after) {
                // Covering the scrollport, or already fully in view.
                0
            } else if (before && fits) || (after && !fits) {
                start
            } else {
                end - port
            }
        }
    }
}
