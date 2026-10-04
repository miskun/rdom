//! The event loop of an [`App`]: [`App::run`] (poll crossterm, route,
//! tick, draw), the poll timeout that wakes it for the next timer, caret
//! blink, smooth-scroll step or autoscroll tick, the scheduler pump, the
//! virtual clock of [`App::advance`], the `on_tick` callback and the
//! [`AppHandle`](super::AppHandle) drains.

use std::io::{self, Stdout};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::time::Duration;

use crossterm::event;

use super::autoscroll::AUTOSCROLL_PERIOD;
use super::redraw::Redraw;
use super::{App, AppContext, ControlFlow};
use crate::render::backend::Backend;
use crate::render::backend_crossterm::{CrosstermBackend, leave_tui_mode};

impl App<CrosstermBackend<Stdout>> {
    /// Block until exit. Runs the event loop: poll crossterm, route
    /// events, tick, cascade + layout + paint when dirty. Exits on
    /// `ControlFlow::Quit`, Ctrl-C, or `AppContext::quit()`.
    ///
    /// On exit, the terminal is restored via [`leave_tui_mode`].
    /// This is also guaranteed on panic — the `Drop` impl on `App`
    /// runs it if `run` unwinds.
    ///
    /// **Startup color-scheme query.** Before the first frame, unless
    /// the app set a scheme ([`App::with_color_scheme`] /
    /// [`App::set_color_scheme`]), `run` asks the terminal for its
    /// background (OSC 11, then DA1 to mark the end of the replies) on
    /// Unix when stdout is a terminal, reading the replies from stdin —
    /// or `/dev/tty` when stdin is redirected. It waits up to 200 ms for
    /// a reply to begin — that is the worst case for a terminal that
    /// answers neither query — and up to 800 ms more once one has (a slow
    /// link); a terminal that answers DA1 ends the wait at once. Keys
    /// typed during the wait are read by the query, not the input
    /// reader, and dropped; a reply arriving after the wait ends reaches
    /// the input reader as keystrokes. The reported background picks the
    /// preferred scheme ([`App::detected_background`]); no answer leaves
    /// it dark.
    pub fn run(mut self) -> io::Result<()> {
        // The terminal's color scheme, asked before the input reader
        // starts (it would see the replies as keys).
        self.detect_color_scheme();
        // Initial paint — user should see something even before any
        // event fires.
        self.redraw.note(Redraw::Cascade);

        // Wrap the whole loop in catch_unwind. If a listener panics
        // mid-handler, the terminal state is still restored via the
        // `TerminalGuard` held in `self.guard` (dropped on unwind),
        // then we resume_unwind to propagate the panic to the
        // caller with a usable shell behind it.
        //
        // AssertUnwindSafe: `App` holds Rc-based state (Tree,
        // DirtyTracker observers) that Rust's unwind-safety
        // analysis flags as !UnwindSafe. In practice the loop body
        // either completes the iteration or panics; there's no
        // "partial" state the caller can inspect after a panic
        // (we re-panic). Explicit assertion is appropriate.
        let loop_result = panic::catch_unwind(AssertUnwindSafe(|| -> io::Result<()> {
            self.draw_if_dirty()?;
            loop {
                self.drain_handle_signals();
                if self.should_quit {
                    break;
                }

                let poll_timeout = self.compute_poll_timeout();
                let has_event = event::poll(poll_timeout).unwrap_or(false);
                if has_event {
                    // Drain ALL currently-queued events before
                    // drawing. At ~100Hz mouse motion, processing
                    // one event then drawing then processing the
                    // next means each paint (which can be 80ms+
                    // for a complex scene) lets ~8 motion events
                    // queue up. The hover state visibly lags
                    // behind the cursor. Draining collapses a
                    // burst into a single paint that reflects the
                    // final state — same pattern ratatui apps
                    // use. We still bound the drain to whatever's
                    // queued right now: an infinite tight drain
                    // would starve drawing if events arrive
                    // faster than we can drain.
                    loop {
                        match event::read() {
                            Ok(ev) => {
                                crate::rdom_trace!("event::read() -> Ok({ev:?})");
                                // Multi-click / type-ahead windows read the
                                // scheduler clock; sync it per event so a
                                // drained burst does not share one stale
                                // instant (`advance` never comes through
                                // here, so it stays deterministic).
                                self.scheduler
                                    .borrow_mut()
                                    .set_now(std::time::Instant::now());
                                self.handle_event(ev);
                            }
                            Err(e) => {
                                crate::rdom_trace!("event::read() -> Err({e:?})");
                                break;
                            }
                        }
                        // Zero-timeout poll: only continue if
                        // another event is already buffered. As
                        // soon as the queue drains we exit the
                        // loop and proceed to draw.
                        if !event::poll(std::time::Duration::ZERO).unwrap_or(false) {
                            break;
                        }
                    }
                } else {
                    self.tick();
                }
                // M3: advance the scheduler clock and pump expired
                // work. Microtasks drain after every chunk so that
                // chained queue_microtask calls during a callback
                // run before the next paint, matching HTML spec.
                self.pump_scheduler();
                self.service_autoscroll();
                self.drain_handle_injections();
                self.draw_if_dirty()?;
            }
            Ok(())
        }));

        // Whether we exited normally or via panic, restore the
        // terminal now. The TerminalGuard drop would also do this,
        // but explicitly doing it here keeps the ordering clear
        // (restore → propagate the Result / panic).
        let mut stdout = io::stdout();
        let _ = leave_tui_mode(&mut stdout);

        match loop_result {
            Ok(inner_result) => inner_result,
            Err(payload) => panic::resume_unwind(payload),
        }
    }
}

