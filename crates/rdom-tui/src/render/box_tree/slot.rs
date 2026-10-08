//! The `::details-content` box (HTML §15.5.20, CSS Pseudo-Elements 4): a
//! generated block box in the box tree that adopts element content.
//!
//! HTML renders a `<details>` with a shadow tree of two slots: the first
//! takes the element's first `<summary>` child, the second — the one
//! `::details-content` styles — everything else. rdom has no shadow tree;
//! the second slot is a box of its own all the same. The cascade keeps,
//! for each `<details>` that has a `::details-content` style, a node
//! outside the document (`style::cascade::details::sync_content_box`)
//! whose computed style is that style; the box tree puts it between the
//! element and its content:
//!
//! - the `<details>`'s box-tree children are its first `<summary>` (if
//!   any) and the slot's box — in that order, wherever the summary is
//!   among the child nodes;
//! - the slot's box-tree children are the `<details>`'s other children,
//!   in tree order;
//! - [`parent`] climbs from slotted content to the box, and from the box
//!   to its `<details>`.
//!
//! So layout, paint, scrolling, intrinsic sizes and margin collapsing
//! treat the slot as the element box it is — its own rects, scroll
//! offsets, line boxes and transitions on its node — through the same
//! walks every box takes ([`children`](super::children)). What faces the
//! DOM never names the node: the hit test reports its `<details>`
//! (`runtime::hit_test`), and its transition events fire on the
//! `<details>` with `pseudoElement` `"::details-content"`.

use rdom_core::{Dom, NodeId};

use crate::ext::{ContentBoxLink, TuiExt};

/// The box of `host`'s `::details-content` slot: `host` is a `<details>`
/// whose slot has a style.
pub(crate) fn content_box(dom: &Dom<TuiExt>, host: NodeId) -> Option<NodeId> {
    match dom.node(host).ext()?.content_box_link() {
        ContentBoxLink::Box(b) => Some(b),
        _ => None,
    }
}

/// The `<details>` whose `::details-content` slot `id` is the box of.
pub(crate) fn host_of(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    match dom.node(id).ext()?.content_box_link() {
        ContentBoxLink::HostedBy(h) => Some(h),
        _ => None,
    }
}

/// `id`'s parent in the box tree's view of the nodes: its parent node,
/// except that a child slotted into a `::details-content` box has that
/// box, and the box has its `<details>`. Every climb that looks for a
/// box — a containing block, a scroll container, the inline formatting
/// context a text belongs to — goes through here.
pub(crate) fn parent(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    if let Some(host) = host_of(dom, id) {
        return Some(host);
    }
    let p = dom.node(id).parent_node()?.id();
    match content_box(dom, p) {
        Some(b) if crate::style::cascade::details::slotted(dom, p, id) => Some(b),
        _ => Some(p),
    }
}
