//! Transition engine — observes property changes between cascade
//! passes, registers an `ActiveAnimation`, and writes interpolated
//! values into `TuiExt.presentation` each tick.
//!
//! Paint / layout / hit-test read the live values via the
//! `effective_*` helpers below — they fall back to `ComputedStyle`
//! when no animation is in flight.

use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId};

use crate::ext::{StyleSlot, TuiExt};
use crate::layout::{Length, Size, ZIndex};
use crate::style::Color;
use crate::style::transition::{AnimatableProperty, TimingFunction};

// ── Property identity (engine-internal) ───────────────────────────

/// Internal property identity tracked by an animation. Maps 1:1 to
/// `AnimatableProperty` plus the cascade-internal modifiers stored
/// on `ComputedStyle.modifiers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnimatedProp {
    Fg,
    Bg,
    BorderFg,
    Width,
    Height,
    Padding,
    Gap,
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,
}

impl AnimatedProp {
    pub fn css_name(self) -> &'static str {
        match self {
            AnimatedProp::Fg => "color",
            AnimatedProp::Bg => "background-color",
            AnimatedProp::BorderFg => "border-color",
            AnimatedProp::Width => "width",
            AnimatedProp::Height => "height",
            AnimatedProp::Padding => "padding",
            AnimatedProp::Gap => "gap",
            AnimatedProp::Top => "top",
            AnimatedProp::Right => "right",
            AnimatedProp::Bottom => "bottom",
            AnimatedProp::Left => "left",
            AnimatedProp::ZIndex => "z-index",
        }
    }

    /// The runtime property a named `transition-property` animates, or
    /// `None` for one this runtime does not interpolate
    /// (`AnimatableProperty` is `#[non_exhaustive]`): such a property
    /// changes discretely, as CSS Transitions 1 §2 does for any
    /// property that is not animatable.
    fn from_animatable(ap: AnimatableProperty) -> Option<Self> {
        Some(match ap {
            AnimatableProperty::Color => AnimatedProp::Fg,
            AnimatableProperty::BackgroundColor => AnimatedProp::Bg,
            AnimatableProperty::BorderColor => AnimatedProp::BorderFg,
            AnimatableProperty::Width => AnimatedProp::Width,
            AnimatableProperty::Height => AnimatedProp::Height,
            AnimatableProperty::Padding => AnimatedProp::Padding,
            AnimatableProperty::Gap => AnimatedProp::Gap,
            AnimatableProperty::Top => AnimatedProp::Top,
            AnimatableProperty::Right => AnimatedProp::Right,
            AnimatableProperty::Bottom => AnimatedProp::Bottom,
            AnimatableProperty::Left => AnimatedProp::Left,
            AnimatableProperty::ZIndex => AnimatedProp::ZIndex,
            _ => return None,
        })
    }
}

/// Boxed value of any animatable property. The variant matches
/// the property type 1:1; mismatched lerps just snap at midpoint.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AnimatedValue {
    Color(Color),
    Size(Size),
    Length(Length),
    U16(u16),
    Padding(crate::layout::Padding),
    ZIndex(ZIndex),
}

// ── Active animation ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActiveAnimation {
    pub node: NodeId,
    /// The element itself or one of its pseudo-elements.
    pub slot: StyleSlot,
    pub property: AnimatedProp,
    pub from: AnimatedValue,
    pub to: AnimatedValue,
    /// Set when the animation was registered. The visual start is
    /// `started_at + delay`.
    pub started_at: Instant,
    pub delay: Duration,
    pub duration: Duration,
    pub timing: TimingFunction,
    /// Tracks whether `transitionstart` already fired (after delay
    /// elapses). The engine's tick advance dispatches
    /// `transitionstart` on the first tick where now ≥ started_at
    /// + delay, then sets this true.
    pub started_dispatched: bool,
}

impl ActiveAnimation {
    /// Linear progress in [0, 1]. Reaches 0 before delay elapses.
    fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        if elapsed < self.delay {
            return 0.0;
        }
        let post_delay = elapsed - self.delay;
        if self.duration.is_zero() {
            return 1.0;
        }
        let t = post_delay.as_secs_f32() / self.duration.as_secs_f32();
        t.clamp(0.0, 1.0)
    }

    /// True once `now >= started_at + delay + duration`.
    fn is_done(&self, now: Instant) -> bool {
        let total = self.delay + self.duration;
        now.saturating_duration_since(self.started_at) >= total
    }

    /// Eased current value. Clamped to `to` once t reaches 1.0.
    fn current(&self, now: Instant) -> AnimatedValue {
        let t = self.timing.ease(self.progress(now));
        interpolate(&self.from, &self.to, t)
    }
}

// ── Registry ──────────────────────────────────────────────────────

