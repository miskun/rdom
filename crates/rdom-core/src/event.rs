//! `Event` — display-agnostic event type, honoring the DOM
//! `stop_propagation` / `stop_immediate_propagation` / `prevent_default`
//! flags. Concrete payloads (KeyEvent, MouseEvent, render context) belong
//! in `rdom-tui`; this core type carries just the routing state.
//!
//! Spec: <https://dom.spec.whatwg.org/#events>

use crate::event_detail::EventDetail;
use crate::node_id::NodeId;

/// Which phase of dispatch is currently running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventPhase {
    /// No dispatch in progress.
    None,
    /// Descending from root toward `target`. Capture-mode listeners fire
    /// on ancestors.
    Capturing,
    /// At the target node. Both capture and bubble listeners fire.
    AtTarget,
    /// Ascending from `target` back to root. Non-capture listeners fire
    /// on ancestors.
    Bubbling,
}

/// Minimal event — just routing state. Attach payload via a typed
/// wrapper in `rdom-tui` or the caller's crate.
///
/// Users typically build one with `Event::new("click")`, optionally
/// call `with_bubbles(false)` / `with_cancelable(true)`, then pass to
/// `Dom::dispatch_event(target, &mut event)`.
///
/// # Cloning
///
/// `clone()` is the web's `new Event(e.type, e)`: a fresh,
/// undispatched event. It copies `event_type`, the init flags
/// (`bubbles`, `cancelable`) and `detail`; everything a dispatch
/// writes starts over — `target` and `current_target` are `None`,
/// `phase` is [`EventPhase::None`], the stop-propagation and canceled
/// flags are clear, the dispatch flag is unset and
/// [`redraw_requested`](Event::redraw_requested) is `false`. A copy
/// is script-made, so it is not [synthetic](Event::is_synthetic)
/// (a scripted copy has `isTrusted` false on the web). A clone taken
/// inside a listener can therefore be dispatched or queued; the
/// in-flight original still cannot (DOM §2.9 step 1).
///
/// `Event` has no `timeStamp`, so there is no creation time to copy
/// or restart; should one be added, a copy takes its own creation
/// time, as `new Event()` does (DOM §2.2).
///
/// Read what a listener needs from the dispatch state (`target`,
/// `phase`, `default_prevented()`) inside the listener: a clone does
/// not carry it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Event {
    /// Event type string — "click", "input", etc. Case-sensitive.
    pub event_type: String,
    /// Whether the event bubbles after the target. Default: true.
    pub bubbles: bool,
    /// Whether `prevent_default` has meaning for this event. Default: true.
    pub cancelable: bool,
    /// The node where dispatch was initiated. Set by `dispatch_event`;
    /// callers don't need to fill this in.
    pub target: Option<NodeId>,
    /// The node currently being visited in dispatch. Updated per node
    /// so handlers see it.
    pub current_target: Option<NodeId>,
    /// Current phase of dispatch.
    pub phase: EventPhase,
    /// Typed payload for event types that carry semantic data.
    /// [`EventDetail::None`] for events that don't carry detail
    /// (default on `Event::new`); [`EventDetail::String`] for
    /// `CustomEvent`-style ad-hoc author payloads; typed variants
    /// for events with structured payloads (transitions, inputs,
    /// submits, toggles, mouse, keyboard). Listeners read via the
    /// `as_*` accessors on [`EventDetail`].
    pub detail: EventDetail,

    /// `true` when the event was synthesized by the runtime (as
    /// opposed to originating from user input or an explicit
    /// `dispatch_event` call from application code). Higher layers
    /// use this to suppress default actions on events they
    /// themselves created — preventing recursion (e.g., a runtime
    /// that dispatches synthetic `click` after `mouseup`, then
    /// would recursively try to dispatch another `click` as that
    /// event's default action).
    ///
    /// Spec-faithful analog to the browser's
    /// `Event.isTrusted` flag, inverted: browsers set `isTrusted =
    /// true` for user-originated events and `false` for scripted
    /// ones; we set `is_synthetic = true` for runtime-originated
    /// events, which is the flag that's actually useful to
    /// dispatch logic. The difference is semantic, not
    /// behavioral.
    pub(crate) is_synthetic: bool,

    pub(crate) propagation_stopped: bool,
    pub(crate) immediate_propagation_stopped: bool,
    pub(crate) default_prevented: bool,
    /// DOM "dispatch flag": set for the duration of `dispatch_event`.
    /// Re-dispatching an in-flight event is `InvalidStateError` on the
    /// web; here it returns `DomError::InvalidState`. A listener panic
    /// that unwinds out of `dispatch_event` leaves the flag set on that
    /// `Event` value (the value is normally dropped with the unwind).
    pub(crate) dispatching: bool,

    /// Set by [`EventCtx::request_redraw`](crate::EventCtx::request_redraw)
    /// when a listener mutated state that the host should repaint —
    /// state the DOM mutation tracker can't see (e.g. a `<canvas>` whose
    /// paint reads external app state). Accumulates across every listener
    /// in the dispatch. **`rdom-core` never acts on this** — it's an
    /// inert intent flag the rendering host reads after dispatch (the
    /// `rdom-tui` runtime ORs it into its repaint decision). Read via
    /// [`Event::redraw_requested`].
    pub(crate) redraw_requested: bool,
}

