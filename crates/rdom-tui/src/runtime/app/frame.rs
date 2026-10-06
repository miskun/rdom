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
        // Before the roots are taken, so what the stages dirty is
        // cascaded in this frame.
        self.run_prelude(PreludeRun::Frame);
        let dirty_roots = self.take_dirty_roots();
        let redraw = self.redraw;

        if redraw == Redraw::Clean && dirty_roots.is_empty() {
            crate::rdom_trace!("draw_if_dirty: SKIP (clean, dirty_roots empty)");
            return Ok(());
        }
        crate::rdom_trace!("draw_if_dirty: DRAW (redraw={redraw:?}, dirty_roots={dirty_roots:?})");

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
                animations,
                (redraw, cascaded_viewport),
                &dirty_roots,
                buf.area,
            );
            dom.paint_dom(buf, buf.area);
            Ok(())
        })?;
        self.note_pass(pass, true);
        self.redraw = Redraw::Clean;
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
        self.prelude.touched |= !pending.is_empty();
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
            // A listener of an earlier event in the batch may have dropped
            // `node`; a dropped element's transition events go nowhere.
            crate::tui_event::dispatch_event_to_live(&mut self.dom, node, &mut ev);
        }
        // Registered custom properties (`--name`).
        let custom = self.animations.take_pending_custom_events();
        self.prelude.touched |= !custom.is_empty();
        for e in custom {
            let event_name = match e.kind {
                TransitionEventKind::Start => "transitionstart",
                TransitionEventKind::End => "transitionend",
                TransitionEventKind::Cancel => "transitioncancel",
            };
            let mut ev = rdom_core::Event::new(event_name);
            ev.detail = rdom_core::EventDetail::Transition(Box::new(
                rdom_core::TransitionDetail::new(&e.property, e.elapsed_seconds.into(), None),
            ));
            crate::tui_event::dispatch_event_to_live(&mut self.dom, e.node, &mut ev);
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
        self.run_prelude(PreludeRun::OffFrame);
        let dirty_roots = self.take_dirty_roots();
        let redraw = self.redraw.max(Redraw::Layout);
        let sheets = self.prelude.cascade_order(&self.stylesheets);
        let pass = style_and_layout(
            &mut self.dom,
            (&sheets, &self.prelude.registry),
            &mut self.animations,
            (redraw, &mut self.cascaded_viewport),
            &dirty_roots,
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
    (sheets, registry): (&[&Stylesheet], &Rc<PropertyRegistry>),
    animations: &mut AnimationRegistry,
    (redraw, cascaded_viewport): (Redraw, &mut Option<Viewport>),
    dirty_roots: &[NodeId],
    area: Rect,
) -> Pass {
    let now = std::time::Instant::now();
    // The viewport-percentage units resolve against the terminal (CSS
    // Values 4 §6.1.2); at a size the tree was not cascaded for, every
    // element's are stale, so the whole tree cascades.
    let viewport = Viewport::new(area.width, area.height);
    dom.set_viewport(viewport);
    let redraw = if *cascaded_viewport == Some(viewport) {
        redraw
    } else {
        Redraw::Cascade
    };
    let cascade = if redraw == Redraw::Cascade {
        cascade_all_with(dom, sheets, Some(registry.clone()));
        *cascaded_viewport = Some(viewport);
        Some(CascadeScope::Full)
    } else if !dirty_roots.is_empty() {
        cascade_subtrees_all_with(dom, sheets, Some(registry.clone()), dirty_roots);
        Some(CascadeScope::Subtrees)
    } else {
        None
    };
    if cascade.is_some() {
        animations.set_registered_properties(registry.clone());
        crate::runtime::animation::diff_and_register(dom, animations, now);
    }
    let laid_out = cascade.is_some() || redraw >= Redraw::Layout;
    if laid_out {
        animations.advance(dom, now);
        // A registered custom property's animated value reaches its
        // `var()` consumers through the cascade.
        let restyle = animations.take_restyle();
        if !restyle.is_empty() {
            // No selector can see the change: reuse the matches.
            let restyled = restyle_vars(dom, sheets, registry.clone(), &restyle);
            // The animated result is the before-change style of the
            // next style change (CSS Transitions 1 §3) — for the
            // elements whose counters it moved too.
            crate::runtime::animation::settle_restyled(dom, &restyled);
        }
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
        if resnapped || focused || revealed {
            dom.layout_dom(area);
        }
    }
    Pass { cascade, laid_out }
}
