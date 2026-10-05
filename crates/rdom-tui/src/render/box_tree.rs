//! The box tree's view of the DOM (CSS Display 3 §2.5): which nodes
//! generate boxes, and the children a block container lays out once
//! `display: contents` elements are taken out.
//!
//! A `display: contents` element generates no box: its children (and
//! its `::before` / `::after`) are its parent's children in the box
//! tree. Every walk that pairs a box with its children — layout, paint,
//! hit-testing, intrinsic sizes — reads them through here rather than
//! through `child_nodes()`.
//!
//! Inside a block container a box-less element is one of two things:
//! an inline-level participant, when it holds no block-level box — its
//! content and pseudo-elements then pack in the parent's line at its
//! turn, as an inline box with no decoration would; or, when it holds a
//! block-level box, its children in its place ([`box_sequence`]), with
//! its static `::before` / `::after` as inline items around them.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, StyleSlot, TuiExt};
use crate::layout::Display;
use crate::node::TuiNodeExt;

#[cfg(test)]
#[path = "box_tree_tests.rs"]
mod tests;

/// Count a child node a box-tree walk looks at (tests only).
fn visit() {
    #[cfg(test)]
    tests::VISITS.with(|c| c.set(c.get() + 1));
}

/// `id` is an element whose computed `display` is `contents`.
pub(crate) fn is_contents(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.node_type() == NodeType::Element
        && node
            .computed()
            .is_some_and(|c| c.display == Display::Contents)
}

/// The nearest ancestor of `id` that is not a box-less element: the
/// box `id`'s box is laid out in (its containing block for in-flow
/// content, CSS 2.1 §10.1).
pub(crate) fn box_parent(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    let mut parent = dom.node(id).parent_node().map(|p| p.id());
    while let Some(p) = parent
        && is_contents(dom, p)
    {
        parent = dom.node(p).parent_node().map(|n| n.id());
    }
    parent
}

/// One item of a block container's inline or block content: a child
/// node, or the static `::before` / `::after` of a box-less child that
/// holds a block-level box (its pseudo-elements are inline boxes in
/// the container's flow, CSS Display 3 §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoxItem {
    Node(NodeId),
    Generated(NodeId, PseudoSlot),
}

impl BoxItem {
    /// The node, for a node item.
    pub(crate) fn node(self) -> Option<NodeId> {
        match self {
            BoxItem::Node(n) => Some(n),
            BoxItem::Generated(..) => None,
        }
    }
}

/// `id`'s child nodes in box-tree order: each `display: contents` child
/// that holds a block-level box replaced by its own sequence, between
/// its visible static `::before` / `::after`. A box-less child holding
/// only inline-level content stays one item (an inline-level one).
pub(crate) fn box_sequence(dom: &Dom<TuiExt>, id: NodeId) -> Vec<BoxItem> {
    let mut out = Vec::new();
    push_sequence(dom, id, &mut out);
    out
}

/// Push `id`'s box-tree children onto `out`; whether they include a
/// block-level box. One walk decides and collects: a box-less child's
/// items are collected in place and kept when they hold a block box,
/// else replaced by the child itself — so each node is visited once,
/// however deeply box-less elements nest.
fn push_sequence(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<BoxItem>) -> bool {
    let mut holds = false;
    for child in dom.node(id).child_nodes() {
        let child = child.id();
        visit();
        if is_contents(dom, child) {
            let mark = out.len();
            let pseudos = crate::render::inline::generated::visible_inline_pseudos(dom, child);
            if pseudos.before {
                out.push(BoxItem::Generated(child, PseudoSlot::Before));
            }
            if push_sequence(dom, child, out) {
                if pseudos.after {
                    out.push(BoxItem::Generated(child, PseudoSlot::After));
                }
                holds = true;
            } else {
                out.truncate(mark);
                out.push(BoxItem::Node(child));
            }
        } else {
            holds |= is_block_level_in_flow(dom, child);
            out.push(BoxItem::Node(child));
        }
    }
    holds
}

/// `id` is an in-flow `display: block` element.
fn is_block_level_in_flow(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).node_type() == NodeType::Element
        && crate::render::layout_pass::is_in_flow(dom, id)
        && dom
            .node(id)
            .computed()
            .is_none_or(|s| s.display == Display::Block)
}

/// Whether `id`'s box-tree children include a block-level box: an
/// in-flow `display: block` element child, or a box-less child holding
/// one.
pub(crate) fn holds_block_box(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).child_nodes().any(|c| {
        let c = c.id();
        visit();
        if is_contents(dom, c) {
            holds_block_box(dom, c)
        } else {
            is_block_level_in_flow(dom, c)
        }
    })
}

/// Whether `id`'s box-tree children include inline content outside any
/// element box: a text child whose data satisfies `text`, or — through
/// a box-less child — such a text, or a visible static `::before` /
/// `::after` (CSS Display 3 §2.5). A flex container whose only content
/// this is lays it out as one anonymous item (CSS Flexbox §4).
pub(crate) fn holds_loose_text(
    dom: &Dom<TuiExt>,
    id: NodeId,
    text: &impl Fn(&str) -> bool,
) -> bool {
    dom.node(id).child_nodes().any(|c| match c.node_type() {
        NodeType::Text => c.node_value().is_some_and(text),
        NodeType::Element if is_contents(dom, c.id()) => {
            let p = crate::render::inline::generated::visible_inline_pseudos(dom, c.id());
            p.before || p.after || holds_loose_text(dom, c.id(), text)
        }
        _ => false,
    })
}

/// The static `::before` / `::after` text of a generated item.
pub(crate) fn generated_text(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> Option<&str> {
    crate::render::inline::generated::static_pseudo_text(dom, host, StyleSlot::from(slot))
}

/// `id` is an element flex container (`display: flex` / `inline-flex`).
/// The document root's children are flex items of rdom's viewport
/// column only as a layout device, so the root is not one.
pub(crate) fn is_flex_container(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.node_type() == NodeType::Element
        && node
            .computed()
            .is_some_and(|c| c.flow == crate::layout::Flow::Flex)
}

/// A flex item's `order` (CSS Flexbox §5.4); 0 for a child that is not
/// a flex item (out of flow: absolutely positioned or `display: none`).
pub(crate) fn order_of(dom: &Dom<TuiExt>, id: NodeId) -> i32 {
    let in_flow = crate::render::layout_pass::is_in_flow(dom, id);
    dom.node(id)
        .computed()
        .filter(|_| in_flow)
        .map_or(0, |c| c.order)
}

/// Sort `items` — flex items in document order — into order-modified
/// document order (CSS Flexbox §5.4): ascending `order`, document order
/// among equals (a stable sort). No-op when every `order` is 0.
pub(crate) fn sort_by_order(dom: &Dom<TuiExt>, items: &mut [NodeId]) {
    if items.iter().any(|&c| order_of(dom, c) != 0) {
        items.sort_by_key(|&c| order_of(dom, c));
    }
}

/// The children of `id` in paint order: its child nodes, except that a
/// flex container's are its items (through fragments and box-less
/// children) in order-modified document order — CSS Flexbox §5.4:
/// `order` affects painting, and so hit-testing, as it does layout.
pub(crate) fn paint_order_children(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    if is_flex_container(dom, id) {
        let mut items = crate::render::layout_pass::element_children_of(dom, id);
        if items.iter().any(|&c| order_of(dom, c) != 0) {
            sort_by_order(dom, &mut items);
            return items;
        }
    }
    dom.node(id).child_nodes().map(|c| c.id()).collect()
}
