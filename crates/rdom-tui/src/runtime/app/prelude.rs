//! The frame prelude of an [`App`](super::App): the stages a frame runs
//! before its cascade, in order, and the state they own
//! (`P7G-FRAME-PRELUDE-1`).
//!
//! [`FramePrelude::run`] is the one place the order is written:
//!
//! 1. **`<select>` selectedness** — settle the selects whose options
//!    changed (`select::Selectedness`), so the options it (re)selects
//!    cascade this frame;
//! 2. **`<style>` elements** — re-parse the sheets whose text changed
//!    (`cssom::style_elements`); a change invalidates the cascade
//!    ([`FramePrelude::sheets_changed`]);
//! 3. **scroll-focus marker** — move `data-rdom-scroll-focus` to the
//!    container the keyboard scrolls (frames only);
//! 4. **validity marks** — dirty the elements whose `:valid` /
//!    `:invalid` flipped (`validation::ValidityMarks`);
//! 5. **caret blink** — flip the caret phase (a paint-only change);
//! 6. **smooth scrolls** — step the scrolls in flight (frames only);
//! 7. **painted check** — any scroll offset moved since the last paint
//!    lays out and repaints (frames only);
//! 8. **tracker flags** — a text change the dirty tracker saw lays out
//!    and repaints, a selection change repaints (frames only; the
//!    off-frame run leaves them for the next frame).
//!
//! Stages 4, 6 and 7 walk the whole tree, so a frame runs them only when
//! code ran since the last frame ([`FramePrelude::touched`],
//! `P7G-IDLE-WALKS-1`) — and 6 also while a smooth scroll is in flight.
//! After the paint, [`FramePrelude::after_paint`] records the painted
//! offsets stage 7 compares against.

use std::time::{Duration, Instant};

use rdom_core::NodeId;

use super::StylesheetId;
use super::redraw::Redraw;
use crate::TuiDom;
use crate::cssom::style_elements::StyleElements;
use crate::runtime::builtins::select::Selectedness;
use crate::runtime::builtins::validation::ValidityMarks;
use crate::runtime::caret_blink::CaretBlink;
use crate::style::Stylesheet;
use crate::style::dirty_tracker::DirtyTracker;

/// Which run of the prelude: a drawn frame's, or the off-frame cascade
/// and layout of the autoscroll tick (`App::cascade_and_layout`), which
/// runs stages 1, 2, 4 (always) and 5 only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreludeRun {
    Frame,
    OffFrame,
}

/// What the prelude reads and writes of the rest of the App.
pub(super) struct PreludeCx<'a> {
    pub(super) dom: &'a mut TuiDom,
    pub(super) tracker: &'a DirtyTracker,
    /// The App's own sheets, in push order.
    pub(super) app_sheets: &'a [(StylesheetId, Stylesheet)],
    /// What the frame must redo; each stage notes the least it needs.
    pub(super) redraw: &'a mut Redraw,
    /// The scheduler clock.
    pub(super) now: Instant,
    /// The animation-frame budget: when the next smooth-scroll step is
    /// due.
    pub(super) animation_frame: Duration,
}

/// The state of the pre-cascade stages.
pub(super) struct FramePrelude {
    /// Runs the `<select>` selectedness setting algorithm on selects
    /// whose options were inserted / removed; flushed before each event
    /// and each frame.
    pub(super) selectedness: Selectedness,
    /// The document's `<style>` sheets, live (`cssom::style_elements`):
    /// they cascade before the App's own sheets, in tree order.
    pub(super) style_elements: StyleElements,
    /// The element currently carrying `data-rdom-scroll-focus`.
    scroll_focus_marked: Option<NodeId>,
    /// Each element's `:valid` / `:invalid` state as of the last frame.
    validity_marks: ValidityMarks,
    /// Caret blink phase (`runtime::caret_blink`). Off unless enabled —
    /// `App::new` enables it at the default rate.
    pub(super) caret_blink: CaretBlink,
    /// When the next smooth-scroll step is due (`runtime::smooth_scroll`),
    /// on the scheduler clock; `None` when no scroll container is
    /// animating.
    pub(super) smooth_scroll_next: Option<Instant>,
    /// Code that may have changed the tree or runtime-managed `TuiExt`
    /// state (scroll offsets, a custom validity, a user edit) ran since
    /// the last frame's checks: an event, a timer / microtask / rAF
    /// callback, an injected closure, the `on_tick` callback, a
    /// stylesheet change, or `dom_mut()` access. The whole-tree stages
    /// run only then, so an idle tick walks nothing (`P7G-IDLE-WALKS-1`).
    pub(super) touched: bool,
}

