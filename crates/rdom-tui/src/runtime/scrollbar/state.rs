//! Scroll bookkeeping that only scroll containers use, kept off the
//! common `TuiExt` path (`P7G-FORM-STATE-BOX-1`): boxed on the first
//! write that differs from the at-rest default, so an element that never
//! scrolls pays one pointer.

use crate::TuiExt;
use crate::runtime::smooth_scroll::SmoothScroll;

/// The scroll state of one scroll container.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct ScrollState {
    /// `(scroll_x, scroll_y)` as the `App`'s last frame painted them
    /// (`runtime::scrollbar::painted`): a difference asks for a frame.
    pub(crate) painted: (i32, i32),
    /// `(scroll_x, scroll_y)` as the last layout placed this box's
    /// children with them: how far a later scroll has moved them since
    /// (`runtime::scrollbar::into_view`).
    pub(crate) laid_out: (i32, i32),
    /// The smooth scroll in flight (`runtime::smooth_scroll`), `None` at
    /// rest. Started by the programmatic scroll API and keyboard
    /// scrolling, stepped by the `App` each frame.
    pub(crate) smooth: Option<SmoothScroll>,
    /// The boxes the container last snapped to, horizontally and
    /// vertically (`runtime::scroll_snap`, CSS Scroll Snap 1 §5.4).
    pub(crate) snapped: (Option<rdom_core::NodeId>, Option<rdom_core::NodeId>),
    /// The scrollbar gutters layout reserved inside the padding box (CSS
    /// Overflow 3 §5.2) — the scrollport is the padding box less them
    /// (`layout_pass::scrollport`). Only a scroll container reserves any.
    pub(crate) gutters: crate::render::layout_pass::gutter::Gutters,
}

/// The offsets `ext` was last painted with; `(0, 0)` before any.
pub(crate) fn painted(ext: &TuiExt) -> (i32, i32) {
    ext.scroll_state.as_ref().map_or((0, 0), |s| s.painted)
}

/// The offsets `ext`'s children were last laid out with; `(0, 0)`
/// before any.
pub(crate) fn laid_out(ext: &TuiExt) -> (i32, i32) {
    ext.scroll_state.as_ref().map_or((0, 0), |s| s.laid_out)
}

/// The smooth scroll in flight on `ext`, if any.
pub(crate) fn smooth(ext: &TuiExt) -> Option<SmoothScroll> {
    ext.scroll_state.as_ref().and_then(|s| s.smooth)
}

/// The boxes `ext` last snapped to on each axis.
pub(crate) fn snapped(ext: &TuiExt) -> (Option<rdom_core::NodeId>, Option<rdom_core::NodeId>) {
    ext.scroll_state
        .as_ref()
        .map_or((None, None), |s| s.snapped)
}

/// The scrollbar gutters layout last reserved in `ext`; none before.
pub(crate) fn gutters(ext: &TuiExt) -> crate::render::layout_pass::gutter::Gutters {
    ext.scroll_state
        .as_ref()
        .map_or_else(Default::default, |s| s.gutters)
}

/// Record the gutters layout reserved. Allocates only for one.
pub(crate) fn set_gutters(ext: &mut TuiExt, gutters: crate::render::layout_pass::gutter::Gutters) {
    if let Some(s) = ext.scroll_state.as_mut() {
        s.gutters = gutters;
    } else if gutters != Default::default() {
        state_mut(ext).gutters = gutters;
    }
}

/// Record the boxes a snap came to rest on. Allocates only for one.
pub(crate) fn set_snapped(
    ext: &mut TuiExt,
    snapped: (Option<rdom_core::NodeId>, Option<rdom_core::NodeId>),
) {
    if let Some(s) = ext.scroll_state.as_mut() {
        s.snapped = snapped;
    } else if snapped != (None, None) {
        state_mut(ext).snapped = snapped;
    }
}

/// Record the current offsets as painted. Allocates only when they
/// are not the at-rest `(0, 0)` default.
pub(crate) fn note_painted(ext: &mut TuiExt) {
    let now = (ext.scroll_x, ext.scroll_y);
    if let Some(s) = ext.scroll_state.as_mut() {
        s.painted = now;
    } else if now != (0, 0) {
        state_mut(ext).painted = now;
    }
}

/// Record the current offsets as the ones the children were laid out
/// with. Allocates only when they are not `(0, 0)`.
pub(crate) fn note_laid_out(ext: &mut TuiExt) {
    let now = (ext.scroll_x, ext.scroll_y);
    if let Some(s) = ext.scroll_state.as_mut() {
        s.laid_out = now;
    } else if now != (0, 0) {
        state_mut(ext).laid_out = now;
    }
}

/// Start, replace or (with `None`) end the smooth scroll in flight.
pub(crate) fn set_smooth(ext: &mut TuiExt, smooth: Option<SmoothScroll>) {
    if smooth.is_none() && ext.scroll_state.is_none() {
        return;
    }
    crate::runtime::state_writes::note();
    state_mut(ext).smooth = smooth;
}

fn state_mut(ext: &mut TuiExt) -> &mut ScrollState {
    ext.scroll_state.get_or_insert_with(Box::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_rest_bookkeeping_allocates_nothing() {
        let mut ext = TuiExt::default();
        note_painted(&mut ext);
        note_laid_out(&mut ext);
        set_smooth(&mut ext, None);
        assert!(ext.scroll_state.is_none());
        assert_eq!(
            (painted(&ext), laid_out(&ext), smooth(&ext)),
            ((0, 0), (0, 0), None)
        );
    }

    #[test]
    fn a_scrolled_element_records_its_offsets() {
        let mut ext = TuiExt {
            scroll_y: 3,
            ..TuiExt::default()
        };
        note_painted(&mut ext);
        assert_eq!(painted(&ext), (0, 3));
        assert_eq!(laid_out(&ext), (0, 0), "not laid out yet");
        note_laid_out(&mut ext);
        assert_eq!(laid_out(&ext), (0, 3));
        ext.scroll_y = 0;
        note_painted(&mut ext);
        assert_eq!(painted(&ext), (0, 0), "back at rest is recorded too");
    }
}
