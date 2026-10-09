//! A scroll without a layout (C15G-SCROLL-NO-RELAYOUT).
//!
//! Layout keeps every box where it is on screen: a scroll container lays
//! its content out moved by its scroll offset (`layout_pass` § Scroll), so
//! paint, hit-testing, the accessors, the caret and the selection read
//! one set of rects. A scroll offset change only translates the content it
//! scrolls, and layout is translation-invariant (`tree::shift_subtree`), so
//! [`update`] moves the laid-out boxes instead of laying the document out
//! again — leaving exactly what a layout at the new offsets would — and
//! redoes only what depends on where things are on screen:
//!
//! 1. The post-placement moves the last layout made (sticky boxes, a
//!    flipped `<select>` picker, relative and sticky `::before` /
//!    `::after`), recorded in its [`journal`], are taken back.
//! 2. Each scroll container whose offset moved since its content was
//!    placed (`scrollbar::state::laid_out`) has its offset clamped, as
//!    layout clamps it, and its content moved by the difference
//!    ([`shift`]): its in-flow boxes and the positioned boxes it contains,
//!    not a positioned box whose containing block is outside it — whose
//!    static position moves with the content all the same.
//! 3. Those boxes, and every anchor-positioned box (its anchors may have
//!    moved apart from it; CSS Anchor Positioning 1 §3's remembered scroll
//!    offset is the current one, DIVERGENCES), are placed again in tree
//!    order (`positioning::place_again`), `position-visibility` decided
//!    again; one that moved is moved with its content.
//! 4. The sticky boxes, the picker flip and the pseudo-element offsets run
//!    again.
//!
//! Anything that would make the result differ from a layout's falls back
//! to [`LayoutExt::layout_dom`](crate::render::LayoutExt::layout_dom): no
//! record of a layout at this viewport, an offset on a box that is not a
//! scroll container, a box placed again at another size, positioned boxes
//! reaching further into their scroll containers (`positioned_overflow`),
//! or a `content-visibility: auto` element changing relevance (which
//! re-cascades). Scroll snapping, the scroll-driven timelines and the
//! caret reveal run after it in the `App`'s frame, as after a layout.

mod journal;
mod shift;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::Rect;
use crate::render::box_tree::BoxItem;

pub(in crate::render::layout_pass) use journal::{
    PlacedRecord, PostMove, begin, begin_placement, note_placed, record, set_sticky,
};
pub(in crate::render::layout_pass) use shift::move_box;

/// What [`update`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Update {
    /// No scroll offset moved: nothing to do.
    Unmoved,
    /// The scrolled boxes were moved, and the scroll-dependent ones
    /// updated, without a layout.
    Scrolled,
    /// The document was laid out ([`LayoutExt::layout_dom`](crate::render::LayoutExt::layout_dom)).
    LaidOut,
}

#[cfg(test)]
thread_local! {
    /// Scroll updates that ran without a layout (cost tests).
    pub(crate) static SCROLLED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Bring the layout up to date with the scroll offsets written since it,
/// in `viewport` — moving boxes when only scroll offsets changed since the
/// last layout, which the caller vouches for (module doc).
pub(crate) fn update(dom: &mut Dom<TuiExt>, viewport: Rect) -> Update {
    let at = LayoutRect::new(
        i32::from(viewport.x),
        i32::from(viewport.y),
        viewport.width,
        viewport.height,
    );
    match try_update(dom, at) {
        Some(u) => u,
        None => {
            crate::render::LayoutExt::layout_dom(dom, viewport);
            Update::LaidOut
        }
    }
}

/// [`update`], `None` where it must lay out.
fn try_update(dom: &mut Dom<TuiExt>, viewport: LayoutRect) -> Option<Update> {
    let scrolled = moved_scrollers(dom)?;
    if scrolled.is_empty() {
        return Some(Update::Unmoved);
    }
    let mut j = journal::take(dom)?;
    if j.viewport != viewport || !j.is_live(dom) {
        return None;
    }
    // 1. Back to where phase 2 left the boxes.
    journal::undo(dom, &j.moves);
    j.moves.clear();
    // 2. The scrolled content.
    let mut stays: Vec<BoxItem> = Vec::new();
    for id in scrolled {
        let computed = dom.node(id).computed_rc()?;
        crate::render::layout_pass::scroll_extent::clamp_scroll_offset(dom, id, &computed);
        let mut node = dom.node_mut(id);
        let ext = node.ext_mut()?;
        let (lx, ly) = crate::runtime::scrollbar::state::laid_out(ext);
        let (dx, dy) = (lx - ext.scroll_x, ly - ext.scroll_y);
        crate::runtime::scrollbar::state::note_laid_out(ext);
        if (dx, dy) != (0, 0) {
            shift::scroll_content(dom, id, dx, dy, &mut stays);
        }
    }
    // 3. The positioned boxes whose placement may have moved.
    let replaced = place_again(dom, &mut j, viewport, stays)?;
    if replaced {
        let items: Vec<BoxItem> = j.positioned.iter().map(|p| p.item).collect();
        if crate::render::layout_pass::positioned_overflow::settle(dom, &items) {
            return None;
        }
    }
    // 4. What depends on where the boxes are now, recorded again.
    let sticky = std::mem::take(&mut j.sticky);
    journal::restore(dom, j);
    crate::render::layout_pass::sticky::place_sticky_in(dom, &sticky);
    journal::set_sticky(dom, &sticky);
    crate::render::layout_pass::picker::place_pickers(dom, viewport);
    crate::render::layout_pass::positioning::offset_in_flow_pseudos(dom);
    if crate::style::content_visibility::would_change(dom, viewport) {
        return None;
    }
    #[cfg(test)]
    SCROLLED.with(|c| c.set(c.get() + 1));
    Some(Update::Scrolled)
}

/// The scroll containers whose offsets moved since their content was laid
/// out, in tree order — `None` when an element that is not one holds a
/// moved offset (layout drops it). Boxes that are not laid out (`display:
/// none`, skipped contents) are not looked at.
fn moved_scrollers(dom: &Dom<TuiExt>) -> Option<Vec<NodeId>> {
    let mut out = Vec::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        if node.node_type() == NodeType::Element {
            let Some(ext) = node.tui_ext() else {
                continue;
            };
            let Some(c) = ext.computed.as_deref() else {
                continue;
            };
            if c.display == Display::None {
                continue;
            }
            if (ext.scroll_x, ext.scroll_y) != crate::runtime::scrollbar::state::laid_out(ext) {
                if !c.is_scroll_container() {
                    return None;
                }
                out.push(id);
            }
            if crate::style::content_visibility::skips_contents(dom, id) {
                continue;
            }
        }
        let children: Vec<NodeId> = crate::render::box_tree::children(dom, id).collect();
        stack.extend(children.into_iter().rev());
    }
    Some(out)
}

