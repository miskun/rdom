//! Event intake of an [`App`]: [`App::handle_event`] routes one
//! crossterm event — keys to the focused element, mouse events through
//! the `Router`, a resize to the document — and folds what the route did
//! into the next frame's work.

use crossterm::event::Event as CtEvent;

use super::App;
use super::redraw::Redraw;
use crate::render::backend::Backend;
use crate::runtime::router::RouteOutcome;
use crate::{TuiDispatchExt, TuiEvent};

impl<B: Backend> App<B> {
    /// Fold a mouse route's outcome into the next frame's work
    /// (`P7G-ROUTE-REDRAW-1`): the router's own work — a hover change,
    /// a wheel scroll, a scrollbar press, a selection drag — lays out
    /// and repaints, cascading only the dirty tracker's roots (the
    /// elements whose `:hover` / `:focus` flipped); a listener's
    /// `request_redraw` cascades the whole tree.
    pub(super) fn note_route(&mut self, outcome: RouteOutcome) {
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
        // Controls the app inserted since the last event get their text
        // node, and option lists it changed settle, before any default
        // action reads them.
        self.prelude.control_seeding.flush(&mut self.dom);
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
}
