//! The pumps: drain the scheduler's due callbacks and run each with a
//! [`TimerCtx`], with the scheduler installed as current and a
//! microtask checkpoint after each task (HTML §8.1.7.3).

use super::{SchedulerGuard, SharedScheduler, TimerCtx, TimerId};
use crate::TuiDom;

/// Pump every expired interval, calling the callback with the
/// supplied ctx-builder closure. Schedules the next fire if the
/// callback returns `true`; removes the entry if it returns
/// `false` (self-cancel).
///
/// Lives outside `Scheduler` because it borrows interval entries
/// for a `FnMut` call which the borrow checker would otherwise
/// reject if we held a mutable borrow on the whole scheduler.
pub(crate) fn pump_intervals(scheduler: &SharedScheduler, dom: &mut TuiDom, expired: &[TimerId]) {
    let _current = SchedulerGuard::install(scheduler);
    for &id in expired {
        // Claim the entry (tolerating a previous callback in this same
        // drain having cleared it), call it with the cell free, then
        // release it back if it wants to keep firing.
        let claimed = {
            let mut s = scheduler.borrow_mut();
            let pos = s.intervals.iter().position(|e| e.id == id);
            pos.map(|p| s.intervals.swap_remove(p))
        };
        let Some(mut entry) = claimed else { continue };
        {
            let mut s = scheduler.borrow_mut();
            s.running_interval = Some(id);
            s.running_cleared = false;
        }
        let keep = {
            let mut ctx = TimerCtx::new(dom, scheduler.clone());
            (entry.callback)(&mut ctx)
        };
        drain_microtasks(scheduler, dom);
        let cleared = {
            let mut s = scheduler.borrow_mut();
            s.running_interval = None;
            std::mem::take(&mut s.running_cleared)
        };
        if keep && !cleared {
            entry.next_fire += entry.period;
            scheduler.borrow_mut().intervals.push(entry);
        }
    }
}

/// Drain expired timeouts and invoke each callback with a real
/// `TimerCtx`. Convenience for App's tick loop. Returns whether any
/// callback ran.
pub(crate) fn pump_timeouts(scheduler: &SharedScheduler, dom: &mut TuiDom) -> bool {
    let _current = SchedulerGuard::install(scheduler);
    let cbs = scheduler.borrow_mut().drain_expired_timeouts();
    let ran = !cbs.is_empty();
    for cb in cbs {
        {
            let mut ctx = TimerCtx::new(dom, scheduler.clone());
            cb(&mut ctx);
        }
        // Each timer callback is a task: microtask checkpoint after it
        // (HTML event loop §8.1.7.3 step 8).
        drain_microtasks(scheduler, dom);
    }
    ran
}

/// Drain queued rAF callbacks for the current frame. All callbacks
/// in this drain receive the same `frame_timestamp_ms()` value —
/// matches the browser contract that one frame = one timestamp.
/// Returns whether any callback ran.
pub(crate) fn pump_raf(scheduler: &SharedScheduler, dom: &mut TuiDom) -> bool {
    let _current = SchedulerGuard::install(scheduler);
    let (timestamp, cbs) = {
        let mut s = scheduler.borrow_mut();
        (s.frame_timestamp_ms(), s.drain_raf())
    };
    let ran = !cbs.is_empty();
    for cb in cbs {
        {
            let mut ctx = TimerCtx::new(dom, scheduler.clone());
            cb(&mut ctx, timestamp);
        }
        // "Run the animation frame callbacks" performs a microtask
        // checkpoint after each callback (HTML §8.1.7.3 step 14.10 →
        // "clean up after running script").
        drain_microtasks(scheduler, dom);
    }
    ran
}

/// Drain microtask queue to empty. Microtasks queued *during*
/// this drain are appended and drained in the same loop —
/// matches HTML spec. Returns whether any microtask ran.
pub(crate) fn drain_microtasks(scheduler: &SharedScheduler, dom: &mut TuiDom) -> bool {
    let _current = SchedulerGuard::install(scheduler);
    let mut ran = false;
    loop {
        let next = scheduler.borrow_mut().pop_microtask();
        let Some(cb) = next else { break };
        ran = true;
        let mut ctx = TimerCtx::new(dom, scheduler.clone());
        cb(&mut ctx);
    }
    ran
}
