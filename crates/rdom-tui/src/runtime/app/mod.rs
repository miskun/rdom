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
//! this file keeps the struct, construction, the builder options, DOM /
//! terminal access and `handle_event`:
//!
//! - `event_loop` — `run`, the poll timeout, the scheduler pump,
//!   `advance`, `tick`, the `AppHandle` drains.
//! - `prelude` — `FramePrelude`, the pre-cascade stages every frame runs
//!   in order (selectedness, `<style>` elements, scroll-focus marker,
//!   validity marks, caret blink, smooth scrolls, the painted check).
//! - `stylesheets` — the author-stylesheet stack + `StylesheetId`.
//! - `keyboard_defaults` — the key pipeline and the runtime-owned
//!   default actions (clipboard, undo / redo, editable keys, focus
//!   navigation).
//! - `autoscroll` — the DRAG-AUTOSCROLL session.
//! - `frame` — `draw_if_dirty`, the off-frame cascade + layout, and the
//!   transition-event drain.
//! - `redraw` — `Redraw`, what the next frame must redo.

pub mod context;
pub mod handle;
pub mod panic_hook;

mod autoscroll;
mod event_loop;
mod frame;
mod keyboard_defaults;
mod prelude;
mod redraw;
mod stylesheets;

#[cfg(test)]
mod frame_work_tests;
#[cfg(test)]
mod idle_tests;
#[cfg(test)]
mod interaction_chain_tests;
#[cfg(test)]
mod off_event_paint_tests;
#[cfg(test)]
mod route_redraw_tests;
#[cfg(test)]
mod scroll_repaint_tests;
#[cfg(test)]
mod setter_mutation_tests;
#[cfg(test)]
mod sibling_mark_tests;
#[cfg(test)]
mod tests;

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::Event as CtEvent;
use redraw::Redraw;

use std::rc::Rc;

use crate::render::backend::Backend;
use crate::render::backend_crossterm::{CrosstermBackend, enter_tui_mode};
use crate::render::{Terminal, TerminalGuard};
use crate::runtime::router::{RouteOutcome, Router};
use crate::runtime::selection::clipboard::{Clipboard, SystemClipboard};
use crate::runtime::url_opener::{SystemUrlOpener, UrlOpener};
use crate::style::{DirtyTracker, Stylesheet};
use crate::{TuiDispatchExt, TuiDom, TuiEvent};

