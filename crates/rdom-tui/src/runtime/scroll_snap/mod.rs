//! Scroll snapping (CSS Scroll Snap 1): a snap container — a scroll
//! container with a `scroll-snap-type` — comes to rest at a snap position
//! after each scroll operation on its snapping axes, and re-snaps after a
//! layout change (§5.4).
//!
//! - `points` — the snap positions on an axis, from the last layout.
//! - `select` — choosing one for a scroll, pure: the nearest for a scroll
//!   to a destination, the next in the direction for a scroll by a delta,
//!   never passing a `scroll-snap-stop: always` position, `proximity`
//!   within [`select::PROXIMITY_CELLS`].
//!
//! Where it runs: the wheel (`router::mouse`), the keyboard and the
//! programmatic scrolls through `smooth_scroll::perform_scroll` (a smooth
//! scroll animates to the snapped destination and settles there),
//! `scrollIntoView`, a scrollbar thumb drag's release and a track click
//! (`scrollbar::drag`), and after layout ([`resnap`], from the frame).
//! The box each axis last snapped to is kept in the container's
//! `ScrollState`, for §5.4.

mod points;
pub(crate) mod select;

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{ScrollSnapStrictness, ScrollSnapType};
use crate::runtime::scrollbar::ScrollAxis;
pub(crate) use select::Intent;

/// How `id` snaps: on the horizontal axis, on the vertical one, and
/// whether `mandatory`. `None` for a box that is no snap container.
fn snapping(dom: &TuiDom, id: NodeId) -> Option<(bool, bool, bool)> {
    let c = dom.node(id).ext()?.computed.as_deref()?;
    if !c.is_scroll_container() {
        return None;
    }
    match c.scroll_snap_type {
        ScrollSnapType::None => None,
        ScrollSnapType::Snap(axis, strictness) => {
            let (x, y) = axis.physical();
            Some((x, y, strictness == ScrollSnapStrictness::Mandatory))
        }
    }
}

/// Where a scroll of `element` to `to` comes to rest (§6.2): on each axis
/// it snaps on, the snap position [`select::choose`] picks for `motion`,
/// else `to`. Records the box snapped to on each
/// axis for [`resnap`].
pub(crate) fn snap(
    dom: &mut TuiDom,
    element: NodeId,
    to: (i32, i32),
    motion: Motion,
) -> (i32, i32) {
    let Some((on_x, on_y, mandatory)) = snapping(dom, element) else {
        return to;
    };
    let mut out = to;
    let mut targets = (None, None);
    if on_x {
        let i = match motion {
            Motion::To => Intent::Nearest,
            Motion::By { from } => Intent::Directional { from: from.0 },
        };
        if let Some((offset, target)) =
            pick(dom, element, ScrollAxis::Horizontal, to.0, i, mandatory)
        {
            out.0 = offset;
            targets.0 = Some(target);
        }
    }
    if on_y {
        let i = match motion {
            Motion::To => Intent::Nearest,
            Motion::By { from } => Intent::Directional { from: from.1 },
        };
        if let Some((offset, target)) = pick(dom, element, ScrollAxis::Vertical, to.1, i, mandatory)
        {
            out.1 = offset;
            targets.1 = Some(target);
        }
    }
    if let Some(ext) = dom.node_mut(element).ext_mut() {
        crate::runtime::scrollbar::state::set_snapped(ext, targets);
    }
    out
}

/// How a scroll moves, on both axes ([`Intent`] per axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motion {
    /// To a destination: the nearest snap position.
    To,
    /// By a delta from `from`: a position in its direction.
    By { from: (i32, i32) },
}

/// The snap position and box a scroll to `dest` on `axis` rests at.
fn pick(
    dom: &TuiDom,
    element: NodeId,
    axis: ScrollAxis,
    dest: i32,
    intent: Intent,
    mandatory: bool,
) -> Option<(i32, NodeId)> {
    let points = points::snap_points(dom, element, axis);
    let positions: Vec<select::Position> = points.iter().map(|p| p.position).collect();
    let i = select::choose(&positions, dest, intent, mandatory)?;
    Some((points[i].position.offset, points[i].target))
}

/// §5.4 "Re-snapping After Layout Changes": each snap container that
/// snapped to a box on an axis stays snapped to it — at its new snap
/// position — while it still is one; a `mandatory` container whose box is
/// gone rests at the position nearest its offset. Returns whether an
/// offset moved (the layout is then stale).
pub(crate) fn resnap(dom: &mut TuiDom) -> bool {
    let mut containers = Vec::new();
    collect_snap_containers(dom, dom.root(), &mut containers);
    let mut moved = false;
    for id in containers {
        let Some((on_x, on_y, mandatory)) = snapping(dom, id) else {
            continue;
        };
        let Some(ext) = dom.node(id).ext() else {
            continue;
        };
        let (cur_x, cur_y) = (ext.scroll_x, ext.scroll_y);
        let snapped = crate::runtime::scrollbar::state::snapped(ext);
        let resolve = |axis: ScrollAxis, on: bool, target: Option<NodeId>, cur: i32| {
            if !on {
                return cur;
            }
            let points = points::snap_points(dom, id, axis);
            if let Some(p) = target.and_then(|t| points.iter().find(|p| p.target == t)) {
                return p.position.offset;
            }
            if !mandatory {
                return cur;
            }
            let positions: Vec<select::Position> = points.iter().map(|p| p.position).collect();
            select::choose(&positions, cur, Intent::Nearest, true)
                .map_or(cur, |i| positions[i].offset)
        };
        let x = resolve(ScrollAxis::Horizontal, on_x, snapped.0, cur_x);
        let y = resolve(ScrollAxis::Vertical, on_y, snapped.1, cur_y);
        if (x, y) != (cur_x, cur_y) {
            moved |= crate::runtime::scrollbar::write_offsets(dom, id, x, y);
        }
    }
    moved
}

/// The snap containers in `id`'s subtree, in tree order.
fn collect_snap_containers(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        let cid = child.id();
        if snapping(dom, cid).is_some() {
            out.push(cid);
        }
        collect_snap_containers(dom, cid, out);
    }
}

#[cfg(test)]
mod tests;
