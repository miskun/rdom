//! [`TuiTimers`]: the HTML timer API on event-listener contexts,
//! routed to the scheduler installed as current.

use super::{TimerCtx, TimerId, with_current};

/// Extension trait on `TuiEventCtx<'_>` exposing the HTML timer
/// API to event listeners. Mirrors `window.setTimeout`,
/// `window.setInterval`, `window.requestAnimationFrame`, etc.
///
/// Routes to the scheduler of the `App` currently running user code on
/// this thread — installed around event dispatch, ticks, timer pumps,
/// injected closures, animation events, and autoscroll — so it works
/// from a listener regardless of how that listener was reached. It
/// panics only when no `App` is active at all (a listener fired from a
/// bare `Dom::dispatch_event` in a test with no `App`); pass a
/// scheduler through your own context in that case.
pub trait TuiTimers: crate::sealed::Sealed {
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
            .expect("set_timeout: no App is running on this thread")
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
            .expect("set_interval: no App is running on this thread")
    }

    fn clear_interval(&mut self, id: TimerId) {
        with_current(|s| s.clear_interval(id));
    }

    fn request_animation_frame(
        &mut self,
        callback: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static,
    ) -> TimerId {
        with_current(|s| s.request_animation_frame(callback))
            .expect("request_animation_frame: no App is running on this thread")
    }

    fn cancel_animation_frame(&mut self, id: TimerId) {
        with_current(|s| s.cancel_animation_frame(id));
    }

    fn queue_microtask(&mut self, callback: impl FnOnce(&mut TimerCtx<'_>) + 'static) {
        with_current(|s| s.queue_microtask(callback));
    }
}
