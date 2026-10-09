//! The transition, animation and timeline declarations of a declaration
//! block, one shared group ([`TuiStyle::motion`](crate::TuiStyle::motion),
//! C15G-STYLE-SIZE).

use crate::Value;

/// The declared `transition-*`, `animation-*`, timeline and range
/// longhands.
///
/// Closed (DESIGN): a new field fails a destructuring pattern.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotionDeclarations {
    /// `transition-property` longhand. Each entry covers one
    /// CSS property (or `all` / `none`) at the matching index in
    /// the duration / timing / delay lists.
    pub transition_property: Option<Value<Vec<crate::transition::TransitionProperty>>>,
    /// `transition-duration` longhand, in milliseconds.
    pub transition_duration: Option<Value<Vec<u32>>>,
    /// `transition-timing-function` longhand.
    pub transition_timing_function: Option<Value<Vec<crate::transition::TimingFunction>>>,
    /// `transition-delay` longhand, in milliseconds.
    pub transition_delay: Option<Value<Vec<i32>>>,
    /// `transition-behavior` (CSS Transitions 2 §3.1).
    pub transition_behavior: Option<Value<Vec<crate::transition::TransitionBehavior>>>,

    // ── Animations (CSS Animations 1 / 2) ────────────────────────────
    /// `animation-name` (CSS Animations 1 §4.1): which `@keyframes` each animation runs.
    pub animation_name: Option<Value<Vec<crate::keyframes::AnimationName>>>,
    /// `animation-duration` (§4.2, CSS Animations 2 §3.3: `auto`).
    pub animation_duration: Option<Value<Vec<crate::keyframes::AnimationDuration>>>,
    /// `animation-timing-function` (§4.3): each keyframe interval's easing.
    pub animation_timing_function: Option<Value<Vec<crate::transition::TimingFunction>>>,
    /// `animation-delay` in milliseconds (§4.7); negative starts part-way.
    pub animation_delay: Option<Value<Vec<i32>>>,
    /// `animation-iteration-count` (§4.4).
    pub animation_iteration_count: Option<Value<Vec<crate::keyframes::IterationCount>>>,
    /// `animation-direction` (§4.5).
    pub animation_direction: Option<Value<Vec<crate::keyframes::AnimationDirection>>>,
    /// `animation-fill-mode` (§4.8).
    pub animation_fill_mode: Option<Value<Vec<crate::keyframes::AnimationFillMode>>>,
    /// `animation-play-state` (§4.6).
    pub animation_play_state: Option<Value<Vec<crate::keyframes::AnimationPlayState>>>,
    /// `animation-composition` (CSS Animations 2 §3.2).
    pub animation_composition: Option<Value<Vec<crate::keyframes::AnimationComposition>>>,
    /// `animation-timeline` (CSS Animations 2 §3.7, Scroll-driven Animations 1 §4.1).
    pub animation_timeline: Option<Value<Vec<crate::keyframes::AnimationTimeline>>>,

    // ── Scroll-driven animations (Scroll-driven Animations 1) ────────
    /// `scroll-timeline-name` (Scroll-driven Animations 1 §2.2.1).
    pub scroll_timeline_name: Option<Value<Vec<crate::keyframes::TimelineName>>>,
    /// `scroll-timeline-axis` (§2.2.2).
    pub scroll_timeline_axis: Option<Value<Vec<crate::keyframes::TimelineAxis>>>,
    /// `view-timeline-name` (§3.2.1).
    pub view_timeline_name: Option<Value<Vec<crate::keyframes::TimelineName>>>,
    /// `view-timeline-axis` (§3.2.2).
    pub view_timeline_axis: Option<Value<Vec<crate::keyframes::TimelineAxis>>>,
    /// `view-timeline-inset` (§3.2.3).
    pub view_timeline_inset: Option<Value<Vec<crate::keyframes::TimelineInset>>>,
    /// `timeline-scope` (§4.2).
    pub timeline_scope: Option<Value<crate::keyframes::TimelineScope>>,
    /// `animation-range-start` (§4.3.1).
    pub animation_range_start: Option<Value<Vec<crate::keyframes::RangeBoundary>>>,
    /// `animation-range-end` (§4.3.2).
    pub animation_range_end: Option<Value<Vec<crate::keyframes::RangeBoundary>>>,
}
