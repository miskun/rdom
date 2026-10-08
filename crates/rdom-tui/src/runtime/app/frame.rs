//! The frame of an [`App`]: [`App::draw_if_dirty`] (the frame prelude —
//! `prelude::FramePrelude`, the pre-cascade stages in order — then
//! cascade + animation step + layout + caret-reveal servicing + paint),
//! the off-frame [`App::cascade_and_layout`] half used by the autoscroll
//! tick, and the transition-event drain that follows a painted frame.
//!
//! Each frame reruns only the stages its causes need (`redraw::Redraw`,
//! `P7G-PAINT-ONLY-FRAME-1`): a caret-blink flip only paints, a scroll
//! offset change lays out and paints, the dirty tracker's roots are
//! cascaded as subtrees, and only what the tracker cannot see
//! (stylesheet changes, resizes, `request_redraw`) cascades the whole
//! tree.

use std::io;
use std::rc::Rc;

use rdom_core::NodeId;

use super::App;
use super::prelude::{PreludeCx, PreludeRun};
use super::redraw::Redraw;
use crate::TuiDom;
use crate::render::backend::Backend;
use crate::render::{LayoutExt, PaintExt, Rect};
use crate::runtime::animation::AnimationRegistry;
use crate::style::CascadeExt;
use crate::style::Stylesheet;
use crate::style::cascade::{
    PropertyRegistry, cascade_all_with, cascade_subtrees_all_with, restyle_vars,
};
use rdom_style::calc::Viewport;

impl<B: Backend> App<B> {
    /// The time a frame runs at: the wall clock, which the scheduler's
    /// clock follows, unless [`App::advance`] drives the clock.
    fn frame_now(&mut self) -> std::time::Instant {
        if !self.virtual_clock {
            self.scheduler
                .borrow_mut()
                .set_now(std::time::Instant::now());
        }
        self.scheduler.borrow().now()
    }

    /// Run the frame prelude (`prelude::FramePrelude::run`: the
    /// pre-cascade stages, in order) against this App.
    fn run_prelude(&mut self, run: PreludeRun) {
        let now = self.scheduler.borrow().now();
        let mut cx = PreludeCx {
            dom: &mut self.dom,
            tracker: &self.tracker,
            app_sheets: &self.stylesheets,
            redraw: &mut self.redraw,
            now,
            animation_frame: std::time::Duration::from_millis(u64::from(self.animation_frame_ms)),
        };
        let walks = self.prelude.run(&mut cx, run);
        self.note_walks(walks);
    }

    /// Count whole-tree walks (test instrumentation).
    #[cfg_attr(not(test), allow(unused_variables, clippy::unused_self))]
    fn note_walks(&mut self, walks: u32) {
        #[cfg(test)]
        {
            self.frame_stats.walks += walks;
        }
    }

