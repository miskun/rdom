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
//! - `fallback` — the position options, their order and the one used.
//! - `visibility` — `position-visibility`: the anchored boxes hidden.
//!
//! A page without anchor positioning pays one `is_anchored` test per
//! positioned box; the anchor index is built on the first anchored one.

mod fallback;
mod lookup;
mod resolve;
#[cfg(test)]
mod tests;
pub(crate) mod visibility;

use rdom_core::{Dom, NodeId};

pub(in crate::render::layout_pass) use lookup::AnchorIndex;
#[cfg(test)]
pub(in crate::render::layout_pass) use lookup::INDEX_BUILDS;
pub(in crate::render::layout_pass) use lookup::Querying;
pub(in crate::render::layout_pass) use resolve::is_anchored;

use super::rect::{Placed, compute_placed_rect};
use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Position, TextDirection};
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

/// Where a positioned box goes: its border box, and whether
/// `position-visibility` hides it (`visibility`).
pub(in crate::render::layout_pass) struct Placement {
    pub(in crate::render::layout_pass) rect: LayoutRect,
    pub(in crate::render::layout_pass) hidden: bool,
}

/// Place the positioned box `placed` (asked for by `querying`), styled
/// `c`, in its containing block `cb`: through its anchors and position
/// options when it has any anchor reference (`resolve`, `fallback`), else
/// as CSS 2.1 places it.
pub(in crate::render::layout_pass) fn placed_rect(
    dom: &Dom<TuiExt>,
    index: &AnchorIndex,
    placed: Placed<'_>,
    querying: Querying,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> Placement {
    if !resolve::is_anchored(c) {
        return Placement {
            rect: compute_placed_rect(dom, placed, c, cb),
            hidden: false,
        };
    }
    let an = anchoring(dom, index, querying, c, cb);
    let options = fallback::options(c);
    // Each option resolved, placed, and its inset-modified containing
    // block (§4.3).
    let tried: Vec<(LayoutRect, LayoutRect)> = options
        .iter()
        .map(|o| {
            // An option may name another default anchor.
            let own = (o.style.anchor.position_anchor != c.anchor.position_anchor)
                .then(|| anchoring(dom, index, querying, &o.style, cb));
            let an = own.as_ref().unwrap_or(&an);
            let (style, area) = resolve::resolve_style(&o.style, &o.tactics, an);
            (
                compute_placed_rect(dom, placed, &style, area),
                resolve::inset_modified(&style, area),
            )
        })
        .collect();
    let sizes: Vec<(u16, u16)> = tried.iter().map(|(_, m)| (m.width, m.height)).collect();
    let order = fallback::order(c.anchor.position_try_order, &sizes);
    let fits = |(rect, imcb): &(LayoutRect, LayoutRect)| {
        rect.x >= imcb.x
            && rect.y >= imcb.y
            && rect.x + i32::from(rect.width) <= imcb.x + i32::from(imcb.width)
            && rect.y + i32::from(rect.height) <= imcb.y + i32::from(imcb.height)
    };
    // §4.3 "determine position fallback styles": the first option in
    // order that avoids overflow — the base style sorted with the others
    // (§4.2) — else "Return current styles", the base style's (option 0;
    // no last successful option is remembered).
    let chosen = order.iter().copied().find(|&i| fits(&tried[i]));
    let rect = tried[chosen.unwrap_or(0)].0;
    let v = &c.anchor.position_visibility;
    let hidden = (v.no_overflow && chosen.is_none())
        || (v.anchors_valid && !an.references_resolve(c))
        || (v.anchors_visible
            && an
                .default_node
                .zip(an.default)
                .is_some_and(|(a, r)| visibility::clipped_out(dom, a, r)));
    Placement { rect, hidden }
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
    let default_node =
        lookup::default_anchor(dom, index, querying, cb_element, &c.anchor.position_anchor);
    let default = default_node.and_then(|a| lookup::anchor_box(dom, a));
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
        default_node,
        cb_rtl: rtl(parent),
        self_rtl: c.text_direction == TextDirection::Rtl,
    }
}
