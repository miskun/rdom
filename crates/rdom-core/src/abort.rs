//! `AbortController` / `AbortSignal` — listener lifetime management.
//!
//! Modern DOM pattern (introduced in the WHATWG Fetch spec, later
//! adopted by DOM, AbortSignal spec §3). The usage pattern:
//!
//! ```ignore
//! let ctrl = AbortController::new();
//! let sig = ctrl.signal();
//!
//! dom.add_event_listener(btn, "click",
//!     ListenerOptions::default().with_signal(sig.clone()),
//!     |ctx| { /* ... */ })?;
//! dom.add_event_listener(btn, "mouseover",
//!     ListenerOptions::default().with_signal(sig.clone()),
//!     |ctx| { /* ... */ })?;
//!
//! // Later — one call removes BOTH listeners on the next dispatch visit.
//! ctrl.abort();
//! ```
//!
//! Far more ergonomic than tracking individual [`ListenerId`]s.
//! Critical for scope-bound handlers (component un-mount, dialog
//! close, event-loop iteration boundaries).
//!
//! ## Semantics
//!
//! - `AbortController` is the *write* handle: construction + `abort()`.
//!   Hold it in the scope that decides when listeners should die.
//! - `AbortSignal` is the *read-only* handle. Clone it cheaply
//!   (`Rc<..>` under the hood). Pass one copy per listener to
//!   `ListenerOptions::with_signal`.
//! - When `abort()` is called: the shared state flips. Listeners
//!   whose signal is aborted are dropped lazily — the next time the
//!   dispatcher visits them, they're skipped *and* removed from
//!   storage. No reverse-index or eager scan needed.
//! - Adding a listener with an already-aborted signal is a no-op:
//!   `add_event_listener` returns a valid `ListenerId` but the
//!   listener never fires and is dropped on first dispatch visit.
//!   Matches browser behavior.
//! - Safe under re-entrancy: a handler calling `controller.abort()`
//!   flips the flag; the *current* handler keeps running to
//!   completion (synchronous dispatch). *Subsequent* listeners on
//!   the current node and later nodes see the aborted flag and skip.
//!
//! ## Single-threaded
//!
//! rdom-core is single-threaded; we use `Rc<Cell<bool>>` internally.
//! If the runtime ever gains a threaded backend (unlikely for a TUI),
//! swap for `Arc<AtomicBool>` — the public API stays the same.
//!
//! [`ListenerId`]: crate::ListenerId

use std::cell::Cell;
use std::rc::Rc;

/// Shared aborted-flag backing a controller + its signals.
#[derive(Debug, Default)]
struct AbortState {
    aborted: Cell<bool>,
}

/// The *write* end of an abort pair — fires the signal via
/// [`abort`](Self::abort).
///
/// Cloneable: multiple controllers can share the same state through
/// clones, so any clone's `abort()` fires the shared signal. Most
/// apps hold one and pass signals out.
#[derive(Debug, Clone)]
pub struct AbortController {
    state: Rc<AbortState>,
}

impl AbortController {
    /// Create a fresh controller whose signal is not yet aborted.
    pub fn new() -> Self {
        Self {
            state: Rc::new(AbortState::default()),
        }
    }

    /// Produce an [`AbortSignal`] bound to this controller. Call
    /// multiple times to attach the same abort semantics to
    /// independent listeners.
    pub fn signal(&self) -> AbortSignal {
        AbortSignal {
            state: Rc::clone(&self.state),
        }
    }

    /// Fire the abort. Every listener whose signal was spawned from
    /// this controller will be skipped + removed on the next
    /// dispatch visit. Idempotent — calling twice has no additional
    /// effect.
    pub fn abort(&self) {
        self.state.aborted.set(true);
    }

    /// Has the signal been aborted?
    pub fn is_aborted(&self) -> bool {
        self.state.aborted.get()
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self::new()
    }
}

/// The *read* end of an abort pair — carried by listeners so
/// dispatch can check whether they should still fire.
///
/// Cheap to clone (`Rc` clone); attach one to each listener you
/// want the controller to govern.
#[derive(Debug, Clone)]
pub struct AbortSignal {
    state: Rc<AbortState>,
}

impl AbortSignal {
    /// Has the backing controller fired?
    pub fn is_aborted(&self) -> bool {
        self.state.aborted.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_controller_not_aborted() {
        let c = AbortController::new();
        assert!(!c.is_aborted());
        assert!(!c.signal().is_aborted());
    }

    #[test]
    fn abort_flips_controller_and_signals() {
        let c = AbortController::new();
        let s1 = c.signal();
        let s2 = c.signal();
        c.abort();
        assert!(c.is_aborted());
        assert!(s1.is_aborted());
        assert!(s2.is_aborted());
    }

    #[test]
    fn signal_clone_shares_state() {
        let c = AbortController::new();
        let s = c.signal();
        let s_clone = s.clone();
        c.abort();
        assert!(s.is_aborted());
        assert!(s_clone.is_aborted());
    }

    #[test]
    fn controller_clone_shares_state() {
        let c1 = AbortController::new();
        let c2 = c1.clone();
        let sig = c1.signal();
        c2.abort();
        // Aborting either clone fires the shared signal.
        assert!(c1.is_aborted());
        assert!(c2.is_aborted());
        assert!(sig.is_aborted());
    }

    #[test]
    fn default_constructs_unaborted() {
        let c: AbortController = Default::default();
        assert!(!c.is_aborted());
    }

    #[test]
    fn abort_is_idempotent() {
        let c = AbortController::new();
        c.abort();
        c.abort();
        c.abort();
        assert!(c.is_aborted());
    }
}
