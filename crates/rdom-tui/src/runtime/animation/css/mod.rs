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
    AnimationComposition, AnimationPlayState, AnimationTimeline, ComputedStyle, KeyframesRule,
    Stylesheet,
};

mod effect;
mod events;
mod info;
mod timing;

pub(crate) use effect::KeyframeEffect;
pub use events::AnimationEventKind;
pub(crate) use events::PendingAnimationEvent;
pub(crate) use info::slot_order;
pub use info::{AnimationInfo, AnimationKind};
use timing::{Phase, Timing};

use super::AnimationRegistry;

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
}

/// The values of an element style's `animation-*` lists at entry `i`
/// (§4.1: a shorter list repeats).
struct Entry {
    timing: Timing,
    easing: TimingFunction,
    composition: AnimationComposition,
    play_state: AnimationPlayState,
    timeline: AnimationTimeline,
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
        let duration = if style.animation_duration.is_empty() {
            0
        } else {
            style.animation_duration[i % style.animation_duration.len()].ms()
        };
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
                duration: f64::from(duration),
                delay: f64::from(delay),
                iterations: at(&style.animation_iteration_count, i).get(),
                direction: at(&style.animation_direction, i),
                fill: at(&style.animation_fill_mode, i),
            },
            easing,
            composition: at(&style.animation_composition, i),
            play_state: at(&style.animation_play_state, i),
            timeline: at(&style.animation_timeline, i),
        }
    }
}

impl CssAnimation {
    /// Its local time at `now` (ms); `None` while its timeline is
    /// inactive (`animation-timeline: none`) — it is idle.
    fn local_time(&self, now: Instant) -> Option<f64> {
        if self.timeline == AnimationTimeline::None {
            return None;
        }
        Some(
            self.hold.unwrap_or_else(|| {
                now.saturating_duration_since(self.start).as_secs_f64() * 1000.0
            }),
        )
    }

    /// Whether it changes with the clock: running on the document
    /// timeline, not yet past its active interval.
    pub(super) fn needs_frames(&self, now: Instant) -> bool {
        self.hold.is_none()
            && self
                .local_time(now)
                .is_some_and(|t| self.timing.phase(t) != Phase::After)
    }

    /// The progress its keyframes are sampled at, at `now`; `None` when
    /// it has no effect.
    fn progress(&self, now: Instant) -> Option<f64> {
        self.timing.sample(self.local_time(now)?).progress
    }

    /// Write its keyframe values at `now` into `out`, over `out`'s own
    /// (the underlying values: the cascade's and the transitions'). The
    /// longhands it wrote, empty when it has no effect.
    pub(super) fn apply(
        &self,
        now: Instant,
        out: &mut ComputedStyle,
    ) -> Vec<rdom_style::animation::Longhand> {
        let Some(progress) = self.progress(now) else {
            return Vec::new();
        };
        let underlying = out.clone();
        self.effect.apply(progress, self.scheme, &underlying, out);
        self.effect.longhands().collect()
    }

