//! When the registry needs the clock (C12G-FRAME-COST, Web Animations 1
//! §4.2): a frame every frame while a transition runs or a CSS animation
//! moves continuously; a frame at a stepped animation's next step; no
//! frame for an animation with an empty effect, only a wake-up at its
//! next event, which [`AnimationRegistry::step_events`] services without
//! drawing. A paused or finished animation, or one on a scroll timeline,
//! holds its value without either.

use std::time::Instant;

use rdom_core::Dom;

use super::AnimationRegistry;
use crate::ext::TuiExt;

impl AnimationRegistry {
    /// When the next frame must step something with the clock: `now` while
    /// a transition runs or an animation moves continuously, a stepped
    /// animation's next step; `None` while nothing visible will change.
    pub(crate) fn next_frame(&self, now: Instant) -> Option<Instant> {
        if !self.active.is_empty() || !self.custom.is_empty() {
            return Some(now);
        }
        self.css.iter().filter_map(|a| a.next_change(now)).min()
    }

    /// Whether a frame is due at `now`: a transition runs, a CSS
    /// animation moves continuously, or a stepped one reached its next
    /// step. `false` between steps, and for a paused or finished
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
