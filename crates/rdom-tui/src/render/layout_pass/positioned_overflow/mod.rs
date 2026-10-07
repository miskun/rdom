//! Absolutely positioned boxes in their scroll container's scrollable
//! overflow (CSS Overflow 3 §2.2: the area covers "the border boxes of
//! all boxes for which it is the containing block", and their own
//! scrollable overflow).
//!
//! Ordering. A scroll container's extent is recorded, its offset clamped
//! and its `auto` scrollbars settled in phase 1 (`layout_node`), and
//! positioned boxes are placed in phase 2 against containing blocks
//! phase 1 laid out. So phase 1 reads the positioned boxes' reach from
//! the document's last [`settle`] — per scroll container, the rect the
//! boxes it contains cover, relative to its border box and unscrolled,
//! so it does not move with the container or its offset — and [`settle`]
//! measures it again once they are placed. When it changed (a box was
//! added, moved or removed), `layout_dom` runs phases 1–2 again with the
//! new reach, so the extent, the clamp and the gutters see it — and once
//! more when that run's scrollbar changed the boxes' containing block
//! (a `left: 0; right: 0` box narrowed by the bar its height added), so
//! the frame painted after a layout is converged. A document whose
//! positioned boxes stay put lays out once; never more than
//! [`MAX_ROUNDS`] times (a reach still changing then flips a scrollbar
//! on and off, and is kept for the next layout).
//!
//! A box counts in the nearest scroll container at or above its
//! containing block — a scroll container between the box and its
//! containing block does not contain it — cut to the overflow clip
//! edges of the `overflow: clip` boxes from its containing block up
//! (CSS 2.1 §11.1.1: a box clips the descendants it contains), and to
//! the reachable side of the scroll container's scrollport (its
//! scrolling area starts at the scroll origin, CSSOM View §4). `fixed`
//! boxes are contained by the viewport and count nowhere. An absolutely
//! positioned `::before` / `::after` counts as an element does, its
//! containing block found from its host up.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use super::ClipEdges;
use super::positioning::{computed_position, parent_id};
use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Position};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

/// The rect the positioned boxes a scroll container contains cover:
/// cells from the container's border-box origin, with its scroll offset
/// added back (unscrolled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Reach {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}

