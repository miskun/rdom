//! `autofocus` global attribute — initial focus on app mount +
//! modal dialog open.
//!
//! ## Contract (from MDN)
//!
//! - `autofocus` is a global HTML attribute. When present on a
//!   focusable element, the element receives focus:
//!   - On page load (app mount, for us)
//!   - When a `<dialog>` containing it is shown modally
//!   - When an `<input>`-ish ancestor subtree is inserted
//! - Only the FIRST `[autofocus]` in document order wins per
//!   activation — if multiple are set, later ones are ignored.
//! - `disabled` or otherwise non-focusable `[autofocus]` elements
//!   are skipped.
//!
//! ## v1 scope
//!
//! - Mount: [`focus_first_autofocus`] walks the whole document
//!   once and focuses the first eligible `[autofocus]` element.
//!   Called from `App::build` after builtins install.
//! - Modal dialog: `runtime::builtins::dialog::show_modal` walks
//!   the dialog subtree and focuses its first `[autofocus]`
//!   descendant (see `dialog/mod.rs`).
//!
//! ## Not shipping in v1
//!
//! - Continuous mutation-observer-driven autofocus-on-subtree-
//!   insert. Apps that add `[autofocus]` elements after mount
//!   call [`focus_first_autofocus`] or [`focus_within`] manually.
//! - Any heuristic for "don't steal focus if user has already
//!   interacted". Matches MDN's note that `autofocus` is
//!   accessibility-risky — we honor the explicit opt-in without
//!   second-guessing.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::runtime::focus;
use crate::runtime::focus::tabindex::is_focusable;

/// Walk the whole document and focus the first focusable element
/// with an `autofocus` attribute. No-op when nothing matches or
/// when something else is already focused (to avoid clobbering
/// an earlier explicit `set_focused`).
pub fn focus_first_autofocus(dom: &mut TuiDom) {
    if dom.focused().is_some() {
        return;
    }
    let root = dom.root();
    if let Some(target) = find_autofocus_in(dom, root) {
        focus::focus_node(dom, Some(target));
    }
}

/// Walk the subtree rooted at `root` and focus the first
/// focusable `[autofocus]` descendant. Used by
/// `dialog::show_modal` — modal dialogs should focus their
/// intended initial element without the app writing that boilerplate.
pub fn focus_within(dom: &mut TuiDom, root: NodeId) {
    if let Some(target) = find_autofocus_in(dom, root) {
        focus::focus_node(dom, Some(target));
    }
}

/// [`focus_within`] for a dialog `showModal()` has just opened, whose
/// computed `display: none` predates the call
/// (`tabindex::is_focusable_in_opened`).
pub(crate) fn focus_within_opened(dom: &mut TuiDom, dialog: NodeId) {
    let focusable =
        |dom: &TuiDom, id| crate::runtime::focus::tabindex::is_focusable_in_opened(dom, id, dialog);
    if let Some(target) = find_autofocus_by(dom, dialog, &focusable) {
        focus::focus_node(dom, Some(target));
    }
}

/// Depth-first, document-order walk starting at `id`. Returns the
/// first element with `[autofocus]` that's also focusable per the
/// C.1 rules.
fn find_autofocus_in(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    find_autofocus_by(dom, id, &is_focusable)
}

fn find_autofocus_by(
    dom: &TuiDom,
    id: NodeId,
    focusable: &dyn Fn(&TuiDom, NodeId) -> bool,
) -> Option<NodeId> {
    // Iterative: a DOM may be any depth (C16G-DEPTH-CAPS).
    std::iter::once(id)
        .chain(dom.descendants(id))
        .find(|&n| dom.node(n).has_attribute("autofocus") && focusable(dom, n))
}

#[cfg(test)]
mod tests;
