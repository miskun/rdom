//! The frame prelude of an [`App`](super::App): the stages a frame runs
//! before its cascade, in order, and the state they own
//! (`P7G-FRAME-PRELUDE-1`).
//!
//! [`FramePrelude::run`] is the one place the order is written:
//!
//! 1. **control seeding** — give the text controls inserted since the
//!    last boundary their text node (`input::ControlSeeding`,
//!    `INPUT-SEED-ON-INSERT-1`), so this frame lays them out with it;
//! 2. **`<select>` selectedness** — settle the selects whose options
//!    changed (`select::Selectedness`), so the options it (re)selects
//!    cascade this frame; hide the popovers whose `popover` attribute
//!    changed state while showing (`popover::attribute`);
//! 3. **`<style>` elements** — re-parse the sheets whose text changed
//!    (`cssom::style_elements`); a change invalidates the cascade
//!    ([`FramePrelude::sheets_changed`]);
//! 4. **scroll-focus marker** — move `data-rdom-scroll-focus` to the
//!    container the keyboard scrolls (frames only);
//! 5. **validity marks** — dirty the elements whose `:valid` /
//!    `:invalid` or a radio group's `:indeterminate` flipped
//!    (`validation::FormStateMarks`);
//! 6. **caret blink** — flip the caret phase (a paint-only change);
//! 7. **smooth scrolls** — step the scrolls in flight (frames only);
//! 8. **painted check** — any scroll offset moved since the last paint
//!    lays out and repaints (frames only);
//! 9. **tracker flags** — a text change the dirty tracker saw lays out
//!    and repaints, a selection change repaints (frames only; the
//!    off-frame run leaves them for the next frame);
//! 10. **painted highlights** — the highlight registry's generation is
//!     not the one last painted (a change no record reported) repaints.
//!
//! Stages 5, 7 and 8 walk the whole tree, so a frame runs them only when
//! code ran since the last frame ([`FramePrelude::touched`],
//! `P7G-IDLE-WALKS-1`) — and 7 also while a smooth scroll is in flight.
//! After the paint, [`FramePrelude::after_paint`] records the painted
//! offsets stage 8 compares against and the highlight generation stage 10
//! does.

use std::rc::Rc;
use std::time::{Duration, Instant};

use rdom_core::NodeId;

use super::StylesheetId;
use super::redraw::Redraw;
use crate::TuiDom;
use crate::cssom::style_elements::StyleElements;
use crate::runtime::builtins::input::ControlSeeding;
use crate::runtime::builtins::popover::attribute::PopoverAttributes;
use crate::runtime::builtins::select::Selectedness;
use crate::runtime::builtins::validation::FormStateMarks;
use crate::runtime::caret_blink::CaretBlink;
use crate::style::Stylesheet;
use crate::style::dirty_tracker::DirtyTracker;

/// Which run of the prelude: a drawn frame's, or the off-frame cascade
/// and layout of the autoscroll tick (`App::cascade_and_layout`), which
/// runs stages 1, 2, 3, 5 (always) and 6 only.
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
    pub(super) app_sheets: &'a [(StylesheetId, Rc<Stylesheet>)],
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
    /// Seeds the text controls inserted after `App::build`; flushed
    /// before each event and each frame.
    pub(super) control_seeding: ControlSeeding,
    /// Runs the `<select>` selectedness setting algorithm on selects
    /// whose options were inserted / removed; flushed before each event
    /// and each frame.
    pub(super) selectedness: Selectedness,
    /// Hides the popovers whose `popover` attribute changed state while
    /// showing; flushed before each event and each frame.
    pub(super) popover_attributes: PopoverAttributes,
    /// The document's `<style>` sheets, live (`cssom::style_elements`):
    /// they cascade before the App's own sheets, in tree order.
    pub(super) style_elements: StyleElements,
    /// `CSS.registerProperty` registrations (`App::register_property`),
    /// cascaded after every other sheet so they win over `@property`.
    pub(super) registrations: std::rc::Rc<crate::style::Stylesheet>,
    /// The custom properties every sheet registers ("later wins"), the
    /// one registry the cascade and the transition engine share; rebuilt
    /// when the sheets change ([`Self::sheets_changed`]).
    pub(super) registry: std::rc::Rc<crate::style::cascade::PropertyRegistry>,
    /// The element currently carrying `data-rdom-scroll-focus`.
    scroll_focus_marked: Option<NodeId>,
    /// Each element's `:valid` / `:invalid` state as of the last frame.
    validity_marks: FormStateMarks,
    /// Caret blink phase (`runtime::caret_blink`). Off unless enabled —
    /// `App::new` enables it at the default rate.
    pub(super) caret_blink: CaretBlink,
    /// When the next smooth-scroll step is due (`runtime::smooth_scroll`),
    /// on the scheduler clock; `None` when no scroll container is
    /// animating.
    pub(super) smooth_scroll_next: Option<Instant>,
    /// Something may have changed the tree or runtime-managed `TuiExt`
    /// state (scroll offsets, a custom validity, a user edit) since the
    /// last frame's checks: an input event, a stylesheet change,
    /// `dom_mut()` access, or a timer / microtask / rAF callback, an
    /// injected closure or the tick handler that left evidence of
    /// a change — a mutation record, a runtime-managed state write
    /// (`runtime::state_writes`), a `request_redraw`
    /// (`P7G-TICK-TOUCHED-1`). The whole-tree stages run only then, so
    /// an idle tick — or a tick handler that finds nothing to do — walks
    /// nothing (`P7G-IDLE-WALKS-1`).
    pub(super) touched: bool,
}

