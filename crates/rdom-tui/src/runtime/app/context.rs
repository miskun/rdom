//! `AppContext` — the handle a tick callback (or an in-loop
//! handler scope) uses to poke the runtime.
//!
//! Surface:
//!
//! - `dom` — mutable DOM access.
//! - `request_redraw()` — explicit dirty bit for mutations the
//!   observer didn't see (scroll offset, direct ext writes, etc.).
//! - `quit()` — exit the loop after this tick / event completes.
//! - `dispatch(target, event)` — synchronous nested dispatch.
//! - `queue_dispatch(target, event)` — runs after the current
//!   task completes but before the next event (browser
//!   microtask-queue semantics).
//!
//! Deferred to a later commit:
//! - `request_animation_frame(cb)` — pre-paint hook.
//! - `set_pointer_capture(id)` — drag routing (Phase 6).

use rdom_core::{Event, NodeId};

use crate::TuiDom;

/// Control flow returned from an `on_tick` callback: keep running
/// or exit the loop after the current task completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlFlow {
    /// Keep running. Redraw if dirty. (Default.)
    #[default]
    Continue,
    /// Exit the loop after this iteration.
    Quit,
}

/// Per-tick handle exposed to `on_tick` callbacks. Not
/// cross-thread (`!Send`, `!Sync`) — it borrows the DOM
/// exclusively. For cross-thread poking, use `AppHandle` (shipped
/// in a follow-up commit).
/// A change to the App's stylesheet stack requested through an
/// [`AppContext`]; the App applies it right after the handler returns.
/// `Set` / `Push` carry the id already allocated for the sheet — the
/// App registers the sheet under it and allocates nothing.
pub(super) enum StylesheetIntent {
    Set(super::StylesheetId, crate::style::Stylesheet),
    Push(super::StylesheetId, crate::style::Stylesheet),
    Remove(super::StylesheetId),
}

pub struct AppContext<'a> {
    /// Mutable DOM access. Mutations flow through the
    /// `DirtyTracker` automatically; the runtime invalidates the
    /// affected subtrees at the next cascade pass.
    pub dom: &'a mut TuiDom,
    /// Set by `request_redraw()`; read by the runtime after the
    /// tick returns.
    pub(super) redraw_requested: bool,
    /// Set by `quit()`.
    pub(super) quit_requested: bool,
    /// Events queued via `queue_dispatch`. Drained by the runtime
    /// after the tick/handler returns, before the next
    /// crossterm-event poll.
    pub(super) queued_dispatches: Vec<(NodeId, Event)>,
    /// Stylesheet-stack changes requested through this context; the
    /// App applies them after the handler returns (`SHOWCASE-EVT-1`).
    pub(super) stylesheet_intents: Vec<StylesheetIntent>,
    /// The App's id allocator, borrowed: a handler gets the id its
    /// intent will be registered under synchronously.
    stylesheet_ids: &'a mut super::stylesheets::StylesheetIdAllocator,
}

impl<'a> AppContext<'a> {
    pub(super) fn new(
        dom: &'a mut TuiDom,
        stylesheet_ids: &'a mut super::stylesheets::StylesheetIdAllocator,
    ) -> Self {
        Self {
            dom,
            redraw_requested: false,
            quit_requested: false,
            queued_dispatches: Vec::new(),
            stylesheet_intents: Vec::new(),
            stylesheet_ids,
        }
    }

    /// Replace every registered stylesheet with `sheet` once this
    /// handler returns; see [`App::set_stylesheet`](super::App::set_stylesheet).
    pub fn set_stylesheet(&mut self, sheet: crate::style::Stylesheet) -> super::StylesheetId {
        let id = self.stylesheet_ids.allocate();
        self.stylesheet_intents
            .push(StylesheetIntent::Set(id, sheet));
        id
    }

    /// Push `sheet` onto the stack once this handler returns; see
    /// [`App::push_stylesheet`](super::App::push_stylesheet). The
    /// returned id is the one the App assigns.
    pub fn push_stylesheet(&mut self, sheet: crate::style::Stylesheet) -> super::StylesheetId {
        let id = self.stylesheet_ids.allocate();
        self.stylesheet_intents
            .push(StylesheetIntent::Push(id, sheet));
        id
    }

    /// Remove the sheet `id` once this handler returns; see
    /// [`App::remove_stylesheet`](super::App::remove_stylesheet).
    pub fn remove_stylesheet(&mut self, id: super::StylesheetId) {
        self.stylesheet_intents.push(StylesheetIntent::Remove(id));
    }

    /// Request a paint after the current tick/event completes —
    /// even when the `DirtyTracker` didn't see a mutation
    /// (e.g., the app wrote `ext.scroll_y` directly, which
    /// bypasses the observer).
    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    /// Signal the runtime to exit the loop after the current task
    /// finishes. Equivalent to returning `ControlFlow::Quit` from
    /// an `on_tick` callback.
    pub fn quit(&mut self) {
        self.quit_requested = true;
    }

    /// Synchronously dispatch an event on `target`. Runs the full
    /// capture → target → bubble walk nested in the current
    /// execution — matches `dispatchEvent()` in the browser.
    ///
    /// Re-entrancy is supported: a listener fired as a result of
    /// this call may itself dispatch more events; each inner
    /// dispatch completes before returning to the outer one.
    pub fn dispatch(&mut self, target: NodeId, event: &mut Event) {
        let _ = self.dom.dispatch_event(target, event);
    }

    /// Queue an event to dispatch after the current task's
    /// listeners complete, but before the runtime moves on to the
    /// next crossterm event (microtask-queue semantics).
    ///
    /// Use when a handler wants a follow-up event to fire without
    /// recursing into dispatch from the current stack (e.g., a
    /// button's "click" handler that wants to emit a "close"
    /// signal to a dialog ancestor — queueing keeps the dispatch
    /// stack shallow and matches legacy-rdom's queued-event
    /// pattern).
    pub fn queue_dispatch(&mut self, target: NodeId, event: Event) {
        self.queued_dispatches.push((target, event));
    }
}