impl Reach {
    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

/// Document data: each scroll container's [`Reach`], from the last
/// settle. Kept across layouts — phase 1 of the next one reads it.
#[derive(Debug, Default, PartialEq)]
struct Reaches(HashMap<NodeId, Reach>);

/// What phase 1 merges into `scroller`'s scrollable overflow: the reach
/// of the positioned boxes it contained at the last settle.
pub(super) fn reach_of(dom: &Dom<TuiExt>, scroller: NodeId) -> Option<Reach> {
    dom.document_data::<Reaches>()?.0.get(&scroller).copied()
}

/// The most runs of phases 1–2 one `layout_dom` makes: the first, one
/// with a changed reach, one with the reach that run's scrollbars moved.
pub(crate) const MAX_ROUNDS: usize = 3;

#[cfg(test)]
thread_local! {
    /// Positioned boxes [`settle`] measured (cost tests).
    pub(super) static MEASURED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Measure the reach of the boxes phase 2 `placed` (in document order)
/// in their scroll containers, store it, and return whether it differs
/// from what phase 1 read — the layout must then run again.
pub(super) fn settle(dom: &mut Dom<TuiExt>, placed: &[BoxItem]) -> bool {
    let mut reaches = Reaches::default();
    for &item in placed {
        let Some((scroller, clip)) = containing_scroller(dom, item) else {
            continue;
        };
        #[cfg(test)]
        MEASURED.with(|c| c.set(c.get() + 1));
        let Some(origin) = unscrolled_origin(dom, scroller) else {
            continue;
        };
        let clip = clip.narrow(reachable(dom, scroller));
        let mut reach: Option<Reach> = None;
        let mut extend = |r: LayoutRect| {
            let r = Reach {
                left: r.x - origin.0,
                top: r.y - origin.1,
                right: r.x + i32::from(r.width) - origin.0,
                bottom: r.y + i32::from(r.height) - origin.1,
            };
            reach = Some(reach.map_or(r, |a| a.union(r)));
        };
        match item {
            BoxItem::Node(id) => {
                super::scroll_extent::extend_box_overflow(dom, id, clip, &mut extend);
            }
            BoxItem::Generated(host, slot) => {
                let boxes = dom
                    .node(host)
                    .ext()
                    .map_or(&[][..], |e| e.positioned_pseudos());
                let laid_out = boxes
                    .iter()
                    .filter(|a| a.generated.is_some_and(|g| g.slot == slot));
                for a in laid_out {
                    if let Some(r) = clip.cut(a.border_box()) {
                        extend(r);
                    }
                }
            }
        }
        if let Some(r) = reach {
            reaches
                .0
                .entry(scroller)
                .and_modify(|a| *a = a.union(r))
                .or_insert(r);
        }
    }
    let changed = match dom.document_data::<Reaches>() {
        Some(old) => *old != reaches,
        None => !reaches.0.is_empty(),
    };
    if changed {
        dom.set_document_data(reaches);
    }
    changed
}

/// The scroll container whose scrollable overflow the absolutely
/// positioned `item` is part of, with the clip edges between: the nearest scroll
/// container at or above its containing block. `None` for a `fixed` box,
/// a box contained by the viewport, or one no scroll container holds.
fn containing_scroller(dom: &Dom<TuiExt>, item: BoxItem) -> Option<(NodeId, ClipEdges)> {
    // Its ancestors: an element's box parent and up, a pseudo-element's
    // host and up (CSS Pseudo 4 §4).
    let (position, from) = match item {
        BoxItem::Node(id) => (computed_position(dom, id), parent_id(dom, id)),
        BoxItem::Generated(host, slot) => {
            (dom.node(host).computed_pseudo(slot)?.position, Some(host))
        }
    };
    if position != Position::Absolute {
        return None;
    }
    // The containing block's ancestor (`positioning::containing_ancestor`).
    let mut cur = super::positioning::containing_ancestor(dom, from);
    let mut clip = ClipEdges::NONE;
    while let Some(p) = cur {
        let ext = dom.node(p).ext()?;
        let c = ext.computed.as_deref()?;
        if c.is_scroll_container() {
            return Some((p, clip));
        }
        clip = clip.narrow(ClipEdges::of_element(dom, p, ext, c));
        cur = parent_id(dom, p);
    }
    None
}

/// `scroller`'s border-box origin less its scroll offset: where a
/// [`Reach`] is measured from.
fn unscrolled_origin(dom: &Dom<TuiExt>, scroller: NodeId) -> Option<(i32, i32)> {
    let ext = dom.node(scroller).ext()?;
    Some((ext.layout.x - ext.scroll_x, ext.layout.y - ext.scroll_y))
}

/// The side of `scroller`'s scrolled content that scrolling can reach:
/// from its scrollport's start edge on, or up to its end edge where the
/// scroll origin is there (`origin_at_end`, CSSOM View §4) — what lies
/// before the origin is unreachable (CSS Overflow 3 §2.2;
/// `scrollport`).
fn reachable(dom: &Dom<TuiExt>, scroller: NodeId) -> ClipEdges {
    let (Some(ext), Some(port)) = (dom.node(scroller).ext(), super::scrollport(dom, scroller))
    else {
        return ClipEdges::NONE;
    };
    let (at_end_x, at_end_y) = super::origin_at_end(dom, scroller);
    let side = |at_end: bool, start: i32, len: u16, offset: i32| {
        let start = start - offset;
        Some(if at_end {
            (i32::MIN / 2, start + i32::from(len))
        } else {
            (start, i32::MAX / 2)
        })
    };
    ClipEdges {
        x: side(at_end_x, port.x, port.width, ext.scroll_x),
        y: side(at_end_y, port.y, port.height, ext.scroll_y),
    }
}

#[cfg(test)]
mod cost_tests;
