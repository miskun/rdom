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

#![allow(dead_code)]

use std::cell::Cell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::TuiDom;

thread_local! {
    /// Raw pointer to the currently-active `Scheduler`. Set by
    /// [`SchedulerGuard`] for the duration of event dispatch /
    /// tick callbacks. Listener code reads it via the
    /// `TuiTimers` extension trait. (Before M4a step 8 the
    /// thread-local pattern was shared with
    /// `runtime::current_event::current_key`; that module was
    /// retired when typed `event.detail` became the canonical
    /// carrier for key/mouse payloads.)
    ///
    /// Single-threaded; the guard / read pair is the only access.
    static CURRENT_SCHEDULER: Cell<*mut Scheduler> = const { Cell::new(std::ptr::null_mut()) };
}

/// RAII guard installed by `App` around event dispatch and tick
/// callbacks. While alive, listener code can call timer
/// scheduling methods on `TuiEventCtx` (via the `TuiTimers`
/// extension trait) and they route to this scheduler.
///
/// On drop, the previous pointer (typically null) is restored —
/// nested guard installs are safe.
pub(crate) struct SchedulerGuard {
    previous: *mut Scheduler,
}

impl SchedulerGuard {
    /// Install `scheduler` as the current one. The caller
    /// guarantees that the scheduler outlives the guard. Pointer
    /// does not borrow-check; the read path (`with_current`) is
    /// `unsafe` internally and scoped to listener callbacks
    /// inside dispatch.
    pub(crate) fn install(scheduler: &mut Scheduler) -> Self {
        let ptr = scheduler as *mut Scheduler;
        let previous = CURRENT_SCHEDULER.with(|s| {
            let prev = s.get();
            s.set(ptr);
            prev
        });
        SchedulerGuard { previous }
    }
}

impl Drop for SchedulerGuard {
    fn drop(&mut self) {
        CURRENT_SCHEDULER.with(|s| s.set(self.previous));
    }
}

/// Run `f` with the currently-installed scheduler. Returns
/// `None` when called outside an event handler / tick callback.
fn with_current<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Scheduler) -> R,
{
    CURRENT_SCHEDULER.with(|s| {
        let ptr = s.get();
        if ptr.is_null() {
            None
        } else {
            // SAFETY: the pointer was set by `SchedulerGuard::install`
            // around an `&mut Scheduler` that outlives this scope
            // (App is single-threaded, owns the scheduler, and
            // installs the guard for the duration of dispatch).
            // Listener callbacks fire only inside that scope.
            // No other path mutates the scheduler concurrently.
            Some(f(unsafe { &mut *ptr }))
        }
    })
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
    scheduler: &'a mut Scheduler,
}

impl<'a> TimerCtx<'a> {
    pub(crate) fn new(dom: &'a mut TuiDom, scheduler: &'a mut Scheduler) -> Self {
        Self { dom, scheduler }
    }

