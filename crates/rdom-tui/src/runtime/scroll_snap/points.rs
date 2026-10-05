//! A snap container's snap positions on one axis (CSS Scroll Snap 1
//! §6.1): for each box it is the snap container of — a descendant whose
//! nearest scroll container it is — with a `scroll-snap-align` on the
//! axis, the scroll offset that aligns the box's scroll snap area (its
//! border box outset by `scroll-margin`, §4.2) with the snapport (the
//! scrollport inset by `scroll-padding`, §4.1), clamped to the scroll
//! range. Read from the last layout.

use rdom_core::{Dom, NodeId, NodeType};

use super::select::Position;
use crate::TuiExt;
use crate::layout::{LayoutRect, ScrollSnapStop, SnapAlign};
use crate::node::TuiNodeExt;
use crate::runtime::scrollbar::ScrollAxis;

/// One snap position and the box it aligns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapPoint {
    pub(crate) position: Position,
    pub(crate) target: NodeId,
}

/// `container`'s snap positions on `axis`, in tree order.
pub(crate) fn snap_points(
    dom: &Dom<TuiExt>,
    container: NodeId,
    axis: ScrollAxis,
) -> Vec<SnapPoint> {
    let Some(ext) = dom.node(container).tui_ext() else {
        return Vec::new();
    };
    let Some(c) = ext.computed.as_deref() else {
        return Vec::new();
    };
    let padding_box = crate::layout::compute_padding_box(ext.layout, c.border);
    let port = crate::runtime::scrollbar::inset(padding_box, c);
    // The children were laid out at these offsets: a box's place in the
    // scrolled content is its rect plus them.
    let laid = crate::runtime::scrollbar::state::laid_out(ext);
    let bounds = crate::runtime::scrollbar::scroll_bounds(dom, container);
    let mut out = Vec::new();
    collect(dom, container, &mut |id, rect, align, stop| {
        let align = axis_align(align, axis);
        let (start, len, port_start, port_len, laid) = match axis {
            ScrollAxis::Vertical => (rect.y, rect.height, port.y, port.height, laid.1),
            ScrollAxis::Horizontal => (rect.x, rect.width, port.x, port.width, laid.0),
        };
        let (len, port_len) = (i32::from(len), i32::from(port_len));
        let start = start + laid - port_start;
        let offset = match align {
            SnapAlign::None => return,
            SnapAlign::Start => start,
            SnapAlign::End => start + len - port_len,
            SnapAlign::Center => start + (len - port_len).div_euclid(2),
        };
        let offset = match (bounds, axis) {
            (Some(b), ScrollAxis::Vertical) => offset.clamp(b.min_y, b.max_y),
            (Some(b), ScrollAxis::Horizontal) => offset.clamp(b.min_x, b.max_x),
            (None, _) => offset,
        };
        out.push(SnapPoint {
            position: Position {
                offset,
                stop: stop == ScrollSnapStop::Always,
            },
            target: id,
        });
    });
    out
}

/// The alignment of `align` on `axis`: the block value on the vertical
/// axis, the inline value on the horizontal one (`horizontal-tb`).
fn axis_align(align: crate::layout::ScrollSnapAlign, axis: ScrollAxis) -> SnapAlign {
    match axis {
        ScrollAxis::Vertical => align.block,
        ScrollAxis::Horizontal => align.inline,
    }
}

/// Call `f` with each rendered box below `id` whose snap container `id`
/// is, its scroll snap area, its alignment on both axes and its stop —
/// not descending into a nested scroll container, whose own boxes are its
/// own (it is one of `id`'s, though).
fn collect(
    dom: &Dom<TuiExt>,
    id: NodeId,
    f: &mut dyn FnMut(NodeId, LayoutRect, crate::layout::ScrollSnapAlign, ScrollSnapStop),
) {
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Fragment => collect(dom, child.id(), f),
            NodeType::Element => {
                let Some(ext) = child.tui_ext() else {
                    continue;
                };
                let Some(c) = ext.computed.as_deref() else {
                    continue;
                };
                if c.display == crate::layout::Display::None {
                    continue;
                }
                if c.display != crate::layout::Display::Contents {
                    let area = crate::runtime::scrollbar::outset(ext.layout, c);
                    f(child.id(), area, c.scroll_snap_align, c.scroll_snap_stop);
                }
                if !c.is_scroll_container() {
                    collect(dom, child.id(), f);
                }
            }
            _ => {}
        }
    }
}