    /// Cascade + layout + paint if anything is dirty. Pairs with
    /// [`Self::handle_event`] for apps running a custom event loop.
    pub fn draw_if_dirty(&mut self) -> io::Result<()> {
        // Animation events (`transitionend`) fire from in here; their
        // listeners may schedule timers.
        let _current = crate::runtime::timers::SchedulerGuard::install(&self.scheduler);
        self.service_animation_clock();
        // Before the roots are taken, so what the stages dirty is
        // cascaded in this frame.
        self.run_prelude(PreludeRun::Frame);
        // `matchMedia` lists report their flips before the frame's style;
        // their listeners' changes are cascaded in this frame.
        self.report_media_changes();
        let dirty_roots = self.take_dirty_roots();
        let flushed = self.take_flushed();
        self.detach_removed();
        let redraw = self.redraw;

        if redraw == Redraw::Clean && dirty_roots.is_empty() {
            crate::rdom_trace!("draw_if_dirty: SKIP (clean, dirty_roots empty)");
            return Ok(());
        }
        crate::rdom_trace!("draw_if_dirty: DRAW (redraw={redraw:?}, dirty_roots={dirty_roots:?})");

        // The transitions run on the app's clock (`frame_now`).
        let now = self.frame_now();
        let dom = &mut self.dom;
        let sheets = self.prelude.cascade_order(&self.stylesheets);
        let registry = &self.prelude.registry;
        let cascaded_viewport = &mut self.cascaded_viewport;
        let animations = &mut self.animations;
        let mut pass = Pass::default();
        self.terminal.draw(|buf| {
            pass = style_and_layout(
                dom,
                (&sheets, registry),
                (animations, now),
                (redraw, cascaded_viewport),
                (&dirty_roots, &flushed),
                buf.area,
            );
            dom.paint_dom(buf, buf.area);
            Ok(())
        })?;
        self.note_pass(pass, true);
        self.redraw = Redraw::Clean;
        // The element under a still pointer, or its style, may have
        // changed (CSS UI 4 §4.1).
        self.update_pointer_shape();
        let walks = self.prelude.after_paint(&mut self.dom, pass.laid_out);
        self.note_walks(walks);
        // HTML "update the rendering": the focus fixup, against this
        // frame's used styles — after any frame that could change them: a
        // cascade, or a running transition stepped (one ending `hidden`
        // changes the used `visibility` with no cascade, C7G-FOCUS-FIXUP).
        // Its `blur` listeners are code the next frame's checks must see.
        if pass.laid_out && crate::runtime::focus::fix_up(&mut self.dom) {
            self.prelude.touched = true;
        }

        // The `scroll` events of the frame's re-snaps (CSS Scroll Snap 1
        // §5.4), queued between layout and paint: HTML fires them at the
        // rendering update, never in the middle of one.
        self.prelude.touched |= crate::runtime::scrollbar::fire_queued_scroll_events(&mut self.dom);
        // Drain transition and animation events queued during this frame.
        self.dispatch_animation_events();

        // Run the next frame too while a transition or an animation is
        // still moving: it composites them, and lays out only when a
        // longhand layout reads moved (`style_and_layout`).
        self.redraw
            .note_if(self.animations.needs_frames(now), Redraw::Paint);
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
        // The hosts whose hovered or active pseudo-element changed
        // (`::before:hover`): the tracker sees no DOM mutation for them.
        dirty_roots.extend(crate::style::pseudo_pointer::take_dirty(&mut self.dom));
        dirty_roots.sort_unstable();
        dirty_roots.dedup();
        dirty_roots
    }

    /// What the animations' clock calls for at the start of a frame
    /// (C12G-FRAME-COST): an animation with an empty effect whose event is
    /// due is stepped and its events dispatched, with no frame; a frame is
    /// noted when a transition or animation is due to move — every frame
    /// for a continuous one, a stepped one only at its steps.
    fn service_animation_clock(&mut self) {
        let now = self.scheduler.borrow().now();
        if self.animations.step_events(&self.dom, now) {
            self.dispatch_animation_events();
        }
        self.redraw
            .note_if(self.animations.needs_frames(now), Redraw::Paint);
    }

    /// Cancel the transitions and animations of the elements removed
    /// since the last frame and forget their before-change styles
    /// (C12G-DETACHED) — before the cascade, so one inserted again is
    /// rendered afresh. The cancel events want a frame to go out in.
    fn detach_removed(&mut self) {
        let removed = self.tracker.take_detached();
        if removed.is_empty() {
            return;
        }
        let running = !self.animations.is_empty();
        let now = self.frame_now();
        self.animations.detach(&mut self.dom, &removed, now);
        self.redraw.note_if(running, Redraw::Paint);
    }

    /// Whether a style flush cascaded subtrees since the last frame
    /// (`runtime::style_flush`): the frame lays out (noted on `redraw`)
    /// and runs its transition hook for them.
    fn take_flushed(&mut self) -> Vec<NodeId> {
        let flushed = self.tracker.take_flushed();
        self.redraw.note_if(!flushed.is_empty(), Redraw::Layout);
        flushed
    }

