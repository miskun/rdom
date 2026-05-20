//! Event listener storage + `dispatch_event` (capture → target → bubble).
//!
//! Listeners are held in a side `HashMap<NodeId, Vec<Listener<Ext>>>` on
//! `Dom` — nodes without listeners don't pay any per-element cost. A
//! listener handle (`ListenerId`) is returned from `add_event_listener`
//! so callers can remove the exact registration later.
//!
//! Dispatch algorithm (mirrors the DOM spec):
//!
//! 1. Compute the ancestor path from target up to root (inclusive).
//! 2. **Capture phase** — iterate path root→target (excluding target); on
//!    each node fire listeners whose `capture == true`.
//! 3. **Target phase** — on target fire every listener (both capture and
//!    bubble variants).
//! 4. **Bubble phase** — if `event.bubbles`, iterate target→root
//!    (excluding target); on each node fire listeners whose `capture == false`.
//!
//! `stop_propagation` cuts at node-boundaries. `stop_immediate_propagation`
//! cuts mid-node (no further listeners on the current node run).
//!
//! Handlers are `FnMut(&mut EventCtx<'_, Ext>)`. They receive a mutable
//! reference to the Dom via the context — mutations during dispatch are
//! allowed; the dispatcher captures the ancestor path up-front so removed
//! nodes mid-flight don't break iteration. Listeners added during a
//! handler fire on *subsequent* dispatches, not the current one.

use std::collections::HashMap;

use crate::abort::AbortSignal;
use crate::dom::Dom;
use crate::error::{DomError, Result};
use crate::event::{Event, EventPhase};
use crate::node_id::NodeId;

/// Handle returned from `add_event_listener` — pass to `remove_event_listener`
/// to detach this specific registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId {
    pub node: NodeId,
    pub seq: u32,
}

/// A single registration.
///
/// `handler` is `Option` so the dispatcher can `take` it out (handing
/// `&mut Dom` to the handler) and put it back after the call. `None`
/// during a call means "currently firing".
pub(crate) struct Listener<Ext: 'static> {
    seq: u32,
    event_type: String,
    capture: bool,
    once: bool,
    /// If `Some`, the listener is dropped + skipped on the next
    /// dispatch visit once `signal.is_aborted()` returns true.
    signal: Option<AbortSignal>,
    handler: Option<EventHandler<Ext>>,
}

/// Boxed event handler stored on a [`Listener`].
pub(crate) type EventHandler<Ext> = Box<dyn FnMut(&mut EventCtx<'_, Ext>) + 'static>;

/// Context passed to a handler: mutable Event + mutable Dom.
pub struct EventCtx<'a, Ext: 'static> {
    pub event: &'a mut Event,
    pub dom: &'a mut Dom<Ext>,
}

/// Options for `add_event_listener` — matches the DOM spec object.
///
/// Not `Copy` because `signal: Option<AbortSignal>` holds an `Rc`.
/// Cloning is cheap (refcount bump).
#[derive(Debug, Clone, Default)]
pub struct ListenerOptions {
    /// Fire during the capture phase instead of the bubble phase.
    pub capture: bool,
    /// Remove the listener after it fires once.
    pub once: bool,
    /// If `Some`, the listener auto-removes once the signal is
    /// aborted. See [`AbortController`] / [`AbortSignal`] for the
    /// lifetime-management pattern.
    ///
    /// [`AbortController`]: crate::AbortController
    /// [`AbortSignal`]: crate::AbortSignal
    pub signal: Option<AbortSignal>,
}

impl ListenerOptions {
    pub fn capture() -> Self {
        Self {
            capture: true,
            ..Default::default()
        }
    }
    pub fn once() -> Self {
        Self {
            once: true,
            ..Default::default()
        }
    }
    /// Builder-style: attach an abort signal. When the signal fires,
    /// this listener is removed on the next dispatch visit.
    pub fn with_signal(mut self, signal: AbortSignal) -> Self {
        self.signal = Some(signal);
        self
    }
}

/// Per-Dom listener storage. One entry per node that has at least one
/// registered listener.
#[derive(Default)]
pub(crate) struct ListenerStore<Ext: 'static> {
    by_node: HashMap<NodeId, Vec<Listener<Ext>>>,
    /// Monotonic sequence counter for unique ListenerIds.
    next_seq: u32,
}

