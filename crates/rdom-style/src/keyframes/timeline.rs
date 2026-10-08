//! The values of the scroll-driven animation properties (Scroll-driven
//! Animations 1): the named timelines a scroll container or a subject
//! declares, the anonymous `scroll()` / `view()` timelines an
//! `animation-timeline` names, `timeline-scope`, and the range an
//! animation is attached to on its timeline.

use std::sync::Arc;

use crate::layout::Length;

/// A timeline's axis (§2.2 `scroll-timeline-axis`): the scroll
/// container's block or inline axis, or a physical one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimelineAxis {
    #[default]
    Block,
    Inline,
    X,
    Y,
}

/// Which scroll container a `scroll()` timeline follows (§2.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimelineScroller {
    /// The nearest ancestor scroll container.
    #[default]
    Nearest,
    /// The document's root element.
    Root,
    /// The element itself.
    SelfElement,
}

/// One `scroll-timeline-name` / `view-timeline-name` entry: `none` or a
/// `<dashed-ident>` (case-sensitive).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TimelineName {
    #[default]
    None,
    Named(Arc<str>),
}

impl TimelineName {
    /// The name, `None` for `none`.
    pub fn name(&self) -> Option<&str> {
        match self {
            TimelineName::None => None,
            TimelineName::Named(n) => Some(n),
        }
    }
}

/// One `view-timeline-inset` entry (§3.2.3): the start and end insets of
/// the scrollport, each `auto` (`Length::Auto`: the scroll container's
/// `scroll-padding`) or a `<length-percentage>` of the scrollport.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimelineInset {
    pub start: Length,
    pub end: Length,
}

/// `timeline-scope` (§4.2): which named timelines of the element's
/// descendants it makes visible to its whole subtree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TimelineScope {
    #[default]
    None,
    /// Every name its descendants declare.
    All,
    Names(Vec<Arc<str>>),
}

impl TimelineScope {
    /// Whether it hoists the timeline `name`.
    pub fn covers(&self, name: &str) -> bool {
        match self {
            TimelineScope::None => false,
            TimelineScope::All => true,
            TimelineScope::Names(names) => names.iter().any(|n| &**n == name),
        }
    }
}

/// A named timeline range (§3.4): a segment of a view progress timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineRangeName {
    /// The subject's whole crossing of the scrollport — the full timeline.
    Cover,
    /// While the subject is fully inside the scrollport (or covers it).
    Contain,
    /// While the subject enters: from `cover` 0% to `contain` 0%.
    Entry,
    /// While the subject exits: from `contain` 100% to `cover` 100%.
    Exit,
    /// While the subject crosses the scrollport's end edge.
    EntryCrossing,
    /// While the subject crosses the scrollport's start edge.
    ExitCrossing,
}

/// One `animation-range-start` / `-end` entry (§4.3).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RangeBoundary {
    /// The start (or end) of the timeline.
    #[default]
    Normal,
    /// An offset into the named range — or into the whole timeline when
    /// `name` is `None` — a `<length-percentage>` from that range's start.
    Offset {
        name: Option<TimelineRangeName>,
        offset: Length,
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
    TimelineAxis { Block = "block", Inline = "inline", X = "x", Y = "y" }
    TimelineScroller { Nearest = "nearest", Root = "root", SelfElement = "self" }
    TimelineRangeName {
        Cover = "cover",
        Contain = "contain",
        Entry = "entry",
        Exit = "exit",
        EntryCrossing = "entry-crossing",
        ExitCrossing = "exit-crossing",
    }
}