impl FramePrelude {
    /// Install the prelude's observers on `dom` (selectedness, `<style>`
    /// elements). Starts touched, so the first frame runs every check.
    pub(super) fn install(dom: &mut TuiDom) -> Self {
        Self {
            selectedness: Selectedness::install(dom),
            style_elements: StyleElements::install(dom),
            scroll_focus_marked: None,
            validity_marks: ValidityMarks::default(),
            caret_blink: CaretBlink::new(None),
            smooth_scroll_next: None,
            touched: true,
        }
    }

    /// Run the stages in order (module doc). Returns the number of
    /// whole-tree walks made (`FrameStats::walks`).
    pub(super) fn run(&mut self, cx: &mut PreludeCx<'_>, run: PreludeRun) -> u32 {
        let mut walks = 0;
        let frame = run == PreludeRun::Frame;
        // 1.
        self.selectedness.flush(cx.dom);
        // 2.
        if self.style_elements.flush(cx.dom) {
            self.sheets_changed(cx.tracker, cx.app_sheets, cx.redraw);
        }
        // 3.
        if frame {
            self.mark_scroll_focus(cx.dom);
        }
        // Frames skip the whole-tree stages when no code ran; the
        // off-frame run always checks validity and leaves the flag for
        // the next frame.
        let mut touched = if frame {
            std::mem::take(&mut self.touched)
        } else {
            true
        };
        // 4.
        if touched {
            let walked = self.validity_marks.flush(
                cx.dom,
                cx.tracker,
                self.style_elements
                    .sheets()
                    .chain(cx.app_sheets.iter().map(|(_, s)| s)),
            );
            walks += u32::from(walked);
        }
        // 5. A blink flip only changes what the caret painter reads.
        let flipped = self.caret_blink.update(cx.dom, cx.now);
        cx.redraw.note_if(flipped, Redraw::Paint);
        if !frame {
            return walks;
        }
        // 6. A smooth scroll starts from code (the scroll API, a scroll
        // key) and then steps each animation frame until it lands. The
        // steps' `scroll` listeners run here, before the cascade, so
        // their mutations land in this frame.
        if touched || self.smooth_scroll_next.is_some() {
            let step = crate::runtime::smooth_scroll::step_all(cx.dom, cx.now);
            walks += 1;
            // Scroll offsets feed layout (children are placed after
            // scroll).
            cx.redraw.note_if(step.moved, Redraw::Layout);
            self.smooth_scroll_next = step.active.then(|| cx.now + cx.animation_frame);
            if step.moved {
                // The steps fired `scroll`: listeners ran.
                touched = true;
                self.touched = true;
            }
        }
        // 7. Any scroll offset change repaints, whoever wrote it
        // (`P7-SCROLL-REPAINT-1`), and lays out again. Only code writes
        // offsets between frames.
        if touched {
            let moved = crate::runtime::scrollbar::moved_since_paint(cx.dom);
            walks += 1;
            cx.redraw.note_if(moved, Redraw::Layout);
        }
        // 8. What the dirty tracker recorded beyond its cascade roots
        // (which the frame drains itself), whoever made the change —
        // a listener, a timer, an injected closure, `dom_mut()`
        // (`P7G-OFF-EVENT-PAINT-1`): text lays out and repaints, a
        // selection move repaints. Last, so the stages' own listeners
        // (stage 6's `scroll`) are covered.
        cx.redraw
            .note_if(cx.tracker.take_paint_dirty(), Redraw::Layout);
        cx.redraw
            .note_if(cx.tracker.take_selection_dirty(), Redraw::Paint);
        walks
    }

