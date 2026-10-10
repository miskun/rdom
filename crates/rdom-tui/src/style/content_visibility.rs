//! `content-visibility` (CSS Containment 2 §4) and the last remembered
//! size (CSS Sizing 4 §6.1): which elements skip their contents, and the
//! size `contain-intrinsic-size: auto` gives one that does.
//!
//! `hidden` always skips; `auto` skips while the element is not relevant
//! to the user (§4.4) — rdom's reading: its border box is off the
//! viewport, it does not hold the focus, and no element in it is in the
//! top layer. Relevance is a fact about the last layout, so it is decided
//! after each layout (`after_layout`), where an element that starts or
//! stops skipping is handed to the layout's restyle pass
//! (`render::layout_pass::container_pass`: its `::before` / `::after`
//! appear or go, and it is laid out again) and its
//! `contentvisibilityautostatechange` event queued for the `App`. A newly
//! rendered `auto` element starts out skipping its contents, and the
//! first layout determines its relevance (the event fires then, whatever
//! it finds) — HTML's initial determination: off-screen ones are never
//! laid out with their contents.
//!
//! An element with `contain-intrinsic-size: auto …` remembers its content
//! box each time it is laid out not skipping its contents; while it skips
//! them, that size is its intrinsic size.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::ContentVisibility;
use crate::style::ComputedStyle;

/// The document's `content-visibility` state (document data).
#[derive(Debug, Default)]
struct State {
    /// The `content-visibility: auto` elements the cascade met.
    auto: RefCell<HashSet<NodeId>>,
    /// Those of them skipping their contents.
    skipped: RefCell<HashSet<NodeId>>,
    /// The elements with `contain-intrinsic-size: auto` the cascade met.
    remembering: RefCell<HashSet<NodeId>>,
    /// Their last remembered content-box sizes.
    remembered: RefCell<HashMap<NodeId, (u16, u16)>>,
    /// `contentvisibilityautostatechange` events to fire: the element and
    /// whether it now skips.
    events: RefCell<Vec<(NodeId, bool)>>,
    /// The `auto` elements whose relevance is not determined yet: they
    /// skip their contents until the first layout decides (§4.4, HTML's
    /// initial determination), and that decision fires their event.
    undetermined: RefCell<HashSet<NodeId>>,
}

fn state(dom: &Dom<TuiExt>) -> Option<&State> {
    dom.document_data::<State>()
}

/// Before a cascade: make sure the state exists.
pub(crate) fn begin(dom: &mut Dom<TuiExt>) {
    if state(dom).is_none() {
        dom.set_document_data(State::default());
    }
}

/// Note an element's computed style: an `auto` one is tracked for
/// relevance, one with `contain-intrinsic-size: auto` for its size.
pub(crate) fn note_style(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) {
    let Some(state) = state(dom) else {
        return;
    };
    if c.content_visibility == ContentVisibility::Auto
        && crate::style::containment::size_applies(c)
        && state.auto.borrow_mut().insert(id)
    {
        // A newly rendered `auto` element skips its contents until the
        // layout determines its relevance (§4.4): a long list's
        // off-screen rows are never laid out with theirs.
        state.skipped.borrow_mut().insert(id);
        state.undetermined.borrow_mut().insert(id);
    }
    if c.contain_intrinsic_width.auto || c.contain_intrinsic_height.auto {
        state.remembering.borrow_mut().insert(id);
    }
}

/// Whether the element `id`, styled `c`, skips its contents (§4).
pub(crate) fn skips(dom: &Dom<TuiExt>, id: NodeId, c: &ComputedStyle) -> bool {
    // §4: it applies only where size containment can (§3.1).
    if !crate::style::containment::size_applies(c) {
        return false;
    }
    match c.content_visibility {
        ContentVisibility::Visible => false,
        ContentVisibility::Hidden => true,
        ContentVisibility::Auto => state(dom).is_some_and(|s| s.skipped.borrow().contains(&id)),
    }
}

