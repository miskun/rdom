//! Which events a CSS animation fires as its phase changes (CSS
//! Animations 2 §4.2's table) and the queued event itself.

use std::sync::Arc;
use std::time::Instant;

use rdom_core::NodeId;

use super::timing::{Phase, Timing};
use crate::ext::StyleSlot;

/// The kind of a CSS animation event (CSS Animations 1 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AnimationEventKind {
    /// `animationstart`: the active interval begins.
    Start,
    /// `animationiteration`: a new iteration begins.
    Iteration,
    /// `animationend`: the active interval ends.
    End,
    /// `animationcancel`: the animation was dropped before it ended.
    Cancel,
}

impl AnimationEventKind {
    /// The event type.
    pub fn event_type(self) -> &'static str {
        match self {
            AnimationEventKind::Start => "animationstart",
            AnimationEventKind::Iteration => "animationiteration",
            AnimationEventKind::End => "animationend",
            AnimationEventKind::Cancel => "animationcancel",
        }
    }
}

/// An animation event waiting to be dispatched after the frame.
#[derive(Debug, Clone)]
pub(crate) struct PendingAnimationEvent {
    pub node: NodeId,
    pub slot: StyleSlot,
    pub kind: AnimationEventKind,
    pub name: Arc<str>,
    /// `elapsedTime`, in seconds.
    pub elapsed_seconds: f64,
    /// When it happened, on the app's clock: events of one frame go out
    /// in this order, then in composite order (§4.2).
    pub scheduled: Instant,
    /// Its animation's place in `animation-name`.
    pub index: usize,
}

/// The events of a change from `before` to `after` — each a phase and
/// the current iteration, `None` while idle — with their `elapsedTime`
/// in ms (CSS Animations 2 §4.2).
pub(super) fn transitions(
    before: Option<(Phase, Option<f64>)>,
    after: Option<(Phase, Option<f64>)>,
    timing: &Timing,
) -> Vec<(AnimationEventKind, f64)> {
    use AnimationEventKind::{Cancel, End, Iteration, Start};
    let (start, end) = (timing.interval_start(), timing.interval_end());
    let was = before.map(|(p, _)| p);
    let Some((now, iteration)) = after else {
        // Not idle and not after → idle.
        return match was {
            Some(Phase::Before | Phase::Active) => vec![(Cancel, 0.0)],
            _ => Vec::new(),
        };
    };
    match (was, now) {
        (None | Some(Phase::Before), Phase::Active) => vec![(Start, start)],
        (None | Some(Phase::Before), Phase::After) => vec![(Start, start), (End, end)],
        (Some(Phase::Active), Phase::Before) => vec![(End, start)],
        (Some(Phase::Active), Phase::Active) => {
            let previous = before.and_then(|(_, i)| i);
            match (previous, iteration) {
                (Some(a), Some(b)) if a != b => vec![(Iteration, b * timing.duration)],
                _ => Vec::new(),
            }
        }
        (Some(Phase::Active), Phase::After) => vec![(End, end)],
        (Some(Phase::After), Phase::Active) => vec![(Start, end)],
        (Some(Phase::After), Phase::Before) => vec![(Start, end), (End, start)],
        _ => Vec::new(),
    }
}
