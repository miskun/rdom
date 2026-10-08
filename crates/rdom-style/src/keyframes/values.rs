//! The single values of the `animation-*` longhands (CSS Animations 1
//! §4, CSS Animations 2 §3). Each longhand is a comma-separated list of
//! these, matched to `animation-name`'s entries by index — the shorter
//! lists repeated, the longer ones cut (§4.1).

use std::sync::Arc;

/// One `animation-name` entry (§4.1): `none` or a `<keyframes-name>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnimationName {
    /// No animation in this slot; its other longhands' entries are kept
    /// for their place in the lists.
    None,
    /// The `@keyframes` name, as written (case-sensitive): a
    /// `<custom-ident>` or a `<string>`.
    Named(Arc<str>),
}

impl AnimationName {
    /// The name, `None` for `none`.
    pub fn name(&self) -> Option<&str> {
        match self {
            AnimationName::None => None,
            AnimationName::Named(n) => Some(n),
        }
    }
}

/// One `animation-duration` entry (§4.2; CSS Animations 2 §3.3):
/// `auto` or a non-negative `<time>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimationDuration {
    /// `auto`: 0s on a time-based timeline; the whole range on a
    /// progress-based one (Scroll-driven Animations 1).
    #[default]
    Auto,
    /// A time in milliseconds.
    Ms(u32),
}

impl AnimationDuration {
    /// The duration on a time-based timeline, in milliseconds: `auto` is 0.
    pub fn ms(self) -> u32 {
        match self {
            AnimationDuration::Auto => 0,
            AnimationDuration::Ms(ms) => ms,
        }
    }
}

/// One `animation-iteration-count` entry (§4.4): `infinite` or a
/// non-negative `<number>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IterationCount {
    Infinite,
    Count(f32),
}

impl Default for IterationCount {
    fn default() -> Self {
        IterationCount::Count(1.0)
    }
}

impl IterationCount {
    /// The count as a number: `infinite` is `f64::INFINITY`.
    pub fn get(self) -> f64 {
        match self {
            IterationCount::Infinite => f64::INFINITY,
            IterationCount::Count(n) => f64::from(n),
        }
    }
}

/// Closed (DESIGN): the four values of `animation-direction` (§4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimationDirection {
    #[default]
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// Closed (DESIGN): the four values of `animation-fill-mode` (§4.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimationFillMode {
    #[default]
    None,
    Forwards,
    Backwards,
    Both,
}

impl AnimationFillMode {
    /// Whether the animation applies its first value during its delay.
    pub fn backwards(self) -> bool {
        matches!(self, AnimationFillMode::Backwards | AnimationFillMode::Both)
    }

    /// Whether the animation keeps its last value once it ends.
    pub fn forwards(self) -> bool {
        matches!(self, AnimationFillMode::Forwards | AnimationFillMode::Both)
    }
}

/// Closed (DESIGN): `animation-play-state` (§4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimationPlayState {
    #[default]
    Running,
    Paused,
}

/// Closed (DESIGN): `animation-composition` (CSS Animations 2 §3.2) —
/// the composite operation (Web Animations 1 §5.4.4) of the animation's
/// keyframes, which a keyframe's own declaration overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimationComposition {
    /// The effect value replaces the underlying value.
    #[default]
    Replace,
    /// The effect value is added to the underlying value.
    Add,
    /// The effect value accumulates onto the underlying value.
    Accumulate,
}

/// One `animation-timeline` entry (CSS Animations 2 §3.7, Scroll-driven
/// Animations 1 §4.1): which timeline drives the animation.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AnimationTimeline {
    /// The document's timeline: the clock.
    #[default]
    Auto,
    /// No timeline: the animation is inactive.
    None,
    /// The scroll or view progress timeline of that `<dashed-ident>`
    /// (`scroll-timeline-name` / `view-timeline-name`, §4.1).
    Named(Arc<str>),
    /// `scroll()`: an anonymous scroll progress timeline (§2.1.1).
    Scroll {
        scroller: super::TimelineScroller,
        axis: super::TimelineAxis,
    },
    /// `view()`: an anonymous view progress timeline of the element
    /// itself (§3.1.1).
    View {
        axis: super::TimelineAxis,
        inset: super::TimelineInset,
    },
}

/// The keyword spelling of each keyword value, both ways.
macro_rules! keywords {
    ($($t:ty { $($v:ident = $k:literal),+ $(,)? })+) => {$(
        impl $t {
            /// The value's keyword.
            pub fn keyword(self) -> &'static str {
                match self { $(<$t>::$v => $k,)+ }
            }

            /// The value spelled `kw` (ASCII case-insensitive).
            pub fn from_keyword(kw: &str) -> Option<Self> {
                $(if kw.eq_ignore_ascii_case($k) { return Some(<$t>::$v); })+
                None
            }
        }
    )+};
}

keywords! {
    AnimationDirection {
        Normal = "normal",
        Reverse = "reverse",
        Alternate = "alternate",
        AlternateReverse = "alternate-reverse",
    }
    AnimationFillMode {
        None = "none",
        Forwards = "forwards",
        Backwards = "backwards",
        Both = "both",
    }
    AnimationPlayState {
        Running = "running",
        Paused = "paused",
    }
    AnimationComposition {
        Replace = "replace",
        Add = "add",
        Accumulate = "accumulate",
    }
}
