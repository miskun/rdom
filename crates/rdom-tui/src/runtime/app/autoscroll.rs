//! DRAG-AUTOSCROLL — the drag-autoscroll session of an [`App`]: arming
//! from the routed pointer, the periodic tick, and disarming when the
//! drag ends.
//!
//! Two drags autoscroll: a captured drag that opted in
//! (`Dom::set_drag_autoscroll`), and the runtime's own text-selection
//! drag, which is router state and takes no pointer capture
//! (P6G-SELECTION-CAPTURE-1) — see [`App::autoscroll_drag_source`].
//!
//! A drag **owns one scroll container** for its lifetime; see the
//! `autoscroll_*` fields on [`App`]. The session is keyed on the
//! scheduler clock so it behaves identically under the live loop and
//! [`App::advance`].

use std::time::Duration;

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};

use super::App;
use crate::render::backend::Backend;

/// Cadence of the drag-autoscroll tick (DRAG-AUTOSCROLL). One internal
/// constant; the live loop floors its poll timeout to this while an autoscroll
/// drag is armed so it wakes to tick even with the pointer held still.
pub(super) const AUTOSCROLL_PERIOD: Duration = Duration::from_millis(50);

impl<B: Backend> App<B> {
    /// The node the active autoscrolling drag started from, or `None` when no
    /// drag autoscrolls. A pointer capture owns the drag when present — it
    /// autoscrolls only if it opted in; otherwise a text-selection drag
    /// autoscrolls from its anchor's inline-flow container, like a browser's
    /// native selection.
    fn autoscroll_drag_source(&self) -> Option<crate::NodeId> {
        match self.dom.pointer_capture() {
            Some(captured) => self.dom.drag_autoscroll().then_some(captured),
            None => self.router.selection_drag.map(|flow| flow.owner()),
        }
    }

    /// Clear all DRAG-AUTOSCROLL session state (pointer, deadline, and the
    /// sticky container). Called when the drag ends (capture released, opt-in
    /// dropped, or the text-selection drag over).
    fn disarm_autoscroll(&mut self) {
        self.autoscroll_pointer = None;
        self.autoscroll_next = None;
        self.autoscroll_container = None;
    }

    /// Update the DRAG-AUTOSCROLL session from the latest pointer (called after
    /// every *real* routed mouse event — never the synthetic re-dispatch, which
    /// routes directly). The session is alive only while an autoscrolling drag
    /// is in progress ([`Self::autoscroll_drag_source`]). The scroll container is resolved **once**, the first
    /// time the pointer reaches an edge zone, then stays **sticky**: subsequent
    /// moves only update the tracked pointer, never re-resolve the container. So
    /// the captured node scrolling out of view, or the pointer overshooting past
    /// the container onto a sibling, neither re-targets nor disarms the scroll.
    pub(super) fn note_autoscroll(&mut self, col: u16, row: u16) {
        let Some(source) = self.autoscroll_drag_source() else {
            self.disarm_autoscroll();
            return;
        };
        if self.autoscroll_container.is_none() {
            // Resolve the container only once the pointer is actually in an edge
            // zone (so it resolves to the container under/at the edge, while the
            // pointer is still inside it). Until then the session stays idle.
            let resolved = crate::runtime::scrollbar::resolve_autoscroll_container(
                &self.dom,
                source,
                (col, row),
            )
            .filter(|&c| {
                crate::runtime::scrollbar::autoscroll_step_for(&self.dom, c, (col, row)).is_some()
            });
            match resolved {
                Some(c) => self.autoscroll_container = Some(c),
                None => {
                    self.autoscroll_pointer = None;
                    self.autoscroll_next = None;
                    return;
                }
            }
        }
        // Armed + sticky: track the pointer; the tick decides scroll vs. idle.
        self.autoscroll_pointer = Some((col, row));
        if self.autoscroll_next.is_none() {
            self.autoscroll_next = Some(self.scheduler.borrow().now() + AUTOSCROLL_PERIOD);
        }
    }

    /// Fire any due autoscroll ticks. Keyed on the scheduler clock so it works
    /// identically under the live loop (wall-synced) and `advance` (virtual).
    /// The session disarms only when the drag ends —
    /// a tick that finds the pointer out of the edge zone (or the container at
    /// its limit) simply idles, keeping the sticky container for the drag.
    pub(super) fn service_autoscroll(&mut self) {
        // The synthetic drag move dispatches listeners that may schedule timers.
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        if self.autoscroll_drag_source().is_none() {
            self.disarm_autoscroll();
            return;
        }
        let (Some((col, row)), Some(container)) =
            (self.autoscroll_pointer, self.autoscroll_container)
        else {
            return;
        };
        let now = self.scheduler.borrow().now();
        let mut guard = 0u8;
        while let Some(next) = self.autoscroll_next {
            if now < next || guard >= 8 {
                break;
            }
            guard += 1;
            self.autoscroll_tick(container, col, row);
            self.autoscroll_next = Some(next + AUTOSCROLL_PERIOD);
        }
    }

    /// One autoscroll tick on the drag's **sticky** `container`: if the pointer
    /// is in an edge zone and there's room, scroll one step and re-dispatch the
    /// drag at the held pointer (routed like a real move) so the consumer — and native text
    /// selection — extend against the new scroll position. Otherwise it's a
    /// no-op idle tick (the session stays armed; it does not disarm here).
    fn autoscroll_tick(&mut self, container: crate::NodeId, col: u16, row: u16) {
        let Some((axis, step)) =
            crate::runtime::scrollbar::autoscroll_step_for(&self.dom, container, (col, row))
        else {
            return; // not in an edge zone — idle, stay armed
        };
        if !crate::runtime::scrollbar::autoscroll_step(&mut self.dom, container, axis, step) {
            return; // container at its scroll limit — idle
        }
        // Re-render (cascade + layout) against the new scroll offset BEFORE the
        // synthetic move, so a layout-dependent consumer re-evaluates at the
        // revealed content. This is the **full** frame path, not a bare
        // `layout_dom`: the `scroll` event fired by `autoscroll_step` runs the
        // consumer's listener, which may MUTATE the DOM (e.g. a virtualized
        // table re-windows its rows + runs column-sizing) — those mutations
        // need a cascade to take effect, or the re-materialized nodes lay out
        // unstyled (collapsed column widths, wrong spacer heights → a clamped
        // `scroll_top` and a mis-mapped pointer). Native text selection doesn't
        // mutate here, so its cascade is a no-op.
        let area = self.terminal.size();
        self.cascade_and_layout(area);
        // A synthetic left-button Drag at the held pointer carries the held
        // coords + the held-button bitmask, so the consumer's move guard accepts
        // it and re-evaluates against the new scroll offset.
        let synthetic = CtMouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        };
        let outcome = self.router.route(&mut self.dom, CtEvent::Mouse(synthetic));
        self.needs_redraw |= outcome.redraw_requested;
        self.needs_redraw = true; // the scroll itself changed the view
    }
}