/// All in-flight transitions for the App. Live in
/// `App.animations`; the cascade hook adds entries, the tick loop
/// advances + retires them.
#[derive(Debug, Default)]
pub struct AnimationRegistry {
    active: Vec<ActiveAnimation>,
    /// Events the engine wants the runtime to dispatch on the
    /// next event-pump cycle. The App drains this via
    /// `take_pending_events` after every tick.
    pending_events: Vec<PendingEvent>,
    /// Running transitions of registered custom properties
    /// (`custom.rs`), their events, the registrations they follow, and
    /// the elements whose animated values moved.
    custom: Vec<custom::CustomAnimation>,
    custom_events: Vec<PendingCustomEvent>,
    registered: std::rc::Rc<crate::style::cascade::PropertyRegistry>,
    restyle: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub node: NodeId,
    pub slot: StyleSlot,
    pub kind: TransitionEventKind,
    pub property: AnimatedProp,
    pub elapsed_seconds: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransitionEventKind {
    Start,
    End,
    Cancel,
}

impl AnimationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty() && self.custom.is_empty()
    }

    pub fn len(&self) -> usize {
        self.active.len() + self.custom.len()
    }

    pub fn take_pending_events(&mut self) -> Vec<PendingEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Register a new animation, replacing any existing one for
    /// the same (node, property). The replaced animation fires
    /// `transitioncancel`. The new animation's `from` is the
    /// previous's *current interpolated* value when an animation
    /// is replaced mid-flight (CSS-faithful).
    pub(super) fn register(&mut self, mut anim: ActiveAnimation, now: Instant) {
        if let Some(pos) = self
            .active
            .iter()
            .position(|a| a.node == anim.node && a.slot == anim.slot && a.property == anim.property)
        {
            let old = self.active.swap_remove(pos);
            self.pending_events.push(PendingEvent {
                node: old.node,
                slot: old.slot,
                kind: TransitionEventKind::Cancel,
                property: old.property,
                elapsed_seconds: now.saturating_duration_since(old.started_at).as_secs_f32(),
            });
            // New animation's `from` becomes the interpolated
            // value of the old one (avoids a discontinuity).
            anim.from = old.current(now);
        }
        self.active.push(anim);
    }

    /// Cancel every in-flight animation on `node` (e.g. node
    /// removed from DOM, display:none cascaded onto it).
    pub fn cancel_for_node(&mut self, node: NodeId, now: Instant) {
        self.cancel_custom_for_node(node, now);
        let mut i = 0;
        while i < self.active.len() {
            if self.active[i].node == node {
                let a = self.active.swap_remove(i);
                self.pending_events.push(PendingEvent {
                    node: a.node,
                    slot: a.slot,
                    kind: TransitionEventKind::Cancel,
                    property: a.property,
                    elapsed_seconds: now.saturating_duration_since(a.started_at).as_secs_f32(),
                });
            } else {
                i += 1;
            }
        }
    }

    /// Advance every active animation, write interpolated values
    /// into `TuiExt.presentation`, fire transitionstart /
    /// transitionend events as appropriate. Removes finished
    /// entries. Caller pumps the resulting events afterwards.
    pub fn advance(&mut self, dom: &mut Dom<TuiExt>, now: Instant) {
        self.advance_custom(dom, now);
        let mut i = 0;
        while i < self.active.len() {
            let anim = &mut self.active[i];

            // Fire transitionstart once delay elapses.
            let elapsed = now.saturating_duration_since(anim.started_at);
            if !anim.started_dispatched && elapsed >= anim.delay {
                anim.started_dispatched = true;
                self.pending_events.push(PendingEvent {
                    node: anim.node,
                    slot: anim.slot,
                    kind: TransitionEventKind::Start,
                    property: anim.property,
                    elapsed_seconds: 0.0,
                });
            }

            // Compute current value + write to presentation.
            let value = anim.current(now);
            write_presentation(dom, anim.node, anim.slot, anim.property, value);

            // Retire on completion.
            if anim.is_done(now) {
                let finished = self.active.swap_remove(i);
                // Clear the presentation slot — the committed
                // value in ComputedStyle is the truth from now on.
                clear_presentation(dom, finished.node, finished.slot, finished.property);
                self.pending_events.push(PendingEvent {
                    node: finished.node,
                    slot: finished.slot,
                    kind: TransitionEventKind::End,
                    property: finished.property,
                    elapsed_seconds: finished.duration.as_secs_f32(),
                });
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
impl AnimationRegistry {
    /// Inject a `PendingEvent` directly into the queue, bypassing
    /// `advance`. Lets tests drive `dispatch_animation_events`
    /// without setting up a real-time-driven transition.
    pub(crate) fn queue_event_for_test(&mut self, e: PendingEvent) {
        self.pending_events.push(e);
    }
}

// ── Effective-value helpers (read by paint/layout/hit-test) ───────

pub fn effective_fg(ext: &TuiExt) -> Color {
    ext.presentation
        .as_deref()
        .and_then(|p| p.fg)
        .or(ext.computed.as_ref().map(|c| c.fg))
        .unwrap_or(Color::Reset)
}

pub fn effective_bg(ext: &TuiExt) -> Color {
    ext.presentation
        .as_deref()
        .and_then(|p| p.bg)
        .or(ext.computed.as_ref().map(|c| c.bg))
        .unwrap_or(Color::Reset)
}

pub fn effective_border_fg(ext: &TuiExt) -> Color {
    ext.presentation
        .as_deref()
        .and_then(|p| p.border_fg)
        .or(ext.computed.as_ref().map(|c| c.border_fg))
        .unwrap_or(Color::Reset)
}

pub fn effective_padding(ext: &TuiExt) -> crate::layout::Padding {
    ext.presentation
        .as_deref()
        .and_then(|p| p.padding.clone())
        .or_else(|| ext.computed.as_ref().map(|c| c.padding.clone()))
        .unwrap_or_default()
}

mod custom;
mod diff;
mod interpolate;
#[cfg(test)]
mod tests;

pub use custom::PendingCustomEvent;
use diff::{clear_presentation, write_presentation};
pub use diff::{diff_and_register, settle_restyled};
use interpolate::interpolate;
#[cfg(test)]
mod custom_tests;
