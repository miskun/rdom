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
#[derive(Debug, Clone)]
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
    /// registered at the current node continue to fire (see
    /// `stop_immediate_propagation` for the harder stop).
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
}
