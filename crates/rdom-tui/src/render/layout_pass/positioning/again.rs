//! A positioned box placed again where it is already laid out — what a
//! scroll update asks of a box whose placement may have changed with the
//! scroll (`scroll_update`): an anchor-positioned box (its anchors may have
//! moved apart from it, CSS Anchor Positioning 1 §3 — rdom remembers the
//! scroll offset of every layout) or one whose static position moved with
//! the scrolled content while its containing block did not. Phase 2's
//! placement (`place`), without the layout of the box's content: the
//! caller moves the laid-out box, or lays out again when its size changed.

use rdom_core::{Dom, NodeId};

use super::anchor::{AnchorIndex, Querying};
use super::place::Placed;
use super::*;
use crate::ext::{PseudoSlot, TuiExt};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::items::AnonymousItem;
use crate::style::ComputedStyle;

/// Where `item` is placed now — `position-visibility` recording it hidden
/// as phase 2 does — and where its border box is laid out: `None` when it
/// has no box.
pub(in crate::render::layout_pass) fn place_again(
    dom: &mut Dom<TuiExt>,
    anchors: &AnchorIndex,
    item: BoxItem,
    viewport: LayoutRect,
) -> Option<(LayoutRect, (i32, i32))> {
    match item {
        BoxItem::Node(id) => {
            let at = dom.node(id).ext().map(|e| (e.layout.x, e.layout.y))?;
            let cb = containing_block(dom, id, viewport);
            let computed = dom
                .node(id)
                .computed_rc()
                .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
            let querying = Querying {
                node: id,
                pseudo: false,
            };
            let placed = super::anchor::placed_rect(
                dom,
                anchors,
                Placed::Element(id),
                querying,
                &computed,
                cb,
            );
            if placed.hidden {
                super::anchor::visibility::hide(dom, id, None);
            }
            Some((placed.rect, at))
        }
        BoxItem::Generated(host, slot) => {
            let at = pseudo_box(dom, host, slot).map(|r| (r.x, r.y))?;
            let item = AnonymousItem::pseudo(dom, host, slot)?;
            let style = item.style_rc();
            let cb = absolute_containing_block(dom, Some(host), &style, viewport);
            let querying = Querying {
                node: host,
                pseudo: true,
            };
            let placed = super::anchor::placed_rect(
                dom,
                anchors,
                Placed::Generated {
                    host,
                    slot,
                    item: &item,
                },
                querying,
                &style,
                cb,
            );
            if placed.hidden {
                super::anchor::visibility::hide(dom, host, Some(slot));
            }
            Some((placed.rect, at))
        }
    }
}

/// The border box of `host`'s positioned `slot` pseudo-element.
fn pseudo_box(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> Option<LayoutRect> {
    dom.node(host)
        .ext()?
        .positioned_pseudo_boxes()
        .iter()
        .filter_map(|a| a.generated)
        .find(|g| g.slot == slot)
        .map(|g| g.border_box)
}

/// Whether `item`'s style asks anything of anchor positioning (its
/// placement then depends on other boxes, which a scroll may move apart
/// from it).
pub(in crate::render::layout_pass) fn is_anchored(dom: &Dom<TuiExt>, item: BoxItem) -> bool {
    match item {
        BoxItem::Node(id) => dom
            .node(id)
            .computed()
            .is_some_and(super::anchor::is_anchored),
        BoxItem::Generated(host, slot) => dom
            .node(host)
            .computed_pseudo(slot)
            .is_some_and(super::anchor::is_anchored),
    }
}
