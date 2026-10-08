//! When the registry needs the clock (C12G-FRAME-COST, Web Animations 1
//! §4.2): a frame every frame while a transition or a CSS animation moves
//! continuously; a frame at a stepped transition's or animation's next
//! step (C12G-CARRYOVER: CSS Easing 1 §2.3, a `steps()` value holds); no
//! frame for an animation with an empty effect, only a wake-up at its
//! next event, which [`AnimationRegistry::step_events`] services without
//! drawing. A paused or finished animation, or one on a scroll timeline,
//! holds its value without either.

use std::time::Instant;

use rdom_core::Dom;

use super::{ActiveAnimation, AnimationRegistry};
use crate::ext::TuiExt;
use crate::style::transition::TimingFunction;

/// A boundary closer than this (in steps) after the last frame is that
/// frame's.
const EPSILON: f64 = 1e-6;

impl ActiveAnimation {
    /// When its value can next change: `now` for a continuous easing (or
    /// before its first frame); for `steps(n, …)` (CSS Easing 1 §2.3) the
    /// end of its delay while in it — `transitionstart`, and a
    /// `jump-start`'s first step — then the next of its `n` equal
    /// divisions after the last frame, the last being its end
    /// (`transitionend`). A boundary passed since the last frame is due at
    /// once.
    pub(super) fn next_change(&self, now: Instant) -> Instant {
        let TimingFunction::Steps { count, .. } = self.timing else {
            return now;
        };
        let Some(last) = self.stepped_at else {
            return now;
        };
        let start = self.started_at + self.delay;
        if last < start {
            return start;
        }
        let n = f64::from(count.max(1));
        let step = (f64::from(self.progress(last)) * n + EPSILON).floor() + 1.0;
        start + self.duration.mul_f64((step / n).min(1.0))
    }
}

impl AnimationRegistry {
    /// When the next frame must step something with the clock: `now` while
    /// a transition or an animation moves continuously, a stepped one's
    /// next step; `None` while nothing visible will change.
    pub(crate) fn next_frame(&self, now: Instant) -> Option<Instant> {
        if !self.custom.is_empty() {
            return Some(now);
        }
        let transitions = self.active.iter().map(|a| a.next_change(now));
        let css = self.css.iter().filter_map(|a| a.next_change(now));
        transitions.chain(css).min()
    }

    /// Whether a frame is due at `now`: a transition or a CSS animation
    /// moves continuously, or a stepped one reached its next step. `false` between steps, and for a paused or finished
    /// animation, one on a scroll timeline or one of nothing rdom renders.
    pub fn needs_frames(&self, now: Instant) -> bool {
        self.next_frame(now).is_some_and(|t| t <= now)
    }

    /// When the App must wake: the next frame, or the next event of an
    /// animation that asks for no frames.
    pub(crate) fn next_wake(&self, now: Instant) -> Option<Instant> {
        let events = self.css.iter().filter_map(|a| a.next_event(now)).min();
        match (self.next_frame(now), events) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Step the animations with an empty effect whose next event is due,
    /// queueing their events (CSS Animations 2 §4.2) — no composite, no
    /// frame. `true` when one was stepped.
    pub(crate) fn step_events(&mut self, dom: &Dom<TuiExt>, now: Instant) -> bool {
        let mut stepped = false;
        for anim in &mut self.css {
            if anim.next_event(now).is_some_and(|t| t <= now) {
                anim.step(dom, now, &mut self.css_events);
                stepped = true;
            }
        }
        stepped
    }
}
