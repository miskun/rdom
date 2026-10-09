//! When the registry needs the clock (C12G-FRAME-COST, Web Animations 1
//! §4.2): a frame every frame while a transition or a CSS animation moves
//! continuously; a frame at a stepped transition's or animation's next
//! step (C12G-CARRYOVER: CSS Easing 1 §2.3, a `steps()` value holds); no
//! frame for an animation with an empty effect, only a wake-up at its
//! next event, whose events [`AnimationRegistry::queue_due_events`]
//! queues without drawing. A paused or finished animation, or one on a scroll timeline,
//! holds its value without either.

use std::time::Instant;

use rdom_core::{Dom, NodeId};

use super::{ActiveAnimation, AnimationRegistry};
use crate::ext::{StyleSlot, TuiExt};
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
    ///
    /// An animation whose target is in skipped contents
    /// ([`Self::note_skipped`]) asks for none: it moves nothing on screen.
    pub(crate) fn next_frame(&self, now: Instant) -> Option<Instant> {
        let shown = |node: NodeId, slot: StyleSlot| !self.throttled.contains(&(node, slot));
        if self.custom.iter().any(|c| shown(c.node(), StyleSlot::Host)) {
            return Some(now);
        }
        let transitions = self
            .active
            .iter()
            .filter(|a| shown(a.node, a.slot))
            .map(|a| a.next_change(now));
        let css = self
            .css
            .iter()
            .filter(|a| shown(a.node, a.slot))
            .filter_map(|a| a.next_change(now));
        transitions.chain(css).min()
    }

    /// After a layout: note the running animations whose target is not
    /// rendered because it is in skipped contents (CSS Containment 2 §4 —
    /// `content-visibility`, a closed `<details>`), or is the `::before` /
    /// `::after` of an element skipping its contents. Their timelines run
    /// on — the engines neither restyle skipped contents nor cancel what
    /// runs there, they throttle it — but they ask for no frames
    /// ([`Self::next_frame`]) until their target renders again.
    /// O(animations × depth).
    pub(crate) fn note_skipped(&mut self, dom: &Dom<TuiExt>) {
        self.throttled.clear();
        if self.is_empty() {
            return;
        }
        let targets: Vec<(NodeId, StyleSlot)> = self
            .active
            .iter()
            .map(|a| (a.node, a.slot))
            .chain(self.css.iter().map(|a| (a.node, a.slot)))
            .chain(self.custom.iter().map(|c| (c.node(), StyleSlot::Host)))
            .collect();
        for (node, slot) in targets {
            if !dom.contains(node) || self.throttled.contains(&(node, slot)) {
                continue;
            }
            let contents = !matches!(slot, StyleSlot::Host)
                && crate::style::content_visibility::skips_contents(dom, node);
            if contents || !crate::node::is_rendered(dom, node) {
                self.throttled.insert((node, slot));
            }
        }
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

    /// HTML §8.1.7.3 "update the rendering", Web Animations 1 §4.4
    /// "update animations and send events": queue the events every
    /// transition and every CSS animation on the document timeline has
    /// reached at `now` — a delay's end, an iteration, an end — before the
    /// frame styles anything, so the App dispatches them first and the
    /// frame draws what their listeners change (ACID-FIX-15; they went out
    /// after its paint, a frame late). The values are composited by the
    /// frame's style pass (`advance_frame`), which finds these events
    /// queued already; an animation with an empty effect needs no frame
    /// at all. `true` when an event was queued.
    pub(crate) fn queue_due_events(&mut self, dom: &Dom<TuiExt>, now: Instant) -> bool {
        let queued = |r: &Self| r.pending_events.len() + r.custom_events.len() + r.css_events.len();
        let before = queued(self);
        for anim in &mut self.active {
            super::queue_transition_events(anim, now, &mut self.pending_events);
        }
        self.queue_custom_due_events(now);
        for anim in &mut self.css {
            if !anim.on_document_timeline() {
                continue;
            }
            if anim.has_effect() {
                anim.queue_events(dom, now, &mut self.css_events);
            } else {
                // No frame steps it: stepped here, its next wake-up counts
                // from now.
                anim.step(dom, now, &mut self.css_events);
            }
        }
        queued(self) > before
    }
}
