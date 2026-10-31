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
use crate::render::buffer::BorderLayer;

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