impl<Ext: 'static> ListenerStore<Ext> {
    fn next(&mut self) -> u32 {
        let n = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        n
    }
}

impl<Ext: 'static> std::fmt::Debug for ListenerStore<Ext> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListenerStore")
            .field("nodes_with_listeners", &self.by_node.len())
            .field("next_seq", &self.next_seq)
            .finish()
    }
}

// Listeners don't Clone — cloning a Dom that owns closures is nonsensical.
// Provide a manual `Clone` impl on Dom that resets listeners to empty (see
// dom.rs). For now ListenerStore is `Default` but not `Clone`.

impl<Ext> Dom<Ext> {
    /// Register a listener on `node` for events of type `event_type`.
    /// Returns a `ListenerId` that can be passed to `remove_event_listener`.
    pub fn add_event_listener(
        &mut self,
        node: NodeId,
        event_type: impl Into<String>,
        options: ListenerOptions,
        handler: impl FnMut(&mut EventCtx<'_, Ext>) + 'static,
    ) -> Result<ListenerId> {
        self.node_or_err(node)?;
        let seq = self.listeners.next();
        let listener = Listener {
            seq,
            event_type: event_type.into(),
            capture: options.capture,
            once: options.once,
            signal: options.signal,
            handler: Some(Box::new(handler)),
        };
        // Registering with an already-aborted signal still succeeds —
        // the listener goes into storage and gets dropped on the
        // first dispatch visit (never fires). Matches browser
        // behavior; tests exercise this path explicitly.
        self.listeners
            .by_node
            .entry(node)
            .or_default()
            .push(listener);
        Ok(ListenerId { node, seq })
    }

    /// Remove a previously-registered listener. Returns `true` if the
    /// listener existed and was removed; `false` if it was already gone
    /// (e.g. a `once` handler that already fired).
    pub fn remove_event_listener(&mut self, handle: ListenerId) -> bool {
        let Some(vec) = self.listeners.by_node.get_mut(&handle.node) else {
            return false;
        };
        let before = vec.len();
        vec.retain(|l| l.seq != handle.seq);
        let removed = vec.len() < before;
        if vec.is_empty() {
            self.listeners.by_node.remove(&handle.node);
        }
        removed
    }

    /// How many listeners are currently registered on `node`.
    pub fn listener_count(&self, node: NodeId) -> usize {
        self.listeners.by_node.get(&node).map_or(0, Vec::len)
    }

    /// Dispatch `event` starting at `target`. Walks capture → target →
    /// bubble firing listeners, honoring `stop_propagation` and
    /// `stop_immediate_propagation`. Returns `Err` on invalid target.
    ///
    /// Handlers may mutate the Dom via `EventCtx::dom`. The ancestor path
    /// is computed up-front so mid-dispatch mutations don't destabilize
    /// iteration. Listeners added during dispatch fire on *subsequent*
    /// dispatches, not the current one.
    pub fn dispatch_event(&mut self, target: NodeId, event: &mut Event) -> Result<()> {
        self.node_or_err(target)?;

        event.target = Some(target);

        // Path from root → target (inclusive). Always non-empty if the
        // node is in the arena (the node itself is the last element).
        let path = self.ancestor_path(target);
        if path.is_empty() {
            return Err(DomError::InvalidNode(target));
        }

        // ── Capture phase ─────────────────────────────────────────
        event.phase = EventPhase::Capturing;
        for &node in path.iter().take(path.len() - 1) {
            if event.propagation_stopped {
                break;
            }
            event.current_target = Some(node);
            self.fire_at(node, event, PhaseFilter::Capture);
        }

        // ── Target phase ──────────────────────────────────────────
        if !event.propagation_stopped {
            event.phase = EventPhase::AtTarget;
            event.current_target = Some(target);
            self.fire_at(target, event, PhaseFilter::All);
        }

        // ── Bubble phase ──────────────────────────────────────────
        if event.bubbles && !event.propagation_stopped {
            event.phase = EventPhase::Bubbling;
            for &node in path.iter().rev().skip(1) {
                if event.propagation_stopped {
                    break;
                }
                event.current_target = Some(node);
                self.fire_at(node, event, PhaseFilter::Bubble);
            }
        }

        event.phase = EventPhase::None;
        event.current_target = None;
        Ok(())
    }

    /// Run matching listeners on `node`. Filter by `filter` (capture /
    /// bubble / all). Handlers get `&mut self` via `EventCtx::dom`.
    ///
    /// Implementation: snapshot the list of listener sequences to run,
    /// then for each one re-find it by seq, mem-replace its handler with
    /// a no-op while calling it, and restore afterwards. Handlers may add
    /// or remove listeners on this or any other node freely.
    fn fire_at(&mut self, node: NodeId, event: &mut Event, filter: PhaseFilter) {
        // Opportunistic sweep: drop any aborted listeners on this
        // node before snapshotting. Cheap (only runs when we're
        // about to dispatch) and keeps storage bounded across abort
        // cycles.
        if let Some(list) = self.listeners.by_node.get_mut(&node) {
            list.retain(|l| !l.signal.as_ref().is_some_and(|s| s.is_aborted()));
            if list.is_empty() {
                self.listeners.by_node.remove(&node);
            }
        }

        // Step 1: snapshot (seq, once) of listeners to run, in order.
        let to_fire: Vec<(u32, bool)> = match self.listeners.by_node.get(&node) {
            Some(list) => list
                .iter()
                .filter(|l| l.event_type == event.event_type && filter.matches(l.capture))
                .map(|l| (l.seq, l.once))
                .collect(),
            None => return,
        };

        // Step 2: fire each in order, re-locating by seq each time in case
        // handlers added/removed entries.
        for (seq, once) in to_fire {
            if event.immediate_propagation_stopped {
                break;
            }
            // Locate the listener.
            let Some(list) = self.listeners.by_node.get_mut(&node) else {
                return;
            };
            let Some(i) = list.iter().position(|l| l.seq == seq) else {
                continue; // removed by a previous handler
            };
            // Per-listener abort check — a prior handler in this
            // same dispatch may have aborted a signal governing
            // subsequent listeners. Drop + skip.
            if list[i].signal.as_ref().is_some_and(|s| s.is_aborted()) {
                list.remove(i);
                continue;
            }
            // Take the handler out so we can hand &mut self to it.
            let Some(mut handler) = list[i].handler.take() else {
                // Re-entrant call on the same listener — skip.
                continue;
            };

            // Run it.
            let mut ctx = EventCtx { event, dom: self };
            handler(&mut ctx);

            // Restore (or expire, for `once`).
            if let Some(list) = self.listeners.by_node.get_mut(&node) {
                if let Some(i) = list.iter().position(|l| l.seq == seq) {
                    if once {
                        list.remove(i);
                    } else {
                        list[i].handler = Some(handler);
                    }
                }
                if list.is_empty() {
                    self.listeners.by_node.remove(&node);
                }
            }
            // If the node was removed from by_node mid-handler we simply
            // drop `handler`. A dropped listener stays dropped — matches
            // `remove_event_listener` semantics.
        }
    }

    /// Internal hook: called from `free` to drop any listeners attached
    /// to a node that's being discarded.
    pub(crate) fn drop_listeners(&mut self, node: NodeId) {
        self.listeners.by_node.remove(&node);
    }
}

/// Which listeners to fire based on their `capture` flag.
#[derive(Debug, Clone, Copy)]
enum PhaseFilter {
    Capture,
    Bubble,
    All,
}

impl PhaseFilter {
    fn matches(self, listener_capture: bool) -> bool {
        match self {
            PhaseFilter::Capture => listener_capture,
            PhaseFilter::Bubble => !listener_capture,
            PhaseFilter::All => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Small helper to build a tree root → a → b → c and return ids.
    fn build_chain() -> (Dom, NodeId, NodeId, NodeId, NodeId) {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        let c = dom.create_element("c");
        dom.append_child(a, b).unwrap();
        dom.append_child(b, c).unwrap();
        dom.append_child(root, a).unwrap();
        (dom, a, b, c, root)
    }

    #[test]
    fn add_and_remove_listener() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let fired = Rc::new(Cell::new(0));
        let f2 = fired.clone();
        let id = dom
            .add_event_listener(el, "click", ListenerOptions::default(), move |_| {
                f2.set(f2.get() + 1);
            })
            .unwrap();
        assert_eq!(dom.listener_count(el), 1);

        let mut e = Event::new("click");
        dom.dispatch_event(el, &mut e).unwrap();
        assert_eq!(fired.get(), 1);

        assert!(dom.remove_event_listener(id));
        assert_eq!(dom.listener_count(el), 0);

        let mut e2 = Event::new("click");
        dom.dispatch_event(el, &mut e2).unwrap();
        assert_eq!(fired.get(), 1); // unchanged
    }

    #[test]
    fn capture_target_bubble_ordering() {
        let (mut dom, a, b, c, _) = build_chain();
        let order = Rc::new(std::cell::RefCell::new(Vec::<&'static str>::new()));

        let o = order.clone();
        dom.add_event_listener(a, "click", ListenerOptions::capture(), move |_| {
            o.borrow_mut().push("a-capture");
        })
        .unwrap();
        let o = order.clone();
        dom.add_event_listener(b, "click", ListenerOptions::capture(), move |_| {
            o.borrow_mut().push("b-capture");
        })
        .unwrap();
        let o = order.clone();
        dom.add_event_listener(c, "click", ListenerOptions::default(), move |_| {
            o.borrow_mut().push("c-target");
        })
        .unwrap();
        let o = order.clone();
        dom.add_event_listener(b, "click", ListenerOptions::default(), move |_| {
            o.borrow_mut().push("b-bubble");
        })
        .unwrap();
        let o = order.clone();
        dom.add_event_listener(a, "click", ListenerOptions::default(), move |_| {
            o.borrow_mut().push("a-bubble");
        })
        .unwrap();

        let mut e = Event::new("click");
        dom.dispatch_event(c, &mut e).unwrap();

        assert_eq!(
            *order.borrow(),
            vec!["a-capture", "b-capture", "c-target", "b-bubble", "a-bubble"]
        );
    }

    #[test]
    fn stop_propagation_cuts_bubble() {
        let (mut dom, a, b, c, _) = build_chain();
        let a_fired = Rc::new(Cell::new(false));
        let af = a_fired.clone();
        dom.add_event_listener(a, "click", ListenerOptions::default(), move |_| {
            af.set(true);
        })
        .unwrap();
        dom.add_event_listener(b, "click", ListenerOptions::default(), move |ctx| {
            ctx.event.stop_propagation();
        })
        .unwrap();

        let mut e = Event::new("click");
        dom.dispatch_event(c, &mut e).unwrap();
        assert!(!a_fired.get());
    }

    #[test]
    fn stop_immediate_cuts_sibling_listener_on_same_node() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let hit_first = Rc::new(Cell::new(false));
        let hit_second = Rc::new(Cell::new(false));
        let h1 = hit_first.clone();
        let h2 = hit_second.clone();
        dom.add_event_listener(el, "x", ListenerOptions::default(), move |ctx| {
            h1.set(true);
            ctx.event.stop_immediate_propagation();
        })
        .unwrap();
        dom.add_event_listener(el, "x", ListenerOptions::default(), move |_| {
            h2.set(true);
        })
        .unwrap();

        let mut e = Event::new("x");
        dom.dispatch_event(el, &mut e).unwrap();
        assert!(hit_first.get());
        assert!(!hit_second.get());
    }

    #[test]
    fn non_bubbling_skips_bubble_phase() {
        let (mut dom, a, _, c, _) = build_chain();
        let a_fired = Rc::new(Cell::new(false));
        let af = a_fired.clone();
        dom.add_event_listener(a, "x", ListenerOptions::default(), move |_| {
            af.set(true);
        })
        .unwrap();

        let mut e = Event::new("x").with_bubbles(false);
        dom.dispatch_event(c, &mut e).unwrap();
        assert!(!a_fired.get());
    }

    #[test]
    fn prevent_default_sets_flag() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
        let mut e = Event::new("click");
        dom.dispatch_event(el, &mut e).unwrap();
        assert!(e.default_prevented());
    }

    #[test]
    fn once_removes_after_firing() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let fired = Rc::new(Cell::new(0));
        let f = fired.clone();
        dom.add_event_listener(el, "x", ListenerOptions::once(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();

        let mut e1 = Event::new("x");
        dom.dispatch_event(el, &mut e1).unwrap();
        let mut e2 = Event::new("x");
        dom.dispatch_event(el, &mut e2).unwrap();
        assert_eq!(fired.get(), 1);
        assert_eq!(dom.listener_count(el), 0);
    }

    #[test]
    fn listener_type_is_filtered() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let click = Rc::new(Cell::new(0));
        let input = Rc::new(Cell::new(0));
        let cc = click.clone();
        let ii = input.clone();
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |_| {
            cc.set(cc.get() + 1);
        })
        .unwrap();
        dom.add_event_listener(el, "input", ListenerOptions::default(), move |_| {
            ii.set(ii.get() + 1);
        })
        .unwrap();
        let mut e = Event::new("click");
        dom.dispatch_event(el, &mut e).unwrap();
        assert_eq!(click.get(), 1);
        assert_eq!(input.get(), 0);
    }

