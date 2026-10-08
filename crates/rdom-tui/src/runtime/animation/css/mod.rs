//! CSS animations (CSS Animations 1 / 2) in the transition engine's
//! effect stack. An element style's `animation-name` list names the
//! animations it runs; each one is a [`CssAnimation`] — its timing
//! (`timing`, Web Animations 1 §4), its keyframes (`effect`) and its
//! place in time — kept in the [`AnimationRegistry`] beside the
//! transitions. Each frame the registry composites them onto the
//! element's style above its transitions (Web Animations 1 §5.4.5: CSS
//! animations sort above CSS transitions), so layout, paint and
//! inheritance read the running values (`super::composite`).
//!
//! The cascade hook ([`AnimationRegistry::update_css`], from
//! `super::diff`) matches an element's new list to its running
//! animations by name (§4.1): a name no longer listed is cancelled, a new
//! one starts, a kept one takes the new values of the other longhands in
//! place — its start time unchanged. A style that changed rebuilds the
//! keyframe values (they are computed against it); a hidden element
//! (`display: none`, itself or above) runs none.
//!
//! `prefers-reduced-motion` (CSS Media Queries 5) needs no hook here: it
//! is a media feature, so once `@media` is evaluated (C14-MEDIA) a
//! reduced-motion sheet turns animations off through the cascade like any
//! declaration. Until then every animation runs (DIVERGENCES §3).

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId};
use rdom_style::color::ColorScheme;

use crate::ext::{StyleSlot, TuiExt};
use crate::style::cascade::PropertyRegistry;
use crate::style::transition::TimingFunction;
use crate::style::{
    AnimationComposition, AnimationDuration, AnimationPlayState, AnimationTimeline, ComputedStyle,
    KeyframesRule, RangeBoundary, Stylesheet,
};

mod effect;
mod events;
mod info;
mod schedule;
mod timeline;
mod timing;
mod update;

pub(crate) use effect::KeyframeEffect;
pub use events::AnimationEventKind;
pub(crate) use events::PendingAnimationEvent;
pub(crate) use info::slot_order;
pub use info::{AnimationInfo, AnimationKind};
use timing::{Phase, Timing};

/// What the cascade hook reads to run CSS animations: the sheets of the
/// cascade (their `@keyframes` rules) and their registrations — the
/// inputs of a keyframe's style.
#[derive(Clone, Copy)]
pub(crate) struct CssInputs<'a> {
    pub sheets: &'a [&'a Stylesheet],
    pub registry: &'a Rc<PropertyRegistry>,
}

impl CssInputs<'_> {
    /// The identity of the sheet set (`Sheets::stamp`): it changes exactly
    /// when the sheets do.
    pub(crate) fn stamp(&self) -> usize {
        Rc::as_ptr(self.registry) as usize
    }
}

/// One running CSS animation of one element style.
#[derive(Debug, Clone)]
pub(crate) struct CssAnimation {
    pub(super) node: NodeId,
    pub(super) slot: StyleSlot,
    /// Its `animation-name` entry.
    pub(super) name: Arc<str>,
    /// Its place in `animation-name`: the composite order among the
    /// element's animations (CSS Animations 2 §3.1).
    pub(super) index: usize,
    timing: Timing,
    /// `animation-timing-function`: the easing of the keyframes that set
    /// none.
    easing: TimingFunction,
    composition: AnimationComposition,
    play_state: AnimationPlayState,
    timeline: AnimationTimeline,
    /// `animation-duration: auto`: on a progress timeline the iterations
    /// fill the attachment range (CSS Animations 2 §3.3).
    duration_auto: bool,
    /// `animation-range-start` / `-end`: where on a progress timeline it
    /// is attached (Scroll-driven Animations 1 §4.3).
    range: (RangeBoundary, RangeBoundary),
    /// When its local time was 0, on the app's clock (running).
    start: Instant,
    /// Its local time while paused, in ms.
    hold: Option<f64>,
    effect: Rc<KeyframeEffect>,
    /// The rule its effect was built from — a changed sheet rebuilds it.
    rule: KeyframesRule,
    /// The element's used color scheme (a `reset` endpoint's canvas).
    scheme: ColorScheme,
    /// The phase and current iteration the last frame saw; `None` while
    /// idle (CSS Animations 2 §4.2's starting point).
    seen: Option<(Phase, Option<f64>)>,
    /// The progress last composited, and whether the next frame must
    /// composite anyway (it started, changed or was rebuilt).
    composited: Option<f64>,
    dirty: bool,
    /// The last frame's local time (ms) and, on a progress timeline,
    /// where the scroll offset stood in its attachment range, and the
    /// timeline itself (it places keyframes on its named ranges).
    local: Option<f64>,
    fraction: Option<f64>,
    progress_timeline: Option<timeline::ProgressTimeline>,
}