pub use context::{AppContext, ControlFlow};
pub use handle::AppHandle;
pub use stylesheets::StylesheetId;

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
    /// The pre-cascade stages each frame runs, in order, and their state
    /// (`prelude::FramePrelude`: selectedness, `<style>` elements,
    /// scroll-focus marker, validity marks, caret blink, smooth scrolls,
    /// the painted-offset check, and the `touched` flag gating the
    /// whole-tree ones).
    prelude: prelude::FramePrelude,
    /// The one [`StylesheetId`] allocator: the App's own
    /// `set_stylesheet` / `push_stylesheet` and every [`AppContext`]
    /// (which borrows it) draw from it, so an id a handler gets back is
    /// the id its sheet is registered under.
    stylesheet_ids: stylesheets::StylesheetIdAllocator,
    pub(super) terminal: Terminal<B>,
    pub(super) tracker: DirtyTracker,
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

    /// DRAG-AUTOSCROLL session state (`autoscroll::AutoscrollSession`).
    autoscroll: autoscroll::AutoscrollSession,
    /// What the next frame must redo, accumulated over a tick
    /// (`redraw::Redraw`): `draw_if_dirty` draws when it is not
    /// `Clean` or the DirtyTracker has roots.
    pub(super) redraw: Redraw,
    /// What the frames drawn since the last `take_frame_stats` ran.
    #[cfg(test)]
    pub(super) frame_stats: redraw::FrameStats,
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
        app.prelude
            .caret_blink
            .set_period(Some(crate::runtime::caret_blink::DEFAULT_CARET_BLINK));
        Ok(app)
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
        // The `style="…"` attributes present at mount — parsed markup,
        // or set before the observer above existed — have not been
        // parsed into `TuiExt::inline_style` yet; later writes are the
        // observer's (`P7G-INLINE-STYLE-SEED-1`). Idempotent, so a
        // consumer's own `seed_inline_styles` call before `App::new`
        // is harmless; that call is also where its warnings surface.
        crate::cssom::seed_inline_styles(&mut dom);
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
        crate::runtime::builtins::validation::install(&mut dom);
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
        let prelude = prelude::FramePrelude::install(&mut dom);
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
        let app = Self {
            dom,
            stylesheets: vec![(stylesheet_ids.allocate(), stylesheet)],
            prelude,
            stylesheet_ids,
            terminal,
            tracker,
            router: Router::new(),
            tick_rate: Duration::from_millis(50),
            animation_frame_ms: 16,
            on_tick: None,
            scheduler: std::rc::Rc::new(std::cell::RefCell::new(
                crate::runtime::timers::Scheduler::new(std::time::Instant::now()),
            )),
            animations: crate::runtime::animation::AnimationRegistry::new(),
            autoscroll: autoscroll::AutoscrollSession::default(),
            redraw: Redraw::Cascade,
            #[cfg(test)]
            frame_stats: Default::default(),
            should_quit: false,
            guard: None,
            shared: AppShared::new(),
            clipboard: Box::new(SystemClipboard::new()),
            url_opener,
        };
        app.prelude
            .sync_sibling_combinators(&app.tracker, &app.stylesheets);
        Ok(app)
    }

    /// Override the animation-frame budget when timers / rAF /
    /// transitions are active. Default 16ms (~60fps). Pass `30`
    /// for ~30fps on slower terminals or to reduce CPU.
    pub fn set_animation_frame_rate(&mut self, fps: u16) {
        let fps = fps.clamp(1, 120);
        self.animation_frame_ms = (1000 / fps as u32).max(1);
    }

    /// Set the caret blink half-period: the caret of a focused editable
    /// is painted for `period`, then hidden for `period`, restarting
    /// visible on every caret move, edit, key or click. `None` (or a zero
    /// duration) keeps a steady caret. Browsers follow the OS setting and
    /// no CSS property controls it, so this is an App option.
    /// [`App::new`] blinks at
    /// [`DEFAULT_CARET_BLINK`](crate::runtime::caret_blink::DEFAULT_CARET_BLINK)
    /// (530 ms); [`App::with_backend`] starts steady, so paint snapshots
    /// do not depend on the clock. Blinking wakes the event loop only
    /// while an editable with a caret is focused and the terminal has
    /// focus.
    pub fn with_caret_blink(mut self, period: Option<Duration>) -> Self {
        self.prelude.caret_blink.set_period(period);
        self
    }

    /// When the caret blink next flips, on the scheduler clock.
    #[cfg(test)]
    pub(crate) fn caret_blink_deadline(&self) -> Option<std::time::Instant> {
        self.prelude.caret_blink.next_deadline()
    }

    /// When the next smooth-scroll step is due, on the scheduler clock.
    #[cfg(test)]
    pub(crate) fn smooth_scroll_deadline(&self) -> Option<std::time::Instant> {
        self.prelude.smooth_scroll_next
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

    /// Mutable DOM access for pre-`run` setup. Listener registration,
    /// tree construction, etc. goes here. For event-loop-era
    /// mutations, use the `AppContext` passed to `on_tick` or receive
    /// `EventCtx` inside an `add_event_listener` callback.
    pub fn dom_mut(&mut self) -> &mut TuiDom {
        // The caller may change anything, scroll offsets included.
        self.prelude.touched = true;
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

    /// Fold a mouse route's outcome into the next frame's work
    /// (`P7G-ROUTE-REDRAW-1`): the router's own work — a hover change,
    /// a wheel scroll, a scrollbar press, a selection drag — lays out
    /// and repaints, cascading only the dirty tracker's roots (the
    /// elements whose `:hover` / `:focus` flipped); a listener's
    /// `request_redraw` cascades the whole tree.
    fn note_route(&mut self, outcome: RouteOutcome) {
        self.redraw
            .note_if(outcome.redraw_requested, Redraw::Layout);
        self.redraw
            .note_if(outcome.cascade_requested, Redraw::Cascade);
        self.should_quit |= outcome.quit_requested;
    }

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
        self.prelude.selectedness.flush(&mut self.dom);
        self.prelude.touched = true;
        match &event {
            CtEvent::Key(key) => {
                // A key may edit without moving the caret (Delete): the
                // caret blink restarts visible on every key, as in browsers.
                self.prelude.caret_blink.note_input();
                self.handle_key_event(*key)
            }
            CtEvent::Mouse(m) => {
                if matches!(m.kind, crossterm::event::MouseEventKind::Down(_)) {
                    self.prelude.caret_blink.note_input();
                }
                let (col, row) = (m.column, m.row);
                let focused_before = self.dom.focused();
                let outcome = self.router.route(&mut self.dom, event);
                crate::runtime::focus::visible::note_pointer_focus(&mut self.dom, focused_before);
                self.note_route(outcome);
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
                self.dom
                    .dispatch_tui_event(root, &mut tui)
                    .expect("the document root is never dropped, and `tui` is fresh");
                self.redraw.note(Redraw::Cascade);

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
                self.prelude.caret_blink.set_terminal_focused(true);
                crate::rdom_trace!(
                    "App::handle_event: FocusGained (capture={:?}, hovered={:?})",
                    self.dom.pointer_capture(),
                    self.dom.hovered()
                );
            }
            CtEvent::FocusLost => {
                self.prelude.caret_blink.set_terminal_focused(false);
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

    /// Take a snapshot of known-dirty subtree roots (for test
    /// introspection).
    #[cfg(test)]
    pub(crate) fn dirty_roots_snapshot(&self) -> Vec<rdom_core::NodeId> {
        self.tracker.roots_snapshot()
    }

    #[cfg(test)]
    pub(crate) fn needs_redraw(&self) -> bool {
        self.redraw != Redraw::Clean || self.tracker.has_pending()
    }

    /// The pipeline stages run since the last call, and reset them.
    #[cfg(test)]
    pub(crate) fn take_frame_stats(&mut self) -> redraw::FrameStats {
        std::mem::take(&mut self.frame_stats)
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
