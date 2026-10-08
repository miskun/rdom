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
//!
//! Inside a flex or grid container every box-less child is its children
//! in its place, with its `::before` / `::after` around them, and the
//! container's own `::before` / `::after` first and last
//! ([`item_sequence`]): each element and pseudo-element there is an item,
//! and each run of text an anonymous one (CSS Flexbox §4, CSS Grid 2
//! §6.1).
//!
//! A `<details>` element's `::details-content` slot is a box between it
//! and its content ([`slot`]): [`children`] and [`box_parent`] see it, so
//! every walk here does.

pub(crate) mod icb;
pub(crate) mod slot;

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
    let mut parent = slot::parent(dom, id);
    while let Some(p) = parent
        && is_contents(dom, p)
    {
        parent = slot::parent(dom, p);
    }
    parent
}

/// `id`'s children in the box tree, in tree order: its child nodes — or,
/// for a `<details>` with a `::details-content` box, its first `<summary>`
/// and that box; for the box, the `<details>`'s other children
/// ([`slot`]). Walked in place, both ways.
pub(crate) fn children(dom: &Dom<TuiExt>, id: NodeId) -> PaintOrder<'_> {
    PaintOrder::tree(dom, id)
}

/// One item of a block container's inline or block content: a child
/// node, or the static `::before` / `::after` of a box-less child that
/// holds a block-level box (its pseudo-elements are inline boxes in
/// the container's flow, CSS Display 3 §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/// its inline-level static `::before` / `::after`. A box-less child
/// holding only inline-level content stays one item (an inline-level
/// one). `id`'s own block-level and floated `::before` / `::after`
/// (`generated::sequence_pseudos`) are its first / last items: block-level
/// boxes and floats of its flow.
pub(crate) fn box_sequence(dom: &Dom<TuiExt>, id: NodeId) -> Vec<BoxItem> {
    let mut out = Vec::new();
    let own = crate::render::inline::generated::sequence_pseudos(dom, id);
    if own.before {
        out.push(BoxItem::Generated(id, PseudoSlot::Before));
    }
    push_sequence(dom, id, &mut out);
    if own.after {
        out.push(BoxItem::Generated(id, PseudoSlot::After));
    }
    out
}

/// The first (`from_end = false`) or last item of `id`'s [`box_sequence`]
/// that `pred` accepts, found by walking the child list in place from that
/// end: it visits the items before the one it finds, not the whole
/// sequence, and allocates nothing (C10G-MARKER-COST — a line's lookups
/// run once per packed flow, and a whole sequence per lookup made sibling
/// rows quadratic).
pub(crate) fn find_in_sequence(
    dom: &Dom<TuiExt>,
    id: NodeId,
    from_end: bool,
    pred: &mut dyn FnMut(BoxItem) -> bool,
) -> Option<BoxItem> {
    let own = crate::render::inline::generated::sequence_pseudos(dom, id);
    find_framed(dom, id, (own.before, own.after), from_end, pred)
}

/// [`find_in_sequence`] over `id`'s box-tree children framed by its
/// `::before` / `::after` items where `pseudos` has them.
fn find_framed(
    dom: &Dom<TuiExt>,
    id: NodeId,
    (before, after): (bool, bool),
    from_end: bool,
    pred: &mut dyn FnMut(BoxItem) -> bool,
) -> Option<BoxItem> {
    let before = before.then_some(BoxItem::Generated(id, PseudoSlot::Before));
    let after = after.then_some(BoxItem::Generated(id, PseudoSlot::After));
    let (lead, trail) = if from_end {
        (after, before)
    } else {
        (before, after)
    };
    if let Some(g) = lead.filter(|&g| pred(g)) {
        return Some(g);
    }
    let mut each = |child: NodeId| {
        visit();
        if is_hidden_text(dom, child) {
            return None;
        }
        // A box-less child holding a block-level box is its items in its
        // place, between its inline-level `::before` / `::after`
        // ([`push_sequence`]); any other child is one item.
        if is_contents(dom, child) && holds_block_box(dom, child) {
            let p = crate::render::inline::generated::inline_level_pseudos(dom, child);
            return find_framed(dom, child, (p.before, p.after), from_end, pred);
        }
        let item = BoxItem::Node(child);
        pred(item).then_some(item)
    };
    let mut children = PaintOrder::tree(dom, id);
    let found = if from_end {
        children.rev().find_map(&mut each)
    } else {
        children.find_map(&mut each)
    };
    found.or_else(|| trail.filter(|&g| pred(g)))
}

