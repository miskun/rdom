//! The scroll writers — the places a scroll offset is set.
//!
//! Every runtime scroll goes through one funnel, [`write`]: it stores the
//! offsets, notes the state write, says whether the scroll was a snap's
//! ([`WriteKind`]: any other scroll that moves the box leaves it unsnapped, so
//! a re-snap after layout does not undo it, CSS Scroll Snap 1 §5.4) and
//! fires `scroll` — at once, or queued for after the frame when the write
//! happens between layout and paint ([`queue_scroll_event`]).
//!
//! Scrollbar page / drag, drag-autoscroll and caret / node reveal call it
//! through [`set_scroll_with`] (one axis, clamped, aborting the box's
//! smooth scroll in flight — CSSOM View "perform a scroll", step 1); the
//! programmatic scroll API, the smooth-scroll steps, the wheel and the
//! re-snap through [`write_offsets`] (both axes, clamped).

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;

/// Whether a scroll write is a snap's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteKind {
    /// The offsets a snap chose (`scroll_snap::snap`), or a step of a
    /// smooth scroll to them: the snap record stands.
    Snap,
    /// Any other scroll: one that moves the box clears its snap record.
    Free,
}

/// When the `scroll` event of a write that moved the box fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fire {
    /// Now: the write is an input's, a script's or a timer's.
    Now,
    /// After the frame ([`queue_scroll_event`]): the write happens
    /// between layout and paint.
    Queued,
}

/// Set the scroll offset for `element` on `axis`, clamped to its
/// legal range ([`ScrollBounds`]). Returns the clamped value actually
/// written.
pub(super) fn set_scroll(
    dom: &mut TuiDom,
    element: NodeId,
    axis: ScrollAxis,
    value: i32,
    kind: WriteKind,
) -> i32 {
    set_scroll_with(dom, element, axis, value, ClampTo::CurrentExtent, kind)
}

/// How `set_scroll_with` bounds the requested offset.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ClampTo {
    /// The legal range against the extent the last layout recorded.
    /// Wheel, scrollbar drag, programmatic writes.
    CurrentExtent,
    /// Only the origin side — `[0, ∞)`, or `(-∞, 0]` for an `rtl`
    /// box's `scrollLeft` — the extent recorded by the last layout is
    /// stale (an edit just added a line) and the next layout's
    /// `clamp_scroll_offset` settles the true extreme. Caret reveal.
    NextLayout,
}

pub(super) fn set_scroll_with(
    dom: &mut TuiDom,
    element: NodeId,
    axis: ScrollAxis,
    value: i32,
    clamp: ClampTo,
    kind: WriteKind,
) -> i32 {
    crate::runtime::smooth_scroll::abort(dom, element);
    let Some(bounds) = scroll_bounds(dom, element) else {
        return 0;
    };
    let (min, max, origin_at_end) = match axis {
        ScrollAxis::Vertical => (bounds.min_y, bounds.max_y, bounds.origin_at_end.1),
        ScrollAxis::Horizontal => (bounds.min_x, bounds.max_x, bounds.origin_at_end.0),
    };
    let clamped = match clamp {
        ClampTo::CurrentExtent => value.clamp(min, max),
        // The origin is the bound at 0; the far one waits for layout.
        ClampTo::NextLayout if origin_at_end => value.min(0),
        ClampTo::NextLayout => value.max(0),
    };
    let Some((x, y)) = dom.node(element).ext().map(|e| (e.scroll_x, e.scroll_y)) else {
        return 0;
    };
    let to = match axis {
        ScrollAxis::Vertical => (x, clamped),
        ScrollAxis::Horizontal => (clamped, y),
    };
    write(dom, element, to, kind, Fire::Now);
    clamped
}

/// The legal scroll offsets of a scroll container: layout's one answer
/// (`layout_pass::scrollport`).
pub(crate) use crate::render::layout_pass::scroll_bounds;

