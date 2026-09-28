//! The frame of an [`App`]: [`App::draw_if_dirty`] (smooth-scroll step +
//! cascade + animation step + layout + caret-reveal servicing + paint), the off-frame
//! [`App::cascade_and_layout`] half used by the autoscroll tick, the
//! keyboard scroll-focus marker and the `:valid` / `:invalid` marks that
//! run before the cascade, and the
//! transition-event drain that follows a painted frame.
//!
//! Each frame reruns only the stages its causes need (`redraw::Redraw`,
//! `P7G-PAINT-ONLY-FRAME-1`): a caret-blink flip only paints, a scroll
//! offset change lays out and paints, the dirty tracker's roots are
//! cascaded as subtrees, and only what the tracker cannot see
//! (stylesheet changes, resizes, `request_redraw`) cascades the whole
//! tree.

use std::io;

use rdom_core::NodeId;

use super::redraw::Redraw;
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
        let walked = self.validity_marks.flush(
            &mut self.dom,
            &self.tracker,
            self.style_elements
                .sheets()
                .chain(self.stylesheets.iter().map(|(_, s)| s)),
        );
        self.note_walk(walked);
    }

    /// Count a whole-tree walk (test instrumentation).
    #[cfg_attr(not(test), allow(unused_variables, clippy::unused_self))]
    fn note_walk(&mut self, walked: bool) {
        #[cfg(test)]
        {
            self.frame_stats.walks += u32::from(walked);
        }
    }

    /// Advance the smooth scrolls in flight to `now`
    /// (`runtime::smooth_scroll`) before this frame's layout, and wake
    /// for the next step one animation frame later while any is still
    /// running. The steps' `scroll` listeners run here, before the
    /// cascade, so their mutations land in this frame.
    /// Returns whether a step moved an offset (and fired `scroll`).
    fn step_smooth_scrolls(&mut self, now: std::time::Instant) -> bool {
        let step = crate::runtime::smooth_scroll::step_all(&mut self.dom, now);
        self.note_walk(true);
        // Scroll offsets feed layout (children are placed after scroll).
        self.redraw.note_if(step.moved, Redraw::Layout);
        self.smooth_scroll_next = step
            .active
            .then(|| now + std::time::Duration::from_millis(u64::from(self.animation_frame_ms)));
        step.moved
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
        // The whole-tree checks look for changes only code can make: skip
        // them when none ran since the last frame (`P7G-IDLE-WALKS-1`).
        let mut touched = std::mem::take(&mut self.touched);
        if touched {
            self.flush_validity_marks();
        }
        let now = self.scheduler.borrow().now();
        // A blink flip only changes what the caret painter reads.
        let flipped = self.caret_blink.update(&mut self.dom, now);
        self.redraw.note_if(flipped, Redraw::Paint);
        // A smooth scroll starts from code (the scroll API, a scroll
        // key) and then steps each animation frame until it lands.
        if (touched || self.smooth_scroll_next.is_some()) && self.step_smooth_scrolls(now) {
            // The steps fired `scroll`: listeners ran.
            touched = true;
            self.touched = true;
        }
        // Any scroll offset change repaints, whoever wrote it
        // (`P7-SCROLL-REPAINT-1`), and lays out again. Only code writes
        // offsets between frames.
        if touched {
            let moved = crate::runtime::scrollbar::moved_since_paint(&self.dom);
            self.note_walk(true);
            self.redraw.note_if(moved, Redraw::Layout);
        }
        let dirty_roots = self.take_dirty_roots();
        let redraw = self.redraw;

        if redraw == Redraw::Clean && dirty_roots.is_empty() {
            crate::rdom_trace!("draw_if_dirty: SKIP (clean, dirty_roots empty)");
            return Ok(());
        }
        crate::rdom_trace!("draw_if_dirty: DRAW (redraw={redraw:?}, dirty_roots={dirty_roots:?})");

        let dom = &mut self.dom;
        let sheets = cascade_order(&self.style_elements, &self.stylesheets);
        let animations = &mut self.animations;
        let mut pass = Pass::default();
        self.terminal.draw(|buf| {
            pass = style_and_layout(dom, &sheets, animations, redraw, &dirty_roots, buf.area);
            dom.paint_dom(buf, buf.area);
            Ok(())
        })?;
        self.note_pass(pass, true);
        self.redraw = Redraw::Clean;
        // Only a frame that laid out can have moved an offset (layout's
        // clamp, the caret reveal); a paint-only frame draws the offsets
        // already noted.
        if pass.laid_out {
            crate::runtime::scrollbar::note_painted(&mut self.dom);
            self.note_walk(true);
        }

        // Drain transition events queued during this frame.
        self.dispatch_animation_events();

        // Lay out and paint the next frame too while any transition is
        // still running — interpolation needs to keep stepping.
        self.redraw
            .note_if(!self.animations.is_empty(), Redraw::Layout);
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
        // Their listeners are code the next frame's checks must see.
        self.touched |= !pending.is_empty();
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
            ev.detail =
                rdom_core::EventDetail::Transition(Box::new(rdom_core::TransitionDetail::new(
                    property.css_name(),
                    elapsed_seconds.into(),
                    slot.pseudo_element().map(str::to_string),
                )));
            let _ = self.dom.dispatch_event(node, &mut ev);
        }
    }

    /// Cascade the dirty subtrees + advance animations + layout — the
    /// non-painting half of [`Self::draw_if_dirty`]'s frame. Used by
    /// [`Self::autoscroll_tick`] for an off-frame re-render so DOM mutations a
    /// consumer made inside a `scroll` handler are fully realized before the
    /// synthetic move. Drains the dirty-root tracker like a real frame, so the
    /// subsequent `draw_if_dirty` only re-cascades what the synthetic move
    /// newly dirtied (it still paints — the autoscroll notes `Redraw::Layout`).
    /// It always lays out, and cascades what a frame would: the whole
    /// tree when `Redraw::Cascade` is pending (then left at `Layout`, as
    /// the cascade is done), else the dirty roots.
    pub(super) fn cascade_and_layout(&mut self, area: Rect) {
        self.selectedness.flush(&mut self.dom);
        self.flush_style_elements();
        self.flush_validity_marks();
        let now = self.scheduler.borrow().now();
        let flipped = self.caret_blink.update(&mut self.dom, now);
        self.redraw.note_if(flipped, Redraw::Paint);
        let dirty_roots = self.take_dirty_roots();
        let redraw = self.redraw.max(Redraw::Layout);
        let sheets = cascade_order(&self.style_elements, &self.stylesheets);
        let pass = style_and_layout(
            &mut self.dom,
            &sheets,
            &mut self.animations,
            redraw,
            &dirty_roots,
            area,
        );
        self.note_pass(pass, false);
        if redraw == Redraw::Cascade {
            self.redraw = Redraw::Layout;
        }
    }

    /// Count what one pipeline run did (test instrumentation).
    #[cfg_attr(not(test), allow(unused_variables, clippy::unused_self))]
    fn note_pass(&mut self, pass: Pass, painted: bool) {
        #[cfg(test)]
        {
            let stats = &mut self.frame_stats;
            match pass.cascade {
                Some(CascadeScope::Full) => stats.full_cascades += 1,
                Some(CascadeScope::Subtrees) => stats.subtree_cascades += 1,
                None => {}
            }
            stats.layouts += u32::from(pass.laid_out);
            stats.paints += u32::from(painted);
        }
    }
}

