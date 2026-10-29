//! The layout flush (CSSOM View §5, HTML §6.6.3 "focusing steps"): a
//! browser brings style and layout up to date before it answers a call
//! that reads geometry — `scrollIntoView()`, `focus()`'s scroll — so code
//! that creates or moves an element and scrolls to it in one handler
//! works, and reads the scroll it made in the same handler.
//!
//! An [`App`](crate::App) records each layout it makes on its document:
//! the area it laid out in and how many mutation records its dirty
//! tracker had seen (`note_laid_out`). [`flush_layout`] brings it up to
//! date as the frame would: first the DOM-derived state the frame's
//! prelude keeps current — control seeding, `<select>` selectedness, the
//! scroll-focus marker, the validity marks (`runtime::derived_state`) —
//! then, when a record arrived since or a subtree is dirty, it cascades
//! every dirty subtree under the sheets the `App` published
//! (`runtime::style_flush`), noting them for the frame's transition hook,
//! and lays the whole tree out in the recorded area. The frame after a flush runs everything a
//! layout starts — re-snaps, layout transitions, the focus fixup, scroll
//! timelines — and reuses the flush's layout when nothing changed since
//! (no record, no restyle of its own, no animation moving what layout
//! reads, the same area), so a flush adds no layout to a frame that
//! needed one anyway.
//!
//! What a flush cannot bring up to date, so it says so
//! ([`LayoutFlush::Stale`]) and the scroll a caller asked for waits for
//! the `App`'s next layout (`pending`): before the `App`'s first frame;
//! after a resize or a change of its sheet set (`invalidate`); after a
//! change of the document's environment — its color scheme, media
//! preferences or color depth, which the frame restyles the whole tree
//! for with no mutation the dirty tracker sees; and while a `<style>`
//! sheet waits to be re-parsed (`cssom::style_elements::sheet_pending`). A
//! terminal resized with no resize event is laid out in the old area
//! until the frame (whose `Terminal::autoresize` sees it).
//!
//! The frame's other pre-style work is not a flush's: `matchMedia` lists
//! report their flips only at a frame (a flip needs an environment change
//! or a resize, where a flush is stale), and the animations of the
//! elements removed since are cancelled at the frame, which lays out
//! again when that resets a computed style (`app::frame`). Changes made
//! around the DOM's mutation API — a direct `TuiExt` write, signalled
//! with `request_redraw` — are not seen by a flush; the frame restyles
//! for them.
//!
//! Readers that borrow the document — `scroll_top()`, `layout_rect()` —
//! read the last layout; call [`flush_layout`] first to bring it up to
//! date. On a document no `App` runs nothing is recorded and a flush does
//! nothing ([`LayoutFlush::Unmanaged`]): the caller cascades and lays it
//! out.

pub(crate) mod pending;

use rdom_style::color::{ColorDepth, ColorScheme};
use rdom_style::conditional::MediaPreferences;

use crate::TuiDom;
use crate::render::{LayoutExt, Rect};
use crate::runtime::derived_state;
use crate::style::cascade::cascade_subtrees_all_with;
use crate::style::dirty_tracker::DirtyTracker;
use crate::style::{CascadeExt, Stylesheet};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod waiting_tests;

/// What [`flush_layout`] found: whether geometry read after it —
/// `layout_rect()`, `scroll_top()`, `bounding_rect()` — is the tree's as it
/// is now.
///
/// `#[non_exhaustive]`: a later reason a layout cannot be brought up to
/// date may be told apart; [`LayoutFlush::is_current`] is the question
/// most callers ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayoutFlush {
    /// The layout was up to date: nothing it reads changed since it was
    /// made.
    Current,
    /// The layout was brought up to date.
    LaidOut,
    /// The layout could not be brought up to date before the `App`'s next
    /// frame — before its first, after a resize, a sheet-set change or a
    /// change of the document's environment (its color scheme, media
    /// preferences or color depth), while a `<style>` sheet waits to be
    /// re-parsed: geometry read now is the last layout's.
    Stale,
    /// No `App` runs the document: its caller cascades and lays it out
    /// (`CascadeExt::cascade`, `LayoutExt::layout_dom`).
    Unmanaged,
}

impl LayoutFlush {
    /// Whether the layout is the tree's as it is now ([`Self::Current`]
    /// or [`Self::LaidOut`]).
    pub fn is_current(self) -> bool {
        matches!(self, LayoutFlush::Current | LayoutFlush::LaidOut)
    }
}

/// The `App`'s last layout of the document: the area, the tracker's
/// record count and the document's environment when it was made
/// (document data).
struct LaidOut {
    area: Rect,
    records: u64,
    environment: Environment,
}

/// What a cascade reads of the document's environment besides its tree
/// and sheets: a change restyles the whole tree at the `App`'s next frame,
/// with no mutation the dirty tracker sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Environment {
    scheme: ColorScheme,
    media: MediaPreferences,
    depth: ColorDepth,
}