/// Push `id`'s box-tree children onto `out`; whether they include a
/// block-level box. One walk decides and collects: a box-less child's
/// items are collected in place and kept when they hold a block box,
/// else replaced by the child itself — so each node is visited once,
/// however deeply box-less elements nest.
fn push_sequence(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<BoxItem>) -> bool {
    let mut holds = false;
    for child in children(dom, id) {
        visit();
        if is_hidden_text(dom, child) {
            continue;
        }
        if is_contents(dom, child) {
            let mark = out.len();
            let pseudos = crate::render::inline::generated::inline_level_pseudos(dom, child);
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

/// The flex or grid container `id`'s box-tree children (CSS Display 3
/// §2.5, CSS Flexbox §4, CSS Grid 2 §6.1): its visible static `::before`,
/// its child nodes — every box-less child (and fragment) replaced by its
/// own, between its visible static `::before` / `::after` — and its
/// visible static `::after`. Every element in it is an item, and every
/// pseudo-element that generates a box (a child box, so blockified —
/// `content: ""` included); its text nodes form the anonymous items'
/// runs. Anonymous items' `child_range`s index it.
pub(crate) fn item_sequence(dom: &Dom<TuiExt>, id: NodeId) -> Vec<BoxItem> {
    let mut out = Vec::new();
    if generates_static_pseudo(dom, id, PseudoSlot::Before) {
        out.push(BoxItem::Generated(id, PseudoSlot::Before));
    }
    push_item_sequence(dom, id, &mut out);
    if generates_static_pseudo(dom, id, PseudoSlot::After) {
        out.push(BoxItem::Generated(id, PseudoSlot::After));
    }
    out
}

/// `host`'s `slot` pseudo-element generates an in-flow box: it has
/// `content` (CSS 2.1 §12.1), is not `display: none` and not absolutely
/// or fixed positioned (phase-2 placement lays that out,
/// `positioning::pseudo`).
fn generates_static_pseudo(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> bool {
    let computed = dom.node(host).computed_pseudo(slot);
    computed.is_some_and(|c| c.display != Display::None)
        && generated_text(dom, host, slot).is_some()
}

fn push_item_sequence(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<BoxItem>) {
    for child in children(dom, id) {
        visit();
        if is_hidden_text(dom, child) {
            continue;
        }
        match dom.node(child).node_type() {
            NodeType::Element if is_contents(dom, child) => {
                if generates_static_pseudo(dom, child, PseudoSlot::Before) {
                    out.push(BoxItem::Generated(child, PseudoSlot::Before));
                }
                push_item_sequence(dom, child, out);
                if generates_static_pseudo(dom, child, PseudoSlot::After) {
                    out.push(BoxItem::Generated(child, PseudoSlot::After));
                }
            }
            NodeType::Fragment => push_item_sequence(dom, child, out),
            _ => out.push(BoxItem::Node(child)),
        }
    }
}

/// `child` is text a `<details>` element's closed content slot hides
/// (`style::cascade::details`): it generates no box. (Hidden element
/// content is `display: none`.)
pub(crate) fn is_hidden_text(dom: &Dom<TuiExt>, child: NodeId) -> bool {
    let node = dom.node(child);
    node.node_type() == NodeType::Text
        && node
            .parent_node()
            .is_some_and(|p| crate::style::cascade::details::hidden(dom, p.id(), child))
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
    children(dom, id).any(|c| {
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
/// `::after` (CSS Display 3 §2.5). A block container whose only content
/// this is packs it as a pure-text leaf; a flex or grid container's text
/// is its anonymous items' (CSS Flexbox §4, `layout_pass::items`).
pub(crate) fn holds_loose_text(
    dom: &Dom<TuiExt>,
    id: NodeId,
    text: &impl Fn(&str) -> bool,
) -> bool {
    children(dom, id).any(|c| match dom.node(c).node_type() {
        NodeType::Text => !is_hidden_text(dom, c) && dom.node(c).node_value().is_some_and(text),
        NodeType::Element if is_contents(dom, c) => {
            let p = crate::render::inline::generated::visible_inline_pseudos(dom, c);
            p.before || p.after || holds_loose_text(dom, c, text)
        }
        _ => false,
    })
}

/// The static `::before` / `::after` text of a generated item.
pub(crate) fn generated_text(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> Option<&str> {
    crate::render::inline::generated::static_pseudo_text(dom, host, StyleSlot::from(slot))
}

/// `id` is an element flex container (`display: flex` / `inline-flex`).
/// The document root is none: its box is the initial containing block, a
/// block container (`icb`).
pub(crate) fn is_flex_container(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.node_type() == NodeType::Element
        && node
            .computed()
            .is_some_and(|c| c.flow == crate::layout::Flow::Flex)
}

/// `id` is an element flex or grid container: its in-flow children are
/// items (`Flow::is_flex_or_grid`) — reordered by `order`, painted
/// atomically, its text runs anonymous items.
pub(crate) fn is_flex_or_grid_container(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.node_type() == NodeType::Element
        && node.computed().is_some_and(|c| c.flow.is_flex_or_grid())
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
/// flex or grid container's are its items (through fragments and
/// box-less children) in order-modified document order — CSS Flexbox
/// §5.4, CSS Grid 2 §6.3 / §6.5:
/// `order` affects painting, and so hit-testing, as it does layout.
/// Walks the child list in place, both ways; only a flex or grid
/// container with an item whose `order` is not 0 collects (and sorts)
/// its items.
pub(crate) fn paint_order_children(dom: &Dom<TuiExt>, id: NodeId) -> PaintOrder<'_> {
    if is_flex_or_grid_container(dom, id) && any_reordered(dom, id) {
        let mut items = crate::render::layout_pass::element_children_of(dom, id);
        sort_by_order(dom, &mut items);
        return PaintOrder::Sorted(items.into_iter());
    }
    PaintOrder::tree(dom, id)
}

/// Whether one of the flex or grid container `id`'s items (through fragments
/// and box-less children) has an `order` other than 0. Allocates
/// nothing.
fn any_reordered(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    children(dom, id).any(|c| match dom.node(c).node_type() {
        NodeType::Element if is_contents(dom, c) => any_reordered(dom, c),
        NodeType::Element => order_of(dom, c) != 0,
        NodeType::Fragment => any_reordered(dom, c),
        _ => false,
    })
}

/// [`children`] and [`paint_order_children`]: a node's box-tree
/// children, walked in place from either end, or a list of them (a flex
/// container's reordered items; a `<details>`'s summary and slot box).
pub(crate) enum PaintOrder<'a> {
    /// The child list between `front` and `back`, inclusive, without
    /// `skip` (the summary a `::details-content` box leaves out).
    Tree {
        dom: &'a Dom<TuiExt>,
        front: Option<NodeId>,
        back: Option<NodeId>,
        skip: Option<NodeId>,
    },
    /// Items in order-modified document order.
    Sorted(std::vec::IntoIter<NodeId>),
}

impl<'a> PaintOrder<'a> {
    /// `id`'s box-tree children in tree order ([`children`]).
    pub(crate) fn tree(dom: &'a Dom<TuiExt>, id: NodeId) -> Self {
        use crate::ext::ContentBoxLink;
        let link = dom
            .node(id)
            .ext()
            .map_or(ContentBoxLink::None, |e| e.content_box_link());
        let (list, skip) = match link {
            ContentBoxLink::None => (id, None),
            ContentBoxLink::Box(slot) => {
                let summary = crate::style::cascade::details::summary(dom, id);
                return PaintOrder::Sorted(
                    summary
                        .into_iter()
                        .chain([slot])
                        .collect::<Vec<_>>()
                        .into_iter(),
                );
            }
            ContentBoxLink::HostedBy(host) => {
                (host, crate::style::cascade::details::summary(dom, host))
            }
        };
        let node = dom.node(list);
        PaintOrder::Tree {
            dom,
            front: node.first_child().map(|c| c.id()),
            back: node.last_child().map(|c| c.id()),
            skip,
        }
    }
}

impl Iterator for PaintOrder<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        match self {
            PaintOrder::Tree {
                dom,
                front,
                back,
                skip,
            } => loop {
                let cur = (*front)?;
                if Some(cur) == *back {
                    *front = None;
                    *back = None;
                } else {
                    *front = dom.node(cur).next_sibling().map(|n| n.id());
                }
                if Some(cur) != *skip {
                    return Some(cur);
                }
            },
            PaintOrder::Sorted(items) => items.next(),
        }
    }
}

impl DoubleEndedIterator for PaintOrder<'_> {
    fn next_back(&mut self) -> Option<NodeId> {
        match self {
            PaintOrder::Tree {
                dom,
                front,
                back,
                skip,
            } => loop {
                let cur = (*back)?;
                if Some(cur) == *front {
                    *front = None;
                    *back = None;
                } else {
                    *back = dom.node(cur).previous_sibling().map(|n| n.id());
                }
                if Some(cur) != *skip {
                    return Some(cur);
                }
            },
            PaintOrder::Sorted(items) => items.next_back(),
        }
    }
}
