//! What the next frame of an [`App`](super::App) must redo
//! (`P7G-PAINT-ONLY-FRAME-1`).
//!
//! The frame pipeline is cascade → layout → paint, and each stage only
//! has to rerun when something it reads changed. [`Redraw`] names the
//! least stage that must rerun; each level includes the ones after it.
//! The dirty tracker's subtree roots are cascaded on any frame that has
//! them, whatever the level: they are the cascade work the tracker
//! could see, and `Redraw::Cascade` is for the work it could not.

/// How much of the frame pipeline the next frame reruns, least to most.
/// Sources raise it with [`Redraw::note`]; the frame resets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum Redraw {
    /// Nothing to draw (beyond the dirty tracker's roots, if any).
    #[default]
    Clean,
    /// Repaint from the current layout and computed styles: only
    /// paint-time state changed — a caret-blink flip, a selection or
    /// caret move made by code (the dirty tracker's selection flag).
    Paint,
    /// Lay out and repaint: state layout reads changed without
    /// changing any selector match or declaration — scroll offsets
    /// (layout places children after scroll), text content (the dirty
    /// tracker queues the elements whose selector state text feeds),
    /// hover and focus moves (their restyles are tracker roots), a
    /// mouse route's own work (`RouteOutcome::redraw_requested`
    /// without `cascade_requested`: wheel, scrollbar press, drag).
    /// (Running transitions and animations ask for `Paint`: their frame
    /// composites them and lays out only when they moved geometry.)
    Layout,
    /// Cascade the whole tree, lay out and repaint: something the dirty
    /// tracker cannot see may have changed the cascade — the
    /// stylesheets (`App::invalidate_cascade`), the viewport, the first
    /// frame, a listener's `request_redraw` (which may follow a direct
    /// `TuiExt` style write that no mutation reports). A mouse route's
    /// own work is `Layout`: `RouteOutcome::cascade_requested` is set
    /// only by a listener.
    Cascade,
}

impl Redraw {
    /// Raise the level to at least `need`.
    pub(crate) fn note(&mut self, need: Redraw) {
        if need > *self {
            *self = need;
        }
    }

    /// [`note`](Self::note) `need` when `cond` holds.
    pub(crate) fn note_if(&mut self, cond: bool, need: Redraw) {
        if cond {
            self.note(need);
        }
    }
}

/// Which stages the frames drawn since the last
/// `App::take_frame_stats` ran — test instrumentation for the
/// frame-work contract.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FrameStats {
    /// Whole-tree cascades.
    pub(crate) full_cascades: u32,
    /// Cascades of the dirty tracker's subtree roots.
    pub(crate) subtree_cascades: u32,
    /// Frames (or off-frame passes) that ran layout.
    pub(crate) layouts: u32,
    /// Frames painted.
    pub(crate) paints: u32,
    /// Element styles the transition engine composited.
    pub(crate) composites: u32,
    /// Whole-tree walks the frame's pre-cascade checks and post-paint
    /// bookkeeping made (validity marks, scroll offsets moved since
    /// paint, smooth scrolls in flight, offsets noted as painted).
    pub(crate) walks: u32,
}
