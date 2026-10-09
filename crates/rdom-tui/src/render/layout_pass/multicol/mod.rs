//! Multi-column layout (CSS Multi-column Layout 1): a multi-column
//! container's content flows through column boxes side by side.
//!
//! The content is laid out once in a column as wide as one column box
//! (§3.4, `geometry`), as one tall column — the column boxes are its
//! fragmentainers (§1). The fragmentation engine (`fragment`) then picks
//! the breaks (§7: balanced over the column count — the least height
//! that fits, `column-fill: balance`, and always where the container's
//! height is not constrained — or each column filled in turn to the
//! constrained height, `column-fill: auto`) and moves each piece of the
//! flow into its column. Columns past the count overflow in the inline
//! direction (§8.2). The multi-column container keeps its column boxes
//! (`KeptLayout::Columns`) for the rules paint draws between them (§4).
//! A spanner (§6, `spanners`) splits the columns into sets: the content
//! before it balanced in its own row of columns, the spanner across the
//! content box, the content after it in the next row.
//!
//! A page without a multi-column container pays one `is_multicol` test per
//! laid-out block container.

pub(crate) mod geometry;
mod spanners;
#[cfg(test)]
mod tests;

#[cfg(test)]
thread_local! {
    /// Multi-column containers [`lay_out`] laid out (cost tests).
    pub(super) static MULTICOL_LAYOUTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

use rdom_core::{Dom, NodeId};

use super::block::BlockMeasurement;
use super::fragment::{self, Fill, Plan, Slice};
use crate::ext::{ColumnBox, ColumnSet, KeptLayout, TuiExt};
use crate::layout::{ColumnFill, Direction, LayoutRect, TextDirection};
use crate::style::ComputedStyle;

pub(crate) use geometry::is_multicol;

/// Lay out the multi-column container `id`'s content in its content box
/// `inner`: its children (or lines) in one column box's width, then
/// fragmented into the column boxes. Returns the content's height — the
/// columns' — which an `auto` height resolves to (§7.1).
pub(super) fn lay_out(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    inner: LayoutRect,
    computed: &ComputedStyle,
    containing_block_width: u16,
) -> Option<BlockMeasurement> {
    #[cfg(test)]
    MULTICOL_LAYOUTS.with(|c| c.set(c.get() + 1));
    let cols = geometry::columns(computed, inner.width);
    // The columns run from the inline-start edge (§2): the right one under
    // `rtl`.
    let rtl = computed.text_direction == TextDirection::Rtl;
    let first_x = if rtl {
        inner.x + i32::from(inner.width) - i32::from(cols.width)
    } else {
        inner.x
    };
    let column = LayoutRect::new(first_x, inner.y, cols.width, inner.height);
    // The column box is the content's containing block (§2): percentages
    // and positions resolve against it while the flow is laid out.
    set_content_box(dom, id, column);
    let measurement = super::dispatch::layout_children(dom, id, column, computed);
    let tall = measurement.map(|m| m.content_height).or_else(|| {
        dom.node(id)
            .ext()
            .and_then(|e| e.inline_layout.as_ref())
            .map(|il| il.height())
    });
    let tall = i32::from(tall.unwrap_or(0));
    let scroll = (
        super::gutter::scroll_offset(dom, id, Direction::Row),
        super::gutter::scroll_offset(dom, id, Direction::Column),
    );
    let origin = (first_x - scroll.0, inner.y - scroll.1);
    let breaks = fragment::collect(dom, id);
    let spanners = spanners::of(dom, id);
    let pieces = spanners::pieces(dom, &spanners, &breaks, origin.1, origin.1 + tall);
    let content_sized = super::auto_height::is_content_sized(dom, id, computed);
    let fill = fill(dom, id, computed, containing_block_width, inner, cols.count);
    let step = if rtl { -cols.pitch() } else { cols.pitch() };
    let mut plan = Plan { slices: Vec::new() };
    let mut sets = Vec::new();
    // Where the next set or spanner goes, in the final layout.
    let mut cursor = origin.1;
    let last = pieces.len().saturating_sub(1);
    for (k, piece) in pieces.iter().enumerate() {
        let (start, end) = piece.rows;
        let within: Vec<_> = breaks
            .iter()
            .filter(|b| b.end > start && b.end < end)
            .copied()
            .collect();
        // §7.1: a set before a spanner is balanced; the last fills as
        // `column-fill` says, in what height is left.
        let set_fill = match fill {
            Fill::Sequential(h) if k == last => Fill::Sequential(
                h.saturating_sub((cursor - origin.1).clamp(0, i32::from(u16::MAX)) as u16),
            ),
            Fill::Balance { count, cap } if k == last => Fill::Balance {
                count,
                cap: cap.map(|c| {
                    c.saturating_sub((cursor - origin.1).clamp(0, i32::from(u16::MAX)) as u16)
                }),
            },
            _ => Fill::Balance {
                count: cols.count,
                cap: None,
            },
        };
        let (rows, mut used) = fragment::fragmentainers(&within, start, end, set_fill);
        if let (Fill::Sequential(h), true) = (set_fill, content_sized) {
            // An `auto` height capped by `max-height`: as tall as the
            // fullest column.
            let fullest = rows.iter().map(|(a, b)| b - a).max().unwrap_or(0);
            used = fullest.clamp(0, i32::from(h)) as u16;
        }
        plan.slices
            .extend(rows.iter().enumerate().map(|(i, &(start, end))| Slice {
                start,
                end,
                dx: step.saturating_mul(i as i32),
                dy: cursor - start,
            }));
        let count = rows.len().max(usize::from(cols.count));
        sets.push(ColumnSet {
            top: cursor - origin.1,
            height: used,
            columns: (0..count)
                .map(|i| ColumnBox {
                    x: first_x - inner.x + step.saturating_mul(i as i32),
                    width: cols.width,
                    filled: rows.get(i).is_some_and(|(a, b)| b > a),
                })
                .collect(),
        });
        cursor += i32::from(used);
        // §6: a spanner is laid out across the whole content box, below
        // the set.
        if let Some(spanner) = piece.spanner {
            cursor = spanners::lay_out(dom, spanner, inner.x - scroll.0, cursor, inner.width);
        }
    }
    if plan.slices.is_empty() {
        plan.slices.push(Slice {
            start: origin.1,
            end: origin.1,
            dx: 0,
            dy: 0,
        });
    }
    set_content_box(dom, id, inner);
    fragment::apply(
        dom,
        id,
        &plan,
        origin,
        (inner.x - scroll.0, inner.y - scroll.1),
        &spanners,
    );
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.kept = Some(Box::new(KeptLayout::Columns(sets)));
    }
    Some(BlockMeasurement {
        content_height: (cursor - origin.1).clamp(0, i32::from(u16::MAX)) as u16,
    })
}

/// How the content fills the columns (§7.1): balanced over the count, no
/// taller than a constrained height — or, under `column-fill: auto` with a
/// constrained height, each column in turn to it. An `auto` height is
/// constrained by `max-height` only.
fn fill(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    inner: LayoutRect,
    count: u16,
) -> Fill {
    let available = if super::auto_height::is_content_sized(dom, id, computed) {
        let max = super::auto_height::used_content_height(
            dom,
            id,
            computed,
            containing_block_width,
            u16::MAX,
        );
        (max < u16::MAX).then_some(max)
    } else {
        Some(inner.height)
    };
    match (computed.multicol.column_fill, available) {
        (ColumnFill::Auto, Some(h)) => Fill::Sequential(h),
        (_, cap) => Fill::Balance { count, cap },
    }
}

fn set_content_box(dom: &mut Dom<TuiExt>, id: NodeId, rect: LayoutRect) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.content_layout = rect;
    }
}
