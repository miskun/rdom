//! `App` — the runtime's public face. Owns the DOM, stylesheet,
//! terminal, dirty tracker, and router; runs the event loop.
//!
//! ## Public API surface
//!
//! - [`App::new`] — real-world constructor, wraps
//!   `CrosstermBackend<Stdout>`; [`App::with_backend`] — generic
//!   constructor for tests and custom backends.
//! - The `with_*` options below, chained after either constructor.
//! - [`App::run`] — block until exit, owning the full event loop. Only
//!   available on `App<CrosstermBackend<Stdout>>`.
//! - [`App::handle_event`], [`App::draw_if_dirty`], [`App::advance`] —
//!   the loop's steps, for tests and custom loops.
//! - DOM, stylesheet and terminal access: [`App::dom`] / [`App::dom_mut`],
//!   [`App::push_stylesheet`] and its siblings, [`App::terminal`].
//!
//! ## Configuration
//!
//! Options an app sets once, when it builds the `App`, are consuming
//! `with_*` builders; the `&mut self` `set_*` methods are for what an app
//! changes while it runs. The two constructors differ only where a test
//! must not depend on the environment or the clock:
//!
//! | Option | Builder | [`App::new`] | [`App::with_backend`] |
//! |---|---|---|---|
//! | Event-poll timeout / tick cadence | [`with_tick_rate`](App::with_tick_rate) | 50 ms | 50 ms |
//! | Tick handler | [`with_tick_handler`](App::with_tick_handler) | none | none |
//! | Frame rate while anything animates | [`with_animation_frame_rate`](App::with_animation_frame_rate) | 60 fps | 60 fps |
//! | Caret blink half-period | [`with_caret_blink`](App::with_caret_blink) | 530 ms | steady (`None`) |
//! | SGR extensions emitted | [`with_sgr_capabilities`](App::with_sgr_capabilities) | from the environment ([`SgrCapabilities::from_env`](crate::SgrCapabilities::from_env)) | the backend's (a `TestBackend`'s `BASIC`) |
//! | Pointer-shape protocol | [`with_pointer_shapes`](App::with_pointer_shapes) | from the environment ([`PointerShapes::from_env`](crate::PointerShapes::from_env)) | `None` |
//! | Preferred color scheme | [`with_color_scheme`](App::with_color_scheme); at run time [`set_color_scheme`](App::set_color_scheme) | asked of the terminal when `run` starts (OSC 11), dark without an answer | dark |
//! | Media preferences (`prefers-reduced-motion`, `prefers-contrast`, the pointer, …) | [`with_media_preferences`](App::with_media_preferences); at run time [`set_media_preferences`](App::set_media_preferences) | no preference, a mouse, 24-bit color ([`MediaPreferences::default`](crate::MediaPreferences::default)) | the same |
//! | Clipboard | [`with_clipboard`](App::with_clipboard) | the system clipboard | the system clipboard |
//! | `<a href>` URL opener | [`with_url_opener`](App::with_url_opener) | the system opener | the system opener |
//! | `@import` loader for `<style>` sheets | [`with_import_loader`](App::with_import_loader) | none (imports unresolved) | none |
//!
//! At run time: [`set_color_scheme`](App::set_color_scheme),
//! [`set_media_preferences`](App::set_media_preferences), the
//! stylesheet stack ([`push_stylesheet`](App::push_stylesheet),
//! [`set_stylesheet`](App::set_stylesheet),
//! [`remove_stylesheet`](App::remove_stylesheet)) and
//! [`register_property`](App::register_property).
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
//! this file keeps the struct, construction, DOM / terminal access and
//! `handle_event`:
//!
//! - `config` — the construction-time `with_*` options.
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
//! - `scheme` — the color-scheme options and the startup query.
//! - `media` — the media preferences, `matchMedia` and its reports.

pub mod context;
pub mod handle;
pub mod panic_hook;

mod animation_events;
mod autoscroll;
mod config;
mod event_loop;
mod frame;
mod input;
mod keyboard_defaults;
mod media;
mod pointer;
mod prelude;
mod redraw;
mod scheme;
mod stylesheets;

#[cfg(test)]
mod animation_event_tests;
#[cfg(test)]
mod calc_size_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod control_click_tests;
#[cfg(test)]
mod control_seeding_tests;
#[cfg(test)]
mod frame_cost_tests;
#[cfg(test)]
mod frame_work_tests;
#[cfg(test)]
mod geometry_transition_tests;
#[cfg(test)]
mod idle_tests;
#[cfg(test)]
mod interaction_chain_tests;
#[cfg(test)]
mod keyframes_tests;
#[cfg(test)]
mod layout_runs_tests;
#[cfg(test)]
mod media_tests;
#[cfg(test)]
mod off_event_paint_tests;
#[cfg(test)]
mod pseudo_chain_tests;
#[cfg(test)]
mod registered_transition_tests;
#[cfg(test)]
mod route_redraw_tests;
#[cfg(test)]
mod scope_invalidation_tests;
#[cfg(test)]
mod scroll_repaint_tests;
#[cfg(test)]
mod scroll_timeline_tests;
#[cfg(test)]
mod setter_mutation_tests;
#[cfg(test)]
mod sibling_mark_tests;
#[cfg(test)]
mod starting_style_tests;
#[cfg(test)]
mod teardown_tests;
#[cfg(test)]
mod tests;

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use redraw::Redraw;

