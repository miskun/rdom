//! Static position (CSS 2.1 §10.3.7 / §10.6.4): where an out-of-flow
//! box's hypothetical in-flow box would have gone, recorded by phase-1
//! block, inline and flex layout for an axis whose insets are both
//! `auto`.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, StaticPosition, TuiExt};
use crate::layout::{Display, LayoutRect, Position};
use crate::render::inline::InlineLayout;

/// Record where `id` would sit if it were `position: static`. Called
/// by phase-1 layout for each out-of-flow positioned child, in the
/// coordinate space of the parent's content area (scroll applied).
pub(in crate::render::layout_pass) fn record_static_position(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    x: i32,
    y: i32,
) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.static_position = Some(StaticPosition { x, y });
    }
}

/// `true` for an element that phase 1 leaves out of the flow because
/// phase 2 places it: `position: absolute | fixed` and not
/// `display: none` (which generates no box at all).
pub(in crate::render::layout_pass) fn is_out_of_flow_positioned(
    dom: &Dom<TuiExt>,
    id: NodeId,
) -> bool {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return false;
    }
    node.ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| {
            c.display != Display::None && matches!(c.position, Position::Absolute | Position::Fixed)
        })
}

/// The direct children of `parent` that [`is_out_of_flow_positioned`],
/// in document order.
pub(in crate::render::layout_pass) fn out_of_flow_positioned_children(
    dom: &Dom<TuiExt>,
    parent: NodeId,
) -> Vec<NodeId> {
    dom.node(parent)
        .child_nodes()
        .map(|c| c.id())
        .filter(|&c| is_out_of_flow_positioned(dom, c))
        .collect()
}

/// Group a flow's out-of-flow positioned children by the in-flow
/// sibling that follows them: `before[k]` are the positioned children
/// immediately ahead of in-flow child `k`; `trailing` are those after
/// the last in-flow child. Block layout records each group's static
/// position from its flow cursor just before it places `k`.
pub(in crate::render::layout_pass) fn static_anchors(
    dom: &Dom<TuiExt>,
    children: &[NodeId],
) -> (HashMap<NodeId, Vec<NodeId>>, Vec<NodeId>) {
    let mut before: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut bucket: Vec<NodeId> = Vec::new();
    for &c in children {
        if is_out_of_flow_positioned(dom, c) {
            bucket.push(c);
        } else if crate::render::layout_pass::is_in_flow(dom, c) && !bucket.is_empty() {
            before.insert(c, std::mem::take(&mut bucket));
        }
    }
    (before, bucket)
}

/// Static position of an out-of-flow child of an inline formatting
/// context. An inline-level hypothetical box continues the line after
/// the preceding in-flow content; a block-level one starts the next
/// line at the content's left edge (CSS 2.1 §9.2.1.1 + §10.3.7). With
/// no preceding in-flow content the box sits at the IFC's origin.
///
/// `layout` is the IFC's packed lines and `origin` the rect they were
/// packed into (the block's content area or the anonymous box's rect).
pub(in crate::render::layout_pass) fn static_position_in_ifc(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    child: NodeId,
    layout: &InlineLayout,
    origin: LayoutRect,
) -> (i32, i32) {
    // Fragments are owned by the (possibly nested) node that carries
    // their text; attribute each to the direct child of `parent` it
    // sits under so it can be ordered against `child`.
    let sibling_index: HashMap<NodeId, usize> = dom
        .node(parent)
        .child_nodes()
        .enumerate()
        .map(|(i, c)| (c.id(), i))
        .collect();
    let Some(&child_index) = sibling_index.get(&child) else {
        return (origin.x, origin.y);
    };
    let top_level_index = |mut node: NodeId| -> Option<usize> {
        loop {
            if let Some(&i) = sibling_index.get(&node) {
                return Some(i);
            }
            node = dom.node(node).parent_node()?.id();
        }
    };
    // The end of the preceding content: the furthest `(line, x)` any
    // ahead item reaches (items on a line are left to right, generated
    // runs interleaved with the fragments).
    let mut last: Option<(usize, i32)> = None;
    for (line_idx, line) in layout.lines.iter().enumerate() {
        // Generated content hosted by a child's subtree is ordered with
        // that child (an inline element's pseudos sit at its start /
        // end); otherwise it is the parent's own (or a list marker
        // riding this first line): its `::before` is ahead of every
        // child, its `::after` after them all.
        for g in &line.generated {
            let ahead = match top_level_index(g.host) {
                Some(i) => i < child_index,
                None => g.slot == PseudoSlot::Before,
            };
            if ahead {
                last = last.max(Some((line_idx, i32::from(g.x) + i32::from(g.width))));
            }
        }
        for f in &line.fragments {
            let owner = top_level_index(f.text_node).or_else(|| top_level_index(f.node));
            if owner.is_some_and(|i| i < child_index) {
                last = last.max(Some((line_idx, i32::from(f.x) + i32::from(f.width))));
            }
        }
    }
    let inline_level = dom
        .node(child)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| matches!(c.display, Display::Inline | Display::InlineBlock));
    match last {
        // On the line's baseline row (its text row), or below the line box.
        Some((line, end)) if inline_level => (
            origin.x + end,
            origin.y + i32::from(layout.lines[line].text_row()),
        ),
        Some((line, _)) => (origin.x, origin.y + i32::from(layout.lines[line].bottom())),
        None => (origin.x, origin.y),
    }
}

/// Record the static position of every out-of-flow positioned child
/// of the IFC `parent` (see [`static_position_in_ifc`]).
pub(in crate::render::layout_pass) fn record_static_positions_in_ifc(
    dom: &mut Dom<TuiExt>,
    parent: NodeId,
    layout: &InlineLayout,
    origin: LayoutRect,
) {
    for n in out_of_flow_positioned_children(dom, parent) {
        let (x, y) = static_position_in_ifc(dom, parent, n, layout, origin);
        record_static_position(dom, n, x, y);
    }
}
