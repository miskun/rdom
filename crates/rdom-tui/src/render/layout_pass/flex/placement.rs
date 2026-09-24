//! Main-axis placement of sized flex items — CSS Flexible Box §9.5.
//!
//! Walks the line in order: resolves each item's `auto` main margins
//! from the leftover free space, positions the item at the running
//! cursor (offset by the container's scroll), asks [`super::cross`]
//! for its cross size and offset, recurses via `layout_node`, then
//! advances past the item, its end margin, the gap, and any collapsed
//! sibling border.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, MarginValue};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::{layout_node, parent_scroll};
use crate::style::ComputedStyle;

use super::collapse::SiblingOverlap;
use super::cross::{CrossPlacement, ResolvedMain, place_cross};
use super::main_axis::{ChildMain, MainNatural};

/// The main-axis free space split across the line's `auto` main
/// margins (§9.5): `share` cells each, with the first `remainder`
/// margins taking one extra cell so the totals add back up exactly.
pub(super) struct AutoMainMargins {
    pub(super) share: u16,
    pub(super) remainder: u32,
}

/// Everything [`place_items`] needs about a sized flex line.
pub(super) struct FlexLine<'a> {
    pub(super) items: &'a [ChildMain],
    /// Resolved main size per item, index-aligned with `items`.
    pub(super) final_main: &'a [u16],
    /// The (collapse-inset) container the items are placed in.
    pub(super) container: LayoutRect,
    pub(super) direction: Direction,
    pub(super) gap: u16,
    pub(super) cross_budget: u16,
    pub(super) auto_margins: AutoMainMargins,
    pub(super) overlap: SiblingOverlap,
}

/// Position each child along the main axis, scrolling by the parent's
/// scroll offset, and lay out each child at its final rect.
pub(super) fn place_items(dom: &mut Dom<TuiExt>, children: &[NodeId], line: FlexLine<'_>) {
    let FlexLine {
        items: child_info,
        final_main,
        container,
        direction,
        gap,
        cross_budget,
        auto_margins,
        overlap,
    } = line;

    let scroll_main = parent_scroll(dom, children, direction);
    // `SCROLL-CROSS-AXIS-1`: the container's other scroll offset moves
    // every item along the cross axis (a column container scrolling
    // horizontally).
    let scroll_cross = parent_scroll(
        dom,
        children,
        match direction {
            Direction::Row => Direction::Column,
            Direction::Column => Direction::Row,
        },
    );

    let mut main_cursor: i32 = match direction {
        Direction::Row => container.x - scroll_main,
        Direction::Column => container.y - scroll_main,
    };

    let child_list: Vec<(NodeId, u16)> = child_info
        .iter()
        .map(|ci| ci.id)
        .zip(final_main.iter().copied())
        .collect();

    // Distribute the remainder (from integer division of auto_share)
    // to the first few auto margins so the totals add back up exactly.
    let mut autos_consumed: u32 = 0;
    let resolve_auto = |consumed: &mut u32| -> u16 {
        let extra = if *consumed < auto_margins.remainder {
            1
        } else {
            0
        };
        *consumed += 1;
        auto_margins.share.saturating_add(extra)
    };

    for (i, (child_id, size)) in child_list.iter().enumerate() {
        let child_computed = dom
            .node(*child_id)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));

        // Resolve this child's main-axis start and end margins.
        // `Calc` was pre-resolved to `Cells` during `ChildMain`
        // construction (see `resolve_margin` in `main_axis`), so only
        // the `Cells | Auto` cases are reachable here.
        let main_start_cells: i32 = match &child_info[i].main_start_margin {
            MarginValue::Cells(n) => i32::from(*n),
            MarginValue::Auto => i32::from(resolve_auto(&mut autos_consumed)),
            MarginValue::Calc(_) => unreachable!("Calc pre-resolved to Cells"),
        };
        let main_end_cells: i32 = match &child_info[i].main_end_margin {
            MarginValue::Cells(n) => i32::from(*n),
            MarginValue::Auto => i32::from(resolve_auto(&mut autos_consumed)),
            MarginValue::Calc(_) => unreachable!("Calc pre-resolved to Cells"),
        };
        main_cursor = main_cursor.saturating_add(main_start_cells);

        // Whether the child's main-axis size was declared `Auto` —
        // needed so the cross resolver knows whether to apply
        // aspect-ratio (which requires the main axis to be explicit).
        let main_was_auto = matches!(child_info[i].main, MainNatural::Auto(_));

        let CrossPlacement {
            size: cross_size,
            offset: cross_offset,
        } = place_cross(
            dom,
            *child_id,
            &child_computed,
            container.width,
            cross_budget,
            direction,
            ResolvedMain {
                size: *size,
                was_auto: main_was_auto,
            },
        );

        let child_rect = match direction {
            Direction::Row => LayoutRect::new(
                main_cursor,
                container.y + cross_offset - scroll_cross,
                *size,
                cross_size,
            ),
            Direction::Column => LayoutRect::new(
                container.x + cross_offset - scroll_cross,
                main_cursor,
                cross_size,
                *size,
            ),
        };

        layout_node(dom, *child_id, child_rect, container.width);

        // Advance cursor past this child + main-end margin + gap.
        main_cursor = main_cursor.saturating_add(*size as i32);
        main_cursor = main_cursor.saturating_add(main_end_cells);
        if i + 1 < child_list.len() {
            main_cursor = main_cursor.saturating_add(gap as i32);
            // Sibling-overlap pullback. Mirrors the gating in the
            // `overlap_savings` computation: only fires when
            // gap == 0 AND parent has collapse AND both children
            // have a border on the shared edge. With gap > 0, the
            // gap is visible and the siblings don't overlap.
            if overlap.between(dom, child_info[i].id, child_info[i + 1].id) {
                main_cursor = main_cursor.saturating_sub(1);
            }
        }
    }
}
