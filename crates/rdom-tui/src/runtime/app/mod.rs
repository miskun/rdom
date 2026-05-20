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

pub mod context;
pub mod handle;
pub mod panic_hook;

#[cfg(test)]
mod tests;

use std::io::{self, Stdout};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crossterm::event::{self, Event as CtEvent, KeyCode, KeyModifiers};

use std::rc::Rc;

use crate::render::backend::Backend;
use crate::render::backend_crossterm::{CrosstermBackend, enter_tui_mode, leave_tui_mode};
use crate::render::{LayoutExt, PaintExt, Terminal, TerminalGuard};
use crate::runtime::router::Router;
use crate::runtime::selection::clipboard::{Clipboard, SystemClipboard};
use crate::runtime::url_opener::{SystemUrlOpener, UrlOpener};
use crate::style::{CascadeExt, DirtyTracker, Stylesheet};
use crate::{TuiDispatchExt, TuiDom, TuiEvent};

pub use context::{AppContext, ControlFlow};
pub use handle::AppHandle;
use handle::AppShared;

type TickCallback = Box<dyn FnMut(&mut AppContext<'_>) -> ControlFlow + 'static>;

/// The runtime. Owns everything needed to paint + interact.
///
/// Generic over `Backend` so tests can construct an `App` with
/// `TestBackend` and drive the event loop synchronously without a
/// real terminal.
pub struct App<B: Backend = CrosstermBackend<Stdout>> {
    dom: TuiDom,
    /// Author stylesheets registered with this App. v0.1.0 always
    /// holds exactly one sheet (the one passed to `App::new` /
    /// `App::with_backend`). The `Vec` shape future-proofs for
    /// multi-sheet registration without an API churn — public
    /// accessor `style_sheets() -> &[Stylesheet]` matches the
    /// browser `Document.styleSheets` shape.
    stylesheets: Vec<Stylesheet>,
    terminal: Terminal<B>,
    tracker: DirtyTracker,
    router: Router,

    tick_rate: Duration,
    /// Frame budget (ms) when an animation or rAF callback is
    /// pending. Default 16ms = ~60fps. Falls back to `tick_rate`
    /// when nothing is animating. Configurable via
    /// [`App::set_animation_frame_rate`].
    animation_frame_ms: u32,
    on_tick: Option<TickCallback>,
    /// Timer / rAF / microtask scheduler.
    pub(crate) scheduler: crate::runtime::timers::Scheduler,
    /// In-flight CSS transitions.
    pub(crate) animations: crate::runtime::animation::AnimationRegistry,

    /// Flags accumulated over a tick: if true, `draw_if_dirty`
    /// triggers a paint regardless of DirtyTracker state.
    needs_redraw: bool,
    /// True once a handler / tick / top-level key combo (Ctrl-C,
    /// etc.) asked the app to exit. `run` sees this at the top of
    /// the next iteration and breaks out.
    should_quit: bool,

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
    clipboard: Box<dyn Clipboard>,

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
                    if let Ok(ev) = event::read() {
                        self.handle_event(ev);
                    }
                } else {
                    self.tick();
                }
                // M3: advance the scheduler clock and pump expired
                // work. Microtasks drain after every chunk so that
                // chained queue_microtask calls during a callback
                // run before the next paint, matching HTML spec.
                self.pump_scheduler();
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
        // Make sure every `<input>` has a text-node child reflecting
        // its `value` attribute. Parsed templates (`<input value="x">`
        // with no children) and direct-API users alike land in the
        // shape that the editing pipeline + paint pass expect.
        crate::runtime::builtins::input::seed_all(&mut dom);
        // Attach the canvas paint callback to every `<input
        // type="range">` declaratively present in the tree.
        // Dynamically-added ranges call `range::attach` themselves.
        crate::runtime::builtins::range::attach_all(&mut dom);
        // Sync column widths across every `<table>` so cells in
        // different rows align. v1 uses content-based measurement;
        // apps that mutate tables at runtime can call the helper
        // themselves to re-sync (see `runtime::builtins::table`).
        crate::runtime::builtins::table::size_all_tables(&mut dom);
        // Honor `[autofocus]` on initial mount. Walks the tree in
        // document order and focuses the first eligible `[autofocus]`
        // element. No-op when something is already focused or when no
        // matching element exists.
        crate::runtime::autofocus::focus_first_autofocus(&mut dom);
        Ok(Self {
            dom,
            stylesheets: vec![stylesheet],
            terminal,
            tracker,
            router: Router::new(),
            tick_rate: Duration::from_millis(50),
            animation_frame_ms: 16,
            on_tick: None,
            scheduler: crate::runtime::timers::Scheduler::new(std::time::Instant::now()),
            animations: crate::runtime::animation::AnimationRegistry::new(),
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
            .next_deadline()
            .map(|d| d.saturating_duration_since(now))
            .unwrap_or(self.tick_rate);
        // If we have pending rAF callbacks (= an animation
        // frame is queued), tighten to the frame budget.
        let frame_floor = if self.scheduler.has_active_raf() {
            Duration::from_millis(self.animation_frame_ms as u64)
        } else {
            self.tick_rate
        };
        to_deadline.min(frame_floor).min(self.tick_rate)
    }

    /// Pump the scheduler: advance the clock, drain microtasks,
    /// fire expired timeouts/intervals, drain microtasks again
    /// (callbacks may queue them), then drain rAF before paint.
    fn pump_scheduler(&mut self) {
        use crate::runtime::timers as t;
        // Sync the clock to wall time — production only ever
        // moves forward; tests use the virtual-clock API
        // directly.
        self.scheduler.set_now(std::time::Instant::now());
        // Microtasks first, in case a previous handler queued
        // one and we haven't drained yet.
        t::drain_microtasks(&mut self.scheduler, &mut self.dom);
        t::pump_timeouts(&mut self.scheduler, &mut self.dom);
        let due = self.scheduler.drain_expired_interval_ids();
        t::pump_intervals(&mut self.scheduler, &mut self.dom, &due);
        t::drain_microtasks(&mut self.scheduler, &mut self.dom);
        t::pump_raf(&mut self.scheduler, &mut self.dom);
        t::drain_microtasks(&mut self.scheduler, &mut self.dom);
    }

    /// Replace the App's primary stylesheet (the one at index 0 of
    /// `style_sheets()`). The next paint runs a full re-cascade.
    /// v0.1.0 ships with only one sheet slot, so this is
    /// equivalent to "replace the stylesheet"; the index-0 framing
    /// future-proofs for multi-sheet registration.
    pub fn set_stylesheet(&mut self, sheet: Stylesheet) {
        self.stylesheets[0] = sheet;
        // Clearing tracker roots and forcing a full cascade: the
        // simplest way is to drop + reinstall the tracker.
        let _ = self.tracker.roots_snapshot();
        self.needs_redraw = true;
    }

    /// All stylesheets registered with this App. Spec-name parity
    /// with `Document.styleSheets`. v0.1.0 always returns a
    /// single-element slice — the sheet passed to construction
    /// (and possibly replaced via `set_stylesheet`).
    ///
    /// Note: stylesheets live on `App`, not on `TuiDom`. This is
    /// deliberate — a stylesheet is an App-lifecycle concept
    /// (registered at construction, replaceable mid-run), whereas
    /// the `Dom` is a pure tree structure. The other three
    /// `TuiDocAccessors` methods (`element_from_point`,
    /// `elements_from_point`, `caret_position_from_point`) operate
    /// on the tree and live on `Dom`; this one operates on the
    /// runtime and lives here.
    pub fn style_sheets(&self) -> &[Stylesheet] {
        &self.stylesheets
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
        let _scheduler_guard = crate::runtime::timers::SchedulerGuard::install(&mut self.scheduler);
        match &event {
            CtEvent::Key(key) => {
                // Clipboard shortcuts intercept the key before it's
                // dispatched as a normal `keydown`. A non-collapsed
                // selection turns Ctrl-C into "copy" instead of
                // "quit" — matches how terminals + browsers behave.
                if try_handle_clipboard_key(self, *key) {
                    return;
                }

                // Ctrl-C: universal exit. Handler listeners don't
                // even see this — the runtime owns it.
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.should_quit = true;
                    return;
                }

                // Dispatch `keydown` to the focused element (or root).
                let target = self.dom.focused().unwrap_or_else(|| self.dom.root());
                let mut tui = TuiEvent::keydown(*key);
                let _ = self.dom.dispatch_tui_event(target, &mut tui);

                // Default actions — only run if the handler didn't
                // call prevent_default. Selection-keyboard defaults
                // (Ctrl-A, Shift+arrows) run first so a selection
                // extend doesn't get overridden by focus nav.
                if !tui.event.default_prevented() {
                    if crate::runtime::selection::keyboard::try_handle_key(&mut self.dom, *key)
                        || try_handle_editable_key(&mut self.dom, *key)
                    {
                        self.needs_redraw = true;
                    } else {
                        match key.code {
                            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                crate::runtime::focus::tabindex::focus_prev(&mut self.dom);
                                self.needs_redraw = true;
                            }
                            KeyCode::Tab => {
                                crate::runtime::focus::tabindex::focus_next(&mut self.dom);
                                self.needs_redraw = true;
                            }
                            KeyCode::BackTab => {
                                // Some terminals report Shift+Tab as BackTab.
                                crate::runtime::focus::tabindex::focus_prev(&mut self.dom);
                                self.needs_redraw = true;
                            }
                            _ => {}
                        }
                    }
                }

                self.needs_redraw |= !self.tracker.roots_snapshot().is_empty();
                // Text-only mutations from event handlers don't dirty
                // the cascade (selectors don't match text content) but
                // they DO change painted output. Without this OR, a
                // handler that calls `set_node_value` is invisible
                // until the next event ticks the cascade.
                self.needs_redraw |= self.tracker.take_paint_dirty();
            }
            CtEvent::Mouse(_) => {
                let outcome = self.router.route(&mut self.dom, event);
                self.needs_redraw |= outcome.redraw_requested;
                self.should_quit |= outcome.quit_requested;
                self.needs_redraw |= self.tracker.take_paint_dirty();
            }
            CtEvent::Resize(_, _) => {
                // `Terminal::autoresize` (called from `draw`) handles
                // buffer resizing + backend.clear() on the next paint.
                // We just need to ensure a paint happens.
                self.needs_redraw = true;
            }
            _ => {
                // FocusGained/Lost, Paste, etc. — currently ignored.
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
        let _scheduler_guard = crate::runtime::timers::SchedulerGuard::install(&mut self.scheduler);
        let queued = {
            let mut ctx = AppContext::new(&mut self.dom);
            let flow = cb(&mut ctx);
            self.needs_redraw |= ctx.redraw_requested;
            self.should_quit |= ctx.quit_requested || flow == ControlFlow::Quit;
            std::mem::take(&mut ctx.queued_dispatches)
        };
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
        let mut queued = Vec::new();
        for f in injections {
            let mut ctx = AppContext::new(&mut self.dom);
            f(&mut ctx);
            self.needs_redraw |= ctx.redraw_requested;
            self.should_quit |= ctx.quit_requested;
            queued.extend(std::mem::take(&mut ctx.queued_dispatches));
        }
        for (target, mut event) in queued {
            let _ = self.dom.dispatch_event(target, &mut event);
        }
    }

    /// Cascade + layout + paint if anything is dirty. Pairs with
    /// [`Self::handle_event`] for apps running a custom event loop.
    pub fn draw_if_dirty(&mut self) -> io::Result<()> {
        let mut dirty_roots = self.tracker.roots_snapshot();
        // dirty_roots is a snapshot; we need to actually drain them
        // so subsequent frames don't re-cascade the same roots.
        if !dirty_roots.is_empty() {
            self.tracker.take_roots();
        }

        if !self.needs_redraw && dirty_roots.is_empty() {
            return Ok(());
        }

        // De-duplicate: multiple mutations under the same root end up
        // registered under the same NodeId.
        dirty_roots.sort_unstable();
        dirty_roots.dedup();

        // v0.1.0 cascade still operates on a single sheet; the
        // primary (index 0) is the only one registered today.
        let sheet = &self.stylesheets[0];
        let dom = &mut self.dom;
        let terminal = &mut self.terminal;
        let animations = &mut self.animations;
        let now = std::time::Instant::now();

        terminal.draw(|buf| {
            if dirty_roots.is_empty() {
                dom.cascade(sheet);
            } else {
                dom.cascade_subtrees(sheet, &dirty_roots);
            }
            // Detect cascade-driven property changes and register
            // transitions before layout / paint pick up the new
            // values.
            crate::runtime::animation::diff_and_register(dom, animations, now);
            // Then advance any in-flight animations (writes
            // interpolated values into TuiExt.presentation).
            animations.advance(dom, now);
            dom.layout_dom(buf.area);
            dom.paint_dom(buf, buf.area);
            Ok(())
        })?;

        // Drain transition events queued during this frame.
        self.dispatch_animation_events();

        // Force redraw next frame if any transitions are still
        // running — interpolation needs to keep stepping.
        self.needs_redraw = !self.animations.is_empty();
        Ok(())
    }

    /// Dispatch transition lifecycle events queued by the
    /// animation registry during this frame.
    ///
    /// Event detail is a typed `EventDetail::Transition` carrying
    /// the CSS property name and elapsed-seconds. Apps read via
    /// `event.detail.as_transition()`.
    fn dispatch_animation_events(&mut self) {
        use crate::runtime::animation::{PendingEvent, TransitionEventKind};
        let pending = self.animations.take_pending_events();
        for PendingEvent {
            node,
            kind,
            property,
            elapsed_seconds,
        } in pending
        {
            let event_name = match kind {
                TransitionEventKind::Start => "transitionstart",
                TransitionEventKind::End => "transitionend",
                TransitionEventKind::Cancel => "transitioncancel",
            };
            let mut ev = rdom_core::Event::new(event_name);
            ev.detail = rdom_core::EventDetail::Transition(Box::new(rdom_core::TransitionDetail {
                property_name: property.css_name().to_string(),
                elapsed: elapsed_seconds.into(),
                pseudo_element: None,
            }));
            let _ = self.dom.dispatch_event(node, &mut ev);
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

// ─── Clipboard key routing ──────────────────────────────────────────

/// Try to consume `key` as a clipboard shortcut (Ctrl-C / Ctrl-X /
/// Ctrl-V, or their Cmd-variants on macOS). Returns `true` when
/// the key was claimed — caller skips the rest of its keydown
/// pipeline (including the "Ctrl-C = quit" fallback).
///
/// Rules:
/// - **`copy` / `cut`**: only fire when the selection is
///   non-collapsed. Otherwise the key falls through (so Ctrl-C
///   without a selection still quits).
/// - **`paste`**: always fires when a focused element exists (or
///   the root as fallback). Clipboard may return `None` — the
///   event still fires so apps can trigger paste-empty UX.
fn try_handle_clipboard_key<B: Backend>(app: &mut App<B>, key: crossterm::event::KeyEvent) -> bool {
    let ctrl_or_super = key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SUPER);
    if !ctrl_or_super {
        return false;
    }

    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => do_copy(app),
        KeyCode::Char('x') | KeyCode::Char('X') => do_cut(app),
        KeyCode::Char('v') | KeyCode::Char('V') => do_paste(app),
        _ => false,
    }
}

fn do_copy<B: Backend>(app: &mut App<B>) -> bool {
    let Some((text, _range)) =
        crate::runtime::selection::clipboard::current_selection_text(&app.dom)
    else {
        return false;
    };
    let target = crate::runtime::selection::clipboard::copy_target(&app.dom)
        .unwrap_or_else(|| app.dom.root());
    let mut tui = TuiEvent::copy(text.clone());
    let _ = app.dom.dispatch_tui_event(target, &mut tui);
    if !tui.event.default_prevented() {
        app.clipboard.write_text(text);
    }
    true
}

fn do_cut<B: Backend>(app: &mut App<B>) -> bool {
    let Some((text, _range)) =
        crate::runtime::selection::clipboard::current_selection_text(&app.dom)
    else {
        return false;
    };
    let target = crate::runtime::selection::clipboard::copy_target(&app.dom)
        .unwrap_or_else(|| app.dom.root());
    let mut tui = TuiEvent::cut(text.clone());
    let _ = app.dom.dispatch_tui_event(target, &mut tui);
    if !tui.event.default_prevented() {
        app.clipboard.write_text(text);
        // If the cut target is an editable, delete the selected
        // range. Routes through `insert_at_selection(dom, "")` so
        // the delete fires `beforeinput`/`input` events and lands an
        // undo entry. Non-editable cut is copy-only (matches what
        // selection-in-prose "Cmd-X" usually does — copies but
        // doesn't delete, since prose isn't editable).
        if crate::node::nearest_editable_ancestor(&app.dom, target).is_some() {
            let _ = crate::runtime::editing::insert_at_selection(&mut app.dom, "");
        }
    }
    true
}

/// Undo / redo keydown default. Intercepts Ctrl-Z (or Cmd-Z) as
/// undo and Ctrl-Y / Ctrl-Shift-Z (or Cmd-Shift-Z) as redo. Gates
/// on focused editable; routes through `runtime::editing::undo_last`
/// / `redo_last`.
///
/// Runs before the movement / character handler so a bare 'z' still
/// types as text in an editable.
fn try_handle_history_key(dom: &mut TuiDom, key: crossterm::event::KeyEvent) -> bool {
    let ctrl_or_super = key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::SUPER);
    if !ctrl_or_super {
        return false;
    }
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let is_undo = matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z')) && !shift;
    let is_redo = matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y'))
        || (matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z')) && shift);

    if is_undo {
        matches!(
            crate::runtime::editing::undo_last(dom),
            crate::runtime::editing::UndoOutcome::Applied
        )
    } else if is_redo {
        matches!(
            crate::runtime::editing::redo_last(dom),
            crate::runtime::editing::UndoOutcome::Applied
        )
    } else {
        false
    }
}

fn do_paste<B: Backend>(app: &mut App<B>) -> bool {
    let text = app.clipboard.read_text().unwrap_or_default();
    let target = app.dom.focused().unwrap_or_else(|| app.dom.root());
    let mut tui = TuiEvent::paste(text.clone());
    let _ = app.dom.dispatch_tui_event(target, &mut tui);
    // If the paste target is an editable and the event wasn't
    // prevented, insert the clipboard text at the current selection
    // (or replace the range if one's active). Non-editable paste is
    // a no-op at the framework level; apps can still intercept the
    // event to do something custom (e.g. open a pasted URL).
    if !tui.event.default_prevented()
        && crate::node::nearest_editable_ancestor(&app.dom, target).is_some()
    {
        let _ = crate::runtime::editing::insert_at_selection(&mut app.dom, &text);
    }
    true
}

/// Editable-keydown default action. Returns `true` when the key
/// was consumed (caller skips remaining defaults like Tab nav).
///
/// Runs in order:
/// 1. **Movement / deletion** — bare arrows, Ctrl+arrows, Home/End,
///    Ctrl+Home/End, Backspace, Delete. Routed through
///    `runtime::editing::movement`. Shift+arrow is already handled
///    upstream by `selection::keyboard`.
/// 2. **Printable character insert** — falls through when movement
///    didn't match. Plain chars (no Ctrl/Super) insert at the
///    current selection via `insert_at_selection`. Control
///    combinations belong to clipboard / selection paths which
///    already ran upstream.
fn try_handle_editable_key(dom: &mut TuiDom, key: crossterm::event::KeyEvent) -> bool {
    // Focused element must have an editable ancestor.
    let Some(focused) = dom.focused() else {
        return false;
    };
    if crate::node::nearest_editable_ancestor(dom, focused).is_none() {
        return false;
    };

    // Undo / redo. Ctrl-Z / Cmd-Z undoes; Ctrl-Y or Cmd-Shift-Z /
    // Ctrl-Shift-Z redoes. Must run before movement/character so a
    // bare 'z' in an editable doesn't short-circuit into character
    // insertion instead.
    if try_handle_history_key(dom, key) {
        return true;
    }

    // Movement / deletion keys.
    if crate::runtime::editing::movement::try_handle_movement_key(dom, key) {
        return true;
    }

    // Enter handling. `<input>` is single-line: bare Enter is
    // consumed but inserts nothing (form-submit handled separately).
    // Other editables (`<textarea>`, contenteditable) insert a
    // literal `\n` — `white-space: pre` on the textarea turns it
    // into a visible hard break.
    if matches!(key.code, KeyCode::Enter)
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER | KeyModifiers::ALT)
    {
        let editable = crate::node::nearest_editable_ancestor(dom, focused);
        let is_input = editable
            .map(|id| dom.node(id).tag_name() == Some("input"))
            .unwrap_or(false);
        if is_input {
            return true;
        }
        let outcome = crate::runtime::editing::insert_at_selection(dom, "\n");
        return matches!(
            outcome,
            crate::runtime::editing::EditOutcome::Applied
                | crate::runtime::editing::EditOutcome::Prevented
        );
    }

    // Printable character insert. Skip modifier combos that belong
    // to other default actions upstream.
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::SUPER)
    {
        return false;
    }
    let ch = match key.code {
        KeyCode::Char(c) if !c.is_control() => c,
        _ => return false,
    };
    let outcome = crate::runtime::editing::insert_at_selection(dom, &ch.to_string());
    matches!(
        outcome,
        crate::runtime::editing::EditOutcome::Applied
            | crate::runtime::editing::EditOutcome::Prevented
    )
}
