//! Main-axis placement of sized flex items — CSS Flexible Box §9.5.
//!
//! Walks the line in order: resolves each item's `auto` main margins
//! from the leftover free space, positions the item at the running
//! cursor (offset by the container's scroll), asks [`super::cross`]
//! for its cross size and offset, recurses via `layout_node`, then
//! advances past the item, its end margin, the gap, and any collapsed
//! sibling border.

use rdom_core::Dom;

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Direction, LayoutRect, MarginValue};
use crate::render::layout_pass::gutter::scroll_offset;
use crate::render::layout_pass::layout_node;

use super::item::FlexItem;

use super::collapse::SiblingOverlap;
use super::cross::{CrossPlacement, CrossSpace, ResolvedMain, place_cross};
use super::main_axis::ChildMain;
use crate::render::layout_pass::margin_trim::FlexTrim;

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
    /// `justify-content`'s extra space before each item (index-aligned).
    pub(super) justify: &'a [i32],
    /// Each item's cross-axis alignment (index-aligned).
    pub(super) align: &'a [super::align::ItemAlign],
    /// The (collapse-inset) container the items are placed in.
    pub(super) container: LayoutRect,
    pub(super) direction: Direction,
    pub(super) gap: u16,
    /// The line's cross size and the container's inner cross size.
    pub(super) space: CrossSpace,
    /// The line's cross-start offset from the container's cross-start
    /// edge (in the frame mirrored on a flipped cross axis).
    pub(super) line_offset: i32,
    pub(super) auto_margins: AutoMainMargins,
    pub(super) overlap: SiblingOverlap,
    /// The line's items' cross-axis `margin-trim` (CSS Box 4 §3.2): a
    /// single-line container trims every item's, a multi-line one its
    /// first line's cross-start and its last line's cross-end margins;
    /// the main-axis half was applied to the items' margins already.
    pub(super) trim: FlexTrim,
    /// The axes that run from their physical end: the items were placed
    /// in a frame mirrored on them and are flipped back across it.
    pub(super) flip: super::AxisFlip,
}

/// Position each item along the main axis, scrolling by the parent's
/// scroll offset, and lay out each item at its final rect — an element
/// through `layout_node`, an anonymous item into `anonymous` (its
/// container's anonymous block boxes).
pub(super) fn place_items(
    dom: &mut Dom<TuiExt>,
    line: FlexLine<'_>,
    anonymous: &mut Vec<AnonymousIfc>,
) {
    let FlexLine {
        items: child_info,
        final_main,
        justify,
        align,
        container,
        direction,
        gap,
        space,
        line_offset,
        auto_margins,
        overlap,
        trim,
        flip,
    } = line;

    // The container's scroll offsets: along the main axis, and
    // (`SCROLL-CROSS-AXIS-1`) along the cross axis, which moves every
    // item there (a column container scrolling horizontally).
    let container_id = child_info.first().and_then(|ci| ci.item.box_parent(dom));
    let scroll = |axis| container_id.map_or(0, |c| scroll_offset(dom, c, axis));
    let scroll_main = scroll(direction);
    let scroll_cross = scroll(match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    });

    let mut main_cursor: i32 = match direction {
        Direction::Row => container.x - scroll_main,
        Direction::Column => container.y - scroll_main,
    };

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

    for (i, (ci, size)) in child_info.iter().zip(final_main).enumerate() {
        let child_computed = ci.item.computed(dom);

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
        main_cursor = main_cursor
            .saturating_add(justify[i])
            .saturating_add(main_start_cells);

        // Whether the child's main-axis size was declared `Auto` —
        // needed so the cross resolver knows whether to apply
        // aspect-ratio (which requires the main axis to be explicit).
        let main_was_auto = child_info[i].main_auto;

        let CrossPlacement {
            size: cross_size,
            offset: cross_offset,
        } = place_cross(
            dom,
            &ci.item,
            &child_computed,
            container.width,
            space,
            direction,
            ResolvedMain {
                size: *size,
                was_auto: main_was_auto,
                trim_cross_start: trim.cross_start,
                trim_cross_end: trim.cross_end,
                mirror: flip.cross,
            },
            align[i],
        );

        let child_rect = match direction {
            Direction::Row => LayoutRect::new(
                main_cursor,
                container.y + line_offset + cross_offset - scroll_cross,
                *size,
                cross_size,
            ),
            Direction::Column => LayoutRect::new(
                container.x + line_offset + cross_offset - scroll_cross,
                main_cursor,
                cross_size,
                *size,
            ),
        };

        // Flip each mirrored axis back across the container: the
        // horizontal one by its `scrollLeft`, the vertical by `scrollTop`.
        let (flip_x, flip_y) = match direction {
            Direction::Row => (flip.main, flip.cross),
            Direction::Column => (flip.cross, flip.main),
        };
        let (scroll_x, scroll_y) = match direction {
            Direction::Row => (scroll_main, scroll_cross),
            Direction::Column => (scroll_cross, scroll_main),
        };
        let mut child_rect = child_rect;
        if flip_x {
            child_rect.x = mirror_x(child_rect.x, child_rect.width, container, scroll_x);
        }
        if flip_y {
            child_rect.y = mirror_y(child_rect.y, child_rect.height, container, scroll_y);
        }
        match &ci.item {
            FlexItem::Element(id) => layout_node(dom, *id, child_rect, container.width),
            FlexItem::Anonymous(anon) => anonymous.push(anon.lay_out(dom, child_rect)),
        }

        // Advance cursor past this child + main-end margin + gap.
        main_cursor = main_cursor.saturating_add(*size as i32);
        main_cursor = main_cursor.saturating_add(main_end_cells);
        if i + 1 < child_info.len() {
            main_cursor = main_cursor.saturating_add(gap as i32);
            // Sibling-overlap pullback. Mirrors the gating in the
            // `overlap_savings` computation: only fires when
            // gap == 0 AND parent has collapse AND both children
            // have a border on the shared edge. With gap > 0, the
            // gap is visible and the siblings don't overlap.
            if overlap.between(dom, &child_info[i].item, &child_info[i + 1].item) {
                main_cursor = main_cursor.saturating_sub(1);
            }
        }
    }
}

/// `x` (a box `width` wide, scrolled left by `scroll_x`) mirrored across
/// `container`: the inline axis of an `rtl` container runs right to
/// left (CSS Writing Modes 4 §2.1).
fn mirror_x(x: i32, width: u16, container: LayoutRect, scroll_x: i32) -> i32 {
    let from_start = x + scroll_x - container.x;
    container.x + i32::from(container.width) - from_start - i32::from(width) - scroll_x
}

/// `y` (a box `height` tall, scrolled up by `scroll_y`) mirrored across
/// `container`: a `column-reverse` main axis runs bottom to top (CSS
/// Flexbox §5.1).
fn mirror_y(y: i32, height: u16, container: LayoutRect, scroll_y: i32) -> i32 {
    let from_start = y + scroll_y - container.y;
    container.y + i32::from(container.height) - from_start - i32::from(height) - scroll_y
}
