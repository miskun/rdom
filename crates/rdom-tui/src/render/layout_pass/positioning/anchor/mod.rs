//! Anchor positioning (CSS Anchor Positioning 1): an absolutely positioned
//! box placed against other boxes, its anchors, once they are laid out —
//! phase 2 places positioned boxes after the in-flow layout, in tree
//! order, so an in-flow anchor is always laid out and a positioned one is
//! when it comes first (`lookup`). An anchor's box is read as it is laid
//! out: inside a scroll container, scrolled — rdom lays out again on a
//! scroll, so an anchored box follows its anchor (§3's remembered scroll
//! offset, taken every layout).
//!
//! - `lookup` — which element an anchor name, or a box's default anchor,
//!   is.
//! - `resolve` — a style's anchor references made cells and containing
//!   blocks.
//!
//! A page without anchor positioning pays one `is_anchored` test per
//! positioned box; the anchor index is built on the first anchored one.

mod lookup;
mod resolve;
#[cfg(test)]
mod tests;

use rdom_core::{Dom, NodeId};

pub(in crate::render::layout_pass) use lookup::AnchorIndex;
#[cfg(test)]
pub(in crate::render::layout_pass) use lookup::INDEX_BUILDS;
pub(in crate::render::layout_pass) use lookup::Querying;

use super::place::{Placed, compute_placed_rect};
use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Position, TextDirection};
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

/// Place the positioned box `placed` (asked for by `querying`), styled
/// `c`, in its containing block `cb`: through its anchors when it has any
/// anchor reference (`resolve`), else as CSS 2.1 places it.
pub(in crate::render::layout_pass) fn placed_rect(
    dom: &Dom<TuiExt>,
    index: &AnchorIndex,
    placed: Placed<'_>,
    querying: Querying,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> LayoutRect {
    if !resolve::is_anchored(c) {
        return compute_placed_rect(dom, placed, c, cb);
    }
    let an = anchoring(dom, index, querying, c, cb);
    let (style, cb) = resolve::resolve_style(c, &an);
    compute_placed_rect(dom, placed, &style, cb)
}

/// What resolving `c`'s anchor references needs (`resolve::Anchoring`).
fn anchoring<'a>(
    dom: &'a Dom<TuiExt>,
    index: &'a AnchorIndex,
    querying: Querying,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> resolve::Anchoring<'a> {
    let parent = if querying.pseudo {
        Some(querying.node)
    } else {
        crate::render::box_tree::box_parent(dom, querying.node)
    };
    // The containing block's element: none for a top-layer box or one
    // whose containing block is the viewport.
    let cb_element = if !querying.pseudo && dom.is_in_top_layer(querying.node) {
        None
    } else if c.position == Position::Fixed {
        super::fixed_containing_ancestor(dom, parent)
    } else {
        super::containing_ancestor(dom, parent)
    };
    let default =
        lookup::default_anchor(dom, index, querying, cb_element, &c.anchor.position_anchor)
            .and_then(|a| lookup::anchor_box(dom, a));
    let rtl = |id: Option<NodeId>| {
        id.and_then(|p| dom.node(p).computed().map(|pc| pc.text_direction))
            .unwrap_or(c.text_direction)
            == TextDirection::Rtl
    };
    resolve::Anchoring {
        dom,
        index,
        querying,
        cb_element,
        cb,
        default,
        cb_rtl: rtl(parent),
        self_rtl: c.text_direction == TextDirection::Rtl,
    }
}