/// Clamp `(x, y)` to [`scroll_bounds`] and write both offsets of
/// `element`, firing one `scroll` event when either moved. Does not
/// touch a smooth scroll in flight — the caller decides (the
/// programmatic API aborts it first, a smooth-scroll step is it).
/// Returns whether an offset moved.
pub(crate) fn write_offsets(
    dom: &mut TuiDom,
    element: NodeId,
    x: i32,
    y: i32,
    kind: WriteKind,
) -> bool {
    let Some(bounds) = scroll_bounds(dom, element) else {
        return false;
    };
    write(dom, element, bounds.clamp(x, y), kind, Fire::Now)
}

/// [`write_offsets`] for a snap made between layout and paint (the
/// re-snap after layout): its `scroll` event is queued for after the
/// frame ([`take_queued_scroll_events`]), as HTML's "run the scroll
/// steps" fires it at the next rendering update rather than in the
/// middle of this one.
pub(crate) fn write_offsets_queued(dom: &mut TuiDom, element: NodeId, x: i32, y: i32) -> bool {
    let Some(bounds) = scroll_bounds(dom, element) else {
        return false;
    };
    write(
        dom,
        element,
        bounds.clamp(x, y),
        WriteKind::Snap,
        Fire::Queued,
    )
}

/// The funnel: store `to`, and when it moved the box note the state
/// write, clear a [`WriteKind::Free`] write's snap record and fire `scroll`.
fn write(dom: &mut TuiDom, element: NodeId, to: (i32, i32), kind: WriteKind, fire: Fire) -> bool {
    let changed = match dom.node_mut(element).ext_mut() {
        Some(ext) => {
            let changed = (ext.scroll_x, ext.scroll_y) != to;
            (ext.scroll_x, ext.scroll_y) = to;
            if changed && kind == WriteKind::Free {
                super::state::set_snapped(ext, (None, None));
            }
            changed
        }
        None => false,
    };
    if !changed {
        return false;
    }
    crate::runtime::state_writes::note();
    match fire {
        Fire::Now => {
            // `scroll`: bubbles, NOT cancelable per HTML.
            let mut tui = crate::TuiEvent::new("scroll");
            tui.event.cancelable = false;
            // `element` held the offsets just written, so it is live.
            let live = crate::tui_event::dispatch_to_live(dom, element, &mut tui);
            debug_assert!(live, "a scroll container that just scrolled is a live node");
        }
        Fire::Queued => queue_scroll_event(dom, element),
    }
    true
}

/// Document data: the boxes whose `scroll` event waits for the end of
/// the frame, in order, each once.
#[derive(Debug, Default)]
struct QueuedScrollEvents(Vec<NodeId>);

/// Queue `element`'s `scroll` event for after the frame (HTML's "pending
/// scroll event targets": one per box per rendering update).
fn queue_scroll_event(dom: &mut TuiDom, element: NodeId) {
    if dom.document_data::<QueuedScrollEvents>().is_none() {
        dom.set_document_data(QueuedScrollEvents::default());
    }
    if let Some(q) = dom.document_data_mut::<QueuedScrollEvents>()
        && !q.0.contains(&element)
    {
        q.0.push(element);
    }
}

/// Take the queued `scroll` targets, in order, emptying the queue.
pub(crate) fn take_queued_scroll_events(dom: &mut TuiDom) -> Vec<NodeId> {
    dom.document_data_mut::<QueuedScrollEvents>()
        .map(|q| std::mem::take(&mut q.0))
        .unwrap_or_default()
}

/// Fire the queued `scroll` events at the boxes still in the document.
/// Returns whether any fired (their listeners are code the next frame's
/// checks must see).
pub(crate) fn fire_queued_scroll_events(dom: &mut TuiDom) -> bool {
    let queued = take_queued_scroll_events(dom);
    for &id in &queued {
        let mut tui = crate::TuiEvent::new("scroll");
        tui.event.cancelable = false;
        // A listener of an earlier one may have removed the box.
        crate::tui_event::dispatch_to_live(dom, id, &mut tui);
    }
    !queued.is_empty()
}
