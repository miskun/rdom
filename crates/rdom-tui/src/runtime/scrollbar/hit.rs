//! Scrollbar hit testing — does `(x, y)` land on a scrollbar track or
//! thumb of some element on the hit-test path, and which part?
//!
//! Shares its geometry math with `render::paint_pass::scrollbar` so
//! click targets match what's rendered.

use rdom_core::NodeId;

use super::ScrollAxis;
use crate::TuiDom;
use crate::layout::Overflow;
use crate::node::TuiNodeExt;
use crate::render::paint_pass::scrollbar::{should_paint, thumb_geometry};

/// What part of a scrollbar got clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    let ext = dom.node(id).tui_ext()?;
    let content = ext.content_layout;
    let computed = dom.node(id).computed()?;
    // CSS Overflow 3 §3: scrollbar gutter is inside the padding-box.
    // Column position uses `content_layout` (gutter already accounted
    // for by `reserve_scrollbar_gutter`); track extent is clamped to
    // padding-box so a click on a border-row column doesn't register
    // as a scrollbar hit under M5.5b border-collapse.
    let padding_box = rdom_style::layout::compute_padding_box(ext.layout, computed.border);

    let y_reserves = matches!(computed.overflow_y, Overflow::Scroll | Overflow::Auto);
    let x_reserves = matches!(computed.overflow_x, Overflow::Scroll | Overflow::Auto);

    // Vertical scrollbar sits in the column just right of
    // content.x + content.width. Horizontal sits in the row just
    // below content.y + content.height.
    let v_col = content.x + content.width as i32;
    let h_row = content.y + content.height as i32;
    let in_v_col = x as i32 == v_col;
    let in_h_row = y as i32 == h_row;

    // Vertical track spans y in [content.y, content.y + height) clamped
    // to padding-box rows; minus one row for the corner if horizontal
    // also reserves.
    let v_top = content.y.max(padding_box.y);
    let mut v_bottom =
        (content.y + content.height as i32).min(padding_box.y + padding_box.height as i32);
    if x_reserves {
        v_bottom -= 1;
    }
    let in_v_rows = (y as i32) >= v_top && (y as i32) < v_bottom;

    let h_left = content.x.max(padding_box.x);
    let mut h_right =
        (content.x + content.width as i32).min(padding_box.x + padding_box.width as i32);
    if y_reserves {
        h_right -= 1;
    }
    let in_h_cols = (x as i32) >= h_left && (x as i32) < h_right;

    if y_reserves && in_v_col && in_v_rows {
        let track_len = (v_bottom - v_top) as u16;
        let viewport = content.height;
        let content_size = ext.scroll_content_height;
        if !should_paint(computed.overflow_y, viewport as usize, content_size) {
            return None;
        }
        let (thumb_size, thumb_off) =
            thumb_geometry(track_len, viewport as usize, content_size, ext.scroll_y);
        let cursor_along = (y as i32 - v_top) as u16;
        return Some(ScrollbarHit {
            element: id,
            axis: ScrollAxis::Vertical,
            part: classify(cursor_along, thumb_off, thumb_size),
            cursor_along_track: cursor_along,
        });
    }

    if x_reserves && in_h_row && in_h_cols {
        let track_len = (h_right - h_left) as u16;
        let viewport = content.width;
        let content_size = ext.scroll_content_width;
        if !should_paint(computed.overflow_x, viewport as usize, content_size) {
            return None;
        }
        let (thumb_size, thumb_off) =
            thumb_geometry(track_len, viewport as usize, content_size, ext.scroll_x);
        let cursor_along = (x as i32 - h_left) as u16;
        return Some(ScrollbarHit {
            element: id,
            axis: ScrollAxis::Horizontal,
            part: classify(cursor_along, thumb_off, thumb_size),
            cursor_along_track: cursor_along,
        });
    }

    None
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
