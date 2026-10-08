//! Smooth scrolling (`P7-SCROLL-BEHAVIOR-1`) — CSSOM View §4.1
//! "perform a scroll" and §12.1 `scroll-behavior`.
//!
//! A programmatic scroll (`scrollTo` / `scrollBy` / `scrollTop = n` /
//! `scrollIntoView`, [`TuiAccessorsMut`](crate::TuiAccessorsMut)) and
//! keyboard scrolling of a focused scroll container go through
//! `perform_scroll` with a [`ScrollBehaviorOption`]: `instant` jumps,
//! `smooth` animates, and `auto` — the default — follows the scroll
//! container's computed `scroll-behavior`. User wheel scrolling and
//! scrollbar track / thumb interaction are always instant, as in
//! browsers, and — like every instant scroll of the same box — abort a
//! smooth scroll in flight (§4.1 step 1).
//!
//! ## The animation
//!
//! A smooth scroll is recorded on the scroll container
//! (`ScrollState::smooth` in `TuiExt::scroll_state`: from, to, start) and stepped by the
//! [`App`](crate::runtime::App) once per frame on the scheduler clock
//! (virtual under `App::advance`, so tests are deterministic) by
//! `step_all`: it starts at the first frame after the request and
//! reaches its target [`SMOOTH_SCROLL_DURATION`] later, easing out
//! (cubic), at whole-cell positions. Each step that moves the offset
//! fires `scroll`, as a browser does once per animation frame. A new
//! request on the same box retargets it: the new animation starts from
//! the current (intermediate) position. While a smooth scroll is in
//! flight the App wakes once per animation frame; once every box has
//! settled it schedules nothing.
//!
//! Relative requests: `scrollBy` adds to the current position (CSSOM
//! View §4.2 `scrollBy` reads `scrollX` / `scrollY`); keyboard scrolling
//! steps from the destination of the smooth scroll in flight, so a held
//! PageDown pages on at the key-repeat rate instead of stalling on the
//! eased first cells — keyboard scrolling is UA-defined, and browsers
//! accumulate the same way.
//!
//! Not smooth: the caret reveal of a text control and the widget
//! cursor reveal (listbox / tree keyboard navigation) — both instant in
//! browsers. rdom has no viewport scrolling, so `scroll-behavior` on the
//! root element has nothing to apply to.

use crate::runtime::scrollbar::state;
use std::time::{Duration, Instant};

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::ScrollBehavior;
use crate::node::TuiNodeExt;
use crate::runtime::scrollbar::{WriteKind, scroll_bounds, write_offsets};

/// How long a smooth scroll takes, whatever its distance. Browsers use
/// a UA-defined duration (Firefox about 150–400 ms, Chromium a
/// distance-dependent curve in the same range); a fixed quarter second
/// reads as smooth at terminal frame rates without delaying a page
/// scroll noticeably.
pub const SMOOTH_SCROLL_DURATION: Duration = Duration::from_millis(250);

/// CSSOM View's `ScrollBehavior` IDL enum (`auto | instant | smooth`),
/// the `behavior` member of [`ScrollToOptions`] and
/// [`ScrollIntoViewOptions`]. Named apart from the `scroll-behavior`
/// property keyword ([`crate::layout::ScrollBehavior`],
/// `auto | smooth`) that `Auto` defers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ScrollBehaviorOption {
    /// Follow the scroll container's computed `scroll-behavior`.
    #[default]
    Auto,
    /// Jump to the target.
    Instant,
    /// Animate to the target.
    Smooth,
}

/// CSSOM View `ScrollToOptions`: the argument of
/// [`TuiAccessorsMut::scroll_with`](crate::TuiAccessorsMut::scroll_with)
/// (`element.scroll(options)` / `scrollTo(options)`) and
/// [`scroll_by_with`](crate::TuiAccessorsMut::scroll_by_with)
/// (`scrollBy(options)`). An absent `left` / `top` leaves that axis
/// where it is (for `scrollBy`, adds nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ScrollToOptions {
    pub left: Option<i32>,
    pub top: Option<i32>,
    pub behavior: ScrollBehaviorOption,
}

impl ScrollToOptions {
    /// `{}` — neither axis, behavior `auto`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `left`.
    pub fn left(mut self, left: i32) -> Self {
        self.left = Some(left);
        self
    }

    /// Set `top`.
    pub fn top(mut self, top: i32) -> Self {
        self.top = Some(top);
        self
    }

    /// Set `behavior`.
    pub fn behavior(mut self, behavior: ScrollBehaviorOption) -> Self {
        self.behavior = behavior;
        self
    }
}

