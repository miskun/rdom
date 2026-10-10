//! Transition and animation engine (CSS Transitions 1 / 2, CSS
//! Animations 1 / 2) — observes the computed values each cascade
//! changes, registers a transition per longhand a
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
//! ([`TuiExt::base_computed_for`]); the next style change is compared with
//! that. How each longhand interpolates is `rdom_style::animation`.
//!
//! Cost: a page with no running transition or animation does nothing
//! here per frame. A running one composites its element's style once per
//! frame, and the frame lays the page out only when a longhand layout
//! reads moved (layout is a whole-tree pass; a color is painted without
//! one); one whose longhand inherits or propagates restyles the
//! element's children too, so they inherit the running value.

use std::rc::Rc;
use std::time::Instant;

use rdom_core::{Dom, NodeId};

use crate::ext::{StyleSlot, TuiExt};
use crate::style::ComputedStyle;

/// The animated longhand of a transition (CSS Transitions 1 §6: the
/// event's `propertyName`).
pub use rdom_style::animation::Longhand;

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
    /// Each with the time on the app's clock it happened, so a frame's
    /// transition and animation events go out merged by time.
    pending_events: Vec<(Instant, PendingEvent)>,
    /// Running transitions of registered custom properties
    /// (`custom.rs`), their events, the registrations they follow, and
    /// the elements whose animated values moved.
    custom: Vec<custom::CustomAnimation>,
    custom_events: Vec<(Instant, PendingCustomEvent)>,
    registered: Rc<crate::style::cascade::PropertyRegistry>,
    restyle: Vec<NodeId>,
    /// The CSS animations (`css`), their queued events, the targets whose
    /// animations were dropped since the last frame (composited back to
    /// their cascaded style), and the sheet set they were matched under.
    css: Vec<css::CssAnimation>,
    css_events: Vec<css::PendingAnimationEvent>,
    css_cancelled: Vec<(NodeId, StyleSlot)>,
    css_stamp: usize,
    /// The transitions a reversal started (`reversing`).
    reversing: Vec<reversing::Reversing>,
    /// The targets in skipped contents at the last layout (`schedule`):
    /// their animations run but ask for no frames.
    throttled: std::collections::HashSet<(NodeId, StyleSlot)>,
    /// The elements the last transition hook found newly rendered
    /// (`diff`): a restyle later in the same frame — the layout's,
    /// `diff_layout_restyles_in` — is still their first style, no style
    /// change (CSS Transitions 1 §3).
    fresh: std::collections::HashSet<NodeId>,
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
    /// A frame whose transition hook does not run: no element is newly
    /// rendered in it (`diff_layout_restyles_in`).
    pub(crate) fn forget_fresh(&mut self) {
        self.fresh.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty() && self.custom.is_empty() && self.css.is_empty()
    }

    pub fn len(&self) -> usize {
        self.active.len() + self.custom.len() + self.css.len()
    }

    pub fn take_pending_events(&mut self) -> Vec<PendingEvent> {
        self.take_timed_events()
            .into_iter()
            .map(|(_, e)| e)
            .collect()
    }

    /// [`take_pending_events`](Self::take_pending_events), each with the
    /// time on the app's clock it happened (CSS Transitions 1 §6: a
    /// transition's start and end at its delay's and duration's end).
    pub(crate) fn take_timed_events(&mut self) -> Vec<(Instant, PendingEvent)> {
        std::mem::take(&mut self.pending_events)
    }

    /// Queue `old`'s `transitioncancel` — unless it completed: CSS
    /// Transitions 2 §4 fires one only for a transition going idle from
    /// "not idle and not after", and one whose `transitionend` is queued
    /// is in its after phase (C16G-TRANSITION-CANCEL).
    fn cancel_event(&mut self, old: &ActiveAnimation, now: Instant) {
        if old.ended_dispatched {
            return;
        }
        push_at(
            &mut self.pending_events,
            now,
            PendingEvent {
                node: old.node,
                slot: old.slot,
                kind: TransitionEventKind::Cancel,
                property: old.property,
                elapsed_seconds: old.active_time(now).as_secs_f32(),
            },
        );
    }

    /// The running transition of `property` on `(node, slot)`: one whose
    /// `transitionend` is queued has completed (CSS Transitions 1 §3) and
    /// stays only until the frame retires it.
    fn running(&self, node: NodeId, slot: StyleSlot, property: Longhand) -> Option<usize> {
        self.active.iter().position(|a| {
            a.node == node && a.slot == slot && a.property == property && !a.ended_dispatched
        })
    }

    /// Register a new animation, replacing any running one for the same
    /// longhand: that one is cancelled (`transitioncancel`) and the new
    /// one starts from its current value (CSS Transitions 1 §3).
    pub(super) fn register(&mut self, mut anim: ActiveAnimation, now: Instant) {
        // A completed one of the same longhand goes silently: no cancel,
        // and nothing to reverse (§3 "running transition").
        self.active.retain(|a| {
            !(a.ended_dispatched
                && a.node == anim.node
                && a.slot == anim.slot
                && a.property == anim.property)
        });
        if let Some(pos) = self.running(anim.node, anim.slot, anim.property) {
            let old = self.active.swap_remove(pos);
            self.cancel_event(&old, now);
            self.reverse(&old, &mut anim, now);
            anim.from = old.current_style(now);
        }
        // CSS Transitions 1 §6: `transitionrun` when it is created.
        push_at(
            &mut self.pending_events,
            now,
            PendingEvent {
                node: anim.node,
                slot: anim.slot,
                kind: TransitionEventKind::Run,
                property: anim.property,
                elapsed_seconds: anim.skipped.as_secs_f32(),
            },
        );
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
            anim.stepped_at = Some(now);
            let target = (anim.node, anim.slot);
            if !targets.contains(&target) {
                targets.push(target);
            }
            queue_transition_events(anim, now, &mut self.pending_events);
            if anim.is_done(now) {
                // The cascade's style holds the end value from now on.
                let done = self.active.swap_remove(i);
                if done.property.reaches_descendants() && done.slot == StyleSlot::Host {
                    self.restyle_children(dom, done.node);
                }
            } else {
                i += 1;
            }
        }
        self.prune_reversing();
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
    /// the value of a longhand layout reads — animated now or before —
    /// changed what layout reads of it (`Longhand::moves_boxes`): the
    /// frame lays out (a stepped `visibility` blink only at its flips, a
    /// turning `rotate()` never).
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
        let old = ext.computed_for(slot).cloned();
        let boxes = (slot == StyleSlot::Host).then_some((ext.layout, ext.content_layout));
        let mut animated: Vec<Longhand> = running.iter().map(|a| a.property).collect();
        let style = if running.is_empty() && css.is_empty() {
            None
        } else {
            let Some(base) = ext.base_computed_for(slot) else {
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
        let calc_size = style
            .as_ref()
            .is_some_and(crate::style::doc_flags::is_calc_sized);
        let moved: Vec<Longhand> = animated
            .iter()
            .chain(&before)
            .copied()
            .filter(|l| l.affects_layout())
            .collect();
        ext.composite(slot, animated, style);
        let layout = match (old.as_deref(), ext.computed_for(slot)) {
            (Some(old), Some(new)) => moved.iter().any(|l| moves::moves(*l, old, new, boxes)),
            _ => !moved.is_empty(),
        };
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
        self.pending_events.push((Instant::now(), e));
    }
}

