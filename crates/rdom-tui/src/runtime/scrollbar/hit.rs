//! Scrollbar hit testing — does `(x, y)` land on a scrollbar track or
//! thumb of some element on the hit-test path, and which part?
//!
//! Shares its geometry math with `render::paint_pass::scrollbar` so
//! click targets match what's rendered.

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;
use crate::render::paint_pass::scrollbar::{Track, tracks};

/// What part of a scrollbar got clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollbarPart {
    /// Mouse on the track above / left of the thumb — page back.
    TrackBefore,
    /// Mouse on the thumb — start a drag.
    Thumb,
    /// Mouse on the track below / right of the thumb — page forward.
    TrackAfter,
}

/// Result of a scrollbar hit test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScrollbarHit {
    pub element: NodeId,
    pub axis: ScrollAxis,
    pub part: ScrollbarPart,
    /// Cursor offset along the scrollbar track in cells (from track
    /// start). Used by `begin_drag` to compute the thumb-relative
    /// anchor so dragging doesn't snap the thumb.
    pub cursor_along_track: u16,
}

/// Check whether `(x, y)` lands on a scrollbar rendered for any
/// ancestor starting at `path_inner` (the hit-test path, innermost-
/// first). Returns the first match walking outward — which is the
/// same element the user perceives as the scrollbar owner.
pub(crate) fn hit(dom: &TuiDom, path: &[NodeId], x: u16, y: u16) -> Option<ScrollbarHit> {
    // Walk the path outward — scrollbars belong to the scrollable
    // container, and its content_layout's gutter is OUTSIDE content
    // but INSIDE its outer rect, so the container is the last path
    // element (or one of its ancestors) containing the point.
    for &id in path.iter().rev() {
        if let Some(h) = check_element(dom, id, x, y) {
            return Some(h);
        }
    }
    None
}

fn check_element(dom: &TuiDom, id: NodeId, x: u16, y: u16) -> Option<ScrollbarHit> {
    // The bars paint draws (`paint_pass::scrollbar::tracks`): one track
    // per shown bar, in its gutter beside the scrollport.
    let (vertical, horizontal) = tracks(dom, id);
    let (x, y) = (i32::from(x), i32::from(y));
    let on = |track: Track, line: i32, along: i32| {
        (line == track.line && along >= track.start && along < track.start + i32::from(track.len))
            .then(|| (along - track.start) as u16)
    };
    let (axis, track, cursor_along) = if let Some(t) = vertical
        && let Some(c) = on(t, x, y)
    {
        (ScrollAxis::Vertical, t, c)
    } else if let Some(t) = horizontal
        && let Some(c) = on(t, y, x)
    {
        (ScrollAxis::Horizontal, t, c)
    } else {
        return None;
    };
    let (thumb_size, thumb_off) = track.thumb();
    Some(ScrollbarHit {
        element: id,
        axis,
        part: classify(cursor_along, thumb_off, thumb_size),
        cursor_along_track: cursor_along,
    })
}

fn classify(cursor: u16, thumb_off: u16, thumb_size: u16) -> ScrollbarPart {
    if cursor < thumb_off {
        ScrollbarPart::TrackBefore
    } else if cursor < thumb_off + thumb_size {
        ScrollbarPart::Thumb
    } else {
        ScrollbarPart::TrackAfter
    }
}
