//! When a CSS animation on the document timeline next needs the clock
//! (C12G-FRAME-COST): a continuously moving one every frame; a stepped
//! one (every keyframe interval `steps()`, CSS Easing 1 §2.3) only at its
//! next step or iteration boundary, as the caret blink wakes only at its
//! flips; one with an empty effect — it animates only what rdom does not
//! render — never for frames, but at its next event (CSS Animations 2
//! §4.2: the start of its active interval, an iteration boundary, its
//! end), which the App steps without a frame.
//!
//! Each answer counts from the local time of the last step (`local`), so
//! a boundary passed since then is due at once.

use std::time::{Duration, Instant};

use super::CssAnimation;
use super::timing::Phase;
use crate::style::AnimationTimeline;

/// A boundary closer than this after the last step (ms) is that step's.
const EPSILON: f64 = 1e-6;

impl CssAnimation {
    /// Whether its keyframes animate something rdom renders.
    pub(in crate::runtime::animation) fn has_effect(&self) -> bool {
        !self.effect.is_empty()
    }

    /// When its composited value can next change, on the app's clock:
    /// `now` while it moves continuously (or was never stepped), the
    /// start of its active interval while in its delay, the next step or
    /// iteration boundary when it is stepped; `None` when it holds —
    /// paused, past its active interval, on a progress timeline (which
    /// scrolling steps), or with an empty effect.
    pub(in crate::runtime::animation) fn next_change(&self, now: Instant) -> Option<Instant> {
        if !self.on_document_timeline() || !self.has_effect() {
            return None;
        }
        let Some(last) = self.local else {
            return Some(now);
        };
        match self.timing.phase(last) {
            Phase::After => None,
            Phase::Before => Some(self.at(self.timing.delay.max(0.0))),
            Phase::Active => match self.effect.change_points() {
                None => Some(now),
                Some(points) => Some(self.at(self.next_boundary(last, &points))),
            },
        }
    }

    /// When an animation with an empty effect next fires an event (it
    /// asks for no frames): the start of its active interval, its next
    /// iteration boundary or its end; `None` for any other animation
    /// (its frames step it) or once it is past its end.
    pub(in crate::runtime::animation) fn next_event(&self, now: Instant) -> Option<Instant> {
        if !self.on_document_timeline() || self.has_effect() {
            return None;
        }
        let Some(last) = self.local else {
            return Some(now);
        };
        match self.timing.phase(last) {
            Phase::After => None,
            Phase::Before => Some(self.at(self.timing.delay.max(0.0))),
            Phase::Active => Some(self.at(self.next_boundary(last, &[0.0, 1.0]))),
        }
    }

    /// Playing on the document timeline.
    pub(in crate::runtime::animation) fn on_document_timeline(&self) -> bool {
        self.hold.is_none() && self.timeline == AnimationTimeline::Auto
    }

    /// The first local time after `last` (ms, in the active interval) at
    /// which the directed progress reaches one of `points` — in the
    /// current iteration, reversed in a backwards one — or the iteration
    /// ends; never past the end of the active interval.
    fn next_boundary(&self, last: f64, points: &[f64]) -> f64 {
        let timing = &self.timing;
        let end = timing.delay + timing.active_duration();
        if timing.duration <= 0.0 {
            return end;
        }
        let active = last - timing.delay;
        let iteration = (active / timing.duration).floor();
        let start = timing.delay + iteration * timing.duration;
        let forwards = timing.forwards(iteration);
        let next = points
            .iter()
            .map(|&p| start + timing.duration * if forwards { p } else { 1.0 - p })
            .filter(|&t| t > last + EPSILON)
            .fold(start + timing.duration, f64::min);
        next.min(end)
    }

    /// The instant local time `ms` falls at, rounded up to the
    /// nanosecond so a frame then samples it on or past the boundary.
    fn at(&self, ms: f64) -> Instant {
        let nanos = (ms.max(0.0) * 1e6).ceil();
        self.start + Duration::from_nanos(nanos as u64)
    }
}
