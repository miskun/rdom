//! Tree helpers of the layout pass: the element children a container
//! lays out (fragments and `display: contents` elements unwrapped), the
//! in-flow predicate, and the geometry reset of `display: none`
//! subtrees and box-less elements.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;

/// Direct *element* children of `id`, document order. Text/Comment
/// are skipped (they have no TuiExt and flow inline via intrinsic
/// measurement). Fragment children and `display: contents` children
/// (CSS Display 3 §2.5: no box of their own) are unwrapped — their
/// element children are returned as if they were direct children of
/// `id`.
pub(crate) fn element_children_of(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    collect_element_children(dom, id, &mut out);
    out
}

/// True iff `id` participates in normal flow. Non-elements (text, comments,
/// fragments) always do; an element does when it's neither `display: none` nor
/// out-of-flow positioned (`absolute` / `fixed`). The single source of truth
/// for the "skip out-of-flow children" filter shared by block + flex layout and
/// the scroll-content walk (`DRY-1`), by the margin-collapse predicates, intrinsic sizing, paint and hit-test.
pub(crate) fn is_in_flow(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    if node.node_type() != NodeType::Element {
        return true; // text, comments, fragments
    }
    let Some(c) = node.ext().and_then(|e| e.computed.as_ref()) else {
        return true;
    };
    use crate::layout::{Display, Position};
    // A box-less element has no box to take out of flow (CSS Display 3
    // §2.5): its `position` applies to nothing.
    c.display == Display::Contents
        || (c.display != Display::None
            && !matches!(c.position, Position::Absolute | Position::Fixed)
            && !is_collapsed_table_row(dom, id))
}

/// A `visibility: collapse` table row (CSS 2.1 §17.5.5): removed from
/// the table's flow — the rows close up — while its cells still size
/// the columns (`runtime::builtins::table` sizes them from every row).
/// rdom's table rows are the `<tr>` children of a `<table>` or its row
/// groups (DIVERGENCES: tables are flex rows).
pub(crate) fn is_collapsed_table_row(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    node.tag_name() == Some("tr")
        && node
            .ext()
            .and_then(|e| e.computed.as_ref())
            .is_some_and(|c| c.visibility == crate::layout::Visibility::Collapse)
        && node
            .parent_node()
            .is_some_and(|p| matches!(p.tag_name(), Some("table" | "thead" | "tbody" | "tfoot")))
}

/// Zero the layout geometry of every `display:none` child subtree of `id`.
/// In-flow layout filters `display:none` children out, so they'd otherwise
/// retain the rect from when they were last visible (LAYOUT-DISPLAY-NONE-STALE-
/// RECT). A `display:none` box generates no box, so its rect — and every
/// descendant's, since the subtree isn't laid out — must read zero.
///
/// A `display: contents` child has no box either (CSS Display 3 §2.5):
/// its own rects read zero, at the container's content origin, while
/// its children are laid out as the container's.
pub(crate) fn collapse_hidden_children(dom: &mut Dom<TuiExt>, id: NodeId) {
    let origin = dom.node(id).ext().map(|e| e.content_layout);
    zero_contents_children(dom, id, origin.unwrap_or_default());
    for child in element_children_of(dom, id) {
        let hidden = dom
            .node(child)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|c| c.display == crate::layout::Display::None)
            .unwrap_or(false)
            || is_collapsed_table_row(dom, child);
        if hidden {
            collapse_subtree_geometry(dom, child);
        }
    }
}

/// Recursively reset `layout` / `content_layout` to the zero rect for `id` and
/// every element descendant. Used to collapse a `display:none` subtree.
fn collapse_subtree_geometry(dom: &mut Dom<TuiExt>, id: NodeId) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        if ext.layout == LayoutRect::default() && ext.content_layout == LayoutRect::default() {
            // Already collapsed — and so is everything below it (we always zero
            // top-down), so stop early. Keeps steady-state hidden subtrees O(1).
            return;
        }
        ext.layout = LayoutRect::default();
        ext.content_layout = LayoutRect::default();
    }
    for child in element_children_of(dom, id) {
        collapse_subtree_geometry(dom, child);
    }
}

/// Give every `display: contents` child of `id` (through nested ones) a
/// zero rect at `origin`.
fn zero_contents_children(dom: &mut Dom<TuiExt>, id: NodeId, origin: LayoutRect) {
    let children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
    for child in children {
        if crate::render::box_tree::is_contents(dom, child) {
            if let Some(ext) = dom.node_mut(child).ext_mut() {
                ext.layout = LayoutRect::new(origin.x, origin.y, 0, 0);
                ext.content_layout = ext.layout;
                ext.layout_dirty = false;
                ext.margin_chain = None;
            }
            zero_contents_children(dom, child, origin);
        }
    }
}

fn collect_element_children(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element if crate::render::box_tree::is_contents(dom, child.id()) => {
                collect_element_children(dom, child.id(), out)
            }
            NodeType::Element => out.push(child.id()),
            NodeType::Fragment => collect_element_children(dom, child.id(), out),
            // Text, comments, and any later node kind (`NodeType` is
            // `#[non_exhaustive]`) are not element children.
            _ => {}
        }
    }
}