/// The values of an element style's `animation-*` lists at entry `i`
/// (§4.1: a shorter list repeats).
struct Entry {
    timing: Timing,
    easing: TimingFunction,
    composition: AnimationComposition,
    play_state: AnimationPlayState,
    timeline: AnimationTimeline,
    duration_auto: bool,
    range: (RangeBoundary, RangeBoundary),
}

impl Entry {
    fn of(style: &ComputedStyle, i: usize) -> Entry {
        fn at<T: Clone + Default>(list: &[T], i: usize) -> T {
            if list.is_empty() {
                T::default()
            } else {
                list[i % list.len()].clone()
            }
        }
        let duration = at(&style.animation_duration, i);
        let easing = if style.animation_timing_function.is_empty() {
            TimingFunction::Ease
        } else {
            style.animation_timing_function[i % style.animation_timing_function.len()].clone()
        };
        let delay = if style.animation_delay.is_empty() {
            0
        } else {
            style.animation_delay[i % style.animation_delay.len()]
        };
        Entry {
            timing: Timing {
                duration: f64::from(duration.ms()),
                delay: f64::from(delay),
                iterations: at(&style.animation_iteration_count, i).get(),
                direction: at(&style.animation_direction, i),
                fill: at(&style.animation_fill_mode, i),
            },
            easing,
            composition: at(&style.animation_composition, i),
            play_state: at(&style.animation_play_state, i),
            timeline: at(&style.animation_timeline, i),
            duration_auto: duration == AnimationDuration::Auto,
            range: (
                at(&style.animation_range_start, i),
                at(&style.animation_range_end, i),
            ),
        }
    }
}

impl CssAnimation {
    /// Its time at `now`: the local time (ms) on its timeline, the timing
    /// it runs under there, and whether the timeline is progress-based —
    /// `None` while its timeline is inactive (it is idle). On the
    /// document timeline that is the clock since its start; on a scroll or
    /// view timeline (Scroll-driven Animations 1) the scroll offset's place
    /// in its attachment range, mapped onto its timing scaled to fill the
    /// range (`duration: auto`: the iterations share it; a time: delay
    /// and iterations keep their proportions; `infinite` counts once).
    /// A paused animation holds its local time.
    fn time(&mut self, dom: &Dom<TuiExt>, now: Instant) -> Option<(f64, Timing, bool)> {
        let resolved = timeline::resolve_timeline(dom, self.node, &self.timeline);
        let (timing, span) = match resolved {
            timeline::Resolved::Inactive => {
                self.fraction = None;
                self.progress_timeline = None;
                return None;
            }
            timeline::Resolved::Document => {
                self.fraction = None;
                self.progress_timeline = None;
                let t = self.hold.unwrap_or_else(|| {
                    now.saturating_duration_since(self.start).as_secs_f64() * 1000.0
                });
                return Some((t, self.timing, false));
            }
            timeline::Resolved::Progress(tl) => {
                self.fraction = Some(tl.fraction(&self.range.0, &self.range.1));
                self.progress_timeline = Some(tl);
                self.progress_timing()
            }
        };
        let t = self
            .hold
            .unwrap_or_else(|| self.fraction.unwrap_or(0.0) * span);
        Some((t, timing, true))
    }

    /// Its local time on the document timeline at `now` (ms); `None` on
    /// any other.
    fn clock_time(&self, now: Instant) -> Option<f64> {
        (self.timeline == AnimationTimeline::Auto).then(|| {
            self.hold
                .unwrap_or_else(|| now.saturating_duration_since(self.start).as_secs_f64() * 1000.0)
        })
    }

