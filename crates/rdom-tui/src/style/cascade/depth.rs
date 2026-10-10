//! The cascade's depth cap (C16G-DEPTH-CAPS): an element
//! [`MAX_LAYOUT_DEPTH`](crate::MAX_LAYOUT_DEPTH) below the root skips its
//! contents (`TuiExt::depth_capped`, read by
//! `content_visibility::skips_contents_for`), and nothing below it is
//! styled — so the walks, which recurse once per level, recurse at most
//! the cap deep, and every box-tree pass after them too.

use rdom_core::{Dom, NodeId};

use super::walk::{CounterState, Mode, Scratch, Sheets, SubtreeFlags, cascade_subtree};
use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// Cascade the subtree at `root` as [`cascade_subtree`](super::walk::cascade_subtree) does, from its
/// depth below the root — or, deeper than
/// [`MAX_LAYOUT_DEPTH`](crate::MAX_LAYOUT_DEPTH), leave it unstyled
/// (forgetting any style it brought along, a moved subtree's) without
/// recursing: it is below an element that skips its contents
/// (C16G-DEPTH-CAPS). The entry point of every walk but the full one.
pub(super) fn cascade_root<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    root: NodeId,
    parent_computed: &ComputedStyle,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
    mode: Mode,
) -> SubtreeFlags {
    let depth = depth_of(dom, root);
    if depth > crate::MAX_LAYOUT_DEPTH {
        unstyle_subtree(dom, root);
        return SubtreeFlags::default();
    }
    let outer = std::mem::replace(&mut scratch.depth, depth);
    let flags = cascade_subtree(dom, sheets, root, parent_computed, counters, scratch, mode);
    scratch.depth = outer;
    flags
}

/// How many nodes `id` is below the root. O(depth), iterative.
fn depth_of(dom: &Dom<TuiExt>, id: NodeId) -> usize {
    std::iter::successors(dom.node(id).parent_node().map(|p| p.id()), |&p| {
        dom.node(p).parent_node().map(|n| n.id())
    })
    .count()
}

/// Note whether the element `id`, `depth` below the root, is at the cap —
/// and if it is, forget its descendants' styles. `true` when it is.
pub(super) fn mark_depth(dom: &mut Dom<TuiExt>, id: NodeId, depth: usize) -> bool {
    let capped = depth >= crate::MAX_LAYOUT_DEPTH;
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.depth_capped = capped;
    }
    if capped {
        let children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
        for c in children {
            unstyle_subtree(dom, c);
        }
    }
    capped
}

/// Forget the styles of the elements in the subtree at `root`, all of
/// it: a styled subtree moved below the cap may sit under unstyled
/// elements, and the walk that reaches the cap may be a coalesced root's
/// above it. O(subtree), iterative — the price of a page deeper than the
/// cap, paid where its cascade reaches the cap.
fn unstyle_subtree(dom: &mut Dom<TuiExt>, root: NodeId) {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if let Some(ext) = dom.node_mut(id).ext_mut()
            && (ext.computed.is_some() || ext.depth_capped)
        {
            ext.forget_rendering(false);
            ext.depth_capped = false;
        }
        stack.extend(dom.node(id).child_nodes().map(|c| c.id()));
    }
}
