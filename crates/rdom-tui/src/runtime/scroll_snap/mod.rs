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
//! What each axis is snapped to — the box and its snap position — is kept
//! in the container's `ScrollState` for §5.4, and cleared by any scroll
//! that is not a snap's (`scrollbar::scroll`'s funnel).

mod points;
pub(crate) mod select;

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{ScrollSnapStrictness, ScrollSnapType};
use crate::runtime::scrollbar::ScrollAxis;
use crate::runtime::scrollbar::state::SnapRecord;
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
/// else `to`. Records the box snapped to on each axis and its snap
/// position for [`resnap`] (the scroll's write keeps the record,
/// `scrollbar::WriteKind::Snap`).
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
            targets.0 = Some(SnapRecord { target, offset });
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
            targets.1 = Some(SnapRecord { target, offset });
        }
    }
    if let Some(ext) = dom.node_mut(element).ext_mut() {
        crate::runtime::scrollbar::state::set_snapped(ext, targets);
    }
    if targets != (None, None) {
        Snapped::insert(dom, element);
    }
    out
}

/// Document data: the snap containers a snap recorded a target in, in
/// the order they first snapped — the ones [`resnap`] looks at.
#[derive(Debug, Default)]
struct Snapped(Vec<NodeId>);

impl Snapped {
    fn insert(dom: &mut TuiDom, id: NodeId) {
        if dom.document_data::<Self>().is_none() {
            dom.set_document_data(Self::default());
        }
        if let Some(set) = dom.document_data_mut::<Self>()
            && !set.0.contains(&id)
        {
            set.0.push(id);
        }
    }
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

#[cfg(test)]
thread_local! {
    /// Containers [`resnap`] examined (cost tests).
    pub(super) static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// §5.4 "Re-snapping After Layout Changes": "If the scroll container was
/// snapped before the content change and that same snap position still
/// exists … the scroll container must be re-snapped to that same snap
/// position after the content change." A container snapped to a box on
/// an axis follows the box only when the layout moved its snap position —
/// a container at rest at it is left alone, so a scroll that is not a
/// snap's (whose write cleared the record, `scrollbar::scroll`) is never
/// undone; a `mandatory` container whose box is gone rests at the
/// position nearest its offset. A smooth scroll in flight is retargeted
/// rather than cut short. Visits only the containers that snapped (the
/// [`Snapped`] set, pruned of those no longer snapped, snapping or in
/// the document); the `scroll` event of a move is queued for after the
/// frame. Returns whether an offset moved (the layout is then stale).
pub(crate) fn resnap(dom: &mut TuiDom) -> bool {
    let containers = dom
        .document_data_mut::<Snapped>()
        .map(|s| std::mem::take(&mut s.0))
        .unwrap_or_default();
    let mut kept = Vec::with_capacity(containers.len());
    let mut moved = false;
    for id in containers {
        #[cfg(test)]
        VISITS.with(|c| c.set(c.get() + 1));
        if !dom.contains(id) {
            continue;
        }
        let Some((on_x, on_y, mandatory)) = snapping(dom, id) else {
            continue;
        };
        let Some(ext) = dom.node(id).ext() else {
            continue;
        };
        let (cur_x, cur_y) = crate::runtime::smooth_scroll::destination(dom, id);
        let (rec_x, rec_y) = crate::runtime::scrollbar::state::snapped(ext);
        if (rec_x, rec_y) == (None, None) {
            continue;
        }
        let x = follow(
            dom,
            id,
            ScrollAxis::Horizontal,
            on_x,
            rec_x,
            cur_x,
            mandatory,
        );
        let y = follow(dom, id, ScrollAxis::Vertical, on_y, rec_y, cur_y, mandatory);
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            crate::runtime::scrollbar::state::set_snapped(ext, (x.1, y.1));
        }
        if (x.1, y.1) != (None, None) {
            kept.push(id);
        }
        if (x.0, y.0) != (cur_x, cur_y) {
            // A smooth scroll in flight lands there instead; its next
            // step moves the box.
            if !crate::runtime::smooth_scroll::retarget(dom, id, (x.0, y.0)) {
                moved |= crate::runtime::scrollbar::write_offsets_queued(dom, id, x.0, y.0);
            }
        }
    }
    if let Some(set) = dom.document_data_mut::<Snapped>() {
        // Containers that snapped while this ran (a listener's scroll)
        // are already in the set; keep them after the survivors.
        let added = std::mem::take(&mut set.0);
        set.0 = kept;
        for id in added {
            if !set.0.contains(&id) {
                set.0.push(id);
            }
        }
    } else if !kept.is_empty() {
        dom.set_document_data(Snapped(kept));
    }
    moved
}

/// Where a container snapped as `record` on `axis` rests after a layout,
/// at `cur` now, and what it is snapped to then: the record's box's snap
/// position when the layout moved it; `cur` when it did not, when the
/// container does not snap on `axis` or was not snapped there; under
/// `mandatory` the position nearest `cur` when the box is gone.
fn follow(
    dom: &TuiDom,
    id: NodeId,
    axis: ScrollAxis,
    on: bool,
    record: Option<SnapRecord>,
    cur: i32,
    mandatory: bool,
) -> (i32, Option<SnapRecord>) {
    let (true, Some(record)) = (on, record) else {
        return (cur, None);
    };
    let points = points::snap_points(dom, id, axis);
    if let Some(p) = points.iter().find(|p| p.target == record.target) {
        let offset = p.position.offset;
        if offset == record.offset {
            return (cur, Some(record));
        }
        return (
            offset,
            Some(SnapRecord {
                target: record.target,
                offset,
            }),
        );
    }
    if !mandatory {
        return (cur, None);
    }
    let positions: Vec<select::Position> = points.iter().map(|p| p.position).collect();
    match select::choose(&positions, cur, Intent::Nearest, true) {
        Some(i) => (
            positions[i].offset,
            Some(SnapRecord {
                target: points[i].target,
                offset: positions[i].offset,
            }),
        ),
        None => (cur, None),
    }
}

#[cfg(test)]
mod resnap_tests;
#[cfg(test)]
mod tests;