/// [`skips`] for `id` as last cascaded.
pub(crate) fn skips_contents(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    skips_contents_for(dom, id, SkippedFor::Rendering)
}

/// Who asks whether an element's contents are skipped (CSS Containment 2
/// §4): the two answers differ for `content-visibility: auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkippedFor {
    /// Layout, paint, hit testing and the animation clock: every skipped
    /// content is absent.
    Rendering,
    /// The user-agent features — the tab order, focus, selection and
    /// copy: `hidden`'s skipped contents are absent, while `auto`'s "must
    /// still be available as normal to user-agent features such as
    /// find-in-page, tab order navigation, etc., and must be focusable and
    /// selectable as normal" (§4). Focusing into them makes the element
    /// relevant to the user (§4.4), and so rendered at the next layout.
    Features,
}

/// Whether `id`, as last cascaded, skips its contents for `who`.
pub(crate) fn skips_contents_for(dom: &Dom<TuiExt>, id: NodeId, who: SkippedFor) -> bool {
    let Some(ext) = dom.node(id).ext() else {
        return false;
    };
    // An element at the box tree's depth cap skips its contents for
    // everyone (`MAX_LAYOUT_DEPTH`, C16G-DEPTH-CAPS).
    if ext.depth_capped {
        return true;
    }
    let Some(c) = ext.computed.as_deref() else {
        return false;
    };
    match who {
        SkippedFor::Rendering => skips(dom, id, c),
        SkippedFor::Features => {
            c.content_visibility == ContentVisibility::Hidden
                && crate::style::containment::size_applies(c)
        }
    }
}

/// The size `contain-intrinsic-size: auto` gives `id` while it skips its
/// contents: its last remembered content box (CSS Sizing 4 §6.1).
pub(crate) fn remembered(dom: &Dom<TuiExt>, id: NodeId) -> Option<(u16, u16)> {
    state(dom)?.remembered.borrow().get(&id).copied()
}

/// Whether any `auto` element is tracked: the layout's restyle pass runs
/// for it.
pub(crate) fn any_auto(dom: &Dom<TuiExt>) -> bool {
    state(dom).is_some_and(|s| !s.auto.borrow().is_empty())
}

/// Whether deciding relevance in `viewport` now would change anything —
/// an `auto` element starting or stopping skipping its contents, or one
/// not determined yet. A scroll update, which moves boxes without laying
/// out, asks it: such a change re-cascades and lays out
/// (`layout_pass::scroll_update`).
pub(crate) fn would_change(dom: &Dom<TuiExt>, viewport: crate::layout::LayoutRect) -> bool {
    let Some(state) = state(dom) else {
        return false;
    };
    let skipped = state.skipped.borrow();
    let undetermined = state.undetermined.borrow();
    state.auto.borrow().iter().any(|&id| {
        dom.contains(id)
            && dom.node(id).is_connected()
            && (undetermined.contains(&id) || relevant(dom, id, viewport) == skipped.contains(&id))
    })
}

/// After a layout in `viewport`: remember the sizes of the elements that
/// rendered their contents, decide each `auto` element's relevance, and
/// return those that started or stopped skipping (their events queued).
pub(crate) fn after_layout(dom: &Dom<TuiExt>, viewport: crate::layout::LayoutRect) -> Vec<NodeId> {
    let Some(state) = state(dom) else {
        return Vec::new();
    };
    #[cfg(test)]
    probe::DECISIONS.with(|c| c.set(c.get() + 1));
    let live = |id: NodeId| dom.contains(id) && dom.node(id).is_connected();
    remember_sizes(dom);
    let mut changed = Vec::new();
    let mut auto = state.auto.borrow_mut();
    auto.retain(|&id| {
        live(id)
            && dom
                .node(id)
                .ext()
                .and_then(|e| e.computed.as_deref())
                .is_some_and(|c| c.content_visibility == ContentVisibility::Auto)
    });
    let mut skipped = state.skipped.borrow_mut();
    skipped.retain(|id| auto.contains(id));
    let mut undetermined = state.undetermined.borrow_mut();
    undetermined.retain(|id| auto.contains(id));
    for &id in auto.iter() {
        let skip = !relevant(dom, id, viewport);
        // The first determination fires the event whatever it finds.
        let first = undetermined.remove(&id);
        if skip != skipped.contains(&id) {
            if skip {
                skipped.insert(id);
            } else {
                skipped.remove(&id);
            }
            changed.push(id);
            state.events.borrow_mut().push((id, skip));
        } else if first {
            state.events.borrow_mut().push((id, skip));
        }
    }
    changed.sort_unstable();
    changed
}