    /// After a drawn frame: record the painted scroll offsets stage 7
    /// compares against. Only a frame that laid out can have moved one
    /// (layout's clamp, the caret reveal); a paint-only frame draws the
    /// offsets already noted. Returns the walks made.
    pub(super) fn after_paint(&mut self, dom: &mut TuiDom, laid_out: bool) -> u32 {
        if !laid_out {
            return 0;
        }
        crate::runtime::scrollbar::note_painted(dom);
        1
    }

    /// The sheets changed (an App sheet added / removed, a `<style>`
    /// element re-parsed): re-read the sibling-combinator hint, drop the
    /// tracker's roots and cascade the whole tree, refresh the validity
    /// marks' sheet check, and run the whole-tree checks next frame.
    pub(super) fn sheets_changed(
        &mut self,
        tracker: &DirtyTracker,
        app_sheets: &[(StylesheetId, Stylesheet)],
        redraw: &mut Redraw,
    ) {
        self.sync_sibling_combinators(tracker, app_sheets);
        tracker.take_roots();
        redraw.note(Redraw::Cascade);
        self.validity_marks.sheets_changed();
        self.touched = true;
    }

    /// Tell the dirty tracker whether the sheets now cascaded use `+` /
    /// `~` (`DirtyTracker::set_sibling_combinators`), so a state change
    /// dirties its siblings only when a selector can read it there.
    pub(super) fn sync_sibling_combinators(
        &self,
        tracker: &DirtyTracker,
        app_sheets: &[(StylesheetId, Stylesheet)],
    ) {
        let used = self
            .cascade_order(app_sheets)
            .into_iter()
            .any(crate::style::dirty_tracker::uses_sibling_combinators);
        tracker.set_sibling_combinators(used);
    }

    /// Every sheet the cascade reads, in cascade order: the document's
    /// `<style>` sheets in tree order, then the App's own in push order
    /// (`cssom::style_elements`). Later sheets win same-specificity
    /// contests.
    pub(super) fn cascade_order<'a>(
        &'a self,
        app_sheets: &'a [(StylesheetId, Stylesheet)],
    ) -> Vec<&'a Stylesheet> {
        self.style_elements
            .sheets()
            .chain(app_sheets.iter().map(|(_, s)| s))
            .collect()
    }

    /// Keep `data-rdom-scroll-focus` on the scroll container the
    /// keyboard scrolls: the nearest overflowing scroll ancestor of the
    /// focus, per the previous frame's layout (`scrollbar::
    /// scroll_focus_target`). The UA sheet colors that container's
    /// scrollbar thumb through the attribute; `:focus-within` alone
    /// would light every overflowing ancestor (`FOCUS-THUMB-NEAREST-1`).
    /// The accent thumb is a focus indicator, so it shows only while the
    /// focus is evident (`Dom::focus_visible`, `P7-FOCUS-VISIBLE-1`).
    fn mark_scroll_focus(&mut self, dom: &mut TuiDom) {
        let target =
            crate::runtime::scrollbar::scroll_focus_target(dom).filter(|_| dom.focus_visible());
        if target == self.scroll_focus_marked {
            return;
        }
        if let Some(prev) = self.scroll_focus_marked.take()
            && dom.contains(prev)
        {
            dom.remove_attribute(prev, crate::runtime::scrollbar::SCROLL_FOCUS_ATTR)
                .expect("a live element accepts attribute removal");
        }
        if let Some(next) = target {
            dom.set_attribute(next, crate::runtime::scrollbar::SCROLL_FOCUS_ATTR, "")
                .expect("the focused element's ancestor is a live element");
        }
        self.scroll_focus_marked = target;
    }
}