/// CSSOM View's `ScrollLogicalPosition` IDL enum: where
/// [`ScrollIntoViewOptions`] aligns the element on one axis of each
/// scroll container (§5.1 "determine the scroll-into-view position").
/// rdom's writing mode is always horizontal-tb, left to right, so the
/// block axis is vertical and the inline axis horizontal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollLogicalPosition {
    /// Align the element's start edge with the scrollport's.
    Start,
    /// Center the element in the scrollport.
    Center,
    /// Align the element's end edge with the scrollport's.
    End,
    /// Scroll as little as brings the element into view: nothing when
    /// it is already fully visible (or covers the whole scrollport),
    /// else align the nearer edge — the start edge of an element
    /// larger than the scrollport.
    Nearest,
}

/// CSSOM View `ScrollIntoViewOptions`, the argument of
/// [`TuiAccessorsMut::scroll_into_view_with`](crate::TuiAccessorsMut::scroll_into_view_with):
/// `behavior`, and the alignment on the `block` (vertical) and
/// `inline` (horizontal) axes. [`new`](Self::new) is the options
/// dictionary's defaults, `{block: "start", inline: "nearest"}`;
/// `From<bool>` is the legacy `scrollIntoView(alignToTop)` form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScrollIntoViewOptions {
    pub behavior: ScrollBehaviorOption,
    pub block: ScrollLogicalPosition,
    pub inline: ScrollLogicalPosition,
}

impl Default for ScrollIntoViewOptions {
    fn default() -> Self {
        Self {
            behavior: ScrollBehaviorOption::Auto,
            block: ScrollLogicalPosition::Start,
            inline: ScrollLogicalPosition::Nearest,
        }
    }
}

impl ScrollIntoViewOptions {
    /// `{}` — behavior `auto`, block `start`, inline `nearest`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `behavior`.
    pub fn behavior(mut self, behavior: ScrollBehaviorOption) -> Self {
        self.behavior = behavior;
        self
    }

    /// Set the `block` (vertical) alignment.
    pub fn block(mut self, block: ScrollLogicalPosition) -> Self {
        self.block = block;
        self
    }

    /// Set the `inline` (horizontal) alignment.
    pub fn inline(mut self, inline: ScrollLogicalPosition) -> Self {
        self.inline = inline;
        self
    }
}

/// The legacy `scrollIntoView(alignToTop)` argument (CSSOM View §5.2
/// step 3): `true` is `{block: "start", inline: "nearest"}`, `false`
/// is `{block: "end", inline: "nearest"}`, both with behavior `auto`.
impl From<bool> for ScrollIntoViewOptions {
    fn from(align_to_top: bool) -> Self {
        let block = if align_to_top {
            ScrollLogicalPosition::Start
        } else {
            ScrollLogicalPosition::End
        };
        Self::new().block(block)
    }
}

/// A smooth scroll in flight on one scroll container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SmoothScroll {
    /// `(scroll_left, scroll_top)` when the animation (re)started.
    from: (i32, i32),
    /// The destination, clamped when requested.
    to: (i32, i32),
    /// The frame the animation started on; `None` until the first
    /// frame after the request.
    start: Option<Instant>,
}

/// CSSOM View §4.1 "perform a scroll" of `element` to `(x, y)`: abort
/// the smooth scroll in flight, then jump or start a smooth scroll per
/// `behavior`. The target is clamped to the scroll range the last
/// layout recorded and, in a snap container, snapped for `motion`
/// (`runtime::scroll_snap`).
pub(crate) fn perform_scroll(
    dom: &mut TuiDom,
    element: NodeId,
    x: i32,
    y: i32,
    behavior: ScrollBehaviorOption,
    motion: crate::runtime::scroll_snap::Motion,
) {
    abort(dom, element);
    let Some(bounds) = scroll_bounds(dom, element) else {
        return;
    };
    // A snap container comes to rest at a snap position (CSS Scroll Snap
    // 1 §6.2): an instant scroll goes there, a smooth one animates there.
    let to = bounds.clamp(x, y);
    let to = crate::runtime::scroll_snap::snap(dom, element, to, motion);
    if !is_smooth(dom, element, behavior) {
        // The destination a snap chose (or the plain one): a snap's write.
        write_offsets(dom, element, to.0, to.1, WriteKind::Snap);
        return;
    }
    let mut node = dom.node_mut(element);
    let Some(ext) = node.ext_mut() else {
        return;
    };
    let from = (ext.scroll_x, ext.scroll_y);
    if from != to {
        state::set_smooth(
            ext,
            Some(SmoothScroll {
                from,
                to,
                start: None,
            }),
        );
    }
}

