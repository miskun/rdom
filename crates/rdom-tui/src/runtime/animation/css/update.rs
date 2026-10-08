//! The cascade hook's side of CSS animations: matching an element
//! style's `animation-name` list to its running animations (CSS
//! Animations 1 §4.1), retiming the kept ones, building keyframe effects,
//! and the registry's per-frame stepping of them.

use std::rc::Rc;
use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId};
use rdom_style::color::ColorScheme;

use super::{CssAnimation, CssInputs, Entry, KeyframeEffect, PendingAnimationEvent};
use crate::ext::{StyleSlot, TuiExt};
use crate::runtime::animation::AnimationRegistry;
use crate::style::{AnimationPlayState, AnimationTimeline, ComputedStyle};

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
                        duration_auto: entry.duration_auto,
                        range: entry.range,
                        effect: Rc::default(),
                        rule: rule.clone(),
                        scheme,
                        seen: None,
                        composited: None,
                        dirty: true,
                        local: None,
                        fraction: None,
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
    pub(in crate::runtime::animation) fn step_css(
        &mut self,
        dom: &Dom<TuiExt>,
        now: Instant,
        targets: &mut Vec<(NodeId, StyleSlot)>,
    ) {
        for (node, slot) in self.css_cancelled.drain(..) {
            if !targets.contains(&(node, slot)) {
                targets.push((node, slot));
            }
        }
        for anim in &mut self.css {
            if anim.step(dom, now, &mut self.css_events)
                && !targets.contains(&(anim.node, anim.slot))
            {
                targets.push((anim.node, anim.slot));
            }
        }
    }

    /// Step the animations on scroll and view timelines again after a
    /// layout, compositing what moved: a timeline whose scroller or
    /// subject the layout moved is stale, and Scroll-driven Animations 1
    /// §5 runs style and layout once more rather than show it a frame
    /// late. `layout` in the result: run layout again.
    pub(crate) fn restep_progress(
        &mut self,
        dom: &mut Dom<TuiExt>,
        now: Instant,
    ) -> crate::runtime::animation::Advanced {
        let mut targets: Vec<(NodeId, StyleSlot)> = Vec::new();
        for anim in &mut self.css {
            if anim.timeline != AnimationTimeline::Auto
                && anim.step(dom, now, &mut self.css_events)
                && !targets.contains(&(anim.node, anim.slot))
            {
                targets.push((anim.node, anim.slot));
            }
        }
        let mut out = crate::runtime::animation::Advanced::default();
        for (node, slot) in targets {
            out.composites += 1;
            out.layout |= self.composite(dom, node, slot, now);
        }
        out
    }

    /// Whether an animation follows a scroll or view timeline.
    pub(crate) fn has_progress_timelines(&self) -> bool {
        self.css.iter().any(|a| {
            !matches!(
                a.timeline,
                AnimationTimeline::Auto | AnimationTimeline::None
            )
        })
    }

    /// The CSS animations of `(node, slot)`, in composite order.
    pub(in crate::runtime::animation) fn css_on(
        &self,
        node: NodeId,
        slot: StyleSlot,
    ) -> Vec<&CssAnimation> {
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
    pub(in crate::runtime::animation) fn cancel_css_for_node(
        &mut self,
        node: NodeId,
        now: Instant,
    ) {
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
                self.hold = Some(if self.timeline == AnimationTimeline::Auto {
                    now.saturating_duration_since(self.start).as_secs_f64() * 1000.0
                } else {
                    self.local.unwrap_or(0.0)
                });
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
        self.duration_auto = entry.duration_auto;
        self.range = entry.range;
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
