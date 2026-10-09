//! The transition and animation setters of the `TuiStyle` builder: the
//! `transition-*` and `animation-*` longhands, list-valued (`Vec`
//! fields can't go through the `setter!` macro: no `Value<T>` per item).

use super::super::{ImportantMask, TuiStyle};
use crate::Value;

impl TuiStyle {
    // ── Transitions (CSS Transitions 1 / 2) ──────────────────────────
    // Vec-typed fields can't go through the `setter!` macro (no
    // `Value<T>` wrapping), so we hand-write a thin layer.
    pub fn transition_property(mut self, v: Vec<crate::transition::TransitionProperty>) -> Self {
        self.motion.transition_property = Some(Value::Specified(v));
        self
    }
    pub fn transition_duration(mut self, v: Vec<u32>) -> Self {
        self.motion.transition_duration = Some(Value::Specified(v));
        self
    }
    pub fn transition_timing_function(mut self, v: Vec<crate::transition::TimingFunction>) -> Self {
        self.motion.transition_timing_function = Some(Value::Specified(v));
        self
    }
    pub fn transition_behavior(mut self, v: Vec<crate::transition::TransitionBehavior>) -> Self {
        self.motion.transition_behavior = Some(Value::Specified(v));
        self
    }

    pub fn transition_delay(mut self, v: Vec<i32>) -> Self {
        self.motion.transition_delay = Some(Value::Specified(v));
        self
    }
    /// Mark the four transition longhands `!important`, as a
    /// `transition: … !important` declaration does (each longhand has
    /// its own bit; `ImportantMask::TRANSITIONS` is their union).
    pub fn transitions_important(mut self) -> Self {
        self.important |= ImportantMask::TRANSITIONS;
        self
    }

    // ── Animations (CSS Animations 1 / 2) ────────────────────────────
    /// `animation-name`.
    pub fn animation_name(mut self, v: Vec<crate::keyframes::AnimationName>) -> Self {
        self.motion.animation_name = Some(Value::Specified(v));
        self
    }
    /// `animation-duration`.
    pub fn animation_duration(mut self, v: Vec<crate::keyframes::AnimationDuration>) -> Self {
        self.motion.animation_duration = Some(Value::Specified(v));
        self
    }
    /// `animation-timing-function`.
    pub fn animation_timing_function(mut self, v: Vec<crate::transition::TimingFunction>) -> Self {
        self.motion.animation_timing_function = Some(Value::Specified(v));
        self
    }
    /// `animation-delay`, in milliseconds.
    pub fn animation_delay(mut self, v: Vec<i32>) -> Self {
        self.motion.animation_delay = Some(Value::Specified(v));
        self
    }
    /// `animation-iteration-count`.
    pub fn animation_iteration_count(mut self, v: Vec<crate::keyframes::IterationCount>) -> Self {
        self.motion.animation_iteration_count = Some(Value::Specified(v));
        self
    }
    /// `animation-direction`.
    pub fn animation_direction(mut self, v: Vec<crate::keyframes::AnimationDirection>) -> Self {
        self.motion.animation_direction = Some(Value::Specified(v));
        self
    }
    /// `animation-fill-mode`.
    pub fn animation_fill_mode(mut self, v: Vec<crate::keyframes::AnimationFillMode>) -> Self {
        self.motion.animation_fill_mode = Some(Value::Specified(v));
        self
    }
    /// `animation-play-state`.
    pub fn animation_play_state(mut self, v: Vec<crate::keyframes::AnimationPlayState>) -> Self {
        self.motion.animation_play_state = Some(Value::Specified(v));
        self
    }
    /// `animation-composition`.
    pub fn animation_composition(mut self, v: Vec<crate::keyframes::AnimationComposition>) -> Self {
        self.motion.animation_composition = Some(Value::Specified(v));
        self
    }
    /// `animation-timeline`.
    pub fn animation_timeline(mut self, v: Vec<crate::keyframes::AnimationTimeline>) -> Self {
        self.motion.animation_timeline = Some(Value::Specified(v));
        self
    }
    /// Mark the animation longhands `!important`, as an `animation: …
    /// !important` declaration does (`ImportantMask::ANIMATIONS`).
    pub fn animations_important(mut self) -> Self {
        self.important |= ImportantMask::ANIMATIONS;
        self
    }

    // ── Scroll-driven animations (Scroll-driven Animations 1) ────────
    /// `scroll-timeline-name`.
    pub fn scroll_timeline_name(mut self, v: Vec<crate::keyframes::TimelineName>) -> Self {
        self.motion.scroll_timeline_name = Some(Value::Specified(v));
        self
    }
    /// `scroll-timeline-axis`.
    pub fn scroll_timeline_axis(mut self, v: Vec<crate::keyframes::TimelineAxis>) -> Self {
        self.motion.scroll_timeline_axis = Some(Value::Specified(v));
        self
    }
    /// `view-timeline-name`.
    pub fn view_timeline_name(mut self, v: Vec<crate::keyframes::TimelineName>) -> Self {
        self.motion.view_timeline_name = Some(Value::Specified(v));
        self
    }
    /// `view-timeline-axis`.
    pub fn view_timeline_axis(mut self, v: Vec<crate::keyframes::TimelineAxis>) -> Self {
        self.motion.view_timeline_axis = Some(Value::Specified(v));
        self
    }
    /// `view-timeline-inset`.
    pub fn view_timeline_inset(mut self, v: Vec<crate::keyframes::TimelineInset>) -> Self {
        self.motion.view_timeline_inset = Some(Value::Specified(v));
        self
    }
    /// `timeline-scope`.
    pub fn timeline_scope(mut self, v: crate::keyframes::TimelineScope) -> Self {
        self.motion.timeline_scope = Some(Value::Specified(v));
        self
    }
    /// `animation-range-start` / `-end`, one pair per animation.
    pub fn animation_range(
        mut self,
        v: Vec<(
            crate::keyframes::RangeBoundary,
            crate::keyframes::RangeBoundary,
        )>,
    ) -> Self {
        let (starts, ends) = v.into_iter().unzip();
        self.motion.animation_range_start = Some(Value::Specified(starts));
        self.motion.animation_range_end = Some(Value::Specified(ends));
        self
    }
    /// Mark the timeline-declaring longhands `!important`
    /// (`ImportantMask::TIMELINES`).
    pub fn timelines_important(mut self) -> Self {
        self.important |= ImportantMask::TIMELINES;
        self
    }
}
