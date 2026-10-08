//! What `Element.getAnimations()` reports (Web Animations 1 §6.7,
//! `Animatable`), as rdom reads it: [`AnimationRegistry::animations_of`].

use std::sync::Arc;
use std::time::Instant;

use rdom_core::NodeId;

use super::super::{AnimationRegistry, Longhand};
use crate::style::AnimationPlayState;

/// What kind of animation an [`AnimationInfo`] is, and what names it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnimationKind {
    /// A CSS animation (`CSSAnimation.animationName`).
    CssAnimation(Arc<str>),
    /// A CSS transition (`CSSTransition.transitionProperty`).
    CssTransition(Longhand),
}

/// One animation running on an element or its pseudo-elements.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AnimationInfo {
    kind: AnimationKind,
    pseudo_element: Option<&'static str>,
    play_state: AnimationPlayState,
    current_time_ms: Option<f64>,
    timeline_progress: Option<f64>,
}

impl AnimationInfo {
    /// The animation's kind and name.
    pub fn kind(&self) -> &AnimationKind {
        &self.kind
    }

    /// The pseudo-element it runs on (`"::before"`), `None` on the element.
    pub fn pseudo_element(&self) -> Option<&'static str> {
        self.pseudo_element
    }

    /// `running` or `paused` (a transition always runs).
    pub fn play_state(&self) -> AnimationPlayState {
        self.play_state
    }

    /// Its current time in ms (`Animation.currentTime`) on the document
    /// timeline: its local time, its delay included; `None` on a progress
    /// timeline or while its timeline is inactive.
    pub fn current_time_ms(&self) -> Option<f64> {
        self.current_time_ms
    }

    /// On a scroll or view progress timeline, where the scroll offset
    /// stood at the last frame in the animation's attachment range: 0 at
    /// its start, 1 at its end (outside before and after it).
    pub fn timeline_progress(&self) -> Option<f64> {
        self.timeline_progress
    }
}

impl AnimationRegistry {
    /// The animations on `node` and its pseudo-elements at `now`:
    /// its transitions, then its CSS animations in composite order —
    /// `Element.getAnimations({ subtree: false })` with the
    /// pseudo-elements' included.
    pub fn animations_of(&self, node: NodeId, now: Instant) -> Vec<AnimationInfo> {
        let mut out: Vec<AnimationInfo> = self
            .active
            .iter()
            .filter(|a| a.node == node)
            .map(|a| AnimationInfo {
                kind: AnimationKind::CssTransition(a.property),
                pseudo_element: a.slot.pseudo_element(),
                play_state: AnimationPlayState::Running,
                current_time_ms: Some(
                    now.saturating_duration_since(a.started_at).as_secs_f64() * 1000.0,
                ),
                timeline_progress: None,
            })
            .collect();
        let mut css: Vec<_> = self.css.iter().filter(|a| a.node == node).collect();
        css.sort_by_key(|a| (slot_order(a.slot), a.index));
        out.extend(css.into_iter().map(|a| AnimationInfo {
            kind: AnimationKind::CssAnimation(a.name.clone()),
            pseudo_element: a.slot.pseudo_element(),
            play_state: a.play_state,
            current_time_ms: a.clock_time(now),
            timeline_progress: a.fraction,
        }));
        out
    }
}

/// The composite order of an element's styles (CSS Animations 2 §3.1:
/// the element, `::marker`, `::before`, the others, `::after`).
pub(crate) fn slot_order(slot: crate::ext::StyleSlot) -> u8 {
    use crate::ext::StyleSlot;
    match slot {
        StyleSlot::Host => 0,
        StyleSlot::Marker => 1,
        StyleSlot::Before => 2,
        StyleSlot::After => 4,
        _ => 3,
    }
}