use std::rc::Rc;

use crate::TuiDom;
use crate::render::backend::Backend;
use crate::render::backend_crossterm::{CrosstermBackend, enter_tui_mode};
use crate::render::{Terminal, TerminalGuard};
use crate::runtime::router::Router;
use crate::runtime::selection::clipboard::{Clipboard, SystemClipboard};
use crate::runtime::url_opener::{SystemUrlOpener, UrlOpener};
use crate::style::{DirtyTracker, Stylesheet};

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
    pub(super) stylesheets: Vec<(StylesheetId, Rc<Stylesheet>)>,
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
    /// pending. Default 16ms = ~60fps. Falls back to the tick rate
    /// when nothing is animating. Configurable via
    /// [`App::with_animation_frame_rate`].
    animation_frame_ms: u32,
    /// The terminal size the tree's styles were last checked against
    /// (CSS Values 4 §6.1.2, Media Queries 4 §4): a frame at another size
    /// cascades the whole tree again when a style read the viewport or a
    /// query flipped (`style::cascade::must_restyle`), else lays out.
    cascaded_viewport: Option<rdom_style::calc::Viewport>,
    /// The `matchMedia` lists (`runtime::media_query`), weakly held.
    media_watches: crate::runtime::media_query::MediaWatches,
    /// True once the app set the color scheme (`App::with_color_scheme`
    /// / `set_color_scheme`): the terminal is not asked at startup.
    color_scheme_explicit: bool,
    /// The background the terminal reported at startup (OSC 11), if any.
    detected_background: Option<crate::style::Color>,
    on_tick: Option<TickCallback>,
    /// Timer / rAF / microtask scheduler.
    pub(crate) scheduler: crate::runtime::timers::SharedScheduler,
    /// In-flight CSS transitions.
    pub(crate) animations: crate::runtime::animation::AnimationRegistry,
    /// The app's clock is driven by [`advance`](App::advance) (a headless
    /// or test driver), not by the wall clock: a frame does not sync it.
    virtual_clock: bool,
    /// The pointer and the shape last sent for it (`pointer`).
    pointer: pointer::Pointer,

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
        // The decorations past ECMA-48's subset go only to a terminal
        // known to read them (CSS Text Decoration's underline styles).
        let backend = CrosstermBackend::new(io::stdout())
            .with_sgr_capabilities(crate::render::SgrCapabilities::from_env());
        let terminal = Terminal::new(backend)?;
        let mut app = Self::build(dom, stylesheet, terminal)?
            .with_pointer_shapes(crate::runtime::pointer_shape::PointerShapes::from_env());
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
        crate::runtime::builtins::popover::install(&mut dom);
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
        // From here a focused element scrolls into view at the frame's
        // layout (`runtime::focus`).
        crate::runtime::focus::laid_out_by_app(&mut dom);
        let mut stylesheet_ids = stylesheets::StylesheetIdAllocator::default();
        let mut app = Self {
            dom,
            stylesheets: vec![(stylesheet_ids.allocate(), Rc::new(stylesheet))],
            prelude,
            stylesheet_ids,
            terminal,
            tracker,
            router: Router::new(),
            tick_rate: Duration::from_millis(50),
            animation_frame_ms: 16,
            cascaded_viewport: None,
            media_watches: Default::default(),
            color_scheme_explicit: false,
            detected_background: None,
            on_tick: None,
            scheduler: std::rc::Rc::new(std::cell::RefCell::new(
                crate::runtime::timers::Scheduler::new(std::time::Instant::now()),
            )),
            animations: crate::runtime::animation::AnimationRegistry::new(),
            virtual_clock: false,
            pointer: pointer::Pointer::default(),
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
            .sync_sheet_set(&mut app.dom, &app.tracker, &app.stylesheets);
        Ok(app)
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

    /// Produce a clone-able, `Send + Sync` handle to this App.
    /// Use from background threads / async tasks to request a
    /// redraw, ask the loop to exit, or inject a closure that
    /// runs on the loop thread with exclusive DOM access.
    pub fn handle(&self) -> AppHandle {
        AppHandle::from_shared(Arc::clone(&self.shared))
    }

    /// Mutable DOM access for pre-`run` setup. Listener registration,
    /// tree construction, etc. goes here. For event-loop-era
    /// mutations, use the `AppContext` passed to the tick handler or receive
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

    // ─── Event + frame plumbing (test + internal entry points) ──────

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
