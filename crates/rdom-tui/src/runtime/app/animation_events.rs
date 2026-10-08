//! Dispatching the transition and animation events of a frame (CSS
//! Transitions 1 §6, CSS Animations 2 §4.2, Web Animations 1 §4.4): every
//! event the registry queued — a transition's, a registered custom
//! property's, a CSS animation's — goes out in one stream ordered by the
//! time on the app's clock it happened; at one time, transitions before
//! animations (their composite order), and the animations in theirs —
//! tree order of their elements, the element before its pseudo-elements,
//! then `animation-name` order (C12G-MISC: transitions went out before
//! every animation event, whatever their times).

use std::time::Instant;

use rdom_core::NodeId;

use super::App;
use crate::TuiDom;
use crate::render::backend::Backend;
use crate::runtime::animation::{PendingCustomEvent, PendingEvent, TransitionEventKind};

/// One queued event of any kind.
enum Queued {
    Transition(PendingEvent),
    Custom(PendingCustomEvent),
    Animation(crate::runtime::animation::PendingAnimationEvent),
}

impl<B: Backend> App<B> {
    /// Dispatch the transition and animation events queued by the
    /// animation registry during this frame, merged by time.
    ///
    /// A transition's detail is a typed `EventDetail::Transition` (the
    /// CSS property name, the elapsed seconds, the pseudo-element), an
    /// animation's an `EventDetail::Animation`; apps read them via
    /// `event.detail.as_transition()` / `as_animation()`.
    pub(super) fn dispatch_animation_events(&mut self) {
        let transitions = self.animations.take_timed_events();
        let custom = self.animations.take_timed_custom_events();
        let mut animations = self.animations.take_pending_animation_events();
        if transitions.is_empty() && custom.is_empty() && animations.is_empty() {
            return;
        }
        // Their listeners are code the next frame's checks must see.
        self.prelude.touched = true;
        let dom = &self.dom;
        animations.sort_by(|a, b| {
            a.scheduled
                .cmp(&b.scheduled)
                .then_with(|| tree_order(dom, a.node, b.node))
                .then_with(|| {
                    let order = crate::runtime::animation::slot_order;
                    order(a.slot).cmp(&order(b.slot))
                })
                .then_with(|| a.index.cmp(&b.index))
        });
        // (time, rank): transitions sort below animations at one time; a
        // stable sort keeps each stream's own order.
        let mut queued: Vec<(Instant, u8, Queued)> = transitions
            .into_iter()
            .map(|(at, e)| (at, 0, Queued::Transition(e)))
            .chain(custom.into_iter().map(|(at, e)| (at, 0, Queued::Custom(e))))
            .chain(
                animations
                    .into_iter()
                    .map(|e| (e.scheduled, 1, Queued::Animation(e))),
            )
            .collect();
        queued.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, _, event) in queued {
            match event {
                Queued::Transition(e) => self.dispatch_transition_event(e),
                Queued::Custom(e) => self.dispatch_custom_event(e),
                Queued::Animation(e) => self.dispatch_css_animation_event(e),
            }
        }
    }

    fn dispatch_transition_event(&mut self, e: PendingEvent) {
        let PendingEvent {
            node,
            slot,
            kind,
            property,
            elapsed_seconds,
        } = e;
        // A `::details-content` box's transitions are its `<details>`'s,
        // for that pseudo-element (CSS Transitions 1 §6.1).
        let (node, pseudo_element) = match self
            .dom
            .contains(node)
            .then(|| crate::render::box_tree::slot::host_of(&self.dom, node))
            .flatten()
        {
            Some(host) => (host, Some("::details-content")),
            None => (node, slot.pseudo_element()),
        };
        let mut ev = rdom_core::Event::new(transition_event_name(kind));
        ev.detail = rdom_core::EventDetail::Transition(Box::new(rdom_core::TransitionDetail::new(
            property.css_name(),
            elapsed_seconds.into(),
            pseudo_element.map(str::to_string),
        )));
        // A listener of an earlier event in the batch may have dropped
        // `node`; a dropped element's transition events go nowhere.
        crate::tui_event::dispatch_event_to_live(&mut self.dom, node, &mut ev);
    }

    /// A registered custom property's (`--name`) transition event.
    fn dispatch_custom_event(&mut self, e: PendingCustomEvent) {
        let mut ev = rdom_core::Event::new(transition_event_name(e.kind));
        ev.detail = rdom_core::EventDetail::Transition(Box::new(rdom_core::TransitionDetail::new(
            &e.property,
            e.elapsed_seconds.into(),
            None,
        )));
        crate::tui_event::dispatch_event_to_live(&mut self.dom, e.node, &mut ev);
    }

    fn dispatch_css_animation_event(
        &mut self,
        e: crate::runtime::animation::PendingAnimationEvent,
    ) {
        // A `::details-content` box's animations are its `<details>`'s,
        // for that pseudo-element (CSS Animations 1 §5.1), as its
        // transitions are.
        let (node, pseudo_element) = match self
            .dom
            .contains(e.node)
            .then(|| crate::render::box_tree::slot::host_of(&self.dom, e.node))
            .flatten()
        {
            Some(host) => (host, Some("::details-content".to_string())),
            None => (e.node, e.slot.pseudo_element().map(str::to_string)),
        };
        // AnimationEvent bubbles and is not cancelable (CSS Animations 1
        // §5.1).
        let mut ev = rdom_core::Event::new(e.kind.event_type()).with_cancelable(false);
        ev.detail = rdom_core::EventDetail::Animation(Box::new(rdom_core::AnimationDetail::new(
            &*e.name,
            e.elapsed_seconds,
            pseudo_element,
        )));
        crate::tui_event::dispatch_event_to_live(&mut self.dom, node, &mut ev);
    }
}

fn transition_event_name(kind: TransitionEventKind) -> &'static str {
    match kind {
        TransitionEventKind::Run => "transitionrun",
        TransitionEventKind::Start => "transitionstart",
        TransitionEventKind::End => "transitionend",
        TransitionEventKind::Cancel => "transitioncancel",
    }
}

/// `a` before `b` in tree order (`Ordering::Less`); equal for one node.
fn tree_order(dom: &TuiDom, a: NodeId, b: NodeId) -> std::cmp::Ordering {
    use rdom_core::DocumentPosition;
    if a == b || !dom.contains(a) || !dom.contains(b) {
        return std::cmp::Ordering::Equal;
    }
    let p = dom.compare_document_position(a, b);
    if p.contains(DocumentPosition::FOLLOWING) {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}
