//! `App` — the runtime's public face. Owns the DOM, stylesheet,
//! terminal, dirty tracker, and router; runs the event loop.
//!
//! ## Public API surface
//!
//! - [`App::new`] — real-world constructor, wraps
//!   `CrosstermBackend<Stdout>`.
//! - [`App::with_backend`] — generic constructor for tests /
//!   custom backends.
//! - [`App::on_tick`] — register a tick callback.
//! - [`App::tick_rate`] — configure event-poll timeout.
//! - [`App::run`] — block until exit, owning the full event loop.
//!   Only available on `App<CrosstermBackend<Stdout>>`.
//! - [`App::handle_event`], [`App::draw_if_dirty`] — granular
//!   hooks for tests and custom loops. Pub-crate for now; may
//!   stabilize later.
//!
//! ## Sub-modules
//!
//! - [`context`] — `AppContext` + `ControlFlow`. The handle a tick
//!   callback / in-loop handler uses to request redraws, quit, or
//!   mutate the DOM.
//! - [`handle`] — `AppHandle`, the cross-thread handle.
//! - [`panic_hook`] — the terminal-restoring panic hook.
//!
//! `App`'s own behavior is split by concern into private siblings;
//! this file keeps the struct, construction, the event loop and its
//! entry points (`run`, `handle_event`, `tick`, `advance`, the
//! scheduler pump, the `AppHandle` drains):
//!
//! - `stylesheets` — the author-stylesheet stack + `StylesheetId`.
//! - `keyboard_defaults` — the key pipeline and the runtime-owned
//!   default actions (clipboard, undo / redo, editable keys, focus
//!   navigation).
//! - `autoscroll` — the DRAG-AUTOSCROLL session.
//! - `frame` — `draw_if_dirty`, the off-frame cascade + layout, the
//!   scroll-focus marker, and the transition-event drain.

pub mod context;
pub mod handle;
pub mod panic_hook;

mod autoscroll;
mod frame;
mod keyboard_defaults;
mod stylesheets;

#[cfg(test)]
mod tests;

use std::io::{self, Stdout};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CtEvent};

use std::rc::Rc;

use crate::render::backend::Backend;
use crate::render::backend_crossterm::{CrosstermBackend, enter_tui_mode, leave_tui_mode};
use crate::render::{Terminal, TerminalGuard};
use crate::runtime::router::Router;
use crate::runtime::selection::clipboard::{Clipboard, SystemClipboard};
use crate::runtime::url_opener::{SystemUrlOpener, UrlOpener};
use crate::style::{DirtyTracker, Stylesheet};
use crate::{TuiDispatchExt, TuiDom, TuiEvent};

pub use context::{AppContext, ControlFlow};
pub use handle::AppHandle;
pub use stylesheets::StylesheetId;

use autoscroll::AUTOSCROLL_PERIOD;
use handle::AppShared;

