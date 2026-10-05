//! Focus management — tabindex ordering, programmatic focus,
//! focus/blur/focusin/focusout event dispatch.
//!
//! ## Sub-modules
//!
//! - [`tabindex`] — `tabindex` attribute parsing, focusable-element
//!   collection, ordering (positive indices ascending, then DOM
//!   order for `tabindex=0`), `Tab` / `Shift+Tab` navigation.
//!
//! ## Events
//!
//! [`focus_node`] is the canonical "change the focus and fire the
//! right events" entry point — the runtime (both Tab navigation
//! and focus-on-click paths) calls it, and apps that want the
//! full browser-ceremony on a programmatic focus change call it
//! instead of raw `dom.set_focused(...)`.
//!
//! Events fire in spec order:
//!
//! 1. `blur` on old (non-bubbling)
//! 2. `focusout` on old (bubbling)
//! 3. `dom.set_focused(new)` commits — `:focus` cascade picks up
//!    (with a pointer-moved focus, `:focus-visible` is decided here too)
//! 4. `focus` on new (non-bubbling)
//! 5. `focusin` on new (bubbling)

pub mod tabindex;
pub(crate) mod visible;

#[cfg(test)]
mod scroll_tests;
#[cfg(test)]
mod tests;

use rdom_core::{NodeId, NodeType, Position, Selection};

use crate::node::{TuiNodeExt, is_descendant_or_self};
use crate::{TuiDom, TuiEvent};

/// The options of a focus change (HTML `FocusOptions`).
///
/// `#[non_exhaustive]`: built with [`FocusOptions::new`] (or
/// `Default`) and its builder methods.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FocusOptions {
    /// HTML `preventScroll`: do not scroll the newly focused element into
    /// view.
    pub prevent_scroll: bool,
}

impl FocusOptions {
    /// The default options: the element is scrolled into view.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `preventScroll`.
    pub fn prevent_scroll(mut self, prevent: bool) -> Self {
        self.prevent_scroll = prevent;
        self
    }
}

/// [`focus_node`] with `options` (HTML `focus(options)`): the focus
/// change, then — unless `prevent_scroll` — HTML's focusing steps' "scroll
/// the element into view": `nearest` on both axes, as browsers reveal a
/// focused element, into each scroll container's optimal viewing region
/// (its `scroll-padding`, the element's `scroll-margin`: CSS Scroll Snap 1
/// §4). Under a running `App` the scroll waits for its next layout — the
/// one the new focus is shown in — so a handler that moved the element
/// before focusing it reveals it where it is laid out, not at a rect its
/// own changes made stale; on a bare document it is done at once, against
/// the last layout.
pub fn focus_node_with_options(dom: &mut TuiDom, new_focus: Option<NodeId>, options: FocusOptions) {
    let before = dom.focused();
    change_focus(dom, new_focus, None);
    if options.prevent_scroll {
        return;
    }
    if let Some(id) = new_focus
        && dom.focused() == Some(id)
        && before != Some(id)
    {
        if crate::runtime::timers::in_app() {
            dom.set_document_data(PendingFocusScroll(id));
        } else {
            scroll_into_view(dom, id);
        }
    }
}

/// The focused element whose scroll into view waits for the next layout
/// (document data; [`focus_node_with_options`], [`service_focus_scroll`]).
struct PendingFocusScroll(NodeId);

/// After a layout: scroll the element focused since the last one into
/// view, when it still has the focus. Whether a scroll offset moved — the
/// caller lays out again.
pub(crate) fn service_focus_scroll(dom: &mut TuiDom) -> bool {
    let Some(PendingFocusScroll(id)) = dom.remove_document_data::<PendingFocusScroll>() else {
        return false;
    };
    if dom.focused() != Some(id) || !dom.contains(id) {
        return false;
    }
    let offsets = |dom: &TuiDom| -> Vec<(i32, i32)> {
        std::iter::successors(dom.node(id).parent_node(), |n| n.parent_node())
            .filter_map(|n| n.tui_ext().map(|e| (e.scroll_x, e.scroll_y)))
            .collect()
    };
    let before = offsets(dom);
    scroll_into_view(dom, id);
    offsets(dom) != before
}

/// Scroll the focused `id` into view, `nearest` on both axes.
fn scroll_into_view(dom: &mut TuiDom, id: NodeId) {
    use crate::runtime::smooth_scroll::{ScrollIntoViewOptions, ScrollLogicalPosition};
    crate::runtime::scrollbar::scroll_element_into_view(
        dom,
        id,
        ScrollIntoViewOptions::new()
            .block(ScrollLogicalPosition::Nearest)
            .inline(ScrollLogicalPosition::Nearest),
    );
}

/// Change focus. Fires `blur` + `focusout` on the old focus,
/// commits the new focus (which updates the `:focus` pseudo via
/// `Mutation::InteractionChanged`), then fires `focus` + `focusin`
/// on the new.
///
/// Idempotent: if `new_focus == dom.focused()`, no events fire and
/// no mutation happens.
///
/// Pass `None` to clear focus (fires only blur + focusout).
///
/// Keyboard and script focus: the element is scrolled into view
/// ([`focus_node_with_options`]). Focus a pointer moved does not scroll
/// (`focus_node_by_pointer`), nor does
/// `FocusOptions::new().prevent_scroll(true)`.
pub fn focus_node(dom: &mut TuiDom, new_focus: Option<NodeId>) {
    focus_node_with_options(dom, new_focus, FocusOptions::new());
}

