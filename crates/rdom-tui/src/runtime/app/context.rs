//! `AppContext` — the handle a tick callback (or an in-loop
//! handler scope) uses to poke the runtime.
//!
//! Surface:
//!
//! - `dom` — mutable DOM access.
//! - `request_redraw()` — explicit dirty bit for mutations the
//!   observer didn't see (direct `TuiExt` writes that change paint).
//!   Scroll offsets need none: any change repaints on its own.
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
    /// (e.g., the app wrote a paint-affecting `TuiExt` field
    /// directly, which bypasses the observer). A scroll offset change
    /// needs none — the App repaints it on its own
    /// (`P7-SCROLL-REPAINT-1`). The frame it asks for re-cascades the
    /// whole tree, lays out and paints, as nothing tells the App which
    /// part of the tree changed.
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
    ///
    /// # Errors
    ///
    /// As [`Dom::dispatch_event`](rdom_core::Dom::dispatch_event):
    /// [`DomError::InvalidNode`](rdom_core::DomError::InvalidNode) when
    /// `target` is not a node of the document, and
    /// [`DomError::InvalidState`](rdom_core::DomError::InvalidState)
    /// when `event` is already being dispatched (DOM §2.9 step 1, the
    /// `InvalidStateError` `dispatchEvent()` throws).
    pub fn dispatch(&mut self, target: NodeId, event: &mut Event) -> rdom_core::Result<()> {
        self.dom.dispatch_event(target, event)
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
    ///
    /// A `target` dropped before the queue runs is skipped: there is
    /// nothing left to dispatch at. A clone of an in-flight event is a
    /// fresh event (see [`Event`]'s "Cloning"), so a listener may
    /// stash one for queueing. An event *moved* out of a dispatch in
    /// progress (`mem::replace(ctx.event, …)`) still carries that
    /// dispatch's flags; the queue runs after that dispatch ended —
    /// when on the web the same object dispatches again rather than
    /// throwing `InvalidStateError` — so it is dispatched as its clone,
    /// `new Event(e.type, e)` (`P7G-QUEUED-INFLIGHT-1`). No queued event
    /// panics the runtime.
    pub fn queue_dispatch(&mut self, target: NodeId, event: Event) {
        self.queued_dispatches.push((target, event));
    }
}

/// Run the dispatches [`AppContext::queue_dispatch`] queued, in order
/// (the tick's and the injected closures' queues share it). A dropped
/// target is skipped; an event still flagged by a finished dispatch it
/// was moved out of is dispatched as a fresh copy (see
/// `queue_dispatch`).
pub(super) fn run_queued_dispatches(dom: &mut TuiDom, queued: Vec<(NodeId, Event)>) {
    for (target, mut event) in queued {
        match dom.dispatch_event(target, &mut event) {
            Ok(()) | Err(rdom_core::DomError::InvalidNode(_)) => {}
            Err(rdom_core::DomError::InvalidState(_)) => {
                let mut fresh = event.clone();
                crate::tui_event::dispatch_event_to_live(dom, target, &mut fresh);
            }
            Err(e) => panic!(
                "dispatching a queued `{}` event failed: {e:?}",
                event.event_type
            ),
        }
    }
}
