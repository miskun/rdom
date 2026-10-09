//! The transition, animation and timeline properties of a computed style,
//! one shared group ([`ComputedStyle::motion`](crate::ComputedStyle::motion),
//! C15G-STYLE-SIZE).

/// The computed `transition-*`, `animation-*`, timeline and range
/// longhands (CSS Transitions 1 / 2, CSS Animations 1 / 2, Scroll-driven
/// Animations 1). None inherits.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values: every list
/// empty.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionStyle {
    /// Resolved `transition-*` longhand lists. Empty when no
    /// transition rules apply. Engine reads them by index per the
    /// CSS L1 reconciliation rule (shorter lists cycle).
    pub transition_property: Vec<crate::transition::TransitionProperty>,
    pub transition_duration: Vec<u32>,
    pub transition_timing_function: Vec<crate::transition::TimingFunction>,
    pub transition_delay: Vec<i32>,
    /// `transition-behavior` (CSS Transitions 2 §3.1); empty is `normal`.
    pub transition_behavior: Vec<crate::transition::TransitionBehavior>,

    // ── Animations (CSS Animations 1 / 2) ────────────────────────────
    /// The `animation-*` longhand lists, matched to `animation-name` by
    /// index (CSS Animations 1 §4.1: shorter lists repeat). Empty is the
    /// longhand's initial value. Not inherited.
    pub animation_name: Vec<crate::keyframes::AnimationName>,
    pub animation_duration: Vec<crate::keyframes::AnimationDuration>,
    pub animation_timing_function: Vec<crate::transition::TimingFunction>,
    pub animation_delay: Vec<i32>,
    pub animation_iteration_count: Vec<crate::keyframes::IterationCount>,
    pub animation_direction: Vec<crate::keyframes::AnimationDirection>,
    pub animation_fill_mode: Vec<crate::keyframes::AnimationFillMode>,
    pub animation_play_state: Vec<crate::keyframes::AnimationPlayState>,
    pub animation_composition: Vec<crate::keyframes::AnimationComposition>,
    pub animation_timeline: Vec<crate::keyframes::AnimationTimeline>,

    // ── Scroll-driven animations (Scroll-driven Animations 1) ────────
    /// The timeline longhands' lists (empty: `none` / the initial axis),
    /// `timeline-scope`, and the animations' ranges (empty: `normal`).
    /// Not inherited.
    pub scroll_timeline_name: Vec<crate::keyframes::TimelineName>,
    pub scroll_timeline_axis: Vec<crate::keyframes::TimelineAxis>,
    pub view_timeline_name: Vec<crate::keyframes::TimelineName>,
    pub view_timeline_axis: Vec<crate::keyframes::TimelineAxis>,
    pub view_timeline_inset: Vec<crate::keyframes::TimelineInset>,
    pub timeline_scope: crate::keyframes::TimelineScope,
    pub animation_range_start: Vec<crate::keyframes::RangeBoundary>,
    pub animation_range_end: Vec<crate::keyframes::RangeBoundary>,
}
