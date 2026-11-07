//! Which change started an animation (`specs/CAPTURE.md` §Events caused
//! by the inspector): the app's own — its DOM and stylesheet changes,
//! input, timers — or an inspector's (an edit, a forced state, a content
//! rewrite in a capture session).
//!
//! An inspector applies its change and draws the frame it causes through
//! `App::inspect`, which draws what the app changed first and then the
//! inspector's frame alone — drawing only, no app timers — so every
//! transition or animation that frame's cascade starts is the
//! inspector's. The cause belongs to the running transition or animation
//! itself, set when it starts, and every event it fires carries it — its
//! `transitionrun` through its `transitioncancel` — so an app's later
//! start of the same property is a new transition with its own cause, and
//! the cancel of the inspector's one it replaces stays the inspector's.
//! The events of an inspector-caused animation are what the event policy
//! governs
//! (`App::dispatch_animation_events`): withheld from app code and listed
//! ([`SuppressedEvent`]).

use rdom_core::NodeId;

use super::AnimationRegistry;

/// Who started a transition or an animation: every event it fires
/// carries it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Cause {
    /// The app's own change: its DOM and sheets, input, timers.
    #[default]
    App,
    /// An inspector's change, drawn by `App::inspect`.
    Inspector,
}

/// Whether an inspector frame is drawing now, and the events withheld.
#[derive(Debug, Default)]
pub(crate) struct Causes {
    inspecting: bool,
    suppressed: Vec<SuppressedEvent>,
}

/// An event an inspector-caused animation fired that app code was not
/// shown.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SuppressedEvent {
    /// The event type (`transitionstart`, `animationend`, …).
    pub event: String,
    /// The element it would have been dispatched to.
    pub node: NodeId,
    /// The property (`color`, `--accent`) or the animation's name.
    pub of: String,
}

impl AnimationRegistry {
    /// Take every animation started from now on as an inspector's (`on`),
    /// or the app's again — around an inspector's frame.
    pub(crate) fn set_inspecting(&mut self, on: bool) {
        self.causes.inspecting = on;
    }

    /// The cause of a transition or an animation starting now.
    pub(super) fn cause_now(&self) -> Cause {
        if self.causes.inspecting {
            Cause::Inspector
        } else {
            Cause::App
        }
    }

    /// Whether `event`, fired by an animation `by` caused, at `node`, of
    /// `of`, goes to app code: an inspector-caused animation's is withheld
    /// and listed.
    pub(crate) fn admit(
        &mut self,
        by: Cause,
        event: &str,
        node: NodeId,
        of: impl FnOnce() -> String,
    ) -> bool {
        if by == Cause::App {
            return true;
        }
        self.causes.suppressed.push(SuppressedEvent {
            event: event.to_string(),
            node,
            of: of(),
        });
        false
    }

    /// The events withheld since the last call, in dispatch order.
    pub(crate) fn take_suppressed(&mut self) -> Vec<SuppressedEvent> {
        std::mem::take(&mut self.causes.suppressed)
    }
}
