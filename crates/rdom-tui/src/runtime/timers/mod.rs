//! Timer scheduler — `setTimeout` / `setInterval` /
//! `requestAnimationFrame` / `queueMicrotask`, all matching the
//! HTML spec shapes (M3 Part A).
//!
//! ## Architecture
//!
//! Scheduler is concrete and lives on `App`. Callbacks are
//! `Box<dyn FnOnce(&mut TimerCtx<'_>)>` (or `FnMut` for intervals)
//! with a higher-ranked lifetime so they accept any
//! `TimerCtx<'a>`. `TimerCtx` exposes the same scheduling methods
//! the event-handler context does — chaining `set_timeout`
//! inside a callback Just Works, matching JS.
//!
//! ## Determinism
//!
//! The scheduler holds an `Instant`-based wall clock for
//! production but `advance_to(now)` is the only way time
//! advances. Tests pass synthetic instants for byte-perfect
//! determinism.
//!
//! `dead_code` allow on the few "wired in next slice"
//! accessors (`now`, `has_active_raf`) that the App tick loop
//! will pick up when §15-integration lands.
//!
//! ## Layout
//!
//! This file holds the shared handle, the thread-local "current
//! scheduler", [`TimerId`] and [`TimerCtx`]; `scheduler` the queues,
//! `pump` the drains that run callbacks, `ext` the [`TuiTimers`]
//! extension on event contexts.

#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::TuiDom;

mod ext;
mod pump;
mod scheduler;

#[cfg(test)]
mod tests;

pub use ext::TuiTimers;
pub(crate) use pump::{drain_microtasks, pump_intervals, pump_raf, pump_timeouts};
pub(crate) use scheduler::Scheduler;

/// The scheduler as shared by the `App`, the timer callbacks, and the
/// `TuiTimers` extension on event contexts. One `Rc` per `App`;
/// borrows are taken per call and never held across a user callback,
/// so a callback that schedules more work (or dispatches an event
/// whose listener does) borrows a free cell.
pub(crate) type SharedScheduler = Rc<RefCell<Scheduler>>;

thread_local! {
    /// The scheduler user code reaches through `TuiTimers`. Installed
    /// by [`SchedulerGuard`] around every path that runs user
    /// callbacks — event dispatch, ticks, timer pumps, injected
    /// closures, animation events, the drag-autoscroll synthetic move.
    /// An `Rc` clone, not a pointer: no aliasing with the `App`'s own
    /// handle, and nothing dangles if a guard is forgotten.
    static CURRENT_SCHEDULER: RefCell<Option<SharedScheduler>> = const { RefCell::new(None) };
}

/// RAII guard that makes `scheduler` the one `TuiTimers` routes to
/// for the guard's lifetime. On drop the previous value is restored,
/// so nested installs (an `App` entry point calling another) are
/// safe and re-entrant.
pub(crate) struct SchedulerGuard {
    previous: Option<SharedScheduler>,
}

impl SchedulerGuard {
    pub(crate) fn install(scheduler: &SharedScheduler) -> Self {
        let previous = CURRENT_SCHEDULER.with(|s| s.replace(Some(scheduler.clone())));
        SchedulerGuard { previous }
    }
}

impl Drop for SchedulerGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        CURRENT_SCHEDULER.with(|s| *s.borrow_mut() = previous);
    }
}

/// Run `f` against the currently-installed scheduler. `None` when no
/// `App` context is active on this thread. The `Rc` is cloned out of
/// the thread-local first so `f` may itself re-enter `with_current`.
fn with_current<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Scheduler) -> R,
{
    let current = CURRENT_SCHEDULER.with(|s| s.borrow().clone())?;
    Some(f(&mut current.borrow_mut()))
}

/// Whether code runs under an `App` — inside one of its entry points,
/// which lays the document out before its next paint.
pub(crate) fn in_app() -> bool {
    CURRENT_SCHEDULER.with(|s| s.borrow().is_some())
}

/// The running `App`'s scheduler clock, when user code is executing
/// under one. Builtins and the router use it for time-window
/// heuristics (multi-click, type-ahead) so `App::advance` drives them
/// deterministically; callers fall back to wall time outside an `App`.
pub(crate) fn current_now() -> Option<Instant> {
    with_current(|s| s.now())
}

/// Numeric handle returned by `set_timeout` / `set_interval` /
/// `request_animation_frame`. Pass to `clear_*` to cancel.
///
/// Allocated monotonically starting at 1 — matches the JS
/// expectation that `0` is falsy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(pub(crate) u32);

impl TimerId {
    /// Sentinel id used as the "no timer" marker. Never
    /// allocated by the scheduler.
    pub const NONE: TimerId = TimerId(0);

    pub fn raw(self) -> u32 {
        self.0
    }
}

/// Callback context passed to every timer callback. Mirrors
/// `window.*` in JS: scheduling methods on the context route
/// into the same scheduler that's currently pumping.
pub struct TimerCtx<'a> {
    pub dom: &'a mut TuiDom,
    scheduler: SharedScheduler,
}

impl<'a> TimerCtx<'a> {
    pub(crate) fn new(dom: &'a mut TuiDom, scheduler: SharedScheduler) -> Self {
        Self { dom, scheduler }
    }

    pub fn set_timeout(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>) + 'static,
        delay_ms: u32,
    ) -> TimerId {
        self.scheduler.borrow_mut().set_timeout(callback, delay_ms)
    }

    pub fn clear_timeout(&mut self, id: TimerId) {
        self.scheduler.borrow_mut().clear_timeout(id);
    }

    pub fn set_interval(
        &mut self,
        callback: impl FnMut(&mut TimerCtx<'_>) -> bool + 'static,
        period_ms: u32,
    ) -> TimerId {
        self.scheduler
            .borrow_mut()
            .set_interval(callback, period_ms)
    }

    pub fn clear_interval(&mut self, id: TimerId) {
        self.scheduler.borrow_mut().clear_interval(id);
    }

    pub fn request_animation_frame(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static,
    ) -> TimerId {
        self.scheduler
            .borrow_mut()
            .request_animation_frame(callback)
    }

    pub fn cancel_animation_frame(&mut self, id: TimerId) {
        self.scheduler.borrow_mut().cancel_animation_frame(id);
    }

    pub fn queue_microtask(&mut self, callback: impl FnOnce(&mut TimerCtx<'_>) + 'static) {
        self.scheduler.borrow_mut().queue_microtask(callback);
    }
}
