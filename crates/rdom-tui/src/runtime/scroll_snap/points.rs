//! A snap container's snap positions on one axis (CSS Scroll Snap 1
//! §6.1): for each box it is the snap container of — a descendant whose
//! nearest scroll container it is — with a `scroll-snap-align` on the
//! axis, the scroll offset that aligns the box's scroll snap area (its
//! border box outset by `scroll-margin`, §4.2) with the snapport (the
//! scrollport inset by `scroll-padding`, §4.1), clamped to the scroll
//! range — and for an area longer than the snapport the range of offsets
//! at which it covers it (§6.2.3). Read from the last layout.

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
    /// For a snap area longer than the snapport, the offsets at which it
    /// covers the snapport, `start ..= end` within the scroll range —
    /// each a valid snap position (§6.2.3).
    pub(crate) cover: Option<(i32, i32)>,
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
    let scrollport = crate::render::layout_pass::scrollport_of(ext, c);
    let port = crate::runtime::scrollbar::inset(scrollport, c);
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
        let clamp = |v: i32| match (bounds, axis) {
            (Some(b), ScrollAxis::Vertical) => v.clamp(b.min_y, b.max_y),
            (Some(b), ScrollAxis::Horizontal) => v.clamp(b.min_x, b.max_x),
            (None, _) => v,
        };
        // §6.2.3: the area covers the snapport from aligning its start
        // with the snapport's to aligning its end.
        let cover = (len > port_len).then(|| (clamp(start), clamp(start + len - port_len)));
        out.push(SnapPoint {
            position: Position {
                offset: clamp(offset),
                stop: stop == ScrollSnapStop::Always,
            },
            target: id,
            cover,
        });
    });
    out
}

/// The positions a scroll may rest at among `points` (§6.2.3), each with
/// the index of its point: every point's aligned position, and for an area
/// longer than the snapport the two ends of its covering range — the
/// offsets aligning its start and its end with the snapport's — which are
/// no `scroll-snap-stop`.
pub(crate) fn positions(points: &[SnapPoint]) -> (Vec<Position>, Vec<usize>) {
    let mut positions = Vec::with_capacity(points.len());
    let mut owners = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        positions.push(p.position);
        owners.push(i);
        if let Some((start, end)) = p.cover {
            for offset in [start, end] {
                if offset != p.position.offset {
                    positions.push(Position {
                        offset,
                        stop: false,
                    });
                    owners.push(i);
                }
            }
        }
    }
    (positions, owners)
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
