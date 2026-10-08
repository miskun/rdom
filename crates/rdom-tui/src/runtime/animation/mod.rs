//! Transition and animation engine (CSS Transitions 1 / 2, CSS
//! Animations 1 / 2) — observes the computed values each cascade
//! changes, registers an [`ActiveAnimation`] per longhand a
//! `transition-*` rule covers and a CSS animation per `animation-name`
//! entry (`css`), and each frame composites the running values onto the
//! element's computed style (Web Animations 1 §5.4.5: the effect stack's
//! result *is* the computed value) — the transitions first, the CSS
//! animations above them.
//!
//! So layout, paint, hit-testing and inheritance all read one style —
//! [`TuiExt::computed_for`] — and a `height` transition grows the box
//! frame by frame as a `color` transition recolors it. The cascade's own
//! style (the *after-change style*) is kept beside it
//! ([`TuiExt::cascaded_for`]); the next style change is compared with
//! that. How each longhand interpolates is `rdom_style::animation`.
//!
//! Cost: a page with no running transition or animation does nothing
//! here per frame. A running one composites its element's style once per
//! frame, and the frame lays the page out only when a longhand layout
//! reads moved (layout is a whole-tree pass; a color is painted without
//! one); one whose longhand inherits or propagates restyles the
//! element's children too, so they inherit the running value.

use std::rc::Rc;
use std::time::{Duration, Instant};

use rdom_core::{Dom, NodeId};
use rdom_style::color::ColorScheme;

use crate::ext::{StyleSlot, TuiExt};
use crate::style::ComputedStyle;
use crate::style::transition::TimingFunction;

/// The animated longhand of a transition (CSS Transitions 1 §6: the
/// event's `propertyName`).
pub use rdom_style::animation::Longhand;

// ── Active animation ──────────────────────────────────────────────

/// One running transition of one longhand of one element style.
#[derive(Debug, Clone)]
pub struct ActiveAnimation {
    pub node: NodeId,
    /// The element itself or one of its pseudo-elements.
    pub slot: StyleSlot,
    pub property: Longhand,
    /// The start value: the before-change style's (CSS Transitions 1 §3),
    /// or a style holding a replaced transition's value at the moment.
    pub from: Rc<ComputedStyle>,
    /// The end value: the after-change style's.
    pub to: Rc<ComputedStyle>,
    /// When the delay began: registration, or earlier by the part a
    /// negative delay skips. The visual start is `started_at + delay`.
    pub started_at: Instant,
    /// The delay left to wait (zero for a negative `transition-delay`).
    pub delay: Duration,
    pub duration: Duration,
    /// The part of the duration a negative delay skipped (CSS Transitions
    /// 1 §2.4) — the `elapsedTime` of `transitionrun` / `transitionstart`.
    pub skipped: Duration,
    pub timing: TimingFunction,
    /// The element's used color scheme when the transition started: what
    /// a `reset` endpoint interpolates as (CSS Color Adjust 1 §2.1).
    pub scheme: ColorScheme,
    /// Whether `transitionstart` fired (on the first tick past the delay).
    pub started_dispatched: bool,
}

impl ActiveAnimation {
    /// Linear progress in [0, 1]; 0 before the delay elapses.
    fn progress(&self, now: Instant) -> f32 {
        if self.in_delay(now) {
            return 0.0;
        }
        let elapsed = now.saturating_duration_since(self.started_at);
        if self.duration.is_zero() {
            return 1.0;
        }
        ((elapsed - self.delay).as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Whether `now` is in the before phase — the delay.
    fn in_delay(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) < self.delay
    }

    /// True once `now >= started_at + delay + duration`.
    fn is_done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) >= self.delay + self.duration
    }

    /// Write the eased current value into `out`.
    fn apply(&self, now: Instant, out: &mut ComputedStyle) {
        // CSS Easing 1 §2.3.1: the before flag holds a step back in the
        // delay.
        let p = self.progress(now);
        let t = f64::from(if self.in_delay(now) {
            self.timing.ease_before(p)
        } else {
            self.timing.ease(p)
        });
        self.property
            .interpolate(&self.from, &self.to, t, self.scheme, out);
    }

    /// A style holding the current value — `to`'s, with this longhand at
    /// `now`: the start of a transition that replaces this one.
    fn current_style(&self, now: Instant) -> Rc<ComputedStyle> {
        let mut out = (*self.to).clone();
        self.apply(now, &mut out);
        Rc::new(out)
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
    registered: Rc<crate::style::cascade::PropertyRegistry>,
    restyle: Vec<NodeId>,
    /// The CSS animations (`css`), their queued events, the targets whose
    /// animations were dropped since the last frame (composited back to
    /// their cascaded style), and the sheet set they were matched under.
    css: Vec<css::CssAnimation>,
    css_events: Vec<css::PendingAnimationEvent>,
    css_cancelled: Vec<(NodeId, StyleSlot)>,
    css_stamp: usize,
}

