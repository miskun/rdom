//! The content-based minimum size of an item (CSS Flexbox §4.5, which
//! CSS Grid 2 §6.6 adopts for grid items): the automatic minimum a flex
//! or grid item's `min-width` / `min-height: auto` resolves to.

use rdom_core::Dom;

use super::Item;
use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
use crate::node::TuiNodeExt;

/// What [`content_based_minimum`] measures against.
#[derive(Debug, Clone, Copy)]
pub(in crate::render::layout_pass) struct Suggestion {
    /// The basis the item's specified size resolves a percentage
    /// against — the flex container's main size; `None` where it is
    /// indefinite (a grid area being sized), so a percentage gives no
    /// specified size suggestion.
    pub(in crate::render::layout_pass) basis: Option<u16>,
    /// The space a `fit-content` specified size fits into.
    pub(in crate::render::layout_pass) available: u16,
    /// The extent the content is measured against on the other axis.
    pub(in crate::render::layout_pass) cross_budget: u16,
    /// The containing block's width, the basis of padding percentages.
    pub(in crate::render::layout_pass) cb_width: u16,
}

/// The content-based minimum size of `item` along `direction`, as a
/// border box (CSS Flexbox §4.5): the smaller of its specified size
/// suggestion (its definite preferred size) and its content size
/// suggestion (its min-content size), and at least its padding and
/// border. A scroll container's is just its padding and border (the
/// §4.5 exception: its content may overflow). Not clamped by the
/// item's `max-*`: the caller does that.
pub(in crate::render::layout_pass) fn content_based_minimum(
    dom: &Dom<TuiExt>,
    item: &Item,
    direction: Direction,
    at: Suggestion,
) -> u16 {
    // An element without a computed style has no box to size.
    if let Item::Element(id) = item
        && dom.node(*id).computed().is_none()
    {
        return 0;
    }
    let computed = item.computed(dom);
    let main_size = match direction {
        Direction::Row => &computed.width,
        Direction::Column => &computed.height,
    };
    // Whatever the suggestion, the content box is never negative: the
    // floor is at least the item's padding and border on the axis (CSS
    // Flexbox §9.7 clamps the target main size to the content box's 0).
    let kw = item.keywords(dom, &computed, direction, at.cross_budget, at.cb_width);
    let sizer = kw.sizer();
    // CSS §4.5 exception: a scroll container's floor is 0 — it may be
    // sized below its content (CSS Grid 2 §6.6 alike). An `overflow:
    // clip` item is not one (CSS Overflow 3 §3.1) and keeps the floor.
    if computed.is_scroll_container() {
        return sizer.chrome();
    }
    // Specified size suggestion per spec: the declared main size, as
    // the border box `box-sizing` makes of it (CSS UI 3 §3.1).
    let specified_cap: Option<u16> = match main_size {
        Size::Flex(_) => Some(0),
        // A keyword height is the automatic size: no cap (CSS Sizing 3
        // §3.1); a keyword width is its content size.
        Size::Intrinsic(_) if direction == Direction::Column => None,
        definite => kw.size(definite, at.basis, at.available),
    };
    // `flex: N` (basis 0%) trivially has specified=0, so auto-min
    // = min(content, 0) = 0. Skip the content walk.
    if matches!(specified_cap, Some(0)) {
        return sizer.chrome();
    }
    let content = item.content_extreme(dom, direction, at.cross_budget, at.cb_width, false);
    sizer.floor(match specified_cap {
        Some(cap) => content.min(cap),
        None => content,
    })
}