    /// Advance its event state to `now`, queueing the events the phase
    /// change calls for (CSS Animations 2 §4.2); `true` when its
    /// composited value moved (or it was changed) and its target must be
    /// composited again.
    fn step(&mut self, now: Instant, events: &mut Vec<PendingAnimationEvent>) -> bool {
        let sample = self.local_time(now).map(|t| (t, self.timing.sample(t)));
        let state = sample.map(|(_, s)| (s.phase, s.iteration));
        for (kind, elapsed) in events::transitions(self.seen, state, &self.timing) {
            events.push(self.event(kind, elapsed, now));
        }
        self.seen = state;
        let progress = sample.and_then(|(_, s)| s.progress);
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
        let scheduled = if self.hold.is_some() || kind == AnimationEventKind::Cancel {
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
        let t = self.local_time(now)?;
        let elapsed = self.timing.clamped_active_time(t);
        Some(self.event(AnimationEventKind::Cancel, elapsed, now))
    }
}

impl AnimationRegistry {
    /// Bring the CSS animations of `(id, slot)` in line with its style:
    /// `style` is its cascaded style while it is rendered, `None` while
    /// it is not (no box: `display: none` itself or above, a pseudo-element
    /// without content). `restyled`: the style is a new one, so the
    /// keyframe values are recomputed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_css(
        &mut self,
        dom: &Dom<TuiExt>,
        inputs: CssInputs<'_>,
        (id, slot): (NodeId, StyleSlot),
        style: Option<&ComputedStyle>,
        scheme: ColorScheme,
        restyled: bool,
        now: Instant,
    ) {
        let mut existing: Vec<usize> = (0..self.css.len())
            .filter(|&i| self.css[i].node == id && self.css[i].slot == slot)
            .collect();
        let names = style.map_or(&[][..], |s| s.animation_name.as_slice());
        let mut kept: Vec<usize> = Vec::new();
        let mut started: Vec<CssAnimation> = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let (Some(name), Some(style)) = (name.name(), style) else {
                continue;
            };
            // §4.1: a name no `@keyframes` rule defines runs nothing.
            let Some(rule) =
                crate::style::cascade::keyframes_rule(dom, (inputs.sheets, inputs.registry), name)
            else {
                continue;
            };
            let entry = Entry::of(style, index);
            match existing
                .iter()
                .position(|&i| &*self.css[i].name == name)
                .map(|k| existing.remove(k))
            {
                Some(i) => {
                    let anim = &mut self.css[i];
                    let rebuild = restyled
                        || anim.rule != *rule
                        || anim.easing != entry.easing
                        || anim.composition != entry.composition;
                    anim.retime(entry, now);
                    anim.index = index;
                    anim.scheme = scheme;
                    if rebuild {
                        anim.rule = rule.clone();
                        anim.effect = Rc::new(build_effect(dom, inputs, (id, slot), anim, style));
                    }
                    anim.dirty = true;
                    kept.push(i);
                }
                None => {
                    let paused = entry.play_state == AnimationPlayState::Paused;
                    let mut anim = CssAnimation {
                        node: id,
                        slot,
                        name: name.into(),
                        index,
                        timing: entry.timing,
                        easing: entry.easing,
                        composition: entry.composition,
                        play_state: entry.play_state,
                        timeline: entry.timeline,
                        start: now,
                        hold: paused.then_some(0.0),
                        effect: Rc::default(),
                        rule: rule.clone(),
                        scheme,
                        seen: None,
                        composited: None,
                        dirty: true,
                    };
                    anim.effect = Rc::new(build_effect(dom, inputs, (id, slot), &anim, style));
                    started.push(anim);
                }
            }
        }
        // §4.1: an animation whose name left the list is cancelled.
        existing.sort_unstable();
        for i in existing.into_iter().rev() {
            let gone = self.css.remove(i);
            if let Some(e) = gone.cancel_event(now) {
                self.css_events.push(e);
            }
            self.css_cancelled.push((gone.node, gone.slot));
        }
        self.css.extend(started);
    }

    /// Advance every CSS animation to `now`: queue its events, and add
    /// the targets whose composited values moved to `targets`.
    pub(super) fn step_css(&mut self, now: Instant, targets: &mut Vec<(NodeId, StyleSlot)>) {
        for (node, slot) in self.css_cancelled.drain(..) {
            if !targets.contains(&(node, slot)) {
                targets.push((node, slot));
            }
        }
        for anim in &mut self.css {
            if anim.step(now, &mut self.css_events) && !targets.contains(&(anim.node, anim.slot)) {
                targets.push((anim.node, anim.slot));
            }
        }
    }

    /// The CSS animations of `(node, slot)`, in composite order.
    pub(super) fn css_on(&self, node: NodeId, slot: StyleSlot) -> Vec<&CssAnimation> {
        let mut out: Vec<&CssAnimation> = self
            .css
            .iter()
            .filter(|a| a.node == node && a.slot == slot)
            .collect();
        out.sort_by_key(|a| a.index);
        out
    }

    /// Take the CSS animation events queued since the last call.
    pub(crate) fn take_pending_animation_events(&mut self) -> Vec<PendingAnimationEvent> {
        std::mem::take(&mut self.css_events)
    }

    /// Cancel the CSS animations of `node` (every slot) at `now`.
    pub(super) fn cancel_css_for_node(&mut self, node: NodeId, now: Instant) {
        let mut i = 0;
        while i < self.css.len() {
            if self.css[i].node == node {
                let gone = self.css.remove(i);
                if let Some(e) = gone.cancel_event(now) {
                    self.css_events.push(e);
                }
                self.css_cancelled.push((gone.node, gone.slot));
            } else {
                i += 1;
            }
        }
    }
}

impl CssAnimation {
    /// Take a new entry's timing in place (§4: "changes to the other
    /// animation properties update the running animation"): a pause holds
    /// the local time it reached, a resume restarts the clock from it.
    fn retime(&mut self, entry: Entry, now: Instant) {
        let paused = entry.play_state == AnimationPlayState::Paused;
        match (self.hold, paused) {
            (None, true) => {
                self.hold = Some(now.saturating_duration_since(self.start).as_secs_f64() * 1000.0);
            }
            (Some(held), false) => {
                self.start = now
                    .checked_sub(Duration::from_secs_f64(held.max(0.0) / 1000.0))
                    .unwrap_or(now);
                self.hold = None;
            }
            _ => {}
        }
        self.timing = entry.timing;
        self.easing = entry.easing;
        self.composition = entry.composition;
        self.play_state = entry.play_state;
        self.timeline = entry.timeline;
    }
}

/// Build `anim`'s keyframe effect for the element style `style` of
/// `(id, slot)`: each keyframe's values computed with its blocks in the
/// animation origin.
fn build_effect(
    dom: &Dom<TuiExt>,
    inputs: CssInputs<'_>,
    (id, slot): (NodeId, StyleSlot),
    anim: &CssAnimation,
    style: &ComputedStyle,
) -> KeyframeEffect {
    let mut style_of = |blocks: &[&crate::style::TuiStyle]| {
        crate::style::cascade::keyframe_style(
            dom,
            (inputs.sheets, inputs.registry),
            id,
            slot,
            blocks,
        )
    };
    KeyframeEffect::build(
        &anim.rule,
        &anim.easing,
        anim.composition,
        style.text_direction,
        &mut style_of,
    )
}
