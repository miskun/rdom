//! The [`Scheduler`]: timeout, interval, rAF and microtask queues on a
//! virtual clock. It only stores and hands out callbacks; `super::pump`
//! runs them.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{TimerCtx, TimerId};

pub(super) type OneShotCb = Box<dyn FnOnce(&mut TimerCtx<'_>) + 'static>;
pub(super) type IntervalCb = Box<dyn FnMut(&mut TimerCtx<'_>) -> bool + 'static>;
/// rAF callbacks receive a `DOMHighResTimeStamp`-equivalent
/// (`f64` ms since `App::new`) as their second argument — matches
/// the browser `requestAnimationFrame` contract. All rAFs drained
/// in the same tick observe the same timestamp.
pub(super) type RafCb = Box<dyn FnOnce(&mut TimerCtx<'_>, f64) + 'static>;

pub(super) struct TimeoutEntry {
    pub(super) id: TimerId,
    pub(super) fires_at: Instant,
    pub(super) callback: OneShotCb,
}

pub(super) struct IntervalEntry {
    pub(super) id: TimerId,
    pub(super) period: Duration,
    pub(super) next_fire: Instant,
    pub(super) callback: IntervalCb,
}

pub(super) struct RafEntry {
    pub(super) id: TimerId,
    pub(super) callback: RafCb,
}

pub(super) struct MicrotaskEntry {
    pub(super) callback: OneShotCb,
}

/// The scheduler. Owned by `App`; one per app instance.
pub(crate) struct Scheduler {
    pub(super) next_id: u32,
    /// The interval whose callback is currently running (its entry is
    /// claimed out of `intervals` for the call). `clear_interval` on it
    /// cannot find the entry, so it flags `running_cleared` instead and
    /// the pump drops the entry on release — `clearInterval(id)` from
    /// inside the callback works as in JS.
    pub(super) running_interval: Option<TimerId>,
    pub(super) running_cleared: bool,
    /// Virtual clock; `advance_to(now)` is the only way time
    /// moves forward. Production sets this from `Instant::now()`
    /// at the top of each tick; tests pass synthetic instants.
    pub(super) now: Instant,
    /// Wall-clock origin captured at `App::new` and never mutated.
    /// rAF callbacks receive `now - app_start` (in ms) as the
    /// `DOMHighResTimeStamp` equivalent. Browser-faithful: zero at
    /// app startup, monotonically non-decreasing thereafter.
    pub(super) app_start: Instant,
    pub(super) timeouts: Vec<TimeoutEntry>,
    pub(super) intervals: Vec<IntervalEntry>,
    pub(super) raf: Vec<RafEntry>,
    pub(super) microtasks: VecDeque<MicrotaskEntry>,
}

impl Scheduler {
    pub(crate) fn new(start: Instant) -> Self {
        Self {
            running_interval: None,
            running_cleared: false,
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
        if self.running_interval == Some(id) {
            self.running_cleared = true;
        }
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
