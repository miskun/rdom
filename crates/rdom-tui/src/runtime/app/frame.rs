//! The frame of an [`App`]: [`App::draw_if_dirty`] (cascade + animation
//! step + layout + caret-reveal servicing + paint), the off-frame
//! [`App::cascade_and_layout`] half used by the autoscroll tick, the
//! keyboard scroll-focus marker that runs before the cascade, and the
//! transition-event drain that follows a painted frame.

use std::io;

use super::App;
use crate::render::backend::Backend;
use crate::render::{LayoutExt, PaintExt};
use crate::style::{CascadeExt, Stylesheet};

impl<B: Backend> App<B> {
    /// Keep `data-rdom-scroll-focus` on the scroll container the
    /// keyboard scrolls: the nearest overflowing scroll ancestor of the
    /// focus, per the previous frame's layout (`scrollbar::
    /// scroll_focus_target`). The UA sheet colors that container's
    /// scrollbar thumb through the attribute; `:focus-within` alone
    /// would light every overflowing ancestor (`FOCUS-THUMB-NEAREST-1`).
    /// Runs before the cascade, so the change lands in this frame.
    fn mark_scroll_focus(&mut self) {
        let target = crate::runtime::scrollbar::scroll_focus_target(&self.dom);
        if target == self.scroll_focus_marked {
            return;
        }
        if let Some(prev) = self.scroll_focus_marked.take()
            && self.dom.contains(prev)
        {
            self.dom
                .remove_attribute(prev, crate::runtime::scrollbar::SCROLL_FOCUS_ATTR)
                .expect("a live element accepts attribute removal");
        }
        if let Some(next) = target {
            self.dom
                .set_attribute(next, crate::runtime::scrollbar::SCROLL_FOCUS_ATTR, "")
                .expect("the focused element's ancestor is a live element");
        }
        self.scroll_focus_marked = target;
    }

    /// Cascade + layout + paint if anything is dirty. Pairs with
    /// [`Self::handle_event`] for apps running a custom event loop.
    pub fn draw_if_dirty(&mut self) -> io::Result<()> {
        // Animation events (`transitionend`) fire from in here; their
        // listeners may schedule timers.
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        // Before the roots snapshot, so a marker move is cascaded in
        // this frame.
        self.mark_scroll_focus();
        let mut dirty_roots = self.tracker.roots_snapshot();
        // dirty_roots is a snapshot; we need to actually drain them
        // so subsequent frames don't re-cascade the same roots.
        if !dirty_roots.is_empty() {
            self.tracker.take_roots();
        }

        if !self.needs_redraw && dirty_roots.is_empty() {
            crate::rdom_trace!("draw_if_dirty: SKIP (needs_redraw=false, dirty_roots empty)");
            return Ok(());
        }
        crate::rdom_trace!(
            "draw_if_dirty: DRAW (needs_redraw={}, dirty_roots={:?})",
            self.needs_redraw,
            dirty_roots
        );

        // De-duplicate: multiple mutations under the same root end up
        // registered under the same NodeId.
        dirty_roots.sort_unstable();
        dirty_roots.dedup();

        // Cascade reads the full registered set; later sheets win
        // same-specificity contests (push order). Build a small
        // ref-slice view over the (id, sheet) storage — allocates
        // a Vec of fat-pointers, negligible vs. the cascade itself.
        let sheets: Vec<&Stylesheet> = self.stylesheets.iter().map(|(_, s)| s).collect();
        let dom = &mut self.dom;
        let terminal = &mut self.terminal;
        let animations = &mut self.animations;
        let now = std::time::Instant::now();

        terminal.draw(|buf| {
            if dirty_roots.is_empty() {
                dom.cascade_all(&sheets);
            } else {
                dom.cascade_subtrees_all(&sheets, &dirty_roots);
            }
            // Detect cascade-driven property changes and register
            // transitions before layout / paint pick up the new
            // values.
            crate::runtime::animation::diff_and_register(dom, animations, now);
            // Then advance any in-flight animations (writes
            // interpolated values into TuiExt.presentation).
            animations.advance(dom, now);
            dom.layout_dom(buf.area);
            // A caret reveal requested by an edit this frame re-runs
            // against the fresh extent; a changed offset needs one more
            // layout before paint.
            if crate::runtime::scrollbar::service_caret_reveal(dom) {
                dom.layout_dom(buf.area);
            }
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
    pub(super) fn dispatch_animation_events(&mut self) {
        use crate::runtime::animation::{PendingEvent, TransitionEventKind};
        let pending = self.animations.take_pending_events();
        for PendingEvent {
            node,
            slot,
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
                pseudo_element: slot.pseudo_element().map(str::to_string),
            }));
            let _ = self.dom.dispatch_event(node, &mut ev);
        }
    }

    /// Cascade the dirty subtrees + advance animations + layout — the
    /// non-painting half of [`Self::draw_if_dirty`]'s frame. Used by
    /// [`Self::autoscroll_tick`] for an off-frame re-render so DOM mutations a
    /// consumer made inside a `scroll` handler are fully realized before the
    /// synthetic move. Drains the dirty-root tracker like a real frame, so the
    /// subsequent `draw_if_dirty` only re-cascades what the synthetic move
    /// newly dirtied (it still paints — `needs_redraw` is set).
    pub(super) fn cascade_and_layout(&mut self, area: crate::render::Rect) {
        let mut dirty_roots = self.tracker.roots_snapshot();
        if !dirty_roots.is_empty() {
            self.tracker.take_roots();
        }
        dirty_roots.sort_unstable();
        dirty_roots.dedup();
        let sheets: Vec<&Stylesheet> = self.stylesheets.iter().map(|(_, s)| s).collect();
        if dirty_roots.is_empty() {
            self.dom.cascade_all(&sheets);
        } else {
            self.dom.cascade_subtrees_all(&sheets, &dirty_roots);
        }
        let now = std::time::Instant::now();
        crate::runtime::animation::diff_and_register(&mut self.dom, &mut self.animations, now);
        self.animations.advance(&mut self.dom, now);
        self.dom.layout_dom(area);
        if crate::runtime::scrollbar::service_caret_reveal(&mut self.dom) {
            self.dom.layout_dom(area);
        }
    }
}