    pub fn set_timeout(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>) + 'static,
        delay_ms: u32,
    ) -> TimerId {
        self.scheduler.set_timeout(callback, delay_ms)
    }

    pub fn clear_timeout(&mut self, id: TimerId) {
        self.scheduler.clear_timeout(id);
    }

    pub fn set_interval(
        &mut self,
        callback: impl FnMut(&mut TimerCtx<'_>) -> bool + 'static,
        period_ms: u32,
    ) -> TimerId {
        self.scheduler.set_interval(callback, period_ms)
    }

    pub fn clear_interval(&mut self, id: TimerId) {
        self.scheduler.clear_interval(id);
    }

    pub fn request_animation_frame(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static,
    ) -> TimerId {
        self.scheduler.request_animation_frame(callback)
    }

    pub fn cancel_animation_frame(&mut self, id: TimerId) {
        self.scheduler.cancel_animation_frame(id);
    }

    pub fn queue_microtask(&mut self, callback: impl FnOnce(&mut TimerCtx<'_>) + 'static) {
        self.scheduler.queue_microtask(callback);
    }
}

type OneShotCb = Box<dyn FnOnce(&mut TimerCtx<'_>) + 'static>;
type IntervalCb = Box<dyn FnMut(&mut TimerCtx<'_>) -> bool + 'static>;
/// rAF callbacks receive a `DOMHighResTimeStamp`-equivalent
/// (`f64` ms since `App::new`) as their second argument — matches
/// the browser `requestAnimationFrame` contract. All rAFs drained
/// in the same tick observe the same timestamp.
type RafCb = Box<dyn FnOnce(&mut TimerCtx<'_>, f64) + 'static>;

struct TimeoutEntry {
    id: TimerId,
    fires_at: Instant,
    callback: OneShotCb,
}

struct IntervalEntry {
    id: TimerId,
    period: Duration,
    next_fire: Instant,
    callback: IntervalCb,
}

struct RafEntry {
    id: TimerId,
    callback: RafCb,
}

struct MicrotaskEntry {
    callback: OneShotCb,
}

/// The scheduler. Owned by `App`; one per app instance.
pub(crate) struct Scheduler {
    next_id: u32,
    /// Virtual clock; `advance_to(now)` is the only way time
    /// moves forward. Production sets this from `Instant::now()`
    /// at the top of each tick; tests pass synthetic instants.
    now: Instant,
    /// Wall-clock origin captured at `App::new` and never mutated.
    /// rAF callbacks receive `now - app_start` (in ms) as the
    /// `DOMHighResTimeStamp` equivalent. Browser-faithful: zero at
    /// app startup, monotonically non-decreasing thereafter.
    app_start: Instant,
    timeouts: Vec<TimeoutEntry>,
    intervals: Vec<IntervalEntry>,
    raf: Vec<RafEntry>,
    microtasks: VecDeque<MicrotaskEntry>,
}

impl Scheduler {
    pub(crate) fn new(start: Instant) -> Self {
        Self {
            next_id: 1, // 0 is the NONE sentinel — matches JS falsy
            now: start,
            app_start: start,
            timeouts: Vec::new(),
            intervals: Vec::new(),
            raf: Vec::new(),
            microtasks: VecDeque::new(),
        }
    }

    /// Frame timestamp in milliseconds since `App::new`. Matches
    /// the browser `DOMHighResTimeStamp` value passed to rAF
    /// callbacks. Stable for the duration of a tick — all rAF
    /// callbacks drained in one `pump_raf` see this same value.
    pub(crate) fn frame_timestamp_ms(&self) -> f64 {
        self.now
            .saturating_duration_since(self.app_start)
            .as_secs_f64()
            * 1000.0
    }

    fn alloc_id(&mut self) -> TimerId {
        let id = TimerId(self.next_id);
        // Wraparound is effectively never (u32::MAX outstanding
        // timers); on overflow we skip 0 to keep the falsy
        // sentinel reserved.
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        id
    }

    pub(crate) fn now(&self) -> Instant {
        self.now
    }

    /// Move the virtual clock forward to `now`. Must be
    /// monotonically non-decreasing — production guarantees this
    /// because it always passes `Instant::now()`.
    pub(crate) fn set_now(&mut self, now: Instant) {
        if now > self.now {
            self.now = now;
        }
    }

    pub fn set_timeout(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>) + 'static,
        delay_ms: u32,
    ) -> TimerId {
        let id = self.alloc_id();
        let fires_at = self.now + Duration::from_millis(delay_ms as u64);
        self.timeouts.push(TimeoutEntry {
            id,
            fires_at,
            callback: Box::new(callback),
        });
        id
    }

    pub fn clear_timeout(&mut self, id: TimerId) {
        self.timeouts.retain(|e| e.id != id);
    }

    pub fn set_interval(
        &mut self,
        callback: impl FnMut(&mut TimerCtx<'_>) -> bool + 'static,
        period_ms: u32,
    ) -> TimerId {
        let id = self.alloc_id();
        let period = Duration::from_millis(period_ms as u64);
        self.intervals.push(IntervalEntry {
            id,
            period,
            next_fire: self.now + period,
            callback: Box::new(callback),
        });
        id
    }

    pub fn clear_interval(&mut self, id: TimerId) {
        self.intervals.retain(|e| e.id != id);
    }

    /// Schedule `callback` to run before the next paint. The
    /// callback receives a `DOMHighResTimeStamp`-equivalent (`f64`
    /// ms since `App::new`) as its second argument. All rAFs that
    /// drain in the same tick observe the same timestamp —
    /// browser-faithful per the HTML spec.
    pub fn request_animation_frame(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static,
    ) -> TimerId {
        let id = self.alloc_id();
        self.raf.push(RafEntry {
            id,
            callback: Box::new(callback),
        });
        id
    }

    pub fn cancel_animation_frame(&mut self, id: TimerId) {
        self.raf.retain(|e| e.id != id);
    }

    pub fn queue_microtask(&mut self, callback: impl FnOnce(&mut TimerCtx<'_>) + 'static) {
        self.microtasks.push_back(MicrotaskEntry {
            callback: Box::new(callback),
        });
    }

    /// Earliest deadline across timeouts + intervals. `None` if
    /// the scheduler has nothing pending. Used by the App tick
    /// loop to size its `crossterm::poll` timeout.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        let timeout = self.timeouts.iter().map(|e| e.fires_at).min();
        let interval = self.intervals.iter().map(|e| e.next_fire).min();
        match (timeout, interval) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// True when the scheduler has any work that benefits from
    /// the tightened animation frame rate. Animations register
    /// elsewhere (animation registry); this only tracks rAF +
    /// pending timers shorter than a frame.
    pub(crate) fn has_active_raf(&self) -> bool {
        !self.raf.is_empty()
    }

    /// Pop and return every timeout entry whose `fires_at <=
    /// self.now`. Caller invokes each callback with a real
    /// `TimerCtx`. Returned in fire order (earliest first).
    pub(crate) fn drain_expired_timeouts(&mut self) -> Vec<OneShotCb> {
        let mut expired_idx: Vec<usize> = self
            .timeouts
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                if e.fires_at <= self.now {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        // Sort by fires_at via a stable mapping. Indices are
        // already in vector order; we want the actual entries in
        // fires_at order.
        expired_idx.sort_by_key(|&i| self.timeouts[i].fires_at);
        // Remove highest-index-first so earlier indices stay valid.
        let mut out = Vec::with_capacity(expired_idx.len());
        for &i in expired_idx.iter().rev() {
            out.push(self.timeouts.remove(i).callback);
        }
        out.reverse(); // restore fire-order
        out
    }

    /// Pop every interval entry whose `next_fire <= self.now`,
    /// returning `(id, callback)` pairs. The caller invokes each;
    /// the scheduler reschedules at `next_fire + period` if the
    /// callback returns `true`. Returned in fire order.
    ///
    /// Note: callbacks for intervals are `FnMut` so we can't
    /// simply move them out and back. Instead we use a
    /// "claim/release" pattern — see `pump_intervals`.
    pub(crate) fn drain_expired_interval_ids(&self) -> Vec<TimerId> {
        let mut due: Vec<(TimerId, Instant)> = self
            .intervals
            .iter()
            .filter(|e| e.next_fire <= self.now)
            .map(|e| (e.id, e.next_fire))
            .collect();
        due.sort_by_key(|(_, t)| *t);
        due.into_iter().map(|(id, _)| id).collect()
    }

    /// Drain all queued rAF callbacks for one frame.
    pub(crate) fn drain_raf(&mut self) -> Vec<RafCb> {
        std::mem::take(&mut self.raf)
            .into_iter()
            .map(|e| e.callback)
            .collect()
    }

    /// Drain one microtask. Caller loops until empty.
    pub(crate) fn pop_microtask(&mut self) -> Option<OneShotCb> {
        self.microtasks.pop_front().map(|e| e.callback)
    }
}

/// Pump every expired interval, calling the callback with the
/// supplied ctx-builder closure. Schedules the next fire if the
/// callback returns `true`; removes the entry if it returns
/// `false` (self-cancel).
///
/// Lives outside `Scheduler` because it borrows interval entries
/// for a `FnMut` call which the borrow checker would otherwise
/// reject if we held a mutable borrow on the whole scheduler.
pub(crate) fn pump_intervals(scheduler: &mut Scheduler, dom: &mut TuiDom, expired: &[TimerId]) {
    for &id in expired {
        // Find the entry; tolerate the case where a previous
        // callback in this same drain cleared it.
        let pos = scheduler.intervals.iter().position(|e| e.id == id);
        let Some(pos) = pos else { continue };
        // Take ownership of the callback so we can call it while
        // also holding `&mut scheduler` for the ctx.
        let mut entry = scheduler.intervals.swap_remove(pos);
        let keep = {
            let mut ctx = TimerCtx::new(dom, scheduler);
            (entry.callback)(&mut ctx)
        };
        if keep {
            entry.next_fire += entry.period;
            scheduler.intervals.push(entry);
        }
    }
}

/// Drain expired timeouts and invoke each callback with a real
/// `TimerCtx`. Convenience for App's tick loop.
pub(crate) fn pump_timeouts(scheduler: &mut Scheduler, dom: &mut TuiDom) {
    let cbs = scheduler.drain_expired_timeouts();
    for cb in cbs {
        let mut ctx = TimerCtx::new(dom, scheduler);
        cb(&mut ctx);
    }
}

/// Drain queued rAF callbacks for the current frame. All callbacks
/// in this drain receive the same `frame_timestamp_ms()` value —
/// matches the browser contract that one frame = one timestamp.
pub(crate) fn pump_raf(scheduler: &mut Scheduler, dom: &mut TuiDom) {
    let timestamp = scheduler.frame_timestamp_ms();
    let cbs = scheduler.drain_raf();
    for cb in cbs {
        let mut ctx = TimerCtx::new(dom, scheduler);
        cb(&mut ctx, timestamp);
    }
}

/// Extension trait on `TuiEventCtx<'_>` exposing the HTML timer
/// API to event listeners. Mirrors `window.setTimeout`,
/// `window.setInterval`, `window.requestAnimationFrame`, etc.
///
/// Routes through the thread-local current scheduler installed by
/// the App during dispatch. Calling these methods *outside* an
/// event handler / tick callback panics. In practice this never
/// surprises users because the extension is `&mut TuiEventCtx`,
/// which is itself only available inside dispatch.
pub trait TuiTimers {
    fn set_timeout(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>) + 'static,
        delay_ms: u32,
    ) -> TimerId;

    fn clear_timeout(&mut self, id: TimerId);

    fn set_interval(
        &mut self,
        callback: impl FnMut(&mut TimerCtx<'_>) -> bool + 'static,
        period_ms: u32,
    ) -> TimerId;

    fn clear_interval(&mut self, id: TimerId);

    fn request_animation_frame(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static,
    ) -> TimerId;

    fn cancel_animation_frame(&mut self, id: TimerId);

    fn queue_microtask(&mut self, callback: impl FnOnce(&mut TimerCtx<'_>) + 'static);
}

impl<'a> TuiTimers for crate::TuiEventCtx<'a> {
    fn set_timeout(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>) + 'static,
        delay_ms: u32,
    ) -> TimerId {
        with_current(|s| s.set_timeout(callback, delay_ms))
            .expect("set_timeout called outside event dispatch")
    }

    fn clear_timeout(&mut self, id: TimerId) {
        with_current(|s| s.clear_timeout(id));
    }

    fn set_interval(
        &mut self,
        callback: impl FnMut(&mut TimerCtx<'_>) -> bool + 'static,
        period_ms: u32,
    ) -> TimerId {
        with_current(|s| s.set_interval(callback, period_ms))
            .expect("set_interval called outside event dispatch")
    }

    fn clear_interval(&mut self, id: TimerId) {
        with_current(|s| s.clear_interval(id));
    }

    fn request_animation_frame(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static,
    ) -> TimerId {
        with_current(|s| s.request_animation_frame(callback))
            .expect("request_animation_frame called outside event dispatch")
    }

    fn cancel_animation_frame(&mut self, id: TimerId) {
        with_current(|s| s.cancel_animation_frame(id));
    }

    fn queue_microtask(&mut self, callback: impl FnOnce(&mut TimerCtx<'_>) + 'static) {
        with_current(|s| s.queue_microtask(callback));
    }
}

/// Drain microtask queue to empty. Microtasks queued *during*
/// this drain are appended and drained in the same loop —
/// matches HTML spec.
pub(crate) fn drain_microtasks(scheduler: &mut Scheduler, dom: &mut TuiDom) {
    while let Some(cb) = scheduler.pop_microtask() {
        let mut ctx = TimerCtx::new(dom, scheduler);
        cb(&mut ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn epoch() -> Instant {
        Instant::now()
    }

    fn dom_for_tests() -> TuiDom {
        TuiDom::new()
    }

    // ── §15.1 — set_timeout fires after delay ─────────────────

    #[test]
    fn timeout_fires_after_delay() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let fired = Rc::new(Cell::new(0u32));
        let f = fired.clone();
        sched.set_timeout(move |_ctx| f.set(f.get() + 1), 100);

        // Before deadline: not fired.
        sched.set_now(start + Duration::from_millis(50));
        pump_timeouts(&mut sched, &mut dom);
        assert_eq!(fired.get(), 0);

        // After deadline: fired exactly once.
        sched.set_now(start + Duration::from_millis(150));
        pump_timeouts(&mut sched, &mut dom);
        assert_eq!(fired.get(), 1);

        // Doesn't fire again.
        sched.set_now(start + Duration::from_millis(300));
        pump_timeouts(&mut sched, &mut dom);
        assert_eq!(fired.get(), 1);
    }

    // ── §15.2 — clear_timeout cancels ─────────────────────────

    #[test]
    fn clear_timeout_cancels_before_deadline() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let fired = Rc::new(Cell::new(0u32));
        let f = fired.clone();
        let id = sched.set_timeout(move |_| f.set(f.get() + 1), 100);

        sched.clear_timeout(id);
        sched.set_now(start + Duration::from_millis(200));
        pump_timeouts(&mut sched, &mut dom);
        assert_eq!(fired.get(), 0);
    }

    // ── §15.3 — set_interval repeats ──────────────────────────

    #[test]
    fn interval_repeats_at_period() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sched.set_interval(
            move |_| {
                c.set(c.get() + 1);
                true
            },
            50,
        );

        // Advance 175ms — 3 fires expected (at 50, 100, 150).
        sched.set_now(start + Duration::from_millis(175));
        let due = sched.drain_expired_interval_ids();
        pump_intervals(&mut sched, &mut dom, &due);
        // First pump catches one entry per drain — but the
        // collected `due` list only has the entry once, so we
        // need a loop in the App. Test the pump-loop semantic:
        while !sched.drain_expired_interval_ids().is_empty() {
            let due = sched.drain_expired_interval_ids();
            pump_intervals(&mut sched, &mut dom, &due);
        }
        assert_eq!(count.get(), 3);
    }

    // ── §15.4 — Interval returning false self-cancels ─────────

    #[test]
    fn interval_self_cancels_on_false_return() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        sched.set_interval(
            move |_| {
                c.set(c.get() + 1);
                c.get() < 2
            },
            50,
        );

        // Advance 500ms — should fire at 50ms (count=1, keep=true)
        // and at 100ms (count=2, keep=false). After that the
        // entry is removed.
        sched.set_now(start + Duration::from_millis(500));
        loop {
            let due = sched.drain_expired_interval_ids();
            if due.is_empty() {
                break;
            }
            pump_intervals(&mut sched, &mut dom, &due);
        }
        assert_eq!(count.get(), 2);
        // No more pending intervals.
        assert!(sched.intervals.is_empty());
    }

    // ── §15.5 — clear_timeout on stale handle is no-op ────────

    #[test]
    fn clear_timeout_on_stale_handle_is_noop() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();

        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        let id = sched.set_timeout(move |_| f.set(true), 50);

        // Fire it.
        sched.set_now(start + Duration::from_millis(100));
        pump_timeouts(&mut sched, &mut dom);
        assert!(fired.get());

        // Clearing the now-stale id is a silent no-op.
        sched.clear_timeout(id);
        // Also: a never-issued id is a no-op.
        sched.clear_timeout(TimerId(99999));
    }

    // ── §15.6 — request_animation_frame fires once per drain ──

    #[test]
    fn raf_fires_once_per_drain() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let fired = Rc::new(Cell::new(0u32));
        let f = fired.clone();
        sched.request_animation_frame(move |_, _ts| f.set(f.get() + 1));

        pump_raf(&mut sched, &mut dom);
        assert_eq!(fired.get(), 1);

        // Second drain finds nothing — rAF is one-shot.
        pump_raf(&mut sched, &mut dom);
        assert_eq!(fired.get(), 1);
    }

    // ── §15.7 — queue_microtask drains FIFO ────────────────────

    #[test]
    fn microtasks_drain_in_fifo_order_including_late_queues() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let order = Rc::new(std::cell::RefCell::new(Vec::<u32>::new()));

        let o = order.clone();
        sched.queue_microtask(move |ctx| {
            o.borrow_mut().push(1);
            // Microtask queued during drain is appended and
            // drained in the same loop (HTML spec).
            let o2 = o.clone();
            ctx.queue_microtask(move |_| o2.borrow_mut().push(3));
        });
        let o = order.clone();
        sched.queue_microtask(move |_| o.borrow_mut().push(2));

        drain_microtasks(&mut sched, &mut dom);
        assert_eq!(*order.borrow(), vec![1, 2, 3]);
    }

    // ── §15.8 — Handles unique and monotonically allocated ────

    #[test]
    fn handles_are_unique_and_monotonic() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let a = sched.set_timeout(|_| {}, 100);
        let b = sched.set_timeout(|_| {}, 100);
        let c = sched.request_animation_frame(|_, _ts| {});
        let d = sched.set_interval(|_| true, 50);
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(c, d);
        // First handle is 1 (0 is the NONE sentinel).
        assert_eq!(a.raw(), 1);
        // Monotonic.
        assert!(b.raw() > a.raw());
        assert!(c.raw() > b.raw());
        assert!(d.raw() > c.raw());
    }

    // ── Listener-side surface via TuiTimers + thread-local ───

    #[test]
    fn listener_can_call_set_timeout_via_extension_trait() {
        use rdom_core::ListenerOptions;

        let mut dom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        let fired = Rc::new(Cell::new(0u32));
        let f = fired.clone();
        // Listener schedules a 100ms timeout.
        dom.add_event_listener(div, "click", ListenerOptions::default(), move |ctx| {
            let f2 = f.clone();
            // Extension-trait method lights up here.
            ctx.set_timeout(move |_| f2.set(f2.get() + 1), 100);
        })
        .unwrap();

        let start = epoch();
        let mut sched = Scheduler::new(start);
        // Install scheduler guard (mimics what App::handle_event does).
        let _g = SchedulerGuard::install(&mut sched);
        // Dispatch the click event manually.
        let mut ev = rdom_core::Event::new("click");
        let _ = dom.dispatch_event(div, &mut ev);
        drop(_g);

        // Before the deadline.
        sched.set_now(start + Duration::from_millis(50));
        pump_timeouts(&mut sched, &mut dom);
        assert_eq!(fired.get(), 0);

        // After the deadline.
        sched.set_now(start + Duration::from_millis(200));
        pump_timeouts(&mut sched, &mut dom);
        assert_eq!(fired.get(), 1);
    }

    #[test]
    fn scheduler_guard_restores_previous_on_drop() {
        let start = epoch();
        let mut a = Scheduler::new(start);
        let mut b = Scheduler::new(start);
        // Outer guard installs `a`.
        let _outer = SchedulerGuard::install(&mut a);
        // Inner guard installs `b`.
        {
            let _inner = SchedulerGuard::install(&mut b);
            // While inner is alive, the current scheduler is `b`.
            let count_b = with_current(|s| s.next_id);
            assert_eq!(count_b, Some(1));
        }
        // After inner drops, `a` is restored.
        let count_a = with_current(|s| s.next_id);
        assert_eq!(count_a, Some(1));
    }

    // ── D-M3-4 — rAF callback receives DOMHighResTimeStamp ────

    #[test]
    fn raf_callback_receives_timestamp_zero_at_app_start() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let observed = Rc::new(Cell::new(-1.0_f64));
        let o = observed.clone();
        sched.request_animation_frame(move |_ctx, ts| o.set(ts));
        pump_raf(&mut sched, &mut dom);
        // No clock advancement → timestamp is exactly 0.0.
        assert_eq!(observed.get(), 0.0);
    }

    #[test]
    fn raf_callback_timestamp_reflects_scheduler_clock() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let observed = Rc::new(Cell::new(-1.0_f64));
        let o = observed.clone();
        sched.request_animation_frame(move |_ctx, ts| o.set(ts));
        sched.set_now(start + Duration::from_millis(16));
        pump_raf(&mut sched, &mut dom);
        // 16ms elapsed since app start → timestamp is 16.0.
        assert_eq!(observed.get(), 16.0);
    }

    #[test]
    fn raf_timestamps_monotonic_across_ticks() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let stamps = Rc::new(std::cell::RefCell::new(Vec::<f64>::new()));

        let s = stamps.clone();
        sched.request_animation_frame(move |_ctx, ts| s.borrow_mut().push(ts));
        sched.set_now(start + Duration::from_millis(16));
        pump_raf(&mut sched, &mut dom);

        let s = stamps.clone();
        sched.request_animation_frame(move |_ctx, ts| s.borrow_mut().push(ts));
        sched.set_now(start + Duration::from_millis(33));
        pump_raf(&mut sched, &mut dom);

        let s = stamps.clone();
        sched.request_animation_frame(move |_ctx, ts| s.borrow_mut().push(ts));
        sched.set_now(start + Duration::from_millis(50));
        pump_raf(&mut sched, &mut dom);

        let captured = stamps.borrow().clone();
        assert_eq!(captured.len(), 3);
        assert!(captured[0] <= captured[1]);
        assert!(captured[1] <= captured[2]);
        // Floats but the values are exact multiples of ms here.
        assert_eq!(captured, vec![16.0, 33.0, 50.0]);
    }

    #[test]
    fn raf_timestamps_coherent_within_one_tick() {
        // Browser semantics: all rAF callbacks within the same
        // frame observe the same timestamp. We schedule two rAFs
        // before advancing the clock and pump them in one drain —
        // both must see the same value.
        let start = epoch();
        let mut sched = Scheduler::new(start);
        let mut dom = dom_for_tests();
        let stamps = Rc::new(std::cell::RefCell::new(Vec::<f64>::new()));

        let s1 = stamps.clone();
        sched.request_animation_frame(move |_ctx, ts| s1.borrow_mut().push(ts));
        let s2 = stamps.clone();
        sched.request_animation_frame(move |_ctx, ts| s2.borrow_mut().push(ts));

        sched.set_now(start + Duration::from_millis(16));
        pump_raf(&mut sched, &mut dom);

        let captured = stamps.borrow().clone();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0], 16.0);
        assert_eq!(captured[1], 16.0);
    }

    // ── §15.9 — next_deadline returns shortest ────────────────

    #[test]
    fn next_deadline_returns_shortest_pending() {
        let start = epoch();
        let mut sched = Scheduler::new(start);
        assert_eq!(sched.next_deadline(), None);

        sched.set_timeout(|_| {}, 200);
        sched.set_timeout(|_| {}, 50); // closer
        sched.set_timeout(|_| {}, 500);

        assert_eq!(
            sched.next_deadline(),
            Some(start + Duration::from_millis(50))
        );

        // Adding an interval that fires sooner wins.
        sched.set_interval(|_| true, 10);
        assert_eq!(
            sched.next_deadline(),
            Some(start + Duration::from_millis(10))
        );
    }
}
