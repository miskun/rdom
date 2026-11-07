//! Which change started an animation (`specs/CAPTURE.md` §Events caused
//! by the inspector): the app's own — its DOM and stylesheet changes,
//! input, timers — or an inspector's (an edit, a forced state, a content
//! rewrite in a capture session).
//!
//! An inspector applies its change and draws the frame it causes inside
//! [`AnimationRegistry::with_inspector_cause`]; a capture session settles
//! every step first, so that frame holds the inspector's change alone and
//! every transition or animation it starts is the inspector's. Each is
//! keyed by what it animates; an app-caused start of the same key later
//! takes it back (when both cause one, the app's wins: an event the app
//! would have seen anyway is never held back). The events of an
//! inspector-caused animation are what the event policy governs
//! (`App::dispatch_animation_events`): withheld from app code and listed
//! ([`SuppressedEvent`]).

use std::collections::HashSet;
use std::sync::Arc;

use rdom_core::NodeId;

use super::{AnimationRegistry, Longhand};
use crate::ext::StyleSlot;

/// What an animation animates, for its cause.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum CauseKey {
    /// A transition of a longhand.
    Transition(NodeId, StyleSlot, Longhand),
    /// A transition of a registered custom property (`--name`).
    Custom(NodeId, String),
    /// A CSS animation, by name.
    Css(NodeId, StyleSlot, Arc<str>),
}

/// The inspector-caused animations, and whether an inspector frame is
/// drawing now.
#[derive(Debug, Default)]
pub(crate) struct Causes {
    inspecting: bool,
    started: HashSet<CauseKey>,
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
    /// or the app's again — around an inspector's change and its frame.
    pub(crate) fn set_inspecting(&mut self, on: bool) {
        self.causes.inspecting = on;
    }

    /// Note an animation starting: the inspector's while it draws, else
    /// the app's (which takes the key back).
    pub(super) fn note_start(&mut self, key: CauseKey) {
        if self.causes.inspecting {
            self.causes.started.insert(key);
        } else if !self.causes.started.is_empty() {
            self.causes.started.remove(&key);
        }
    }

    /// Whether the animation `key` names was started by an inspector.
    pub(crate) fn caused_by_inspector(&self, key: &CauseKey) -> bool {
        self.causes.started.contains(key)
    }

    /// Forget every inspector cause (an inspector's reset).
    pub(crate) fn clear_inspector_causes(&mut self) {
        self.causes.started.clear();
    }

    /// Whether the event `event` of the animation `key` names goes to app
    /// code: an inspector-caused animation's is withheld and listed.
    pub(crate) fn admit(&mut self, key: CauseKey, event: &str) -> bool {
        if !self.caused_by_inspector(&key) {
            return true;
        }
        let (node, of) = match key {
            CauseKey::Transition(n, _, p) => (n, p.css_name().to_string()),
            CauseKey::Custom(n, name) => (n, name),
            CauseKey::Css(n, _, name) => (n, name.to_string()),
        };
        self.causes.suppressed.push(SuppressedEvent {
            event: event.to_string(),
            node,
            of,
        });
        false
    }

    /// The events withheld since the last call, in dispatch order.
    pub(crate) fn take_suppressed(&mut self) -> Vec<SuppressedEvent> {
        std::mem::take(&mut self.causes.suppressed)
    }
}
