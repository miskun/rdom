//! Spanners (CSS Multi-column 1 §6): a `column-span: all` child of a
//! multi-column container spans all its columns, splitting its content into
//! the column sets before and after it (§6.1).
//!
//! rdom honours `column-span: all` on the container's in-flow block-level
//! children; deeper in the flow, where the spanner's ancestors would have to
//! be split around it, the box is laid out in its column, as `none`
//! (DIVERGENCES).

use rdom_core::{Dom, NodeId};

use super::super::fragment::Break;
use crate::ext::TuiExt;
use crate::layout::ColumnSpan;
use crate::node::TuiNodeExt;

/// The spanners of the multi-column container `id`, in flow order.
pub(super) fn of(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    if dom
        .node(id)
        .ext()
        .is_some_and(|e| e.inline_layout.is_some())
    {
        return Vec::new();
    }
    let mut out: Vec<NodeId> = super::super::element_children_of(dom, id)
        .into_iter()
        .filter(|&c| {
            super::super::is_in_flow(dom, c)
                && super::super::block::is_block_level(dom, c)
                && dom
                    .node(c)
                    .tui_ext()
                    .and_then(|e| e.computed.as_deref())
                    .is_some_and(|s| s.multicol.column_span == ColumnSpan::All)
        })
        .collect();
    out.sort_by_key(|&c| dom.node(c).tui_ext().map_or(0, |e| e.layout.y));
    out
}

/// A run of the flow between spanners: its rows in the one-column layout,
/// and the spanner after it (`None` for the last run).
pub(super) struct Piece {
    pub(super) rows: (i32, i32),
    pub(super) spanner: Option<NodeId>,
}

/// The runs of the flow `start .. end` the `spanners` split it into, cut at
/// the breaks before and after each (`breaks`): a run ends where the
/// content before its spanner does, and the next starts where the content
/// after it does — the margins around a spanner truncated, as at a break.
pub(super) fn pieces(
    dom: &Dom<TuiExt>,
    spanners: &[NodeId],
    breaks: &[Break],
    start: i32,
    end: i32,
) -> Vec<Piece> {
    let mut out = Vec::with_capacity(spanners.len() + 1);
    let mut s = start;
    for &spanner in spanners {
        let Some(r) = dom.node(spanner).tui_ext().map(|e| e.layout) else {
            continue;
        };
        let before = breaks
            .iter()
            .find(|b| b.resume == r.y)
            .map_or(r.y, |b| b.end)
            .max(s);
        out.push(Piece {
            rows: (s, before),
            spanner: Some(spanner),
        });
        let bottom = r.y + i32::from(r.height);
        s = breaks
            .iter()
            .find(|b| b.end == bottom)
            .map_or(bottom, |b| b.resume);
    }
    out.push(Piece {
        rows: (s, end.max(s)),
        spanner: None,
    });
    out
}

/// Lay the spanner out across the content box `width` cells wide whose
/// left edge is `x`, below `y` (its margins outside it): the row after its
/// bottom margin.
pub(super) fn lay_out(dom: &mut Dom<TuiExt>, spanner: NodeId, x: i32, y: i32, width: u16) -> i32 {
    let (rect, bottom_margin) = super::super::block::place_alone(dom, spanner, x, y, width);
    super::super::layout_node(dom, spanner, rect, width);
    let laid = dom.node(spanner).tui_ext().map_or(rect, |e| e.layout);
    laid.y + i32::from(laid.height) + bottom_margin
}