impl<B: Backend> App<B> {
    /// Compute the right `poll` timeout based on the next
    /// scheduled deadline + tick rate + animation frame budget.
    /// When the scheduler has nothing pending, this is just
    /// `tick_rate` — preserves the original idle behavior.
    fn compute_poll_timeout(&self) -> Duration {
        let now = std::time::Instant::now();
        let to_deadline = self
            .scheduler
            .borrow()
            .next_deadline()
            .map(|d| d.saturating_duration_since(now))
            .unwrap_or(self.tick_rate);
        // If we have pending rAF callbacks (= an animation
        // frame is queued), tighten to the frame budget.
        let frame_floor = if self.scheduler.borrow().has_active_raf() {
            Duration::from_millis(self.animation_frame_ms as u64)
        } else {
            self.tick_rate
        };
        let mut base = to_deadline.min(frame_floor).min(self.tick_rate);
        if let Some(flip) = self.prelude.caret_blink.next_deadline() {
            base = base.min(flip.saturating_duration_since(now));
        }
        if let Some(step) = self.prelude.smooth_scroll_next {
            base = base.min(step.saturating_duration_since(now));
        }
        // While a drag-autoscroll is armed, wake at least once per period so the
        // tick fires even with the pointer held still (no new input events).
        if self.autoscroll.pointer.is_some() {
            base.min(AUTOSCROLL_PERIOD)
        } else {
            base
        }
    }

    /// Pump the scheduler: advance the clock, drain microtasks,
    /// fire expired timeouts/intervals, drain microtasks again
    /// (callbacks may queue them), then drain rAF before paint.
    fn pump_scheduler(&mut self) {
        // Sync the clock to wall time — production only ever
        // moves forward; tests use the virtual-clock API
        // (`advance`) directly.
        self.scheduler
            .borrow_mut()
            .set_now(std::time::Instant::now());
        self.pump_due();
    }

    /// Drain everything due at the scheduler's *current* clock — microtasks,
    /// expired timeouts/intervals, then rAF — without touching the clock.
    /// `pump_scheduler` (live loop) syncs the clock to wall time first;
    /// `advance` (headless/test) moves the virtual clock first. Both then call
    /// this so the drain order is identical.
    fn pump_due(&mut self) {
        use crate::runtime::timers as t;
        // Microtasks first, in case a previous handler queued
        // one and we haven't drained yet.
        let sched = self.scheduler.clone();
        // One checkpoint for anything queued since the last task; each
        // pump then checkpoints after every callback it runs (HTML
        // §8.1.7.3), so nothing is left for a trailing drain.
        // The callbacks count as having touched the App only when they
        // left evidence of a change (`P7G-TICK-TOUCHED-1`): an interval
        // that fires and changes nothing leaves the next frame's
        // whole-tree checks skipped.
        let before = self.change_evidence();
        t::drain_microtasks(&sched, &mut self.dom);
        t::pump_timeouts(&sched, &mut self.dom);
        let due = sched.borrow().drain_expired_interval_ids();
        t::pump_intervals(&sched, &mut self.dom, &due);
        t::pump_raf(&sched, &mut self.dom);
        self.prelude.touched |= self.change_evidence() != before;
    }