/// The focus fixup (HTML "update the rendering", after style and
/// layout): when the focused element is no longer rendered and visible
/// ([`tabindex::is_rendered_and_visible`] — it or an ancestor became
/// `display: none`, or its used `visibility` is not `visible`), it is no
/// focusable area, so the focusing steps run for the viewport: `blur` /
/// `focusout` fire and nothing is focused. Returns whether it blurred.
pub(crate) fn fix_up(dom: &mut TuiDom) -> bool {
    match dom.focused() {
        Some(f) if !tabindex::is_rendered_and_visible(dom, f) => {
            focus_node(dom, None);
            true
        }
        _ => false,
    }
}

/// [`focus_node`] for focus moved by a pointer press: the new element's
/// `:focus-visible` answer ([`visible::pointer_focus_is_evident`]) is
/// committed with the focus, before `focus` / `focusin` fire —
/// browsers decide it before the focus events, so a listener that asks
/// sees the pointer's answer, not the previous modality's.
pub(crate) fn focus_node_by_pointer(dom: &mut TuiDom, new_focus: Option<NodeId>) {
    let evident = new_focus.map(|id| visible::pointer_focus_is_evident(dom, id));
    change_focus(dom, new_focus, evident);
}

/// The focus-change steps; `visible`, when given, is committed as
/// `Dom::focus_visible` right after the focus itself.
fn change_focus(dom: &mut TuiDom, new_focus: Option<NodeId>, visible: Option<bool>) {
    let old = dom.focused();
    if old == new_focus {
        return;
    }

    // blur + focusout on the old target; a `blur` listener that dropped
    // it leaves no target for `focusout`.
    if let Some(old_id) = old {
        let mut blur = TuiEvent::blur();
        if crate::tui_event::dispatch_to_live(dom, old_id, &mut blur) {
            let mut out = TuiEvent::focusout();
            crate::tui_event::dispatch_to_live(dom, old_id, &mut out);
        }
    }
    // A `blur` / `focusout` listener that dropped the new target leaves
    // nothing to focus: the old focus is already blurred, so focus
    // clears. Committing the dead id would leave `Dom::focused` naming
    // a freed node, which `Dom` otherwise never does (a dropped
    // subtree purges its interaction state).
    let new_focus = new_focus.filter(|&id| dom.contains(id));

    // Commit the new state — drives :focus cascade via the
    // InteractionChanged mutation the DirtyTracker observes.
    dom.set_focused(new_focus);
    if let Some(visible) = visible {
        dom.set_focus_visible(visible);
    }

    // Seed a collapsed caret for editable focus targets. The
    // mouse-click drag-select path does this automatically (it
    // computes a position from the click coordinates and calls
    // `dom.set_selection(...)`); Tab and programmatic focus had no
    // such path, so a freshly-focused `<input>` had a `None`
    // selection. `editing::perform::insert_at_selection` returns
    // `NoEditableTarget` when the selection is `None`, which means
    // the first keystroke after Tab-focusing an input was silently
    // dropped. Browsers seed a caret on focus (or select-all on
    // most platforms); the simpler caret-at-0 matches what we do
    // for mouse focus and is sufficient for v1.
    if let Some(new_id) = new_focus {
        seed_caret_for_editable_focus(dom, new_id);
    }

    // focus + focusin on the new target.
    if let Some(new_id) = new_focus {
        let mut foc = TuiEvent::focus();
        if crate::tui_event::dispatch_to_live(dom, new_id, &mut foc) {
            let mut fin = TuiEvent::focusin();
            crate::tui_event::dispatch_to_live(dom, new_id, &mut fin);
        }
    }
}

/// Seed a collapsed caret at the start of `id`'s first text-node
/// child when `id` is editable and the current selection is `None`
/// or points outside `id`'s subtree. No-op for non-editable elements
/// or when a selection already covers this subtree (we don't clobber
/// an existing caret position the user has navigated to).
fn seed_caret_for_editable_focus(dom: &mut TuiDom, id: NodeId) {
    if !dom.node(id).is_editable() {
        return;
    }

    // Ensure the editable has its text-node child. The `App` seeds a
    // control inserted after `App::build` at its next event or frame
    // (`input::ControlSeeding`), but one focused before that boundary — a
    // listener that mounts a form and focuses a field, `[autofocus]` run on
    // a freshly mounted view, a `TuiDom` with no `App` — has none yet, and
    // the caret seed + first keystroke would silently no-op. Seed it here so
    // any focused editable is typeable, whenever it was created.
    crate::runtime::builtins::input::ensure_seeded(dom, id);

    // Preserve any pre-existing selection that already lives inside
    // `id`'s subtree — re-focusing the same element after blur
    // shouldn't reset the user's caret position.
    if let Some(sel) = dom.selection()
        && is_descendant_or_self(dom, sel.focus.node, id)
    {
        return;
    }

    // Find the first Text-node child of `id`. `<input>` and
    // `<textarea>` always have one after `ensure_seeded` above;
    // `contenteditable` elements may or may not — skip seeding if
    // there's none.
    let text_child = dom
        .node(id)
        .child_nodes()
        .find(|n| n.node_type() == NodeType::Text)
        .map(|n| n.id());

    if let Some(text_id) = text_child {
        dom.set_selection(Some(Selection::caret(Position::new(text_id, 0))));
    }
}

/// Walk up from `start` via parent_node, returning the nearest
/// ancestor (including `start` itself) that is tab-focusable.
/// Used by the runtime's focus-on-click path: clicking a
/// non-focusable child focuses the nearest focusable ancestor,
/// matching browser behavior.
///
/// Returns `None` if no ancestor is focusable — e.g., clicking in
/// non-interactive chrome like a decoration element.
pub fn nearest_focusable_ancestor(dom: &TuiDom, start: NodeId) -> Option<NodeId> {
    let mut cur = Some(start);
    while let Some(id) = cur {
        if tabindex::is_focusable(dom, id) {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}