/// After a layout: remember the content-box size of each element with
/// `contain-intrinsic-size: auto` that rendered its contents (CSS Sizing 4
/// §6.1) — the part of [`after_layout`] a layout that is not followed by
/// another (the container pass's cap) still owes.
pub(crate) fn remember_sizes(dom: &Dom<TuiExt>) {
    let Some(state) = state(dom) else {
        return;
    };
    let live = |id: NodeId| dom.contains(id) && dom.node(id).is_connected();
    let mut remembering = state.remembering.borrow_mut();
    // CSS Sizing 4 §6.1: a remembered size goes once the element no
    // longer has `contain-intrinsic-size: auto`.
    remembering.retain(|&id| {
        live(id)
            && dom
                .node(id)
                .ext()
                .and_then(|e| e.computed.as_deref())
                .is_some_and(|c| c.contain_intrinsic_width.auto || c.contain_intrinsic_height.auto)
    });
    let mut remembered = state.remembered.borrow_mut();
    remembered.retain(|id, _| remembering.contains(id));
    for &id in remembering.iter() {
        if !skips_contents(dom, id)
            && let Some(ext) = dom.node(id).ext()
        {
            let r = ext.content_layout;
            remembered.insert(id, (r.width, r.height));
        }
    }
}

/// §4.4 "relevant to the user": on screen, holding the focus, or holding
/// an element of the top layer.
fn relevant(dom: &Dom<TuiExt>, id: NodeId, viewport: crate::layout::LayoutRect) -> bool {
    let on_screen = dom.node(id).ext().is_some_and(|e| {
        let r = e.layout;
        let (x0, y0) = (viewport.x, viewport.y);
        let (x1, y1) = (
            x0 + i32::from(viewport.width),
            y0 + i32::from(viewport.height),
        );
        r.x < x1 && r.y < y1 && r.x + i32::from(r.width) >= x0 && r.y + i32::from(r.height) >= y0
    });
    let holds = |n: NodeId| dom.node(id).contains(n);
    on_screen
        || dom.focused().is_some_and(holds)
        || dom.top_layer().iter().any(|&n| n != id && holds(n))
}

/// The queued `contentvisibilityautostatechange` events, taken.
pub(crate) fn take_events(dom: &Dom<TuiExt>) -> Vec<(NodeId, bool)> {
    state(dom).map_or_else(Vec::new, |s| std::mem::take(&mut *s.events.borrow_mut()))
}

/// Fire the queued `contentvisibilityautostatechange` events (CSS
/// Containment 2 §4: at the element, not bubbling, with `skipped`) — an
/// `App` does after each painted frame. `true` when one fired.
pub(crate) fn fire_queued_events(dom: &mut crate::TuiDom) -> bool {
    let events = take_events(dom);
    for &(id, skipped) in &events {
        let mut tui = crate::TuiEvent::new("contentvisibilityautostatechange");
        tui.event.bubbles = false;
        tui.event.cancelable = false;
        tui.event.detail = rdom_core::EventDetail::ContentVisibilityAutoStateChange { skipped };
        crate::tui_event::dispatch_to_live(dom, id, &mut tui);
    }
    !events.is_empty()
}

/// Test-only: how many times relevance was decided on this thread.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static DECISIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        DECISIONS.with(|c| c.replace(0))
    }
}
