//! The layout flush (CSSOM View §5, HTML §6.6.3 "focusing steps"): a
//! browser brings style and layout up to date before it answers a call
//! that reads geometry — `scrollIntoView()`, `focus()`'s scroll — so code
//! that creates or moves an element and scrolls to it in one handler
//! works, and reads the scroll it made in the same handler.
//!
//! An [`App`](crate::App) records each layout it makes on its document:
//! the area it laid out in and how many mutation records its dirty
//! tracker had seen (`note_laid_out`). Between frames the layout is
//! current while no record has arrived since. [`flush_layout`] brings a
//! stale one up to date as the frame would: it cascades every dirty
//! subtree under the sheets the `App` published (`runtime::style_flush`),
//! noting them for the frame's transition hook, and lays the whole tree
//! out in the recorded area. The frame after a flush runs everything a
//! layout starts — re-snaps, layout transitions, the focus fixup, scroll
//! timelines — and reuses the flush's layout when nothing changed since
//! (no record, no restyle of its own, no animation moving what layout
//! reads, the same area), so a flush adds no layout to a frame that
//! needed one anyway.
//!
//! What a flush cannot bring up to date, so it declines and the scroll a
//! caller asked for waits for the `App`'s next layout (`pending`):
//!
//! - **before the `App`'s first frame** — there is no layout yet;
//! - **after a resize or a change of the sheet set** — the area or the
//!   sheets the record holds are stale (`invalidate`), and the frame
//!   decides what to restyle.
//!
//! Not flushed, as with a style flush: `<style>` text edited since the
//! last frame, and a whole-tree restyle a colour-scheme change asks for.
//! Readers that borrow the document — `scroll_top()`, `layout_rect()` —
//! read the last layout; call [`flush_layout`] first to bring it up to
//! date. On a document no `App` runs nothing is recorded and a flush
//! does nothing: the caller cascades and lays it out.

pub(crate) mod pending;

use crate::TuiDom;
use crate::render::{LayoutExt, Rect};
use crate::style::Stylesheet;
use crate::style::cascade::cascade_subtrees_all_with;

#[cfg(test)]
mod tests;

/// The `App`'s last layout of the document: the area, and the tracker's
/// record count when it was made (document data).
struct LaidOut {
    area: Rect,
    records: u64,
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
    dom.set_document_data(LaidOut { area, records });
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

/// Whether the recorded layout matches the tree: one was made, and no
/// mutation record has arrived since.
fn is_current(dom: &TuiDom) -> bool {
    match (dom.document_data::<LaidOut>(), records_seen(dom)) {
        (Some(laid), Some(now)) => laid.records == now,
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
    crate::runtime::style_flush::published(dom)?
        .tracker
        .map(|t| t.records_seen())
}

/// Bring the layout of the document an `App` runs up to date (CSSOM
/// View's "update the layout"): cascade every dirty subtree under the
/// `App`'s sheets and lay the tree out in the area of its last frame, so
/// geometry read after it — `layout_rect()`, `scroll_top()`,
/// `scroll_height()` — reflects every change made since. Returns whether
/// it laid out: `false` when the layout is current already, on a
/// document no `App` runs, and when it cannot be brought up to date
/// before the next frame (before the first, after a resize or a sheet-set
/// change; module doc).
///
/// `focus()` and `scroll_into_view()` call it before they scroll. Each
/// flush that lays out is a whole-tree layout; the next frame lays out
/// again only if something changed since (it reuses the flush's
/// layout otherwise): interleaving changes and flushes in a loop lays
/// out every time, as a browser's forced layout does.
pub fn flush_layout(dom: &mut TuiDom) -> bool {
    let Some(area) = dom.document_data::<LaidOut>().map(|l| l.area) else {
        return false;
    };
    if is_current(dom) {
        return false;
    }
    let Some(inputs) = crate::runtime::style_flush::published(dom) else {
        return false;
    };
    let Some(tracker) = inputs.tracker else {
        return false;
    };
    let mut roots = tracker.take_roots();
    // The hosts whose hovered or active pseudo-element changed: the
    // tracker sees no mutation for them (`app::frame::take_dirty_roots`).
    roots.extend(crate::style::pseudo_pointer::take_dirty(dom));
    roots.sort_unstable();
    roots.dedup();
    if !roots.is_empty() {
        let sheets: Vec<&Stylesheet> = inputs.sheets.iter().map(|s| &**s).collect();
        let cascaded = cascade_subtrees_all_with(dom, &sheets, Some(inputs.registry), &roots);
        tracker.note_flushed(&cascaded);
    }
    dom.layout_dom(area);
    note_laid_out(dom, area);
    dom.set_document_data(FlushedLayout);
    true
}

/// Make the layout current for a scroll an `App`'s document asks for:
/// `true` when it is (current already, or flushed), `false` when the
/// scroll must wait for the next frame.
pub(crate) fn make_current(dom: &mut TuiDom) -> bool {
    flush_layout(dom);
    is_current(dom)
}