/// Place again, in tree order, every anchor-positioned box and the boxes
/// in `stays`; `None` when one's size changed (it must be laid out).
/// Whether one moved.
fn place_again(
    dom: &mut Dom<TuiExt>,
    j: &mut journal::Journal,
    viewport: LayoutRect,
    stays: Vec<BoxItem>,
) -> Option<bool> {
    use crate::render::layout_pass::positioning;
    let mut pending: HashSet<BoxItem> = stays.into_iter().collect();
    let anchored = j
        .positioned
        .iter()
        .any(|p| positioning::is_anchored(dom, p.item));
    if pending.is_empty() && !anchored {
        return Some(false);
    }
    // Only anchor-positioned boxes are ever hidden; each is placed again.
    positioning::begin_hidden(dom);
    let anchors = positioning::AnchorIndex::default();
    crate::render::layout_pass::intrinsic::begin_pass(dom);
    let mut any = false;
    let mut result = Some(());
    for k in 0..j.positioned.len() {
        let record = j.positioned[k];
        if !pending.remove(&record.item) && !positioning::is_anchored(dom, record.item) {
            continue;
        }
        let Some((rect, now)) = positioning::place_again(dom, &anchors, record.item, viewport)
        else {
            continue;
        };
        // Where the box would be had nothing but its containing block moved.
        let (mx, my) = (now.0 - record.at.0, now.1 - record.at.1);
        let expected = LayoutRect::new(
            record.rect.x + mx,
            record.rect.y + my,
            record.rect.width,
            record.rect.height,
        );
        if (rect.width, rect.height) != (expected.width, expected.height) {
            result = None;
            break;
        }
        let (dx, dy) = (rect.x - expected.x, rect.y - expected.y);
        if (dx, dy) != (0, 0) {
            any = true;
            let mut more = Vec::new();
            match record.item {
                BoxItem::Node(id) => move_box(dom, id, dx, dy, &mut more),
                BoxItem::Generated(host, slot) => move_pseudo(dom, host, slot, (dx, dy)),
            }
            pending.extend(more);
        }
        j.positioned[k] = PlacedRecord {
            item: record.item,
            rect,
            at: (now.0 + dx, now.1 + dy),
        };
    }
    crate::render::layout_pass::intrinsic::end_pass(dom);
    result.map(|()| any)
}

/// Move `host`'s positioned `slot` pseudo-element's box.
fn move_pseudo(dom: &mut Dom<TuiExt>, host: NodeId, slot: crate::ext::PseudoSlot, d: (i32, i32)) {
    let mut node = dom.node_mut(host);
    let Some(ext) = node.ext_mut() else {
        return;
    };
    for anon in ext.positioned_pseudos.as_deref_mut().into_iter().flatten() {
        if anon.generated.is_some_and(|g| g.slot == slot) {
            shift::move_anon(anon, d);
        }
    }
}
