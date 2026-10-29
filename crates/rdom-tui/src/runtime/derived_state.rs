//! The DOM-derived state an `App` keeps current before each cascade —
//! what a browser keeps current as the tree changes — with one owner,
//! published on the document, so the frame and a layout flush
//! (`runtime::layout_flush`) bring it up to date the same way:
//!
//! 1. **control seeding** — the text controls inserted since the last
//!    boundary get their text node (`input::ControlSeeding`,
//!    `INPUT-SEED-ON-INSERT-1`);
//! 2. **`<select>` selectedness** — the selects whose options changed
//!    settle (`select::Selectedness`), and the popovers whose `popover`
//!    attribute changed state while showing hide (`popover::attribute`);
//! 3. (the `App`'s `<style>` sheets, between 2 and 4 — not here: they
//!    change the sheet set, which only the `App` can; [`style_pending`]
//!    says whether one waits);
//! 4. **scroll-focus marker** — `data-rdom-scroll-focus` moves to the
//!    container the keyboard scrolls, per the last layout;
//! 5. **validity marks** — the elements whose `:valid` / `:invalid` or a
//!    radio group's `:indeterminate` flipped are dirtied
//!    (`validation::FormStateMarks`).
//!
//! Stages 1–2 also run before each input event (`App::handle_event`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rdom_core::NodeId;

use crate::TuiDom;
use crate::runtime::builtins::input::ControlSeeding;
use crate::runtime::builtins::popover::attribute::PopoverAttributes;
use crate::runtime::builtins::select::Selectedness;
use crate::runtime::builtins::validation::FormStateMarks;
use crate::style::Stylesheet;
use crate::style::dirty_tracker::DirtyTracker;

/// The stages' state (module doc).
struct DerivedState {
    control_seeding: ControlSeeding,
    selectedness: Selectedness,
    popover_attributes: PopoverAttributes,
    /// Set by the `<style>` elements' observer while a sheet waits to be
    /// re-parsed (`cssom::style_elements`).
    style_pending: Rc<Cell<bool>>,
    /// The element currently carrying `data-rdom-scroll-focus`.
    scroll_focus_marked: Option<NodeId>,
    validity_marks: FormStateMarks,
}

/// Document data: the shared state.
#[derive(Clone)]
struct Shared(Rc<RefCell<DerivedState>>);

fn shared(dom: &TuiDom) -> Option<Shared> {
    dom.document_data::<Shared>().cloned()
}

/// Install the stages' observers on `dom` and publish their state on it
/// (`App`'s construction). `style_pending` is the `<style>` elements'
/// dirty flag.
pub(crate) fn install(dom: &mut TuiDom, style_pending: Rc<Cell<bool>>) {
    let state = DerivedState {
        control_seeding: ControlSeeding::install(dom),
        selectedness: Selectedness::install(dom),
        popover_attributes: PopoverAttributes::install(dom),
        style_pending,
        scroll_focus_marked: None,
        validity_marks: FormStateMarks::default(),
    };
    dom.set_document_data(Shared(Rc::new(RefCell::new(state))));
}

/// Stages 1–2: seed the inserted controls, settle the selects and the
/// popovers. Their listeners (a popover's `toggle`) may re-enter — a
/// `focus()` flushes layout — so no borrow is held across them.
pub(crate) fn settle_controls(dom: &mut TuiDom) {
    let Some(Shared(state)) = shared(dom) else {
        return;
    };
    let (seeding, selectedness, popovers) = {
        let s = state.borrow();
        (
            s.control_seeding.clone(),
            s.selectedness.clone(),
            s.popover_attributes.clone(),
        )
    };
    seeding.flush(dom);
    selectedness.flush(dom);
    crate::runtime::builtins::select::settle_pickers(dom);
    popovers.flush(dom);
}

/// Whether a `<style>` element's sheet waits to be re-parsed: the sheet
/// set is stale until the `App`'s next frame.
pub(crate) fn style_pending(dom: &TuiDom) -> bool {
    shared(dom).is_some_and(|Shared(s)| s.borrow().style_pending.get())
}

/// Stage 4: move `data-rdom-scroll-focus` to the scroll container the
/// keyboard scrolls, per the last layout, while the focus is evident.
pub(crate) fn mark_scroll_focus(dom: &mut TuiDom) {
    let Some(Shared(state)) = shared(dom) else {
        return;
    };
    let target =
        crate::runtime::scrollbar::scroll_focus_target(dom).filter(|_| dom.focus_visible());
    // Released before the writes: a mutation observer may re-enter.
    let previous = {
        let mut s = state.borrow_mut();
        if target == s.scroll_focus_marked {
            return;
        }
        std::mem::replace(&mut s.scroll_focus_marked, target)
    };
    if let Some(prev) = previous
        && dom.contains(prev)
    {
        dom.remove_attribute(prev, crate::runtime::scrollbar::SCROLL_FOCUS_ATTR)
            .expect("a live element accepts attribute removal");
    }
    if let Some(next) = target {
        dom.set_attribute(next, crate::runtime::scrollbar::SCROLL_FOCUS_ATTR, "")
            .expect("the focused element's ancestor is a live element");
    }
}

/// Stage 5: dirty the elements whose tracked form state flipped since the
/// last run. Returns whether it walked the tree.
pub(crate) fn mark_validity<'s>(
    dom: &mut TuiDom,
    tracker: &DirtyTracker,
    sheets: impl IntoIterator<Item = &'s Stylesheet>,
) -> bool {
    let Some(Shared(state)) = shared(dom) else {
        return false;
    };
    state
        .borrow_mut()
        .validity_marks
        .flush(dom, tracker, sheets)
}

/// The sheets changed: the validity marks recompute what they read.
pub(crate) fn sheets_changed(dom: &TuiDom) {
    if let Some(Shared(state)) = shared(dom) {
        state.borrow_mut().validity_marks.sheets_changed();
    }
}
