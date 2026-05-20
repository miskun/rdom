//! `AppContext` — the handle a tick callback (or an in-loop
//! handler scope) uses to poke the runtime.
//!
//! Surface:
//!
//! - `dom` — mutable DOM access.
//! - `request_redraw()` — explicit dirty bit for mutations the
//!   observer didn't see (scroll offset, direct ext writes, etc.).
//! - `quit()` — exit the loop after this tick / event completes.
//! - `dispatch(target, event)` — synchronous nested dispatch.
//! - `queue_dispatch(target, event)` — runs after the current
//!   task completes but before the next event (browser
//!   microtask-queue semantics).
//!
//! Deferred to a later commit:
//! - `request_animation_frame(cb)` — pre-paint hook.
//! - `set_pointer_capture(id)` — drag routing (Phase 6).

use rdom_core::{Event, NodeId};

use crate::TuiDom;

/// Control flow returned from an `on_tick` callback: keep running
/// or exit the loop after the current task completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlFlow {
    /// Keep running. Redraw if dirty. (Default.)
    #[default]
    Continue,
    /// Exit the loop after this iteration.
    Quit,
}

/// Per-tick handle exposed to `on_tick` callbacks. Not
/// cross-thread (`!Send`, `!Sync`) — it borrows the DOM
/// exclusively. For cross-thread poking, use `AppHandle` (shipped
/// in a follow-up commit).
pub struct AppContext<'a> {
    /// Mutable DOM access. Mutations flow through the
    /// `DirtyTracker` automatically; the runtime invalidates the
    /// affected subtrees at the next cascade pass.
    pub dom: &'a mut TuiDom,
    /// Set by `request_redraw()`; read by the runtime after the
    /// tick returns.
    pub(super) redraw_requested: bool,
    /// Set by `quit()`.
    pub(super) quit_requested: bool,
    /// Events queued via `queue_dispatch`. Drained by the runtime
    /// after the tick/handler returns, before the next
    /// crossterm-event poll.
    pub(super) queued_dispatches: Vec<(NodeId, Event)>,
}

impl<'a> AppContext<'a> {
    pub(super) fn new(dom: &'a mut TuiDom) -> Self {
        Self {
            dom,
            redraw_requested: false,
            quit_requested: false,
            queued_dispatches: Vec::new(),
        }
    }

    /// Request a paint after the current tick/event completes —
    /// even when the `DirtyTracker` didn't see a mutation
    /// (e.g., the app wrote `ext.scroll_y` directly, which
    /// bypasses the observer).
    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    /// Signal the runtime to exit the loop after the current task
    /// finishes. Equivalent to returning `ControlFlow::Quit` from
    /// an `on_tick` callback.
    pub fn quit(&mut self) {
        self.quit_requested = true;
    }

    /// Synchronously dispatch an event on `target`. Runs the full
    /// capture → target → bubble walk nested in the current
    /// execution — matches `dispatchEvent()` in the browser.
    ///
    /// Re-entrancy is supported: a listener fired as a result of
    /// this call may itself dispatch more events; each inner
    /// dispatch completes before returning to the outer one.
    pub fn dispatch(&mut self, target: NodeId, event: &mut Event) {
        let _ = self.dom.dispatch_event(target, event);
    }

    /// Queue an event to dispatch after the current task's
    /// listeners complete, but before the runtime moves on to the
    /// next crossterm event (microtask-queue semantics).
    ///
    /// Use when a handler wants a follow-up event to fire without
    /// recursing into dispatch from the current stack (e.g., a
    /// button's "click" handler that wants to emit a "close"
    /// signal to a dialog ancestor — queueing keeps the dispatch
    /// stack shallow and matches legacy-rdom's queued-event
    /// pattern).
    pub fn queue_dispatch(&mut self, target: NodeId, event: Event) {
        self.queued_dispatches.push((target, event));
    }
}