fn environment(dom: &TuiDom) -> Environment {
    Environment {
        scheme: dom.color_scheme(),
        media: dom.media_preferences(),
        depth: crate::style::cascade::document_color_depth(dom),
    }
}

/// A flush laid the document out since the frame last took this
/// (document data): the frame lays out again, for its post-layout work.
struct FlushedLayout;

/// Record a layout the `App` made in `area` (the frame, after its layout
/// and the scrolls that follow it).
pub(crate) fn note_laid_out(dom: &mut TuiDom, area: Rect) {
    let Some(records) = records_seen(dom) else {
        return;
    };
    let environment = environment(dom);
    dom.set_document_data(LaidOut {
        area,
        records,
        environment,
    });
}

/// Forget the recorded layout: its area or its sheets are stale (a
/// resize, a sheet-set change), so a flush declines until the next frame.
pub(crate) fn invalidate(dom: &mut TuiDom) {
    dom.remove_document_data::<LaidOut>();
}

/// Whether a flush laid the document out since the last call (the frame
/// asks, to lay out again).
pub(crate) fn take_flushed(dom: &mut TuiDom) -> bool {
    dom.remove_document_data::<FlushedLayout>().is_some()
}

/// Whether the recorded layout matches the tree: one was made, no
/// mutation record has arrived since, and the environment is the same.
fn is_current(dom: &TuiDom) -> bool {
    match (dom.document_data::<LaidOut>(), records_seen(dom)) {
        (Some(laid), Some(now)) => laid.records == now && laid.environment == environment(dom),
        _ => false,
    }
}

/// Whether the recorded layout is current and was made in `area` (the
/// frame, which then runs its post-layout work on it without laying out).
pub(crate) fn is_current_in(dom: &TuiDom, area: Rect) -> bool {
    dom.document_data::<LaidOut>()
        .is_some_and(|l| l.area == area)
        && is_current(dom)
}

/// The record count of the dirty tracker the `App` running `dom`
/// published, or `None` on a document no `App` runs.
fn records_seen(dom: &TuiDom) -> Option<u64> {
    crate::runtime::style_flush::published_tracker(dom).map(DirtyTracker::records_seen)
}

/// Bring the layout of the document an `App` runs up to date (CSSOM
/// View's "update the layout"), so geometry read after it —
/// `layout_rect()`, `scroll_top()`, `bounding_rect()` — reflects every
/// change made since: bring the DOM-derived state up to date, cascade
/// every dirty subtree under the `App`'s sheets and lay the tree out in
/// the area of its last frame. Returns what it found ([`LayoutFlush`]):
/// [`is_current`](LayoutFlush::is_current) says whether the geometry is
/// the tree's as it is now.
///
/// `focus()` and `scroll_into_view()` call it before they scroll. Each
/// call brings the derived state up to date — seeding the controls
/// inserted since, settling `<select>`s and popovers (their listeners run
/// in it), and a whole-tree walk when a sheet reads `:valid` /
/// `:invalid` — and each flush that lays out is a whole-tree layout; the
/// next frame lays out again only if something changed since (it reuses
/// the flush's layout otherwise). Interleaving changes and flushes in a
/// loop lays out every time, as a browser's forced layout does.
pub fn flush_layout(dom: &mut TuiDom) -> LayoutFlush {
    let Some(inputs) = crate::runtime::style_flush::published(dom) else {
        return LayoutFlush::Unmanaged;
    };
    let Some(tracker) = inputs.tracker else {
        return LayoutFlush::Unmanaged;
    };
    let Some((area, recorded)) = dom
        .document_data::<LaidOut>()
        .map(|l| (l.area, l.environment))
    else {
        return LayoutFlush::Stale;
    };
    // The derived state first, in the frame prelude's order: what its
    // stages write is cascaded here, and the frame finds it current.
    // Stages 1–2 run listeners; the sheets and environment are checked
    // after them, where the frame re-parses its `<style>` sheets.
    derived_state::settle_controls(dom);
    if crate::cssom::style_elements::sheet_pending(dom) || recorded != environment(dom) {
        return LayoutFlush::Stale;
    }
    let sheets: Vec<&Stylesheet> = inputs.sheets.iter().map(|s| &**s).collect();
    derived_state::mark_scroll_focus(dom);
    derived_state::mark_validity(dom, &tracker, sheets.iter().copied());
    let mut roots = tracker.take_roots();
    // The hosts whose hovered or active pseudo-element changed: the
    // tracker sees no mutation for them (`app::frame::take_dirty_roots`).
    roots.extend(crate::style::pseudo_pointer::take_dirty(dom));
    roots.sort_unstable();
    roots.dedup();
    if roots.is_empty() && is_current(dom) {
        return LayoutFlush::Current;
    }
    if !roots.is_empty() {
        let cascaded = cascade_subtrees_all_with(dom, &sheets, Some(inputs.registry), &roots);
        tracker.note_flushed(&cascaded);
    }
    dom.layout_dom(area);
    note_laid_out(dom, area);
    dom.set_document_data(FlushedLayout);
    LayoutFlush::LaidOut
}