/// Point the smooth scroll in flight on `element` at `to` (a re-snap
/// after layout moved its snap position, CSS Scroll Snap 1 §5.4),
/// clamped to the scroll range. Returns `false` when none is in flight.
pub(crate) fn retarget(dom: &mut TuiDom, element: NodeId, to: (i32, i32)) -> bool {
    let to = match scroll_bounds(dom, element) {
        Some(b) => b.clamp(to.0, to.1),
        None => to,
    };
    let mut node = dom.node_mut(element);
    let Some(ext) = node.ext_mut() else {
        return false;
    };
    let Some(anim) = state::smooth(ext) else {
        return false;
    };
    state::set_smooth(ext, Some(SmoothScroll { to, ..anim }));
    true
}

/// Abort the smooth scroll in flight on `element`, if any.
pub(crate) fn abort(dom: &mut TuiDom, element: NodeId) {
    if let Some(ext) = dom.node_mut(element).ext_mut() {
        state::set_smooth(ext, None);
    }
}

/// Where `element` is scrolling to: the smooth scroll's destination
/// while one is in flight, else its current offsets.
pub(crate) fn destination(dom: &TuiDom, element: NodeId) -> (i32, i32) {
    match dom.node(element).tui_ext() {
        Some(ext) => state::smooth(ext).map_or((ext.scroll_x, ext.scroll_y), |s| s.to),
        None => (0, 0),
    }
}

/// Whether a scroll of `element` with `behavior` animates: CSSOM View
/// §4.1 — `smooth`, or `auto` on a box whose `scroll-behavior` is
/// `smooth`.
fn is_smooth(dom: &TuiDom, element: NodeId, behavior: ScrollBehaviorOption) -> bool {
    match behavior {
        ScrollBehaviorOption::Instant => false,
        ScrollBehaviorOption::Smooth => true,
        ScrollBehaviorOption::Auto => dom
            .node(element)
            .computed()
            .is_some_and(|c| c.scroll_behavior == ScrollBehavior::Smooth),
    }
}

/// What a frame's [`step_all`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct StepOutcome {
    /// Some scroll offset moved: the frame must be laid out and painted.
    pub moved: bool,
    /// Some smooth scroll is still in flight: step again next frame.
    pub active: bool,
}

/// Advance every smooth scroll in the tree to `now`.
pub(crate) fn step_all(dom: &mut TuiDom, now: Instant) -> StepOutcome {
    let mut outcome = StepOutcome::default();
    for element in in_flight(dom) {
        step(dom, element, now, &mut outcome);
    }
    outcome
}

/// The connected elements with a smooth scroll in flight, in tree order.
fn in_flight(dom: &TuiDom) -> Vec<NodeId> {
    let mut found = Vec::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        if node.tui_ext().is_some_and(|e| state::smooth(e).is_some()) {
            found.push(id);
        }
        let first = stack.len();
        stack.extend(crate::render::box_tree::children(dom, id));
        stack[first..].reverse();
    }
    found
}

fn step(dom: &mut TuiDom, element: NodeId, now: Instant, outcome: &mut StepOutcome) {
    let (x, y) = {
        let mut node = dom.node_mut(element);
        let Some(ext) = node.ext_mut() else {
            return;
        };
        let Some(mut anim) = state::smooth(ext) else {
            return;
        };
        let start = *anim.start.get_or_insert(now);
        let t = now.saturating_duration_since(start).as_secs_f64()
            / SMOOTH_SCROLL_DURATION.as_secs_f64();
        if t >= 1.0 {
            state::set_smooth(ext, None);
            anim.to
        } else {
            state::set_smooth(ext, Some(anim));
            outcome.active = true;
            let p = ease_out(t);
            (
                lerp(anim.from.0, anim.to.0, p),
                lerp(anim.from.1, anim.to.1, p),
            )
        }
    };
    // The state is settled before the write: a `scroll` listener that
    // starts another scroll of this box retargets or aborts it.
    // A step towards the destination `perform_scroll` snapped: it keeps
    // the snap record.
    outcome.moved |= write_offsets(dom, element, x, y, WriteKind::Snap);
}

/// Cubic ease-out: fast start, gentle landing.
fn ease_out(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

/// `from → to` at progress `p`, rounded to a whole cell.
fn lerp(from: i32, to: i32, p: f64) -> i32 {
    (f64::from(from) + (f64::from(to) - f64::from(from)) * p).round() as i32
}

#[cfg(test)]
mod tests;