/// DOM §2.2 `new Event(e.type, e)` — see [`Event`]'s "Cloning".
impl Clone for Event {
    fn clone(&self) -> Self {
        let mut copy = Event::new(self.event_type.clone())
            .with_bubbles(self.bubbles)
            .with_cancelable(self.cancelable);
        copy.detail = self.detail.clone();
        copy
    }
}

impl Event {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            bubbles: true,
            cancelable: true,
            target: None,
            current_target: None,
            phase: EventPhase::None,
            detail: EventDetail::None,
            is_synthetic: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            default_prevented: false,
            dispatching: false,
            redraw_requested: false,
        }
    }

    pub fn with_bubbles(mut self, bubbles: bool) -> Self {
        self.bubbles = bubbles;
        self
    }

    pub fn with_cancelable(mut self, cancelable: bool) -> Self {
        self.cancelable = cancelable;
        self
    }

    /// Builder-style `detail` setter for string payloads —
    /// `Event::new("custom").with_detail("hello")`. Produces an
    /// [`EventDetail::String`]; for typed variants set
    /// `event.detail` directly to the relevant variant.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = EventDetail::String(detail.into());
        self
    }

    /// Mark this event as synthesized by the runtime. Default is
    /// `false` (not synthesized). Use when composing higher-level
    /// events from lower-level ones — e.g., the runtime creates a
    /// synthetic `click` after matching `mousedown`+`mouseup`, so
    /// handlers firing during `click` can distinguish it from a
    /// handler-scripted `dispatch_event("click", ...)`.
    pub fn with_synthetic(mut self, synthetic: bool) -> Self {
        self.is_synthetic = synthetic;
        self
    }

    /// `true` iff this event was synthesized by the runtime. See
    /// [`Event::with_synthetic`].
    pub fn is_synthetic(&self) -> bool {
        self.is_synthetic
    }

    /// Stop bubbling/capturing on subsequent nodes. Listeners still
    /// registered at the current node *and phase* continue to fire (see
    /// `stop_immediate_propagation` for the harder stop); at the target,
    /// calling this from a capture listener suppresses the target's
    /// bubble listeners, which belong to the next pass. The flag is
    /// cleared when the dispatch ends (DOM §2.9 step 5.9).
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Stop this event immediately: no further listeners run on this
    /// node, no further propagation.
    pub fn stop_immediate_propagation(&mut self) {
        self.propagation_stopped = true;
        self.immediate_propagation_stopped = true;
    }

    /// Signal "please skip the default action". Only meaningful if
    /// `cancelable` is true. The Dom itself has no notion of "default
    /// action"; higher layers check `default_prevented()` to decide.
    pub fn prevent_default(&mut self) {
        if self.cancelable {
            self.default_prevented = true;
        }
    }

    /// `true` if any listener called
    /// [`EventCtx::request_redraw`](crate::EventCtx::request_redraw)
    /// during this dispatch. The rendering host reads this after
    /// `dispatch_event` returns to decide whether to repaint.
    pub fn redraw_requested(&self) -> bool {
        self.redraw_requested
    }

    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    pub fn is_immediate_propagation_stopped(&self) -> bool {
        self.immediate_propagation_stopped
    }

    pub fn default_prevented(&self) -> bool {
        self.default_prevented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_defaults() {
        let e = Event::new("click");
        assert_eq!(e.event_type, "click");
        assert!(e.bubbles);
        assert!(e.cancelable);
        assert_eq!(e.phase, EventPhase::None);
        assert!(!e.is_propagation_stopped());
        assert!(!e.default_prevented());
    }

    #[test]
    fn stop_propagation_sets_flag() {
        let mut e = Event::new("click");
        e.stop_propagation();
        assert!(e.is_propagation_stopped());
        assert!(!e.is_immediate_propagation_stopped());
    }

    #[test]
    fn stop_immediate_sets_both_flags() {
        let mut e = Event::new("click");
        e.stop_immediate_propagation();
        assert!(e.is_propagation_stopped());
        assert!(e.is_immediate_propagation_stopped());
    }

    #[test]
    fn prevent_default_only_when_cancelable() {
        let mut e = Event::new("click");
        e.prevent_default();
        assert!(e.default_prevented());

        let mut e2 = Event::new("click").with_cancelable(false);
        e2.prevent_default();
        assert!(!e2.default_prevented());
    }

    #[test]
    fn synthetic_default_is_false() {
        let e = Event::new("click");
        assert!(!e.is_synthetic());
    }

    #[test]
    fn with_synthetic_sets_flag() {
        let e = Event::new("click").with_synthetic(true);
        assert!(e.is_synthetic());

        let e2 = Event::new("click").with_synthetic(false);
        assert!(!e2.is_synthetic());
    }

    #[test]
    fn synthetic_flag_independent_of_other_state() {
        // Synthetic is orthogonal to bubbling/cancelable/propagation.
        let mut e = Event::new("click")
            .with_synthetic(true)
            .with_bubbles(false)
            .with_cancelable(false);
        e.stop_propagation();
        e.prevent_default();
        assert!(e.is_synthetic());
        assert!(!e.bubbles);
        assert!(!e.cancelable);
        assert!(e.is_propagation_stopped());
        assert!(!e.default_prevented()); // cancelable=false blocks prevent_default
    }

    // ── Clone is `new Event(e.type, e)` (P7G-EVENT-CLONE-1) ──────────

    use crate::dispatch::ListenerOptions;
    use crate::{Dom, DomError, EventDetail};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// DOM §2.2 / §2.9: a copy is a fresh event — type, init flags and
    /// detail carry over; the dispatch flag, target, current target,
    /// phase and the stop / canceled flags do not.
    #[test]
    fn clone_taken_mid_dispatch_is_a_fresh_undispatched_event() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let b = dom.create_element("b");
        dom.append_child(root, b).unwrap();
        let stash = Rc::new(RefCell::new(Vec::new()));
        {
            // Cancel and stop the event first, so the clone has every
            // flag to (not) copy.
            let stash = stash.clone();
            dom.add_event_listener(b, "ping", ListenerOptions::default(), move |ctx| {
                ctx.event.prevent_default();
                ctx.event.stop_immediate_propagation();
                ctx.request_redraw();
                stash.borrow_mut().push(ctx.event.clone());
            })
            .unwrap();
        }
        let mut e = Event::new("ping")
            .with_bubbles(false)
            .with_cancelable(true)
            .with_detail("payload");
        dom.dispatch_event(b, &mut e).unwrap();
        let copy = stash.borrow_mut().pop().expect("listener ran");
        assert_eq!(copy.event_type, "ping");
        assert!(!copy.bubbles);
        assert!(copy.cancelable);
        assert_eq!(copy.detail, EventDetail::String("payload".into()));
        assert_eq!(copy.phase, EventPhase::None);
        assert_eq!(copy.target, None);
        assert_eq!(copy.current_target, None);
        assert!(
            !copy.default_prevented(),
            "a copy of a canceled event is not canceled"
        );
        assert!(!copy.is_propagation_stopped());
        assert!(!copy.is_immediate_propagation_stopped());
        assert!(!copy.redraw_requested());
    }

    /// The copy dispatches normally, both after the original's dispatch
    /// and nested inside it, while re-dispatching the original in
    /// flight is still `InvalidStateError` (DOM §2.9 step 1).
    #[test]
    fn clone_dispatches_while_the_original_in_flight_still_cannot() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        dom.append_child(root, a).unwrap();
        let results = Rc::new(RefCell::new(Vec::new()));
        let fired = Rc::new(RefCell::new(Vec::new()));
        {
            let fired = fired.clone();
            dom.add_event_listener(a, "ping", ListenerOptions::default(), move |ctx| {
                fired.borrow_mut().push(ctx.event.detail.clone());
            })
            .unwrap();
        }
        {
            // A listener is skipped while it is running, so the nested
            // dispatch reaches only the recorder above.
            let results = results.clone();
            dom.add_event_listener(a, "ping", ListenerOptions::default(), move |ctx| {
                if ctx.event.detail == EventDetail::String("outer".into()) {
                    let original = ctx.dom.dispatch_event(a, ctx.event);
                    let mut copy = ctx.event.clone();
                    copy.detail = EventDetail::String("copy".into());
                    let nested = ctx.dom.dispatch_event(a, &mut copy);
                    results.borrow_mut().push((original, nested, copy));
                }
            })
            .unwrap();
        }
        let mut e = Event::new("ping").with_detail("outer");
        dom.dispatch_event(a, &mut e).unwrap();
        let (original, nested, mut copy) = results.borrow_mut().pop().unwrap();
        assert!(matches!(original, Err(DomError::InvalidState(_))));
        assert_eq!(nested, Ok(()));
        assert_eq!(
            *fired.borrow(),
            vec![
                EventDetail::String("outer".into()),
                EventDetail::String("copy".into())
            ]
        );
        dom.dispatch_event(a, &mut copy).unwrap();
        assert_eq!(
            fired.borrow().len(),
            3,
            "the copy dispatches again afterwards"
        );
    }

    /// A copy of a runtime-synthesized event is script-made: untrusted
    /// on the web (`isTrusted` false), not synthetic here.
    #[test]
    fn clone_is_not_synthetic() {
        let e = Event::new("click").with_synthetic(true);
        assert!(!e.clone().is_synthetic());
    }
}
