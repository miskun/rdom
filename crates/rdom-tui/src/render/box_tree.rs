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

fn push_sequence(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<BoxItem>) {
    for child in dom.node(id).child_nodes() {
        let child = child.id();
        if is_contents(dom, child) && holds_block_box(dom, child) {
            let pseudos = crate::render::inline::generated::visible_inline_pseudos(dom, child);
            if pseudos.before {
                out.push(BoxItem::Generated(child, PseudoSlot::Before));
            }
            push_sequence(dom, child, out);
            if pseudos.after {
                out.push(BoxItem::Generated(child, PseudoSlot::After));
            }
        } else {
            out.push(BoxItem::Node(child));
        }
    }
}

/// Whether `id`'s box-tree children include a block-level box: an
/// in-flow `display: block` element child, or a box-less child holding
/// one.
pub(crate) fn holds_block_box(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).child_nodes().any(|c| {
        let c = c.id();
        if c == id || dom.node(c).node_type() != NodeType::Element {
            return false;
        }
        if is_contents(dom, c) {
            return holds_block_box(dom, c);
        }
        crate::render::layout_pass::is_in_flow(dom, c)
            && dom
                .node(c)
                .computed()
                .is_none_or(|s| s.display == Display::Block)
    })
}

/// The static `::before` / `::after` text of a generated item.
pub(crate) fn generated_text(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> Option<&str> {
    crate::render::inline::generated::static_pseudo_text(dom, host, StyleSlot::from(slot))
}