/// Which cascade a pipeline run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CascadeScope {
    /// The whole tree.
    Full,
    /// The dirty tracker's subtree roots.
    Subtrees,
}

/// What one run of the frame pipeline up to paint did (read by the
/// test instrumentation only).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(not(test), allow(dead_code))]
struct Pass {
    cascade: Option<CascadeScope>,
    laid_out: bool,
}

/// Every sheet the cascade reads, in cascade order: the document's
/// `<style>` sheets in tree order, then the App's own in push order
/// (`cssom::style_elements`). Later sheets win same-specificity
/// contests.
pub(super) fn cascade_order<'a>(
    style_elements: &'a crate::cssom::style_elements::StyleElements,
    stylesheets: &'a [(StylesheetId, Stylesheet)],
) -> Vec<&'a Stylesheet> {
    style_elements
        .sheets()
        .chain(stylesheets.iter().map(|(_, s)| s))
        .collect()
}

/// The frame pipeline up to paint, shared by [`App::draw_if_dirty`] and
/// [`App::cascade_and_layout`], each stage only when `redraw` or the
/// dirty roots need it (`redraw::Redraw`, `P7G-PAINT-ONLY-FRAME-1`):
/// cascade (the whole tree for `Redraw::Cascade`, else `dirty_roots`'
/// subtrees, else nothing) → register the transitions a cascade's
/// property changes start → when anything was cascaded or `redraw` is
/// at least `Layout`: advance the running transitions (writing
/// interpolated values into `TuiExt::presentation`), lay out, and
/// service a caret reveal requested this frame against the fresh
/// extent, re-laying out when it moved a scroll offset. A
/// `Redraw::Paint` frame with no dirty roots runs none of it.
///
/// A free function over split borrows because `draw_if_dirty` runs it
/// inside `Terminal::draw` while the terminal is borrowed.
fn style_and_layout(
    dom: &mut TuiDom,
    sheets: &[&Stylesheet],
    animations: &mut AnimationRegistry,
    redraw: Redraw,
    dirty_roots: &[NodeId],
    area: Rect,
) -> Pass {
    let now = std::time::Instant::now();
    let cascade = if redraw == Redraw::Cascade {
        dom.cascade_all(sheets);
        Some(CascadeScope::Full)
    } else if !dirty_roots.is_empty() {
        dom.cascade_subtrees_all(sheets, dirty_roots);
        Some(CascadeScope::Subtrees)
    } else {
        None
    };
    if cascade.is_some() {
        crate::runtime::animation::diff_and_register(dom, animations, now);
    }
    let laid_out = cascade.is_some() || redraw >= Redraw::Layout;
    if laid_out {
        animations.advance(dom, now);
        dom.layout_dom(area);
        if crate::runtime::scrollbar::service_caret_reveal(dom) {
            dom.layout_dom(area);
        }
    }
    Pass { cascade, laid_out }
}