    /// The animations running on `node` and its pseudo-elements at the
    /// app's clock — `Element.getAnimations()` (Web Animations 1 §6.7):
    /// its transitions, then its CSS animations in composite order, each
    /// with its name, play state and current time.
    pub fn get_animations(&self, node: NodeId) -> Vec<crate::runtime::animation::AnimationInfo> {
        let now = self.scheduler.borrow().now();
        self.animations.animations_of(node, now)
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
        self.run_prelude(PreludeRun::OffFrame);
        let dirty_roots = self.take_dirty_roots();
        let flushed = self.take_flushed();
        self.detach_removed();
        let redraw = self.redraw.max(Redraw::Layout);
        let now = self.frame_now();
        let sheets = self.prelude.cascade_order(&self.stylesheets);
        let pass = style_and_layout(
            &mut self.dom,
            (&sheets, &self.prelude.registry),
            (&mut self.animations, now),
            (redraw, &mut self.cascaded_viewport),
            (&dirty_roots, &flushed),
            area,
        );
        self.note_pass(pass, false);
        self.prelude.touched |= crate::runtime::scrollbar::fire_queued_scroll_events(&mut self.dom);
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
            stats.composites += pass.composites;
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
    /// Element styles the transition engine composited.
    composites: u32,
}

/// Carry the animated values the last step moved to where the cascade
/// takes them: a registered custom property's to its `var()` consumers,
/// an inherited longhand's to the element's descendants. `true` when
/// something was restyled.
fn restyle_animated(
    dom: &mut TuiDom,
    (sheets, registry): (&[&Stylesheet], &Rc<PropertyRegistry>),
    animations: &mut AnimationRegistry,
) -> bool {
    let restyle = animations.take_restyle();
    if restyle.is_empty() {
        return false;
    }
    // No selector can see the change: reuse the matches.
    let restyled = restyle_vars(dom, sheets, registry.clone(), &restyle);
    // The animated result is the before-change style of the next style
    // change (CSS Transitions 1 §3) — for the elements whose counters it
    // moved too.
    crate::runtime::animation::settle_restyled(dom, &restyled);
    true
}

/// The frame pipeline up to paint, shared by [`App::draw_if_dirty`] and
/// [`App::cascade_and_layout`], each stage only when `redraw` or the
/// dirty roots need it (`redraw::Redraw`, `P7G-PAINT-ONLY-FRAME-1`):
/// cascade (the whole tree for `Redraw::Cascade`, else `dirty_roots`'
/// subtrees, else nothing) → register the transitions a cascade's
/// property changes start — or a style flush's since the last frame
/// (`flushed`, `runtime::style_flush`) → when anything was cascaded or `redraw` is
/// at least `Layout`: advance the running transitions (compositing their
/// values onto the animated styles), lay out, and
/// service a caret reveal requested this frame against the fresh
/// extent, re-laying out when it moved a scroll offset. A
/// `Redraw::Paint` frame with no dirty roots runs none of it.
///
/// A free function over split borrows because `draw_if_dirty` runs it
/// inside `Terminal::draw` while the terminal is borrowed.
fn style_and_layout(
    dom: &mut TuiDom,
    (sheets, registry): (&[&Stylesheet], &Rc<PropertyRegistry>),
    (animations, now): (&mut AnimationRegistry, std::time::Instant),
    (redraw, cascaded_viewport): (Redraw, &mut Option<Viewport>),
    (dirty_roots, flushed): (&[NodeId], &[NodeId]),
    area: Rect,
) -> Pass {
    // The viewport-percentage units resolve against the terminal (CSS
    // Values 4 §6.1.2) and `@media` reads its size (Media Queries 4 §4):
    // at a new size the whole tree cascades when a style read the viewport
    // or a query flipped, and otherwise only lays out again.
    let viewport = Viewport::new(area.width, area.height);
    dom.set_viewport(viewport);
    let redraw = if *cascaded_viewport == Some(viewport) {
        redraw
    } else if cascaded_viewport.is_none()
        || crate::style::cascade::must_restyle(dom, sheets, registry, true)
    {
        Redraw::Cascade
    } else {
        redraw.max(Redraw::Layout)
    };
    *cascaded_viewport = Some(viewport);
    // The cascade, then the transition and animation hook under its sheets.
    let mut cascaded: Vec<NodeId> = Vec::new();
    let cascade = if redraw == Redraw::Cascade {
        cascade_all_with(dom, sheets, Some(registry.clone()));
        Some(CascadeScope::Full)
    } else if !dirty_roots.is_empty() {
        cascaded = cascade_subtrees_all_with(dom, sheets, Some(registry.clone()), dirty_roots);
        Some(CascadeScope::Subtrees)
    } else {
        None
    };
    let flushed_any = !flushed.is_empty();
    if cascade.is_some() || flushed_any {
        animations.set_registered_properties(registry.clone());
        // A newly rendered element's transitions start from its starting
        // style (CSS Transitions 2 §3, `@starting-style`); its CSS
        // animations follow its `animation-*` lists (CSS Animations 1 §4).
        // The hook visits what was cascaded: the whole tree, or the dirty
        // and flushed subtrees (C12G-FRAME-COST).
        let inputs = crate::runtime::animation::CssInputs { sheets, registry };
        let scope: Option<Vec<NodeId>> = (cascade != Some(CascadeScope::Full))
            .then(|| cascaded.iter().chain(flushed).copied().collect());
        crate::runtime::animation::diff_and_register_in(
            dom,
            animations,
            now,
            inputs,
            scope.as_deref(),
        );
    }
    let must_lay_out = cascade.is_some() || flushed_any || redraw >= Redraw::Layout;
    // A frame the animation pump asked for (`Redraw::Paint`) steps the
    // transitions and animations — the one that ends them too — and lays
    // out only when they moved a longhand layout reads. Stepping one that
    // did not move composites nothing.
    let animate = must_lay_out || (redraw >= Redraw::Paint && !animations.is_empty());
    let mut laid_out = must_lay_out;
    let mut composites = 0;
    if animate {
        let advanced = animations.advance_frame(dom, now);
        composites = advanced.composites;
        laid_out |= advanced.layout;
        // The elements waiting to leave the top layer leave once their
        // `overlay` is not `auto` (CSS Position 4 §3.3); the removal
        // re-cascades them next frame (its mutation dirties them).
        crate::runtime::top_layer::finish_removals(dom);
        restyle_animated(dom, (sheets, registry), animations);
    }
    if laid_out {
        dom.layout_dom(area);
        // Against this layout, each correcting for the offsets moved since
        // it (`scrollbar::state::laid_out`), then one relayout for all:
        // CSS Scroll Snap 1 §5.4 — a snap container whose snap target moved
        // re-snaps to it; HTML's focusing steps — a newly focused element
        // scrolls into view against the layout it is shown in; a caret
        // edited out of its box's view is revealed.
        let resnapped = crate::runtime::scroll_snap::resnap(dom);
        let focused = crate::runtime::focus::service_focus_scroll(dom);
        let revealed = crate::runtime::scrollbar::service_caret_reveal(dom);
        // Scroll-driven Animations 1 §5: a scroll or view timeline this
        // layout or those scrolls moved is stale — step it again (the
        // timelines read the offsets as they are now, against this layout),
        // and share the one relayout: a frame lays out at most twice.
        let mut restepped = false;
        if animations.has_progress_timelines() {
            let again = animations.restep_progress(dom, now);
            composites += again.composites;
            // An inherited value it moved reaches the descendants in this
            // frame, not the next (C12G-MISC).
            restepped = again.layout || restyle_animated(dom, (sheets, registry), animations);
        }
        if resnapped || focused || revealed || restepped {
            dom.layout_dom(area);
        }
    }
    Pass {
        cascade,
        laid_out,
        composites,
    }
}
