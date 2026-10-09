//! The document's media environment (Media Queries 4 / 5): what `@media`
//! and `matchMedia` evaluate against — the viewport and the preferred
//! color scheme (each its own document data, `viewport.rs` and
//! `scheme.rs`) and the preferences an app reports
//! ([`MediaPreferences`], stored here, a terminal's defaults until set).
//!
//! Also what the cascades leave behind for a change of that environment
//! (`MediaState`): whether a computed style read the viewport — a
//! viewport-percentage length (CSS Values 4 §6.1.2), counted by
//! `rdom_style::calc::viewport_reads` — and the condition results the
//! last cascade ran under. A resize or a new preference restyles the tree
//! only when one of them says it must ([`must_restyle`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rdom_core::Dom;
use rdom_style::Stylesheet;
use rdom_style::conditional::{MediaEnvironment, MediaPreferences};

use super::conditions::ConditionResults;
use super::registered::PropertyRegistry;
use super::sheets::Sheets;
use crate::ext::TuiExt;

/// The document-data slot holding the preferences.
#[derive(Debug, Clone, Copy)]
struct DocumentMediaPreferences(MediaPreferences);

/// The media preferences `dom` reports (the defaults until set).
pub(crate) fn document_media_preferences(dom: &Dom<TuiExt>) -> MediaPreferences {
    dom.document_data::<DocumentMediaPreferences>()
        .map_or_else(MediaPreferences::default, |p| p.0)
}

/// Set the media preferences `dom` reports.
pub(crate) fn set_document_media_preferences(dom: &mut Dom<TuiExt>, prefs: MediaPreferences) {
    match dom.document_data_mut::<DocumentMediaPreferences>() {
        Some(slot) => slot.0 = prefs,
        None => {
            dom.set_document_data(DocumentMediaPreferences(prefs));
        }
    }
}

/// The environment `dom`'s media queries evaluate in.
pub(crate) fn document_media(dom: &Dom<TuiExt>) -> MediaEnvironment {
    let env = MediaEnvironment::new(
        super::document_viewport(dom),
        super::document_color_scheme(dom),
        document_media_preferences(dom),
    );
    // No viewport yet (a headless cascade before `set_viewport` or a
    // layout): the size features are unknown, not 0 × 0.
    if super::viewport::has_document_viewport(dom) {
        env
    } else {
        env.without_viewport()
    }
}

/// What the cascades left behind for a change of the media environment.
#[derive(Debug, Default)]
struct MediaState {
    /// A computed style read a viewport-percentage length: set by any
    /// cascade that read one, cleared by a whole-tree cascade
    /// ([`begin_tree`]), which sets it again if one still does.
    reads_viewport: Cell<bool>,
    /// A keyframe or a starting style read one (sticky: those are
    /// resolved when an animation or a transition starts, not by every
    /// whole-tree cascade).
    animations_read_viewport: Cell<bool>,
    /// The condition results the last cascade ran under.
    conditions: RefCell<Option<Rc<ConditionResults>>>,
}

/// Before a cascade: make sure the state exists, and sample the viewport
/// reads.
pub(crate) fn begin(dom: &mut Dom<TuiExt>) -> u64 {
    if dom.document_data::<MediaState>().is_none() {
        dom.set_document_data(MediaState::default());
    }
    rdom_style::calc::viewport_reads()
}

/// Before a whole-tree cascade: every element is about to say again
/// whether its style reads the viewport.
pub(crate) fn begin_tree(dom: &Dom<TuiExt>) {
    if let Some(state) = dom.document_data::<MediaState>() {
        state.reads_viewport.set(false);
    }
}

/// After a whole-tree cascade under `sheets` that began at `before`: note
/// a viewport read and the condition results it ran under. (A subtree
/// cascade notes only the read: the rest of the tree is still under the
/// results recorded last.)
pub(super) fn finish(dom: &Dom<TuiExt>, before: u64, sheets: &Sheets<'_>) {
    note_reads(dom, before);
    if let Some(state) = dom.document_data::<MediaState>() {
        *state.conditions.borrow_mut() = Some(sheets.conditions().clone());
    }
}

/// After a subtree cascade that began at `before`: note a viewport read.
pub(super) fn note_reads(dom: &Dom<TuiExt>, before: u64) {
    if rdom_style::calc::viewport_reads() != before
        && let Some(state) = dom.document_data::<MediaState>()
    {
        state.reads_viewport.set(true);
    }
}

/// Whether the media environment `dom` is in now makes its styles stale:
/// a computed style read the viewport (`viewport_moved`: the size is
/// what changed), or a conditional rule of `stylesheets` holds now where
/// it did not at the last cascade, or the reverse. `true` before the
/// first cascade.
pub(crate) fn must_restyle(
    dom: &Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    registry: &PropertyRegistry,
    viewport_moved: bool,
) -> bool {
    let Some(state) = dom.document_data::<MediaState>() else {
        return true;
    };
    if viewport_moved && (state.reads_viewport.get() || state.animations_read_viewport.get()) {
        return true;
    }
    let now = registry
        .facts
        .conditions
        .get(stylesheets, &document_media(dom));
    state
        .conditions
        .borrow()
        .as_ref()
        .is_none_or(|then| !Rc::ptr_eq(then, &now))
}

/// Notes a viewport read when dropped ([`note_reads`]): a style
/// computation with several returns (a keyframe's, a starting style)
/// holds one for its run.
pub(super) struct ReadsGuard<'d> {
    dom: &'d Dom<TuiExt>,
    before: u64,
}

impl<'d> ReadsGuard<'d> {
    pub(super) fn new(dom: &'d Dom<TuiExt>) -> Self {
        ReadsGuard {
            dom,
            before: rdom_style::calc::viewport_reads(),
        }
    }
}

impl Drop for ReadsGuard<'_> {
    fn drop(&mut self) {
        if rdom_style::calc::viewport_reads() != self.before
            && let Some(state) = self.dom.document_data::<MediaState>()
        {
            state.animations_read_viewport.set(true);
        }
    }
}