type TickCallback = Box<dyn FnMut(&mut AppContext<'_>) -> ControlFlow + 'static>;

/// The runtime. Owns everything needed to paint + interact.
///
/// Generic over `Backend` so tests can construct an `App` with
/// `TestBackend` and drive the event loop synchronously without a
/// real terminal.
pub struct App<B: Backend = CrosstermBackend<Stdout>> {
    pub(super) dom: TuiDom,
    /// Author stylesheets registered with this App, in push order,
    /// paired with their opaque ids. The cascade reads this slice;
    /// later sheets win same-specificity contests, matching
    /// `Document.styleSheets` ordering on the web.
    ///
    /// Mutated via [`App::push_stylesheet`] (append + returns id),
    /// [`App::remove_stylesheet`] (delete by id), or
    /// [`App::set_stylesheet`] (clear + push). Public accessor
    /// [`App::style_sheets`] returns the sheets-only view.
    pub(super) stylesheets: Vec<(StylesheetId, Stylesheet)>,
    /// The one [`StylesheetId`] allocator: the App's own
    /// `set_stylesheet` / `push_stylesheet` and every [`AppContext`]
    /// (which borrows it) draw from it, so an id a handler gets back is
    /// the id its sheet is registered under.
    stylesheet_ids: stylesheets::StylesheetIdAllocator,
    pub(super) terminal: Terminal<B>,
    pub(super) tracker: DirtyTracker,
    /// Runs the `<select>` selectedness setting algorithm on selects
    /// whose options were inserted / removed; flushed before each event
    /// and each frame.
    pub(super) selectedness: crate::runtime::builtins::select::Selectedness,
    pub(super) router: Router,

    tick_rate: Duration,
    /// Frame budget (ms) when an animation or rAF callback is
    /// pending. Default 16ms = ~60fps. Falls back to `tick_rate`
    /// when nothing is animating. Configurable via
    /// [`App::set_animation_frame_rate`].
    animation_frame_ms: u32,
    on_tick: Option<TickCallback>,
    /// Timer / rAF / microtask scheduler.
    pub(crate) scheduler: crate::runtime::timers::SharedScheduler,
    /// In-flight CSS transitions.
    pub(crate) animations: crate::runtime::animation::AnimationRegistry,

    /// DRAG-AUTOSCROLL session state. A drag **owns one scroll container** for
    /// its lifetime: `autoscroll_container` is resolved **once** the first time
    /// the pointer reaches an edge zone, then stays sticky until the capture
    /// releases — so the captured node scrolling out of view, or the pointer
    /// overshooting past the container onto a sibling, neither re-targets nor
    /// disarms it. `autoscroll_pointer` is the latest pointer of the armed drag
    /// (`None` until the session arms); `autoscroll_next` is the next tick's
    /// deadline on the scheduler clock.
    pub(super) autoscroll_pointer: Option<(u16, u16)>,
    pub(super) autoscroll_next: Option<Instant>,
    pub(super) autoscroll_container: Option<crate::NodeId>,
    /// The element currently carrying `data-rdom-scroll-focus` (see
    /// [`Self::mark_scroll_focus`]).
    pub(super) scroll_focus_marked: Option<crate::NodeId>,

    /// Flags accumulated over a tick: if true, `draw_if_dirty`
    /// triggers a paint regardless of DirtyTracker state.
    pub(super) needs_redraw: bool,
    /// True once a handler / tick / top-level key combo (Ctrl-C,
    /// etc.) asked the app to exit. `run` sees this at the top of
    /// the next iteration and breaks out.
    pub(super) should_quit: bool,

    /// Holds a `TerminalGuard` in the real-crossterm case so
    /// terminal mode is restored even if `run` panics. `None` for
    /// the generic / test App (no TUI mode was entered).
    guard: Option<TerminalGuard>,

    /// Shared flags + inject queue for cross-thread `AppHandle`s.
    /// Always populated; a handle is cheap to construct even if
    /// no one ever clones it.
    shared: Arc<AppShared>,

    /// Clipboard backend used for copy / cut / paste default
    /// actions. Defaults to `SystemClipboard` (arboard); tests
    /// swap in `MemoryClipboard` via [`App::with_clipboard`] to
    /// avoid touching the real pasteboard.
    pub(super) clipboard: Box<dyn Clipboard>,

    /// URL opener used by the `<a href>` click default action to
    /// hand external URLs (http/https/mailto/...) to the OS.
    /// Defaults to `SystemUrlOpener` (shells out via `open`
    /// crate); tests swap in `MemoryUrlOpener` via
    /// [`App::with_url_opener`] so clicks don't launch browsers.
    ///
    /// Double-`Rc` shape: the outer `Rc<RefCell<...>>` is shared
    /// between `App` and the click listener installed on the
    /// document root. The inner `Rc<dyn UrlOpener>` is the
    /// swappable backend. Mutating through the `RefCell` makes
    /// swaps visible to the listener without re-install.
    url_opener: crate::runtime::builtins::a_href::SharedOpener,
}

impl App<CrosstermBackend<Stdout>> {
    /// Construct a real-world App wired to stdout. Enters TUI mode
    /// (alt-screen, raw input, mouse capture) and installs the
    /// `DirtyTracker` on the provided DOM.
    ///
    /// Drop restores the terminal — even on panic.
    pub fn new(dom: TuiDom, stylesheet: Stylesheet) -> io::Result<Self> {
        // Install the terminal-restoring panic hook before we enter
        // TUI mode. If anything between here and `App::run` panics,
        // the hook will print the panic on the main screen instead
        // of the alt-screen buffer (which would be invisible).
        panic_hook::install();

        let mut stdout = io::stdout();
        enter_tui_mode(&mut stdout)?;
        let guard = TerminalGuard::new();
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let mut app = Self::build(dom, stylesheet, terminal)?;
        app.guard = Some(guard);
        Ok(app)
    }

    /// Block until exit. Runs the event loop: poll crossterm, route
    /// events, tick, cascade + layout + paint when dirty. Exits on
    /// `ControlFlow::Quit`, Ctrl-C, or `AppContext::quit()`.
    ///
    /// On exit, the terminal is restored via [`leave_tui_mode`].
    /// This is also guaranteed on panic — the `Drop` impl on `App`
    /// runs it if `run` unwinds.
    pub fn run(mut self) -> io::Result<()> {
        // Initial paint — user should see something even before any
        // event fires.
        self.needs_redraw = true;

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
    /// Construct an App from a pre-built `Terminal<B>`. Used by
    /// `App::new` (crossterm backend) and by tests (`TestBackend`).
    pub fn with_backend(
        dom: TuiDom,
        stylesheet: Stylesheet,
        terminal: Terminal<B>,
    ) -> io::Result<Self> {
        Self::build(dom, stylesheet, terminal)
    }

    fn build(mut dom: TuiDom, stylesheet: Stylesheet, terminal: Terminal<B>) -> io::Result<Self> {
        let tracker = DirtyTracker::install(&mut dom);
        // Install default CSSOM observers — currently just the
        // inline-style observer that refreshes `TuiExt::inline_style`
        // when the `style="…"` attribute is mutated post-build.
        // CSSOM writes from `StyleDeclarationMut` set `CSSOM_REENTRY`
        // and the observer self-suppresses — see `cssom::reentry`.
        // Apps constructing a `TuiDom` directly should call
        // `cssom::install_default_observers` themselves.
        crate::cssom::install_default_observers(&mut dom);
        let default_opener: Rc<dyn UrlOpener> = Rc::new(SystemUrlOpener);
        let url_opener: crate::runtime::builtins::a_href::SharedOpener =
            Rc::new(std::cell::RefCell::new(default_opener));
        // Install built-in element default actions. Each module
        // registers its own root-level listeners. Order of
        // install doesn't matter — listeners at the root fire in
        // registration order during bubble, and none of them
        // depend on each other's side effects.
        crate::runtime::builtins::a_href::install(&mut dom, url_opener.clone());
        crate::runtime::builtins::button::install(&mut dom);
        crate::runtime::builtins::label::install(&mut dom);
        crate::runtime::builtins::details::install(&mut dom);
        crate::runtime::builtins::toggle::install(&mut dom);
        crate::runtime::builtins::number::install(&mut dom);
        crate::runtime::builtins::form::install(&mut dom);
        crate::runtime::builtins::dialog::install(&mut dom);
        crate::runtime::builtins::select::install(&mut dom);
        crate::runtime::builtins::range::install(&mut dom);
        crate::runtime::builtins::tree::install(&mut dom);
        // Make sure every `<input>` has a text-node child reflecting
        // its `value` attribute. Parsed templates (`<input value="x">`
        // with no children) and direct-API users alike land in the
        // shape that the editing pipeline + paint pass expect.
        crate::runtime::builtins::input::seed_all(&mut dom);
        // Attach the canvas paint callback to every `<input
        // type="range">` declaratively present in the tree.
        // Dynamically-added ranges call `range::attach` themselves.
        crate::runtime::builtins::range::attach_all(&mut dom);
        // HTML §4.10.7: a single-select dropdown shows one option — run
        // the selectedness setting algorithm on every `<select>` now,
        // and on each select whose options change from here on.
        crate::runtime::builtins::select::seed_all(&mut dom);
        let selectedness = crate::runtime::builtins::select::Selectedness::install(&mut dom);
        // Sync column widths across every `<table>` so cells in
        // different rows align. v1 uses content-based measurement;
        // apps that mutate tables at runtime can call the helper
        // themselves to re-sync (see `runtime::builtins::table`).
        crate::runtime::builtins::table::size_all_tables(&mut dom);
        // Install the implicit-detach event observer: dispatches
        // `blur` / `focusout` / `mouseout` / `mouseleave` when the
        // focused or hovered element is removed from the tree,
        // matching browser behavior. Closes `EVT-DETACH-1`.
        crate::runtime::implicit_events::install(&mut dom);
        // Honor `[autofocus]` on initial mount. Walks the tree in
        // document order and focuses the first eligible `[autofocus]`
        // element. No-op when something is already focused or when no
        // matching element exists.
        crate::runtime::autofocus::focus_first_autofocus(&mut dom);
        let mut stylesheet_ids = stylesheets::StylesheetIdAllocator::default();
        Ok(Self {
            dom,
            stylesheets: vec![(stylesheet_ids.allocate(), stylesheet)],
            stylesheet_ids,
            terminal,
            tracker,
            selectedness,
            router: Router::new(),
            tick_rate: Duration::from_millis(50),
            animation_frame_ms: 16,
            on_tick: None,
            scheduler: std::rc::Rc::new(std::cell::RefCell::new(
                crate::runtime::timers::Scheduler::new(std::time::Instant::now()),
            )),
            animations: crate::runtime::animation::AnimationRegistry::new(),
            autoscroll_pointer: None,
            autoscroll_next: None,
            autoscroll_container: None,
            scroll_focus_marked: None,
            needs_redraw: true,
            should_quit: false,
            guard: None,
            shared: AppShared::new(),
            clipboard: Box::new(SystemClipboard::new()),
            url_opener,
        })
    }

    /// Override the animation-frame budget when timers / rAF /
    /// transitions are active. Default 16ms (~60fps). Pass `30`
    /// for ~30fps on slower terminals or to reduce CPU.
    pub fn set_animation_frame_rate(&mut self, fps: u16) {
        let fps = fps.clamp(1, 120);
        self.animation_frame_ms = (1000 / fps as u32).max(1);
    }

    /// Replace the clipboard backend. Useful for tests
    /// (`MemoryClipboard`) and for apps that want custom format
    /// or transport on `copy`.
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// Replace the URL opener backend used by the `<a href>`
    /// click default action. Useful for tests (`MemoryUrlOpener`
    /// to avoid launching browsers) and for apps that want custom
    /// external-URL handling (e.g. an in-app preview for
    /// `https://` instead of shelling out).
    ///
    /// Swap propagates to the already-installed root click
    /// listener — no need to call this before `build`.
    pub fn with_url_opener(self, opener: Rc<dyn UrlOpener>) -> Self {
        *self.url_opener.borrow_mut() = opener;
        self
    }

    /// Produce a clone-able, `Send + Sync` handle to this App.
    /// Use from background threads / async tasks to request a
    /// redraw, ask the loop to exit, or inject a closure that
    /// runs on the loop thread with exclusive DOM access.
    pub fn handle(&self) -> AppHandle {
        AppHandle::from_shared(Arc::clone(&self.shared))
    }

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
        let base = to_deadline.min(frame_floor).min(self.tick_rate);
        // While a drag-autoscroll is armed, wake at least once per period so the
        // tick fires even with the pointer held still (no new input events).
        if self.autoscroll_pointer.is_some() {
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
        t::drain_microtasks(&sched, &mut self.dom);
        t::pump_timeouts(&sched, &mut self.dom);
        let due = sched.borrow().drain_expired_interval_ids();
        t::pump_intervals(&sched, &mut self.dom, &due);
        t::pump_raf(&sched, &mut self.dom);
    }

    /// Advance the virtual scheduler clock by `ms` and service everything that
    /// comes due, then redraw if dirty. For **headless / simulation / test**
    /// drivers that don't run the live [`run`](Self::run) loop (which syncs to
    /// wall time). Fires timeouts, intervals, rAF, and microtasks whose deadline
    /// falls within the elapsed window, exactly as the loop would — making
    /// timer-driven runtime behavior (animations, autoscroll) deterministically
    /// testable. Advance one period at a time to step a repeating timer
    /// tick-by-tick.
    pub fn advance(&mut self, ms: u64) -> io::Result<()> {
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        let target = self.scheduler.borrow().now() + std::time::Duration::from_millis(ms);
        self.scheduler.borrow_mut().set_now(target);
        self.pump_due();
        self.service_autoscroll();
        self.draw_if_dirty()
    }

    /// Mutable DOM access for pre-`run` setup. Listener registration,
    /// tree construction, etc. goes here. For event-loop-era
    /// mutations, use the `AppContext` passed to `on_tick` or receive
    /// `EventCtx` inside an `add_event_listener` callback.
    pub fn dom_mut(&mut self) -> &mut TuiDom {
        &mut self.dom
    }

    /// Read-only DOM access.
    pub fn dom(&self) -> &TuiDom {
        &self.dom
    }

    /// Access the underlying terminal (for integration tests that
    /// want to inspect the backend).
    pub fn terminal(&self) -> &Terminal<B> {
        &self.terminal
    }

    /// Mutable terminal access — use sparingly, app-internal
    /// invariants can desync.
    pub fn terminal_mut(&mut self) -> &mut Terminal<B> {
        &mut self.terminal
    }

    /// Configure the crossterm event-poll timeout / tick cadence.
    /// Default 50 ms. Set to `Duration::ZERO` to disable tick
    /// firing entirely (loop blocks until a real event arrives).
    pub fn tick_rate(mut self, d: Duration) -> Self {
        self.tick_rate = d;
        self
    }

    /// Register a callback that fires each iteration where no
    /// crossterm event arrived within `tick_rate`. Primarily for
    /// draining app-level channels (watch streams, timers,
    /// inter-thread signals) into DOM mutations.
    ///
    /// Return `ControlFlow::Quit` to exit the loop.
    pub fn on_tick<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut AppContext<'_>) -> ControlFlow + 'static,
    {
        self.on_tick = Some(Box::new(f));
        self
    }

    // ─── Event + frame plumbing (test + internal entry points) ──────

    /// Process one crossterm event. Routes mouse events through
    /// `Router`; dispatches key events to the focused element;
    /// marks redraw on resize.
    ///
    /// Public so apps that want a custom event loop (e.g.
    /// integrating with an external runtime) can drive the App
    /// via this + [`Self::draw_if_dirty`] instead of [`Self::run`].
    pub fn handle_event(&mut self, event: CtEvent) {
        // Install the scheduler thread-local so listener callbacks
        // can call ctx.set_timeout(...) etc. through the `TuiTimers`
        // extension trait. Dropped at the end of this method,
        // restoring the previous value (typically null).
        let _scheduler_guard = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        // RAW ENTRY trace — every event from `event::read()` lands
        // here. If trace shows clicks but no `Moved`, we know
        // motion events are NOT crossing this boundary — i.e.,
        // crossterm itself isn't producing them.
        crate::rdom_trace!("App::handle_event RAW: {event:?}");
        // Option lists the app changed since the last event: settle the
        // selects before any default action reads them.
        self.selectedness.flush(&mut self.dom);
        match &event {
            CtEvent::Key(key) => self.handle_key_event(*key),
            CtEvent::Mouse(m) => {
                let (col, row) = (m.column, m.row);
                let outcome = self.router.route(&mut self.dom, event);
                self.needs_redraw |= outcome.redraw_requested;
                self.should_quit |= outcome.quit_requested;
                self.needs_redraw |= self.tracker.take_paint_dirty();
                // DRAG-AUTOSCROLL: (re)arm from the pointer's current position.
                self.note_autoscroll(col, row);
            }
            CtEvent::Resize(_, _) => {
                // `Terminal::autoresize` (called from `draw`) handles
                // buffer resizing + backend.clear() on the next paint.
                // Dispatch a `resize` event on the document root
                // (the "Window" target per HTML §UIEvents) so apps
                // can react to viewport changes before the paint.
                // Coalesced naturally — crossterm delivers one
                // Resize signal per terminal change, not per cell.
                let root = self.dom.root();
                // `resize`: bubbles, NOT cancelable per HTML.
                let mut tui = TuiEvent::new("resize");
                tui.event.cancelable = false;
                let _ = self.dom.dispatch_tui_event(root, &mut tui);
                self.needs_redraw = true;

                // Re-arm mouse tracking after the resize signal.
                // Terminals (and tmux in some configurations) reset
                // DEC private-mode state across viewport changes;
                // EnableMouseCapture is idempotent on conformant
                // terminals and recovers motion-event delivery
                // where it was lost.
                let mut stdout = std::io::stdout();
                if let Err(e) = crate::render::backend_crossterm::enter_mouse_capture(&mut stdout) {
                    crate::rdom_trace!("Resize: failed to re-arm mouse capture: {e}");
                } else {
                    crate::rdom_trace!("Resize: re-armed mouse capture");
                }
            }
            CtEvent::FocusGained => {
                crate::rdom_trace!(
                    "App::handle_event: FocusGained (capture={:?}, hovered={:?})",
                    self.dom.pointer_capture(),
                    self.dom.hovered()
                );
            }
            CtEvent::FocusLost => {
                crate::rdom_trace!(
                    "App::handle_event: FocusLost (capture={:?}, hovered={:?})",
                    self.dom.pointer_capture(),
                    self.dom.hovered()
                );
            }
            _ => {
                // Paste, etc. — currently ignored.
            }
        }
    }

    /// Fire the registered `on_tick` callback, if any. No-op when
    /// unset.
    pub(crate) fn tick(&mut self) {
        let Some(mut cb) = self.on_tick.take() else {
            return;
        };
        // Install the scheduler thread-local so on_tick handlers
        // can use the `TuiTimers` extension surface too (apps
        // that schedule fade-outs from a tick callback, etc.).
        let _scheduler_guard = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        let (queued, intents) = {
            let mut ctx = AppContext::new(&mut self.dom, &mut self.stylesheet_ids);
            let flow = cb(&mut ctx);
            self.needs_redraw |= ctx.redraw_requested;
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
        for (target, mut event) in queued {
            let _ = self.dom.dispatch_event(target, &mut event);
        }
    }

    /// Pull flags from the shared state into the local ones. Called
    /// at the top of every loop iteration.
    pub(crate) fn drain_handle_signals(&mut self) {
        if self.shared.redraw_requested.swap(false, Ordering::Relaxed) {
            self.needs_redraw = true;
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
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        let mut queued = Vec::new();
        for f in injections {
            let mut ctx = AppContext::new(&mut self.dom, &mut self.stylesheet_ids);
            f(&mut ctx);
            self.needs_redraw |= ctx.redraw_requested;
            self.should_quit |= ctx.quit_requested;
            queued.extend(std::mem::take(&mut ctx.queued_dispatches));
            let intents = std::mem::take(&mut ctx.stylesheet_intents);
            self.apply_stylesheet_intents(intents);
        }
        for (target, mut event) in queued {
            let _ = self.dom.dispatch_event(target, &mut event);
        }
    }

    /// Take a snapshot of known-dirty subtree roots (for test
    /// introspection).
    #[cfg(test)]
    pub(crate) fn dirty_roots_snapshot(&self) -> Vec<rdom_core::NodeId> {
        self.tracker.roots_snapshot()
    }

    #[cfg(test)]
    pub(crate) fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    #[cfg(test)]
    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Mutable access to the animation registry — lets tests
    /// inject a `PendingEvent` and drive
    /// `dispatch_animation_events` without setting up a real-
    /// time-driven transition.
    #[cfg(test)]
    pub(crate) fn animations_mut_for_test(
        &mut self,
    ) -> &mut crate::runtime::animation::AnimationRegistry {
        &mut self.animations
    }

    /// Test-only alias for the private dispatch helper.
    #[cfg(test)]
    pub(crate) fn dispatch_animation_events_for_test(&mut self) {
        self.dispatch_animation_events();
    }
}

// ─── Teardown ───────────────────────────────────────────────────────
//
// The real-crossterm App holds a `TerminalGuard` in `self.guard`.
// Its `Drop` runs `leave_tui_mode` on stdout automatically — works
// even on panic. Normal exit path (`run` returning Ok) also calls
// `leave_tui_mode` explicitly, which is idempotent enough (the
// second emission of the ANSI sequences is harmless).