    #[test]
    fn handler_may_read_dom() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        dom.set_attribute(a, "id", "target").unwrap();
        dom.append_child(root, a).unwrap();
        let seen = Rc::new(Cell::new(false));
        let s = seen.clone();
        dom.add_event_listener(a, "click", ListenerOptions::default(), move |ctx| {
            if ctx.dom.get_element_by_id("target").is_some() {
                s.set(true);
            }
        })
        .unwrap();

        let mut e = Event::new("click");
        dom.dispatch_event(a, &mut e).unwrap();
        assert!(seen.get());
    }

    #[test]
    fn handler_may_mutate_dom() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |ctx| {
            let added = ctx.dom.create_element("added");
            ctx.dom.append_child(el, added).unwrap();
        })
        .unwrap();
        let mut e = Event::new("click");
        dom.dispatch_event(el, &mut e).unwrap();
        assert_eq!(dom.node(el).child_element_count(), 1);
    }

    #[test]
    fn dispatch_on_orphan_node_still_fires_target() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |_| {
            f.set(true);
        })
        .unwrap();
        let mut e = Event::new("click");
        dom.dispatch_event(el, &mut e).unwrap();
        assert!(fired.get());
    }

    #[test]
    fn current_target_reflects_node_during_dispatch() {
        let (mut dom, a, _b, c, _) = build_chain();
        let ct_at_a = Rc::new(Cell::new(None));
        let p = ct_at_a.clone();
        dom.add_event_listener(a, "click", ListenerOptions::default(), move |ctx| {
            p.set(ctx.event.current_target);
        })
        .unwrap();
        let mut e = Event::new("click");
        dom.dispatch_event(c, &mut e).unwrap();
        assert_eq!(ct_at_a.get(), Some(a));
        // After dispatch, current_target is reset.
        assert_eq!(e.current_target, None);
    }

    #[test]
    fn removing_node_unregisters_listeners_via_drop_subtree() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let _ = dom
            .add_event_listener(el, "click", ListenerOptions::default(), |_| {})
            .unwrap();
        let root = dom.root();
        dom.append_child(root, el).unwrap();
        dom.drop_subtree(el).unwrap();
        assert_eq!(dom.listener_count(el), 0);
    }

    // ── AbortSignal / AbortController ────────────────────────────────

    #[test]
    fn abort_signal_removes_listener_on_next_dispatch() {
        use crate::AbortController;

        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let fired = Rc::new(Cell::new(0));
        let f = fired.clone();

        let ctrl = AbortController::new();
        let sig = ctrl.signal();
        dom.add_event_listener(
            el,
            "click",
            ListenerOptions::default().with_signal(sig),
            move |_| {
                f.set(f.get() + 1);
            },
        )
        .unwrap();

        // First dispatch: fires normally.
        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(fired.get(), 1);

        // Abort, then dispatch again: listener is skipped + removed.
        ctrl.abort();
        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(fired.get(), 1);
        assert_eq!(dom.listener_count(el), 0);
    }

    #[test]
    fn abort_signal_removes_multiple_listeners_at_once() {
        use crate::AbortController;

        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let ctrl = AbortController::new();
        let sig = ctrl.signal();

        for _ in 0..5 {
            dom.add_event_listener(
                el,
                "click",
                ListenerOptions::default().with_signal(sig.clone()),
                |_| {},
            )
            .unwrap();
        }
        assert_eq!(dom.listener_count(el), 5);

        ctrl.abort();
        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(dom.listener_count(el), 0);
    }

    #[test]
    fn abort_signal_independent_of_other_listeners() {
        use crate::AbortController;

        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let ctrl = AbortController::new();
        let sig = ctrl.signal();

        let fired_aborted = Rc::new(Cell::new(0));
        let fa = fired_aborted.clone();
        dom.add_event_listener(
            el,
            "click",
            ListenerOptions::default().with_signal(sig),
            move |_| {
                fa.set(fa.get() + 1);
            },
        )
        .unwrap();

        let fired_normal = Rc::new(Cell::new(0));
        let fn_ = fired_normal.clone();
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |_| {
            fn_.set(fn_.get() + 1);
        })
        .unwrap();

        ctrl.abort();
        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(fired_aborted.get(), 0, "aborted listener must not fire");
        assert_eq!(fired_normal.get(), 1, "unrelated listener still fires");
    }

    #[test]
    fn adding_listener_with_already_aborted_signal_never_fires() {
        use crate::AbortController;

        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let ctrl = AbortController::new();
        ctrl.abort();

        let fired = Rc::new(Cell::new(0));
        let f = fired.clone();
        dom.add_event_listener(
            el,
            "click",
            ListenerOptions::default().with_signal(ctrl.signal()),
            move |_| {
                f.set(f.get() + 1);
            },
        )
        .unwrap();

        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(fired.get(), 0);
        assert_eq!(dom.listener_count(el), 0);
    }

    #[test]
    fn handler_aborting_mid_dispatch_skips_later_listeners_on_same_node() {
        use crate::AbortController;

        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let ctrl = AbortController::new();

        let order = Rc::new(std::cell::RefCell::new(Vec::<&'static str>::new()));

        // Listener A: fires + aborts the controller. Shares the
        // controller via `.clone()` into the closure.
        let order_a = order.clone();
        let ctrl_a = ctrl.clone();
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |_| {
            order_a.borrow_mut().push("a");
            ctrl_a.abort();
        })
        .unwrap();

        // Listener B: governed by the signal; should be skipped
        // after A aborts.
        let order_b = order.clone();
        dom.add_event_listener(
            el,
            "click",
            ListenerOptions::default().with_signal(ctrl.signal()),
            move |_| {
                order_b.borrow_mut().push("b");
            },
        )
        .unwrap();

        // Listener C: not governed by the signal; fires regardless.
        let order_c = order.clone();
        dom.add_event_listener(el, "click", ListenerOptions::default(), move |_| {
            order_c.borrow_mut().push("c");
        })
        .unwrap();

        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(*order.borrow(), vec!["a", "c"]);
    }

    #[test]
    fn signal_clone_governs_same_listener() {
        use crate::AbortController;

        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        let ctrl = AbortController::new();
        let sig1 = ctrl.signal();
        let sig2 = sig1.clone();

        let fired = Rc::new(Cell::new(0));
        let f = fired.clone();
        dom.add_event_listener(
            el,
            "click",
            ListenerOptions::default().with_signal(sig2),
            move |_| {
                f.set(f.get() + 1);
            },
        )
        .unwrap();

        // Dispatch once: fires. Use sig1 to validate (not aborted).
        assert!(!sig1.is_aborted());
        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(fired.get(), 1);

        // Abort via controller, dispatch again: skipped.
        ctrl.abort();
        assert!(sig1.is_aborted());
        dom.dispatch_event(el, &mut Event::new("click")).unwrap();
        assert_eq!(fired.get(), 1);
    }
}