/// Queue `event`, which happened at `at`.
fn push_at<E>(queue: &mut Vec<(Instant, E)>, at: Instant, event: E) {
    queue.push((at, event));
}

/// Queue the events `anim` has reached at `now` and not yet queued (CSS
/// Transitions 1 §6): `transitionstart` past its delay, `transitionend`
/// once done, each at the time on the app's clock it happened.
fn queue_transition_events(
    anim: &mut ActiveAnimation,
    now: Instant,
    events: &mut Vec<(Instant, PendingEvent)>,
) {
    if !anim.started_dispatched && !anim.in_delay(now) {
        anim.started_dispatched = true;
        let at = (anim.started_at + anim.delay).min(now);
        push_at(
            events,
            at,
            PendingEvent {
                node: anim.node,
                slot: anim.slot,
                kind: TransitionEventKind::Start,
                property: anim.property,
                elapsed_seconds: anim.skipped.as_secs_f32(),
            },
        );
    }
    if !anim.ended_dispatched && anim.is_done(now) {
        anim.ended_dispatched = true;
        let at = (anim.started_at + anim.delay + anim.duration).min(now);
        push_at(
            events,
            at,
            PendingEvent {
                node: anim.node,
                slot: anim.slot,
                kind: TransitionEventKind::End,
                property: anim.property,
                elapsed_seconds: anim.duration.as_secs_f32(),
            },
        );
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

mod active;
mod css;
mod custom;
mod diff;
#[cfg(test)]
mod longhand_tests;
mod moves;
mod reversing;
mod rule;
mod schedule;
mod teardown;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_tests;
#[cfg(test)]
mod visibility_tests;

pub use crate::style::AnimationPlayState;
pub(crate) use active::ActiveAnimation;
pub use css::{AnimationInfo, AnimationKind};
pub(crate) use css::{CssInputs, PendingAnimationEvent, slot_order};
pub use custom::PendingCustomEvent;
#[cfg(test)]
pub(crate) use diff::DIFF_VISITS;
pub use diff::{diff_and_register, diff_and_register_with, settle_restyled};
pub(crate) use diff::{diff_and_register_in, diff_layout_restyles_in};
#[cfg(test)]
pub(crate) use teardown::TEARDOWN_STEPS;
#[cfg(test)]
mod behavior_tests;
#[cfg(test)]
mod custom_tests;
