//! The frame of an [`App`]: [`App::draw_if_dirty`] (smooth-scroll step +
//! cascade + animation step + layout + caret-reveal servicing + paint), the off-frame
//! [`App::cascade_and_layout`] half used by the autoscroll tick, the
//! keyboard scroll-focus marker and the `:valid` / `:invalid` marks that
//! run before the cascade, and the
//! transition-event drain that follows a painted frame.

use std::io;

use rdom_core::NodeId;

use super::{App, StylesheetId};
use crate::TuiDom;
use crate::render::backend::Backend;
use crate::render::{LayoutExt, PaintExt, Rect};
use crate::runtime::animation::AnimationRegistry;
use crate::style::{CascadeExt, Stylesheet};

impl<B: Backend> App<B> {
    /// Keep `data-rdom-scroll-focus` on the scroll container the
    /// keyboard scrolls: the nearest overflowing scroll ancestor of the
    /// focus, per the previous frame's layout (`scrollbar::
    /// scroll_focus_target`). The UA sheet colors that container's
    /// scrollbar thumb through the attribute; `:focus-within` alone
    /// would light every overflowing ancestor (`FOCUS-THUMB-NEAREST-1`).
    /// Only while the focus is evident (`Dom::focus_visible`,
    /// `P7-FOCUS-VISIBLE-1`). Runs before the cascade, so the change lands in this frame.
    fn mark_scroll_focus(&mut self) {
        // The accent thumb is a focus indicator: shown only while the
        // focus is evident (`:focus-visible`), like the control tint.
        let target = crate::runtime::scrollbar::scroll_focus_target(&self.dom)
            .filter(|_| self.dom.focus_visible());
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

    /// Mark the elements whose `:valid` / `:invalid` state changed since
    /// the last frame style-dirty (`validation::ValidityMarks`). Runs
    /// before the roots are taken, so they re-cascade in this frame.
    fn flush_validity_marks(&mut self) {
        self.validity_marks.flush(
            &mut self.dom,
            &self.tracker,
            self.style_elements
                .sheets()
                .chain(self.stylesheets.iter().map(|(_, s)| s)),
        );
    }

    /// Advance the smooth scrolls in flight to `now`
    /// (`runtime::smooth_scroll`) before this frame's layout, and wake
    /// for the next step one animation frame later while any is still
    /// running. The steps' `scroll` listeners run here, before the
    /// cascade, so their mutations land in this frame.
    fn step_smooth_scrolls(&mut self, now: std::time::Instant) {
        let step = crate::runtime::smooth_scroll::step_all(&mut self.dom, now);
        self.needs_redraw |= step.moved;
        self.smooth_scroll_next = step
            .active
            .then(|| now + std::time::Duration::from_millis(u64::from(self.animation_frame_ms)));
    }

    /// Cascade + layout + paint if anything is dirty. Pairs with
    /// [`Self::handle_event`] for apps running a custom event loop.
    pub fn draw_if_dirty(&mut self) -> io::Result<()> {
        // Animation events (`transitionend`) fire from in here; their
        // listeners may schedule timers.
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        // Before the roots are taken, so a marker move and the options
        // the selectedness algorithm (re)selects are cascaded in this
        // frame.
        self.selectedness.flush(&mut self.dom);
        self.flush_style_elements();
        self.mark_scroll_focus();
        self.flush_validity_marks();
        let now = self.scheduler.borrow().now();
        self.needs_redraw |= self.caret_blink.update(&mut self.dom, now);
        self.step_smooth_scrolls(now);
        // Any scroll offset change repaints, whoever wrote it
        // (`P7-SCROLL-REPAINT-1`).
        self.needs_redraw |= crate::runtime::scrollbar::moved_since_paint(&self.dom);
        let dirty_roots = self.take_dirty_roots();

        if !self.needs_redraw && dirty_roots.is_empty() {
            crate::rdom_trace!("draw_if_dirty: SKIP (needs_redraw=false, dirty_roots empty)");
            return Ok(());
        }
        crate::rdom_trace!(
            "draw_if_dirty: DRAW (needs_redraw={}, dirty_roots={:?})",
            self.needs_redraw,
            dirty_roots
        );

        let dom = &mut self.dom;
        let sheets = cascade_order(&self.style_elements, &self.stylesheets);
        let animations = &mut self.animations;
        self.terminal.draw(|buf| {
            style_and_layout(dom, &sheets, animations, &dirty_roots, buf.area);
            dom.paint_dom(buf, buf.area);
            Ok(())
        })?;
        crate::runtime::scrollbar::note_painted(&mut self.dom);

        // Drain transition events queued during this frame.
        self.dispatch_animation_events();

        // Force redraw next frame if any transitions are still
        // running — interpolation needs to keep stepping.
        self.needs_redraw = !self.animations.is_empty();
        Ok(())
    }

    /// Drain the dirty tracker: the subtree roots this frame
    /// re-cascades, sorted and de-duplicated (several mutations under
    /// one root register it more than once). Empty means "no subtree is
    /// dirty" — a frame that still runs cascades the whole tree.
    fn take_dirty_roots(&mut self) -> Vec<NodeId> {
        let mut dirty_roots = self.tracker.roots_snapshot();
        // Drain only when there is something to drain, so the tracker's
        // bookkeeping is left alone on a clean frame.
        if !dirty_roots.is_empty() {
            self.tracker.take_roots();
        }
        dirty_roots.sort_unstable();
        dirty_roots.dedup();
        dirty_roots
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
    pub(super) fn cascade_and_layout(&mut self, area: Rect) {
        self.selectedness.flush(&mut self.dom);
        self.flush_style_elements();
        self.flush_validity_marks();
        let now = self.scheduler.borrow().now();
        self.needs_redraw |= self.caret_blink.update(&mut self.dom, now);
        let dirty_roots = self.take_dirty_roots();
        let sheets = cascade_order(&self.style_elements, &self.stylesheets);
        style_and_layout(
            &mut self.dom,
            &sheets,
            &mut self.animations,
            &dirty_roots,
            area,
        );
    }
}

/// Every sheet the cascade reads, in cascade order: the document's
/// `<style>` sheets in tree order, then the App's own in push order
/// (`cssom::style_elements`). Later sheets win same-specificity
/// contests.
fn cascade_order<'a>(
    style_elements: &'a crate::cssom::style_elements::StyleElements,
    stylesheets: &'a [(StylesheetId, Stylesheet)],
) -> Vec<&'a Stylesheet> {
    style_elements
        .sheets()
        .chain(stylesheets.iter().map(|(_, s)| s))
        .collect()
}

/// The frame pipeline up to paint, shared by [`App::draw_if_dirty`] and
/// [`App::cascade_and_layout`]: cascade (`dirty_roots`' subtrees, or the
/// whole tree when there are none) → register the transitions the
/// cascade's property changes start and advance the running ones
/// (writing interpolated values into `TuiExt::presentation`) → layout →
/// service a caret reveal requested this frame against the fresh
/// extent, re-laying out when it moved a scroll offset.
///
/// A free function over split borrows because `draw_if_dirty` runs it
/// inside `Terminal::draw` while the terminal is borrowed.
fn style_and_layout(
    dom: &mut TuiDom,
    sheets: &[&Stylesheet],
    animations: &mut AnimationRegistry,
    dirty_roots: &[NodeId],
    area: Rect,
) {
    let now = std::time::Instant::now();
    if dirty_roots.is_empty() {
        dom.cascade_all(sheets);
    } else {
        dom.cascade_subtrees_all(sheets, dirty_roots);
    }
    crate::runtime::animation::diff_and_register(dom, animations, now);
    animations.advance(dom, now);
    dom.layout_dom(area);
    if crate::runtime::scrollbar::service_caret_reveal(dom) {
        dom.layout_dom(area);
    }
}