    /// Its timing on a progress timeline and the local time that fills
    /// the attachment range.
    fn progress_timing(&self) -> (Timing, f64) {
        let iterations = if self.timing.iterations.is_finite() {
            self.timing.iterations
        } else {
            1.0
        };
        if self.duration_auto {
            let duration = if iterations > 0.0 {
                1000.0 / iterations
            } else {
                0.0
            };
            let timing = Timing {
                duration,
                delay: 0.0,
                iterations,
                ..self.timing
            };
            (timing, 1000.0)
        } else {
            let timing = Timing {
                iterations,
                ..self.timing
            };
            (
                timing,
                self.timing.delay.max(0.0) + timing.active_duration(),
            )
        }
    }

    /// Write its keyframe values, at the progress its last step reached,
    /// into `out` over `out`'s own (the underlying values: the cascade's
    /// and the transitions'). The longhands it wrote, empty when it has
    /// no effect.
    pub(super) fn apply(&self, out: &mut ComputedStyle) -> Vec<rdom_style::animation::Longhand> {
        let Some(progress) = self.composited else {
            return Vec::new();
        };
        let underlying = out.clone();
        let place = |name, fraction| {
            self.progress_timeline
                .as_ref()?
                .place(name, fraction, (&self.range.0, &self.range.1))
        };
        self.effect
            .apply(progress, self.scheme, &underlying, out, Some(&place));
        self.effect.longhands().collect()
    }

    /// Advance it to `now`, queueing the events its phase change calls
    /// for (CSS Animations 2 §4.2); `true` when its composited value moved
    /// (or it was changed) and its target must be composited again.
    pub(in crate::runtime::animation) fn step(
        &mut self,
        dom: &Dom<TuiExt>,
        now: Instant,
        events: &mut Vec<PendingAnimationEvent>,
    ) -> bool {
        let sample = self
            .time(dom, now)
            .map(|(t, timing, progress_based)| (t, timing, timing.sample(t, progress_based)));
        let state = sample.map(|(_, _, s)| (s.phase, s.iteration));
        let timing = sample.map_or(self.timing, |(_, t, _)| t);
        for (kind, elapsed) in events::transitions(self.seen, state, &timing) {
            events.push(self.event(kind, elapsed, now));
        }
        self.seen = state;
        self.local = sample.map(|(t, ..)| t);
        let progress = sample.and_then(|(_, _, s)| s.progress);
        let moved = self.dirty || progress != self.composited;
        self.composited = progress;
        self.dirty = false;
        moved
    }

    /// An event of this animation with `elapsed` ms, scheduled at the
    /// time on the app's clock it happened: its start plus the local time
    /// the elapsed time stands for (a paused or cancelled one: now).
    fn event(&self, kind: AnimationEventKind, elapsed: f64, now: Instant) -> PendingAnimationEvent {
        let local = self.timing.delay + elapsed;
        let scheduled = if self.hold.is_some()
            || kind == AnimationEventKind::Cancel
            || self.timeline != AnimationTimeline::Auto
        {
            now
        } else {
            self.start + Duration::from_secs_f64(local.max(0.0) / 1000.0)
        };
        PendingAnimationEvent {
            node: self.node,
            slot: self.slot,
            kind,
            name: self.name.clone(),
            elapsed_seconds: elapsed / 1000.0,
            scheduled: scheduled.min(now),
            index: self.index,
        }
    }

    /// The `animationcancel` of an animation dropped at `now`, if it was
    /// in its before or active phase (§4.2: "not idle and not after").
    fn cancel_event(&self, now: Instant) -> Option<PendingAnimationEvent> {
        let (phase, _) = self.seen?;
        if phase == Phase::After {
            return None;
        }
        let t = if self.timeline == AnimationTimeline::Auto && self.hold.is_none() {
            now.saturating_duration_since(self.start).as_secs_f64() * 1000.0
        } else {
            self.local?
        };
        let elapsed = self.timing.clamped_active_time(t);
        Some(self.event(AnimationEventKind::Cancel, elapsed, now))
    }
}