    /// Evidence that code the App ran changed what the frame's
    /// whole-tree checks read (`P7G-TICK-TOUCHED-1`): the mutation
    /// records the dirty tracker observed, and the runtime-managed
    /// `TuiExt` writes no mutation reports (`runtime::state_writes`:
    /// scroll offsets through the scroll API, smooth scrolls, custom
    /// validity). Compared before and after a callback.
    fn change_evidence(&self) -> (u64, u64) {
        (
            self.tracker.records_seen(),
            crate::runtime::state_writes::generation(),
        )
    }

    /// Advance the virtual scheduler clock by `ms` and service everything that
    /// comes due, then redraw if dirty. For **headless / simulation / test**
    /// drivers that don't run the live [`run`](Self::run) loop (which syncs to
    /// wall time). Fires timeouts, intervals, rAF, and microtasks whose deadline
    /// falls within the elapsed window, services drag autoscroll, and runs the
    /// closures queued by [`AppHandle::inject`](super::AppHandle::inject), exactly as the loop would —
    /// making timer-driven runtime behavior (animations, autoscroll) and
    /// handler-queued work (stylesheet intents) deterministically testable.
    /// `advance(0)` finishes the current loop iteration without moving the
    /// clock. Advance one period at a time to step a repeating timer
    /// tick-by-tick.
    pub fn advance(&mut self, ms: u64) -> io::Result<()> {
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        let target = self.scheduler.borrow().now() + std::time::Duration::from_millis(ms);
        self.scheduler.borrow_mut().set_now(target);
        self.pump_due();
        self.service_autoscroll();
        self.drain_handle_injections();
        self.draw_if_dirty()
    }

    /// Fire the registered `on_tick` callback, if any. No-op when
    /// unset.
    pub(crate) fn tick(&mut self) {
        let Some(mut cb) = self.on_tick.take() else {
            return;
        };
        // Touched only on evidence of a change: the documented pattern of
        // draining an (often empty) channel here must not walk the tree
        // every tick (`P7G-TICK-TOUCHED-1`).
        let before = self.change_evidence();
        // Install the scheduler thread-local so on_tick handlers
        // can use the `TuiTimers` extension surface too (apps
        // that schedule fade-outs from a tick callback, etc.).
        let _scheduler_guard = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        let (queued, intents) = {
            let mut ctx = AppContext::new(&mut self.dom, &mut self.stylesheet_ids);
            let flow = cb(&mut ctx);
            self.redraw.note_if(ctx.redraw_requested, Redraw::Cascade);
            // A `request_redraw` may follow a direct `TuiExt` write.
            self.prelude.touched |= ctx.redraw_requested;
            self.should_quit |= ctx.quit_requested || flow == ControlFlow::Quit;
            (
                std::mem::take(&mut ctx.queued_dispatches),
                std::mem::take(&mut ctx.stylesheet_intents),
            )
        };
        self.apply_stylesheet_intents(intents);
        self.on_tick = Some(cb);
        // Fire queued dispatches now — after the tick returns but
        // before the next event poll, matching the HTML microtask
        // queue. Each dispatch may itself mutate the DOM, triggering
        // DirtyTracker updates.
        super::context::run_queued_dispatches(&mut self.dom, queued);
        self.prelude.touched |= self.change_evidence() != before;
    }

    /// Pull flags from the shared state into the local ones. Called
    /// at the top of every loop iteration.
    pub(crate) fn drain_handle_signals(&mut self) {
        if self.shared.redraw_requested.swap(false, Ordering::Relaxed) {
            self.redraw.note(Redraw::Cascade);
        }
        if self.shared.quit_requested.load(Ordering::Relaxed) {
            self.should_quit = true;
        }
    }

    /// Run any injected closures queued by an `AppHandle::inject`.
    pub(crate) fn drain_handle_injections(&mut self) {
        let injections = self.shared.drain_injections();
        if injections.is_empty() {
            return;
        }
        // As for `on_tick`: touched on evidence of a change only.
        let before = self.change_evidence();
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        let mut queued = Vec::new();
        for f in injections {
            let mut ctx = AppContext::new(&mut self.dom, &mut self.stylesheet_ids);
            f(&mut ctx);
            self.redraw.note_if(ctx.redraw_requested, Redraw::Cascade);
            self.prelude.touched |= ctx.redraw_requested;
            self.should_quit |= ctx.quit_requested;
            queued.extend(std::mem::take(&mut ctx.queued_dispatches));
            let intents = std::mem::take(&mut ctx.stylesheet_intents);
            self.apply_stylesheet_intents(intents);
        }
        super::context::run_queued_dispatches(&mut self.dom, queued);
        self.prelude.touched |= self.change_evidence() != before;
    }
}
