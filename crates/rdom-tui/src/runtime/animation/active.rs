//! One running transition ([`ActiveAnimation`]): the longhand of one
//! element style it moves, its endpoints and timing, and its value at a
//! moment — progress through the delay and duration, eased, interpolated
//! by `rdom_style::animation`.

use std::rc::Rc;
use std::time::{Duration, Instant};

use rdom_core::NodeId;
use rdom_style::color::ColorScheme;

use super::Longhand;
use crate::ext::StyleSlot;
use crate::style::ComputedStyle;
use crate::style::transition::TimingFunction;

/// One running transition of one longhand of one element style. The
/// engine's own record — what a consumer inspects is
/// [`AnimationInfo`] (`App::get_animations`).
#[derive(Debug, Clone)]
pub(crate) struct ActiveAnimation {
    pub node: NodeId,
    /// The element itself or one of its pseudo-elements.
    pub slot: StyleSlot,
    pub property: Longhand,
    /// The start value: the before-change style's (CSS Transitions 1 §3),
    /// or a style holding a replaced transition's value at the moment.
    pub from: Rc<ComputedStyle>,
    /// The end value: the after-change style's.
    pub to: Rc<ComputedStyle>,
    /// When the delay began: registration, or earlier by the part a
    /// negative delay skips. The visual start is `started_at + delay`.
    pub started_at: Instant,
    /// The delay left to wait (zero for a negative `transition-delay`).
    pub delay: Duration,
    pub duration: Duration,
    /// The part of the duration a negative delay skipped (CSS Transitions
    /// 1 §2.4) — the `elapsedTime` of `transitionrun` / `transitionstart`.
    pub skipped: Duration,
    pub timing: TimingFunction,
    /// The element's used color scheme when the transition started: what
    /// a `reset` endpoint interpolates as (CSS Color Adjust 1 §2.1).
    pub scheme: ColorScheme,
    /// Whether `transitionstart` fired (on the first tick past the delay).
    pub started_dispatched: bool,
    /// When a frame last composited it (`None` before its first): a
    /// stepped transition's next change counts from there
    /// ([`next_change`](Self::next_change)).
    pub stepped_at: Option<Instant>,
}

impl ActiveAnimation {
    /// Linear progress in [0, 1]; 0 before the delay elapses.
    pub(super) fn progress(&self, now: Instant) -> f32 {
        if self.in_delay(now) {
            return 0.0;
        }
        let elapsed = now.saturating_duration_since(self.started_at);
        if self.duration.is_zero() {
            return 1.0;
        }
        ((elapsed - self.delay).as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Whether `now` is in the before phase — the delay.
    pub(super) fn in_delay(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) < self.delay
    }

    /// Its active time at `now` (CSS Transitions 1 §6.1, a
    /// `transitioncancel`'s `elapsedTime`): the time since its delay
    /// ended — a negative delay's skipped part included — within its
    /// duration.
    pub(super) fn active_time(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
            .saturating_sub(self.delay)
            .min(self.duration)
    }

    /// True once `now >= started_at + delay + duration`.
    pub(super) fn is_done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) >= self.delay + self.duration
    }

    /// The output of its timing function at `now`.
    pub(super) fn eased(&self, now: Instant) -> f32 {
        // CSS Easing 1 §2.3.1: the before flag holds a step back in the
        // delay.
        let p = self.progress(now);
        if self.in_delay(now) {
            self.timing.ease_before(p)
        } else {
            self.timing.ease(p)
        }
    }

    /// Write the eased current value into `out`.
    pub(super) fn apply(&self, now: Instant, out: &mut ComputedStyle) {
        let t = f64::from(self.eased(now));
        self.property
            .interpolate(&self.from, &self.to, t, self.scheme, out);
    }

    /// A style holding the current value — `to`'s, with this longhand at
    /// `now`: the start of a transition that replaces this one.
    pub(super) fn current_style(&self, now: Instant) -> Rc<ComputedStyle> {
        let mut out = (*self.to).clone();
        self.apply(now, &mut out);
        Rc::new(out)
    }
}