/// What one frame's [`AnimationRegistry::advance`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Advanced {
    /// Element styles composited.
    pub composites: u32,
    /// A longhand layout reads moved: the frame must lay out.
    pub layout: bool,
}

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub node: NodeId,
    pub slot: StyleSlot,
    pub kind: TransitionEventKind,
    pub property: Longhand,
    pub elapsed_seconds: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransitionEventKind {
    /// `transitionrun`: the transition was created, before its delay.
    Run,
    Start,
    End,
    Cancel,
}

impl AnimationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// No transition or animation is registered (a finished CSS
    /// animation stays registered while its name is listed).
    pub fn is_empty(&self) -> bool {
        self.active.is_empty() && self.custom.is_empty() && self.css.is_empty()
    }

    pub fn len(&self) -> usize {
        self.active.len() + self.custom.len() + self.css.len()
    }

    /// Whether the next frame must step something with the clock: a
    /// transition runs, or a CSS animation plays on the document
    /// timeline before or in its active interval. A paused or finished
    /// animation holds its value without frames.
    pub fn needs_frames(&self, now: Instant) -> bool {
        !self.active.is_empty()
            || !self.custom.is_empty()
            || self.css.iter().any(|a| a.needs_frames(now))
    }

    pub fn take_pending_events(&mut self) -> Vec<PendingEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn cancel_event(&mut self, old: &ActiveAnimation, now: Instant) {
        self.pending_events.push(PendingEvent {
            node: old.node,
            slot: old.slot,
            kind: TransitionEventKind::Cancel,
            property: old.property,
            elapsed_seconds: now.saturating_duration_since(old.started_at).as_secs_f32(),
        });
    }

    /// The running transition of `property` on `(node, slot)`.
    fn running(&self, node: NodeId, slot: StyleSlot, property: Longhand) -> Option<usize> {
        self.active
            .iter()
            .position(|a| a.node == node && a.slot == slot && a.property == property)
    }

    /// Register a new animation, replacing any running one for the same
    /// longhand: that one is cancelled (`transitioncancel`) and the new
    /// one starts from its current value (CSS Transitions 1 §3).
    pub(super) fn register(&mut self, mut anim: ActiveAnimation, now: Instant) {
        if let Some(pos) = self.running(anim.node, anim.slot, anim.property) {
            let old = self.active.swap_remove(pos);
            self.cancel_event(&old, now);
            anim.from = old.current_style(now);
        }
        // CSS Transitions 1 §6: `transitionrun` when it is created.
        self.pending_events.push(PendingEvent {
            node: anim.node,
            slot: anim.slot,
            kind: TransitionEventKind::Run,
            property: anim.property,
            elapsed_seconds: anim.skipped.as_secs_f32(),
        });
        self.active.push(anim);
    }

    /// Cancel the running transition of `property` on `(node, slot)`, if
    /// any (its rule no longer covers it, or its value stopped
    /// interpolating); `true` if there was one.
    pub(super) fn cancel(
        &mut self,
        node: NodeId,
        slot: StyleSlot,
        property: Longhand,
        now: Instant,
    ) -> bool {
        let Some(pos) = self.running(node, slot, property) else {
            return false;
        };
        let old = self.active.swap_remove(pos);
        self.cancel_event(&old, now);
        true
    }

    /// The longhands running on `(node, slot)`.
    pub(super) fn running_on(&self, node: NodeId, slot: StyleSlot) -> Vec<Longhand> {
        self.active
            .iter()
            .filter(|a| a.node == node && a.slot == slot)
            .map(|a| a.property)
            .collect()
    }

    /// Cancel every in-flight animation on `node` (e.g. node
    /// removed from DOM, display:none cascaded onto it).
    pub fn cancel_for_node(&mut self, node: NodeId, now: Instant) {
        self.cancel_custom_for_node(node, now);
        self.cancel_css_for_node(node, now);
        let mut i = 0;
        while i < self.active.len() {
            if self.active[i].node == node {
                let a = self.active.swap_remove(i);
                self.cancel_event(&a, now);
            } else {
                i += 1;
            }
        }
    }

    /// Advance every running transition to `now`: fire
    /// `transitionstart` past the delay, retire the finished ones
    /// (`transitionend`), and composite the running values onto each
    /// animated style ([`TuiExt::computed_for`]). The elements whose
    /// running values reach their descendants have their children queued
    /// for a restyle ([`take_restyle`](Self::take_restyle)).
    pub fn advance(&mut self, dom: &mut Dom<TuiExt>, now: Instant) {
        self.advance_frame(dom, now);
    }

    /// [`advance`](Self::advance), reporting what it did.
    pub(crate) fn advance_frame(&mut self, dom: &mut Dom<TuiExt>, now: Instant) -> Advanced {
        // An element out of the document has its transitions and
        // animations cancelled; a dropped one's go with it, silently.
        if !self.is_empty() {
            self.cancel_disconnected(dom, now);
        }
        self.advance_custom(dom, now);
        if self.active.is_empty() && self.css.is_empty() && self.css_cancelled.is_empty() {
            return Advanced::default();
        }
        let mut targets: Vec<(NodeId, StyleSlot)> = Vec::new();
        let mut i = 0;
        while i < self.active.len() {
            let anim = &mut self.active[i];
            let target = (anim.node, anim.slot);
            if !targets.contains(&target) {
                targets.push(target);
            }
            if !anim.started_dispatched && !anim.in_delay(now) {
                anim.started_dispatched = true;
                self.pending_events.push(PendingEvent {
                    node: anim.node,
                    slot: anim.slot,
                    kind: TransitionEventKind::Start,
                    property: anim.property,
                    elapsed_seconds: anim.skipped.as_secs_f32(),
                });
            }
            if anim.is_done(now) {
                // The cascade's style holds the end value from now on.
                let done = self.active.swap_remove(i);
                self.pending_events.push(PendingEvent {
                    node: done.node,
                    slot: done.slot,
                    kind: TransitionEventKind::End,
                    property: done.property,
                    elapsed_seconds: done.duration.as_secs_f32(),
                });
                if done.property.reaches_descendants() && done.slot == StyleSlot::Host {
                    self.restyle_children(dom, done.node);
                }
            } else {
                i += 1;
            }
        }
        self.step_css(dom, now, &mut targets);
        let mut out = Advanced::default();
        for (node, slot) in targets {
            if !dom.contains(node) {
                continue;
            }
            out.composites += 1;
            out.layout |= self.composite(dom, node, slot, now);
        }
        out
    }

    /// Composite the running transitions of `(node, slot)` onto its
    /// cascaded style, then its CSS animations over them (Web Animations
    /// 1 §5.4.5: CSS animations sort above CSS transitions). `true` when
    /// a longhand layout reads was or is animated: the frame lays out.
    fn composite(
        &mut self,
        dom: &mut Dom<TuiExt>,
        node: NodeId,
        slot: StyleSlot,
        now: Instant,
    ) -> bool {
        let running: Vec<&ActiveAnimation> = self
            .active
            .iter()
            .filter(|a| a.node == node && a.slot == slot)
            .collect();
        let css = self.css_on(node, slot);
        let mut node_mut = dom.node_mut(node);
        let Some(ext) = node_mut.ext_mut() else {
            return false;
        };
        let before: Vec<Longhand> = ext
            .presentation_for(slot)
            .map(|p| p.animated().to_vec())
            .unwrap_or_default();
        let mut animated: Vec<Longhand> = running.iter().map(|a| a.property).collect();
        let style = if running.is_empty() && css.is_empty() {
            None
        } else {
            let Some(base) = ext.cascaded_for(slot) else {
                return false;
            };
            let mut style = (**base).clone();
            for a in &running {
                a.apply(now, &mut style);
            }
            for a in &css {
                for l in a.apply(&mut style) {
                    if !animated.contains(&l) {
                        animated.push(l);
                    }
                }
            }
            Some(style)
        };
        let reaches = animated.iter().any(|l| l.reaches_descendants());
        let layout = animated.iter().chain(&before).any(|l| l.affects_layout());
        let calc_size = style
            .as_ref()
            .is_some_and(crate::style::doc_flags::is_calc_sized);
        ext.composite(slot, animated, style);
        if calc_size {
            crate::style::doc_flags::note_calc_size(dom);
        }
        if reaches && slot == StyleSlot::Host {
            self.restyle_children(dom, node);
        }
        layout
    }

    /// Queue `node`'s children (in the box tree) for a restyle: they
    /// inherit its new value.
    fn restyle_children(&mut self, dom: &Dom<TuiExt>, node: NodeId) {
        for child in crate::render::box_tree::children(dom, node) {
            if dom.node(child).node_type() == rdom_core::NodeType::Element {
                self.restyle.push(child);
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

/// Whether `style`'s transitions take the longhand `name` when its values
/// do not interpolate: a rule covers it with a positive combined duration
/// and `transition-behavior: allow-discrete` (CSS Transitions 2 §3.1).
pub(crate) fn transitions_discretely(style: &ComputedStyle, name: &str) -> bool {
    let Some(l) = Longhand::from_name(name) else {
        return false;
    };
    rule::Coverage::of(style).rule(style, l).is_some_and(|r| {
        r.runs() && r.behavior == crate::style::transition::TransitionBehavior::AllowDiscrete
    })
}

// ── Effective-value helpers ───────────────────────────────────────

/// The element's `color`: its computed value, which holds a running
/// transition's value.
pub fn effective_fg(ext: &TuiExt) -> crate::style::Color {
    ext.computed
        .as_ref()
        .map_or(crate::style::Color::Reset, |c| c.fg)
}

/// The element's `background-color` (see [`effective_fg`]).
pub fn effective_bg(ext: &TuiExt) -> crate::style::Color {
    ext.computed
        .as_ref()
        .map_or(crate::style::Color::Reset, |c| c.bg)
}

/// The element's `border-*-color`s (see [`effective_fg`]).
pub fn effective_border_color(ext: &TuiExt) -> crate::layout::Sides<crate::style::Color> {
    ext.computed
        .as_ref()
        .map_or(crate::layout::Sides::all(crate::style::Color::Reset), |c| {
            c.border_color
        })
}

/// The element's `padding` (see [`effective_fg`]).
pub fn effective_padding(ext: &TuiExt) -> crate::layout::Padding {
    ext.computed
        .as_ref()
        .map(|c| c.padding.clone())
        .unwrap_or_default()
}

mod css;
mod custom;
mod diff;
#[cfg(test)]
mod longhand_tests;
mod rule;
mod teardown;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_tests;
#[cfg(test)]
mod visibility_tests;

pub use crate::style::AnimationPlayState;
pub use css::{AnimationEventKind, AnimationInfo, AnimationKind};
pub(crate) use css::{CssInputs, slot_order};
pub use custom::PendingCustomEvent;
pub(crate) use diff::diff_and_register_in;
pub use diff::{diff_and_register, diff_and_register_with, settle_restyled};
#[cfg(test)]
mod behavior_tests;
#[cfg(test)]
mod custom_tests;
