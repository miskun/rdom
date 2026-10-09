//! The style flush (HTML §6.6.3 "focusing steps", §6.6.6 `focus()`):
//! whether a focus target is a focusable area — rendered, visible, not
//! inert — is read from its style, and a browser updates style before it
//! reads it, so code that shows a panel and focuses an input in it in one
//! handler works.
//!
//! An [`App`](crate::App) owns the cascade's inputs — its author sheets,
//! the document's `<style>` sheets, the `CSS.registerProperty` sheet, the
//! property registry and the dirty tracker. It publishes them on its
//! document whenever the sheet set changes (`publish`, document data,
//! as the focus deferral is), so a handler's `&mut TuiDom` reaches them.
//! [`flush_style`] then cascades exactly the dirty tracker's subtree
//! roots that hold the element — the element and its ancestors — and
//! takes them from the tracker, so the frame does not cascade them again;
//! the rest of the document stays dirty for the frame.
//!
//! What a flush does not do, each left to the `App`'s next frame:
//!
//! - **layout** — `bounding_rect`, the scroll metrics and the scroll a
//!   focus change makes read the last layout (layout is a whole-tree pass
//!   in rdom; the focus scroll waits for the frame, `runtime::focus`);
//! - **transitions and animations** — the change a flush cascades is
//!   compared with the before-change style at the frame's transition hook
//!   (CSS Transitions 1 §3), which still holds the last frame's styles, so
//!   its transitions start at that frame's time, and two flushes in one
//!   handler are one style change;
//! - **a whole-tree restyle the `App` has pending** — before its first
//!   frame, or after a change of the sheet set, the color scheme or the
//!   terminal size — and `<style>` text edited since the last frame;
//! - **readers that borrow the document** — `computed()`, which takes
//!   `&self`, reads the last style update; call [`flush_style`] first to
//!   bring an element's up to date.
//!
//! On a document no `App` runs, nothing is published and a flush does
//! nothing: the caller cascades it (`CascadeExt::cascade`).

use std::rc::Rc;

use rdom_core::NodeId;

use crate::TuiDom;
use crate::style::Stylesheet;
use crate::style::cascade::{PropertyRegistry, cascade_subtrees_all_with};
use crate::style::dirty_tracker::DirtyTracker;

#[cfg(test)]
mod tests;

/// The cascade's inputs, as of the last sheet-set change (document
/// data an `App` publishes).
struct StyleInputs {
    /// Every sheet the cascade reads, in cascade order.
    sheets: Rc<[Rc<Stylesheet>]>,
    registry: Rc<PropertyRegistry>,
    tracker: DirtyTracker,
}

/// Publish the cascade's inputs on `dom` (an `App`, at construction and
/// at every sheet-set change): `sheets` in cascade order, the property
/// registry they build, and the dirty tracker observing `dom`.
pub(crate) fn publish(
    dom: &mut TuiDom,
    sheets: Vec<Rc<Stylesheet>>,
    registry: Rc<PropertyRegistry>,
    tracker: DirtyTracker,
) {
    dom.set_document_data(StyleInputs {
        sheets: sheets.into(),
        registry,
        tracker,
    });
}

/// What a cascade outside the frame runs with: the sheets in cascade
/// order, their registry, and the dirty tracker of the `App` running the
/// document, if one does.
pub(crate) struct CascadeInputs {
    pub sheets: Rc<[Rc<Stylesheet>]>,
    pub registry: Rc<PropertyRegistry>,
    pub tracker: Option<DirtyTracker>,
}

/// The cascade's inputs an `App` published on `dom`, or `None` on a
/// document no `App` runs. The layout pass re-cascades a query
/// container's subtree with them (`render::layout_pass::container_pass`).
pub(crate) fn published(dom: &TuiDom) -> Option<CascadeInputs> {
    let inputs = dom.document_data::<StyleInputs>()?;
    Some(CascadeInputs {
        sheets: inputs.sheets.clone(),
        registry: inputs.registry.clone(),
        tracker: Some(inputs.tracker.clone()),
    })
}

/// Bring `id`'s computed style up to date (HTML's "update style" before
/// the focusing steps): cascade the dirty subtrees that hold it, under
/// the sheets the `App` running `dom` cascades, and leave the rest of the
/// document for the next frame (module doc). Returns whether anything
/// was cascaded — `false` on a document no `App` runs, or when nothing
/// above `id` is dirty.
///
/// `TuiAccessorsMut::focus` and `focus_with` call it; call it before
/// reading `computed()` in a handler that just changed what `id`'s style
/// depends on.
pub fn flush_style(dom: &mut TuiDom, id: NodeId) -> bool {
    if !dom.contains(id) {
        return false;
    }
    let Some(inputs) = dom.document_data::<StyleInputs>() else {
        return false;
    };
    let (sheets, registry, tracker) = (
        inputs.sheets.clone(),
        inputs.registry.clone(),
        inputs.tracker.clone(),
    );
    let roots = tracker.take_roots_holding(dom, id);
    if roots.is_empty() {
        return false;
    }
    let sheets: Vec<&Stylesheet> = sheets.iter().map(|s| &**s).collect();
    let cascaded = cascade_subtrees_all_with(dom, &sheets, Some(registry), &roots);
    tracker.note_flushed(&cascaded);
    true
}