impl FramePrelude {
    /// Install the prelude's observers on `dom` (control seeding,
    /// selectedness, `<style>` elements). Starts touched, so the first
    /// frame runs every check.
    pub(super) fn install(dom: &mut TuiDom) -> Self {
        Self {
            control_seeding: ControlSeeding::install(dom),
            selectedness: Selectedness::install(dom),
            popover_attributes: PopoverAttributes::install(dom),
            style_elements: StyleElements::install(dom),
            registrations: std::rc::Rc::new(crate::style::Stylesheet::bare()),
            registry: std::rc::Rc::default(),
            scroll_focus_marked: None,
            validity_marks: FormStateMarks::default(),
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
        self.control_seeding.flush(cx.dom);
        // 2.
        self.selectedness.flush(cx.dom);
        crate::runtime::builtins::select::settle_pickers(cx.dom);
        self.popover_attributes.flush(cx.dom);
        // 3.
        if self.style_elements.flush(cx.dom) {
            self.sheets_changed(cx.dom, cx.tracker, cx.app_sheets, cx.redraw);
        }
        // 4.
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
        // 5.
        if touched {
            let walked = self.validity_marks.flush(
                cx.dom,
                cx.tracker,
                self.style_elements
                    .sheets()
                    .chain(cx.app_sheets.iter().map(|(_, s)| &**s)),
            );
            walks += u32::from(walked);
        }
        // 6. A blink flip only changes what the caret painter reads.
        let flipped = self.caret_blink.update(cx.dom, cx.now);
        cx.redraw.note_if(flipped, Redraw::Paint);
        if !frame {
            return walks;
        }
        // 7. A smooth scroll starts from code (the scroll API, a scroll
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
        // 8. Any scroll offset change repaints, whoever wrote it
        // (`P7-SCROLL-REPAINT-1`), and lays out again. Only code writes
        // offsets between frames.
        if touched {
            let moved = crate::runtime::scrollbar::moved_since_paint(cx.dom);
            walks += 1;
            cx.redraw.note_if(moved, Redraw::Layout);
        }
        // 9. What the dirty tracker recorded beyond its cascade roots
        // (which the frame drains itself), whoever made the change —
        // a listener, a timer, an injected closure, `dom_mut()`
        // (`P7G-OFF-EVENT-PAINT-1`): text lays out and repaints, a
        // selection move repaints. Last, so the stages' own listeners
        // (stage 7's `scroll`) are covered.
        cx.redraw
            .note_if(cx.tracker.take_paint_dirty(), Redraw::Layout);
        cx.redraw
            .note_if(cx.tracker.take_selection_dirty(), Redraw::Paint);
        // 10. The highlight registry moved since the last paint without a
        // record reaching the tracker: a `HighlightsMut` guard dropped
        // while unwinding bumps the generation but may run no observer.
        let generation = cx.dom.highlights().generation();
        let painted = cx.dom.document_data::<PaintedHighlights>().map(|p| p.0);
        cx.redraw
            .note_if(painted.is_some_and(|p| p != generation), Redraw::Paint);
        walks
    }

    /// After a drawn frame: record the painted scroll offsets stage 8
    /// compares against. Only a frame that laid out can have moved one
    /// (layout's clamp, the caret reveal); a paint-only frame draws the
    /// offsets already noted. Returns the walks made.
    pub(super) fn after_paint(&mut self, dom: &mut TuiDom, laid_out: bool) -> u32 {
        let generation = dom.highlights().generation();
        dom.set_document_data(PaintedHighlights(generation));
        if !laid_out {
            return 0;
        }
        crate::runtime::scrollbar::note_painted(dom);
        1
    }

    /// The sheets changed (an App sheet added / removed, a `<style>`
    /// element re-parsed, a property registered): rebuild the property
    /// registry, re-read the sibling-combinator hint, drop the
    /// tracker's roots and cascade the whole tree, refresh the validity
    /// marks' sheet check, and run the whole-tree checks next frame.
    pub(super) fn sheets_changed(
        &mut self,
        dom: &mut TuiDom,
        tracker: &DirtyTracker,
        app_sheets: &[(StylesheetId, Rc<Stylesheet>)],
        redraw: &mut Redraw,
    ) {
        self.sync_sheet_set(dom, tracker, app_sheets);
        tracker.take_roots();
        redraw.note(Redraw::Cascade);
        self.validity_marks.sheets_changed();
        self.touched = true;
    }

    /// Rebuild what is derived from the stylesheet set: the property
    /// registry, the dirty tracker's sibling-combinator hint, whether
    /// the runtime keeps the pseudo-elements' pointer state (a sheet has a
    /// `::before:hover`-like rule, `style::pseudo_pointer`), and the
    /// cascade inputs a style flush reads off the document
    /// (`runtime::style_flush`). Once per stylesheet set (construction,
    /// [`Self::sheets_changed`]).
    pub(super) fn sync_sheet_set(
        &mut self,
        dom: &mut TuiDom,
        tracker: &DirtyTracker,
        app_sheets: &[(StylesheetId, Rc<Stylesheet>)],
    ) {
        let (registry, tracked) = {
            let sheets = self.cascade_order(app_sheets);
            (
                crate::style::cascade::PropertyRegistry::new(&sheets),
                crate::style::pseudo_pointer::sheets_read_it(sheets),
            )
        };
        self.registry = std::rc::Rc::new(registry);
        crate::style::pseudo_pointer::set_tracked(dom, tracked);
        let sheets = self
            .style_elements
            .sheet_handles()
            .chain(app_sheets.iter().map(|(_, s)| s))
            .chain(std::iter::once(&self.registrations))
            .cloned()
            .collect();
        crate::runtime::style_flush::publish(dom, sheets, self.registry.clone(), tracker.clone());
        self.sync_sibling_combinators(tracker, app_sheets);
    }

    /// Tell the dirty tracker which changes the sheets now cascaded can
    /// read through `+` / `~` (`style::sibling_triggers`,
    /// `P7G-SIBLING-MARK-NARROW-1`), so a state change dirties its
    /// siblings only when a selector can read it there. Once per
    /// stylesheet set.
    fn sync_sibling_combinators(
        &self,
        tracker: &DirtyTracker,
        app_sheets: &[(StylesheetId, Rc<Stylesheet>)],
    ) {
        tracker.set_sibling_triggers(crate::style::sibling_triggers::SiblingTriggers::of_sheets(
            self.cascade_order(app_sheets),
        ));
        tracker.set_has_triggers(crate::style::has_triggers::HasTriggers::of_sheets(
            self.cascade_order(app_sheets),
        ));
        tracker.set_column_selectors(
            self.cascade_order(app_sheets)
                .into_iter()
                .any(crate::style::dirty_tracker::uses_column_selectors),
        );
        tracker.set_root_state(
            self.cascade_order(app_sheets)
                .into_iter()
                .any(crate::style::dirty_tracker::uses_root_state),
        );
    }

    /// Every sheet the cascade reads, in cascade order: the document's
    /// `<style>` sheets in tree order, then the App's own in push order
    /// (`cssom::style_elements`), then the rule-less sheet holding the
    /// `App::register_property` registrations. Later sheets win
    /// same-specificity contests.
    pub(super) fn cascade_order<'a>(
        &'a self,
        app_sheets: &'a [(StylesheetId, Rc<Stylesheet>)],
    ) -> Vec<&'a Stylesheet> {
        self.style_elements
            .sheets()
            .chain(app_sheets.iter().map(|(_, s)| &**s))
            .chain(std::iter::once(&*self.registrations))
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

/// The highlight registry generation the last frame painted (document
/// data; stage 10 of the prelude).
struct PaintedHighlights(u64);
