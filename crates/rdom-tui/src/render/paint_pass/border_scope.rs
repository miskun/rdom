//! Who a border belongs to, for the border table (LAYER-BORDERS): the
//! stacking context it paints in, and the collapse group whose shared
//! edges it resolves with.
//!
//! - **Layer.** Borders meet and join only inside one stacking context;
//!   a later context covers what an earlier one left (CSS 2.1 Appendix
//!   E paints each context atomically; `buffer::layering`). The paint
//!   walk enters the layer of each context it paints
//!   ([`context_layer`]).
//! - **Collapse group.** A collapsing container's shared edges resolve
//!   as one, by CSS Tables 3 §11.5 — a table's grid, or a `border-collapse:
//!   collapse` container and its direct children (BORDER-MODEL-1). A
//!   member of a group stays in its group's layer even where it roots a
//!   stacking context of its own (a z-indexed cell): its edges are the
//!   group's, resolved there, as a browser paints a collapsed table's
//!   borders with the table.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{BorderCollapse, Display, Flow};
use crate::node::TuiNodeExt;
use crate::render::buffer::{BorderContribution, BorderGroup, BorderLayer};

/// Who a box's border contributions are from: their CSS Tables 3 §11.5
/// structural priority within their collapse group, and the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BorderOrigin {
    /// §11.5 rules 5–6: the more nested, then the earlier box wins.
    pub(super) priority: u64,
    /// The collapse group the contributions resolve in.
    pub(super) group: BorderGroup,
}

/// The origin of the element `id`'s border contributions — its border,
/// its column rules.
///
/// - **Priority** (§11.5 rules 5 + 6): deeper elements outrank their
///   ancestors, so the depth counts; within one depth the earlier in the
///   document wins, by `NodeId` order — nodes are created in monotonic
///   order, which for a tree built top-down matches leftmost / topmost
///   geometric order (re-parented elements and mixed creation order are
///   accepted simplifications, DIVERGENCES).
/// - **Group**: the top of the chain of collapse groups `id` shares its
///   edges in ([`shares_collapsed_borders`]).
pub(super) fn origin_of(dom: &Dom<TuiExt>, id: NodeId) -> BorderOrigin {
    BorderOrigin {
        priority: BorderContribution::pack_priority(depth_of(dom, id), id.as_u32()),
        group: group_of(dom, id),
    }
}

/// The origin of a generated box's border: one level below its `host`,
/// as a child box, in a group of its own (`slot`, its pseudo-element).
pub(super) fn generated_origin(dom: &Dom<TuiExt>, host: NodeId, slot: u8) -> BorderOrigin {
    BorderOrigin {
        priority: BorderContribution::pack_priority(
            depth_of(dom, host).saturating_add(1),
            host.as_u32(),
        ),
        group: BorderGroup::of_pseudo(host, slot),
    }
}

/// The collapse group of `id`'s borders: the box at the top of the
/// chain of collapse groups it shares its edges in, or `id` itself.
pub(super) fn group_of(dom: &Dom<TuiExt>, id: NodeId) -> BorderGroup {
    let mut root = id;
    while shares_collapsed_borders(dom, root) {
        match crate::render::box_tree::box_parent(dom, root) {
            Some(parent) => root = parent,
            None => break,
        }
    }
    BorderGroup::of_element(root)
}

/// How far below the document root `id` sits, in the slot-aware tree.
fn depth_of(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    let mut depth: u16 = 0;
    let mut cur = crate::render::box_tree::slot::parent(dom, id);
    while let Some(node) = cur {
        depth = depth.saturating_add(1);
        cur = crate::render::box_tree::slot::parent(dom, node);
    }
    depth
}

/// The layer the stacking context rooted at `root` paints its borders
/// in: the document's for the document root, `root`'s own for any other
/// box — or `None`, the layer it is painted from, for a member of a
/// collapse group ([`shares_collapsed_borders`]).
pub(super) fn context_layer(dom: &Dom<TuiExt>, root: NodeId) -> Option<BorderLayer> {
    if dom.node(root).node_type() == NodeType::Fragment {
        return Some(BorderLayer::ROOT);
    }
    if shares_collapsed_borders(dom, root) {
        return None;
    }
    Some(BorderLayer::of_element(root))
}

/// True when the element `id` shares its edges with its box parent's
/// collapse group: a table part of a collapsing table, or a direct child
/// of a `border-collapse: collapse` container.
pub(super) fn shares_collapsed_borders(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let Some(parent) = crate::render::box_tree::box_parent(dom, id) else {
        return false;
    };
    let Some(c) = dom.node(id).computed() else {
        return false;
    };
    if matches!(c.display, Display::TablePart(_)) {
        return in_collapsing_table(dom, parent);
    }
    dom.node(parent)
        .computed()
        .is_some_and(|p| p.border_collapse == BorderCollapse::Collapse && p.flow != Flow::Table)
}

/// True when `id` — the box parent of a table part — is a collapsing
/// table, or a table part inside one.
fn in_collapsing_table(dom: &Dom<TuiExt>, mut id: NodeId) -> bool {
    loop {
        let Some(c) = dom.node(id).computed() else {
            return false;
        };
        if c.flow == Flow::Table {
            return c.border_collapse == BorderCollapse::Collapse;
        }
        if !matches!(c.display, Display::TablePart(_)) {
            return false;
        }
        let Some(parent) = crate::render::box_tree::box_parent(dom, id) else {
            return false;
        };
        id = parent;
    }
}
